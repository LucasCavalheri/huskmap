use std::collections::BTreeSet;

use crate::clock::{Clock, Millis, SystemClock};
use crate::domain::{
    ActionKind, Fingerprint, Husk, HuskKind, PLAN_VERSION, Plan, PlanAction, PlanPreset, Risk,
    ScanReport,
};
use crate::ids::HuskId;

const DAY_MS: Millis = 86_400_000;

fn action_kind(husk: &Husk) -> ActionKind {
    if husk.kind == HuskKind::Worktree && husk.git.is_some() {
        ActionKind::GitWorktreeRemove
    } else {
        ActionKind::Trash
    }
}

/// Can this husk go at all? `force` lifts dirty/stranded/locked, never occupied/primary/secrets.
pub fn admissible(husk: &Husk, force: bool) -> bool {
    if husk.contains_secrets || husk.live || husk.risk == Risk::Forbidden {
        return false;
    }
    if husk.git.as_ref().is_some_and(|g| g.is_primary) {
        return false;
    }
    if husk.wards.iter().any(|w| w.absolute()) {
        return false;
    }
    if force {
        return true;
    }
    husk.reclaimable && husk.risk != Risk::Dangerous
}

fn preset_takes(husk: &Husk, preset: PlanPreset, now_ms: Millis) -> bool {
    match preset {
        PlanPreset::Safe => husk.risk == Risk::Safe,
        PlanPreset::AgentOnly => husk.agent.is_some(),
        PlanPreset::Older { days } => husk
            .age_ms(now_ms)
            .is_some_and(|age| age >= days.saturating_mul(DAY_MS)),
        PlanPreset::Marked => true,
    }
}

fn build(
    report: &ScanReport,
    preset: PlanPreset,
    now: Millis,
    take: impl Fn(&Husk) -> bool,
) -> Plan {
    let mut actions: Vec<PlanAction> = report
        .husks
        .iter()
        .filter(|h| take(h))
        .map(|husk| PlanAction {
            husk_id: husk.id.clone(),
            path: husk.path.clone(),
            kind: action_kind(husk),
            fingerprint: Fingerprint::from_husk(husk),
        })
        .collect();
    // A parent in the plan already takes its children with it.
    let paths: Vec<_> = actions.iter().map(|a| a.path.clone()).collect();
    actions.retain(|action| {
        !paths
            .iter()
            .any(|other| other != &action.path && action.path.starts_with(other))
    });
    actions.sort_by(|a, b| a.path.cmp(&b.path));
    Plan {
        version: PLAN_VERSION,
        created_at_ms: now,
        preset,
        roots: report.roots.clone(),
        actions,
    }
}

pub fn plan_with_clock(report: &ScanReport, preset: PlanPreset, clock: &dyn Clock) -> Plan {
    let now = clock.now_ms();
    build(report, preset, now, |h| {
        admissible(h, false) && preset_takes(h, preset, now)
    })
}

pub fn plan(report: &ScanReport, preset: PlanPreset) -> Plan {
    plan_with_clock(report, preset, &SystemClock)
}

/// Exactly the marked husks that survive the safety rules.
pub fn plan_marked(
    report: &ScanReport,
    marked: &BTreeSet<HuskId>,
    force: bool,
    clock: &dyn Clock,
) -> Plan {
    build(report, PlanPreset::Marked, clock.now_ms(), |h| {
        marked.contains(&h.id) && admissible(h, force)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::FrozenClock;
    use crate::domain::{AgentKind, GitFacts, Holder, Ward};
    use std::path::PathBuf;

    fn husk(kind: HuskKind, path: &str, risk: Risk) -> Husk {
        let mut h = Husk::bare(kind, path, 100);
        h.risk = risk;
        h.mtime_ms = Some(1_000);
        h.agent = Some(AgentKind::Claude);
        h
    }

    fn report(husks: Vec<Husk>) -> ScanReport {
        let mut r = ScanReport::empty(PathBuf::from("/home/me"), 10_000);
        r.roots = vec![PathBuf::from("/home/me")];
        r.husks = husks;
        r
    }

    fn clock() -> FrozenClock {
        FrozenClock::from_millis(10_000)
    }

    #[test]
    fn safe_takes_only_safe_reclaimable() {
        let mut blocked = husk(HuskKind::Ballast, "/b", Risk::Safe);
        blocked.reclaimable = false;
        let r = report(vec![
            husk(HuskKind::Cache, "/c", Risk::Safe),
            husk(HuskKind::Afterimage, "/a", Risk::Caution),
            blocked,
        ]);
        let p = plan_with_clock(&r, PlanPreset::Safe, &clock());
        assert_eq!(p.actions.len(), 1);
        assert_eq!(p.actions[0].path, PathBuf::from("/c"));
        assert_eq!(p.actions[0].kind, ActionKind::Trash);
        assert_eq!(p.reclaimable_bytes(), 100);
        assert_eq!(p.created_at_ms, 10_000);
    }

    #[test]
    fn agent_only_and_worktree_action_kind() {
        let mut no_agent = husk(HuskKind::Ballast, "/nm", Risk::Safe);
        no_agent.agent = None;
        let mut wt = husk(HuskKind::Worktree, "/wt", Risk::Caution);
        wt.git = Some(GitFacts::default());
        let slot = husk(HuskKind::Worktree, "/slot", Risk::Caution);
        let r = report(vec![wt, slot, no_agent]);
        let p = plan_with_clock(&r, PlanPreset::AgentOnly, &clock());
        let kinds: Vec<_> = p.actions.iter().map(|a| (a.path.clone(), a.kind)).collect();
        assert_eq!(
            kinds,
            vec![
                (PathBuf::from("/slot"), ActionKind::Trash),
                (PathBuf::from("/wt"), ActionKind::GitWorktreeRemove),
            ]
        );
    }

    #[test]
    fn older_uses_mtime() {
        let mut old = husk(HuskKind::Cache, "/old", Risk::Safe);
        old.mtime_ms = Some(0);
        let mut young = husk(HuskKind::Cache, "/new", Risk::Safe);
        young.mtime_ms = Some(DAY_MS * 2);
        let mut none = husk(HuskKind::Cache, "/none", Risk::Safe);
        none.mtime_ms = None;
        let r = report(vec![old, young, none]);
        let p = plan_with_clock(
            &r,
            PlanPreset::Older { days: 1 },
            &FrozenClock::from_millis(DAY_MS * 2),
        );
        assert_eq!(p.actions.len(), 1);
        assert_eq!(p.actions[0].path, PathBuf::from("/old"));
    }

    #[test]
    fn admissible_table() {
        let base = || husk(HuskKind::Worktree, "/w", Risk::Caution);
        assert!(admissible(&base(), false));
        let mut secret = base();
        secret.contains_secrets = true;
        let mut live = base();
        live.live = true;
        let mut forbidden = base();
        forbidden.risk = Risk::Forbidden;
        let mut primary = base();
        primary.git = Some(GitFacts {
            is_primary: true,
            ..Default::default()
        });
        let mut occupied = base();
        occupied.ward(Ward::Occupied {
            holders: vec![Holder {
                pid: 1,
                name: "claude".into(),
                cwd: PathBuf::from("/w"),
                agent: Some(AgentKind::Claude),
            }],
        });
        for h in [&secret, &live, &forbidden, &primary, &occupied] {
            assert!(!admissible(h, false));
            assert!(!admissible(h, true), "force never lifts absolute guards");
        }
        let mut dirty = base();
        dirty.ward(Ward::Dirty { files: 2 });
        let mut stranded = base();
        stranded.ward(Ward::Stranded { commits: 3 });
        for h in [&dirty, &stranded] {
            assert!(!admissible(h, false));
            assert!(admissible(h, true));
        }
    }

    #[test]
    fn marked_plan_respects_marks_and_force() {
        let a = husk(HuskKind::Cache, "/a", Risk::Safe);
        let mut b = husk(HuskKind::Worktree, "/b", Risk::Caution);
        b.ward(Ward::Stranded { commits: 1 });
        let c = husk(HuskKind::Cache, "/c", Risk::Safe);
        let marked: BTreeSet<HuskId> = [a.id.clone(), b.id.clone()].into_iter().collect();
        let r = report(vec![a, b, c]);
        let p = plan_marked(&r, &marked, false, &clock());
        assert_eq!(p.actions.len(), 1);
        assert_eq!(p.preset, PlanPreset::Marked);
        let p = plan_marked(&r, &marked, true, &clock());
        assert_eq!(p.actions.len(), 2);
    }

    #[test]
    fn nested_child_dropped_when_parent_planned() {
        let parent = husk(HuskKind::Worktree, "/wt", Risk::Safe);
        let child = husk(HuskKind::Ballast, "/wt/node_modules", Risk::Safe);
        let r = report(vec![child, parent]);
        let p = plan_with_clock(&r, PlanPreset::Safe, &clock());
        assert_eq!(p.actions.len(), 1);
        assert_eq!(p.actions[0].path, PathBuf::from("/wt"));
    }

    #[test]
    fn plan_wrapper_uses_system_clock() {
        let p = plan(&report(vec![]), PlanPreset::Safe);
        assert_eq!(p.preset, PlanPreset::Safe);
        assert_eq!(p.version, PLAN_VERSION);
        assert!(p.created_at_ms > 0);
    }
}
