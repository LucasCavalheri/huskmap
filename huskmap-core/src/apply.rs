use std::path::{Path, PathBuf};

use crate::copy;
use crate::domain::{
    ActionKind, AppliedAction, ApplyResult, Fingerprint, GitFacts, PLAN_VERSION, Plan, PlanAction,
    SkippedAction,
};
use crate::error::Error;
use crate::git::{GitProbe, inspect_tree};
use crate::process::{HolderIndex, ProcessProbe};
use crate::reclaim::Reclaimer;
use crate::safety::{is_forbidden_path, is_within_roots};
use crate::secrets::guarding_secrets;
use crate::size::dir_size;

pub struct ApplyOptions<'a> {
    pub git: &'a dyn GitProbe,
    pub reclaimer: &'a dyn Reclaimer,
    pub processes: &'a dyn ProcessProbe,
    /// Lifts dirty / stranded / locked. Never lifts occupied, primary, secrets, outside-roots.
    pub force: bool,
}

struct Current {
    fingerprint: Fingerprint,
    git: Option<GitFacts>,
}

fn current(action: &PlanAction, git: &dyn GitProbe) -> Result<Option<Current>, Error> {
    let path = action.path.as_path();
    if !path.exists() {
        return Ok(None);
    }
    let size = dir_size(path)?;
    let facts = match action.kind {
        ActionKind::GitWorktreeRemove => inspect_tree(git, path)?,
        ActionKind::Trash => None,
    };
    Ok(Some(Current {
        fingerprint: Fingerprint {
            size_bytes: size.bytes,
            mtime_ms: size.mtime_ms,
            dirty: facts.as_ref().is_some_and(|g| g.dirty),
            unpushed: facts.as_ref().is_some_and(|g| g.unpushed),
            is_primary: facts.as_ref().is_some_and(|g| g.is_primary),
            contains_secrets: is_forbidden_path(path)
                || !guarding_secrets(action.husk_id.kind, path, &size.secrets, facts.as_ref())
                    .is_empty(),
        },
        git: facts,
    }))
}

fn drifted(expected: &Fingerprint, actual: &Fingerprint) -> Option<&'static str> {
    if expected.size_bytes != actual.size_bytes {
        return Some("size drifted");
    }
    if expected.mtime_ms != actual.mtime_ms {
        return Some("mtime drifted");
    }
    if expected.dirty != actual.dirty {
        return Some("git dirty state drifted");
    }
    if expected.unpushed != actual.unpushed {
        return Some("git stranded state drifted");
    }
    if expected.contains_secrets != actual.contains_secrets {
        return Some("secret marker drifted");
    }
    None
}

fn refuse(
    path: &Path,
    now: &Current,
    holders: &HolderIndex,
    roots: &[PathBuf],
    force: bool,
) -> Option<String> {
    let deck = copy::get();
    let fp = &now.fingerprint;
    if fp.contains_secrets {
        return Some(deck.forbidden_path.into());
    }
    if !is_within_roots(path, roots) {
        return Some(deck.outside_roots.into());
    }
    if fp.is_primary {
        return Some(deck.primary_checkout.into());
    }
    let inside = holders.inside(path);
    if let Some(h) = inside.first() {
        return Some(deck.occupied_refuse.replace("{who}", &h.label()));
    }
    if !force && fp.dirty {
        return Some(deck.dirty_worktree.into());
    }
    if !force && fp.unpushed {
        return Some(deck.unpushed_worktree.into());
    }
    if !force && now.git.as_ref().is_some_and(|g| g.locked) {
        return Some(deck.worktree_locked.into());
    }
    None
}

fn reclaim(action: &PlanAction, now: &Current, opts: &ApplyOptions<'_>, result: &mut ApplyResult) {
    let path = &action.path;
    if let Err(err) = opts.reclaimer.send_to_trash(path) {
        result.errors.push(format!("{}: {err}", path.display()));
        return;
    }
    result.applied.push(AppliedAction {
        path: path.clone(),
        kind: action.kind,
        bytes: now.fingerprint.size_bytes,
    });
    if action.kind != ActionKind::GitWorktreeRemove {
        return;
    }
    if let Some(facts) = now.git.as_ref().filter(|g| g.is_worktree) {
        let checkout = if facts.checkout.as_os_str().is_empty() {
            path.as_path()
        } else {
            facts.checkout.as_path()
        };
        if let Err(err) = opts.git.prune_worktree(&facts.common_dir, checkout) {
            tracing::warn!(error = %err, path = %path.display(), "worktree trashed but git admin entry not pruned");
            result.errors.push(format!(
                "{}: {} ({err})",
                path.display(),
                copy::get().prune_failed
            ));
        }
    }
}

/// Re-validate every action, then mutate. A plan is stale if size/mtime/git drifted.
pub fn apply(plan: &Plan, opts: &ApplyOptions<'_>) -> Result<ApplyResult, Error> {
    if plan.version != PLAN_VERSION {
        return Err(Error::Version {
            kind: "plan",
            found: plan.version,
            want: PLAN_VERSION,
        });
    }
    let home = std::env::temp_dir();
    let holders = HolderIndex::new(opts.processes.snapshot(), &home);
    let mut result = ApplyResult::default();
    for action in &plan.actions {
        let path = &action.path;
        let skip = |result: &mut ApplyResult, reason: String| {
            result.skipped.push(SkippedAction {
                path: path.clone(),
                reason,
            })
        };
        if is_forbidden_path(path) {
            skip(&mut result, copy::get().forbidden_path.into());
            continue;
        }
        let now = match current(action, opts.git) {
            Ok(Some(now)) => now,
            Ok(None) => {
                skip(&mut result, copy::get().already_gone.into());
                continue;
            }
            Err(err) => {
                result.errors.push(format!("{}: {err}", path.display()));
                continue;
            }
        };
        if let Some(why) = drifted(&action.fingerprint, &now.fingerprint) {
            skip(&mut result, format!("{}: {why}", copy::get().stale_plan));
            continue;
        }
        if let Some(why) = refuse(path, &now, &holders, &plan.roots, opts.force) {
            skip(&mut result, why);
            continue;
        }
        reclaim(action, &now, opts, &mut result);
    }
    Ok(result)
}

pub fn apply_or_refuse_empty_plan(
    plan: &Plan,
    opts: &ApplyOptions<'_>,
) -> Result<ApplyResult, Error> {
    if plan.actions.is_empty() {
        return Err(Error::safety(copy::get().nothing_to_apply));
    }
    apply(plan, opts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AgentKind, Holder, HuskKind, PlanPreset};
    use crate::git::MapGitProbe;
    use crate::ids::HuskId;
    use crate::process::{DeadProcessProbe, FixedProcessProbe};
    use crate::reclaim::FsReclaimer;
    use std::fs;

    struct Fx {
        tmp: tempfile::TempDir,
    }

    impl Fx {
        fn new() -> Self {
            Self {
                tmp: tempfile::tempdir().unwrap(),
            }
        }

        fn root(&self) -> PathBuf {
            self.tmp.path().to_path_buf()
        }

        /// A dir with one file and the fingerprint the scanner would have taken.
        fn husk(&self, name: &str) -> (PathBuf, Fingerprint) {
            let dir = self.root().join(name);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("f"), "hello").unwrap();
            let size = dir_size(&dir).unwrap();
            (
                dir,
                Fingerprint {
                    size_bytes: size.bytes,
                    mtime_ms: size.mtime_ms,
                    dirty: false,
                    unpushed: false,
                    is_primary: false,
                    contains_secrets: size.contains_secrets,
                },
            )
        }

        fn plan(&self, actions: Vec<PlanAction>) -> Plan {
            Plan {
                version: PLAN_VERSION,
                created_at_ms: 1,
                preset: PlanPreset::Marked,
                roots: vec![self.root()],
                actions,
            }
        }
    }

    fn action(path: &Path, kind: ActionKind, fingerprint: Fingerprint) -> PlanAction {
        PlanAction {
            husk_id: HuskId::new(HuskKind::Debris, path),
            path: path.to_path_buf(),
            kind,
            fingerprint,
        }
    }

    fn run(plan: &Plan, git: &MapGitProbe, rec: &FsReclaimer, force: bool) -> ApplyResult {
        run_with(plan, git, rec, &DeadProcessProbe, force)
    }

    fn run_with(
        plan: &Plan,
        git: &MapGitProbe,
        rec: &FsReclaimer,
        procs: &dyn ProcessProbe,
        force: bool,
    ) -> ApplyResult {
        apply(
            plan,
            &ApplyOptions {
                git,
                reclaimer: rec,
                processes: procs,
                force,
            },
        )
        .unwrap()
    }

    fn worktree_facts(path: &Path) -> GitFacts {
        GitFacts {
            is_worktree: true,
            common_dir: PathBuf::from("/repo/.git"),
            checkout: path.to_path_buf(),
            ..Default::default()
        }
    }

    fn git_with(path: &Path, facts: GitFacts) -> MapGitProbe {
        fs::create_dir_all(path.join(".git")).ok();
        MapGitProbe {
            facts: [(path.to_path_buf(), facts)].into_iter().collect(),
            ..Default::default()
        }
    }

    #[test]
    fn trashes_matching_husk() {
        let fx = Fx::new();
        let (path, fp) = fx.husk("cache");
        let rec = FsReclaimer::default();
        let result = run(
            &fx.plan(vec![action(&path, ActionKind::Trash, fp)]),
            &MapGitProbe::default(),
            &rec,
            false,
        );
        assert_eq!(result.applied.len(), 1);
        assert_eq!(result.bytes_reclaimed(), 5);
        assert!(!path.exists());
        assert!(result.is_clean());
    }

    #[test]
    fn drift_table_skips() {
        let fx = Fx::new();
        let (path, fp) = fx.husk("cache");
        let mutations: Vec<fn(&mut Fingerprint)> = vec![
            |f| f.size_bytes += 1,
            |f| f.mtime_ms = Some(1),
            |f| f.dirty = true,
            |f| f.unpushed = true,
            |f| f.contains_secrets = true,
        ];
        for m in mutations {
            let mut stale = fp.clone();
            m(&mut stale);
            let rec = FsReclaimer::default();
            let result = run(
                &fx.plan(vec![action(&path, ActionKind::Trash, stale)]),
                &MapGitProbe::default(),
                &rec,
                true,
            );
            assert!(result.applied.is_empty());
            assert!(result.skipped[0].reason.contains("drifted"));
            assert!(path.exists());
        }
    }

    #[test]
    fn worktree_dirty_needs_force_then_trashes_and_prunes() {
        let fx = Fx::new();
        let (path, mut fp) = fx.husk("wt");
        let facts = GitFacts {
            dirty: true,
            dirty_files: 1,
            ..worktree_facts(&path)
        };
        let git = git_with(&path, facts);
        let size = dir_size(&path).unwrap();
        fp.size_bytes = size.bytes;
        fp.mtime_ms = size.mtime_ms;
        fp.dirty = true;
        let plan = fx.plan(vec![action(&path, ActionKind::GitWorktreeRemove, fp)]);
        let rec = FsReclaimer::default();
        let result = run(&plan, &git, &rec, false);
        assert!(result.applied.is_empty());
        assert_eq!(result.skipped[0].reason, copy::get().dirty_worktree);
        let result = run(&plan, &git, &rec, true);
        assert_eq!(result.applied.len(), 1);
        assert!(!path.exists(), "went to the trash");
        assert_eq!(
            git.pruned.lock().unwrap().as_slice(),
            std::slice::from_ref(&path)
        );
    }

    #[test]
    fn stranded_and_locked_need_force() {
        let fx = Fx::new();
        type Case = (&'static str, fn(&Path) -> GitFacts);
        let cases: [Case; 2] = [
            ("s", |p: &Path| GitFacts {
                unpushed: true,
                stranded_commits: 2,
                ..worktree_facts(p)
            }),
            ("l", |p: &Path| GitFacts {
                locked: true,
                ..worktree_facts(p)
            }),
        ];
        for (name, facts) in cases {
            let (path, mut fp) = fx.husk(name);
            let facts = facts(&path);
            fp.unpushed = facts.unpushed;
            let git = git_with(&path, facts);
            let size = dir_size(&path).unwrap();
            fp.size_bytes = size.bytes;
            fp.mtime_ms = size.mtime_ms;
            let plan = fx.plan(vec![action(&path, ActionKind::GitWorktreeRemove, fp)]);
            let rec = FsReclaimer::default();
            let result = run(&plan, &git, &rec, false);
            assert_eq!(result.skipped.len(), 1, "{name}");
            let result = run(&plan, &git, &rec, true);
            assert_eq!(result.applied.len(), 1, "{name}");
        }
    }

    #[test]
    fn occupied_is_refused_even_with_force() {
        let fx = Fx::new();
        let (path, fp) = fx.husk("busy");
        let procs = FixedProcessProbe(vec![Holder {
            pid: 77,
            name: "node".into(),
            cwd: path.join("sub"),
            agent: Some(AgentKind::Codex),
        }]);
        let rec = FsReclaimer::default();
        let result = run_with(
            &fx.plan(vec![action(&path, ActionKind::Trash, fp)]),
            &MapGitProbe::default(),
            &rec,
            &procs,
            true,
        );
        assert!(result.applied.is_empty());
        assert!(result.skipped[0].reason.contains("codex · pid 77"));
        assert!(path.exists());
    }

    #[test]
    fn primary_outside_secret_are_absolute() {
        let fx = Fx::new();
        // primary
        let (path, mut fp) = fx.husk("repo");
        let git = git_with(
            &path,
            GitFacts {
                is_primary: true,
                ..Default::default()
            },
        );
        let size = dir_size(&path).unwrap();
        fp.size_bytes = size.bytes;
        fp.mtime_ms = size.mtime_ms;
        fp.is_primary = true;
        let rec = FsReclaimer::default();
        let result = run(
            &fx.plan(vec![action(&path, ActionKind::GitWorktreeRemove, fp)]),
            &git,
            &rec,
            true,
        );
        assert_eq!(result.skipped[0].reason, copy::get().primary_checkout);
        // outside roots
        let other = tempfile::tempdir().unwrap();
        let out = other.path().join("x");
        fs::create_dir_all(&out).unwrap();
        fs::write(out.join("f"), "hello").unwrap();
        let size = dir_size(&out).unwrap();
        let fp = Fingerprint {
            size_bytes: size.bytes,
            mtime_ms: size.mtime_ms,
            dirty: false,
            unpushed: false,
            is_primary: false,
            contains_secrets: false,
        };
        let result = run(
            &fx.plan(vec![action(&out, ActionKind::Trash, fp)]),
            &MapGitProbe::default(),
            &rec,
            true,
        );
        assert_eq!(result.skipped[0].reason, copy::get().outside_roots);
        // secrets inside
        let (sec, _) = fx.husk("sec");
        fs::write(sec.join(".env"), "S=1").unwrap();
        let size = dir_size(&sec).unwrap();
        let fp = Fingerprint {
            size_bytes: size.bytes,
            mtime_ms: size.mtime_ms,
            dirty: false,
            unpushed: false,
            is_primary: false,
            contains_secrets: true,
        };
        let result = run(
            &fx.plan(vec![action(&sec, ActionKind::Trash, fp)]),
            &MapGitProbe::default(),
            &rec,
            true,
        );
        assert_eq!(result.skipped[0].reason, copy::get().forbidden_path);
        // a secret file named directly
        let env = fx.root().join(".env");
        fs::write(&env, "x").unwrap();
        let fp = Fingerprint {
            size_bytes: 1,
            mtime_ms: None,
            dirty: false,
            unpushed: false,
            is_primary: false,
            contains_secrets: true,
        };
        let result = run(
            &fx.plan(vec![action(&env, ActionKind::Trash, fp)]),
            &MapGitProbe::default(),
            &rec,
            true,
        );
        assert_eq!(result.skipped[0].reason, copy::get().forbidden_path);
        assert!(env.exists());
    }

    #[test]
    fn gone_trash_fail_prune_fail_and_inspect_error() {
        let fx = Fx::new();
        let (path, fp) = fx.husk("gone");
        fs::remove_dir_all(&path).unwrap();
        let rec = FsReclaimer::default();
        let result = run(
            &fx.plan(vec![action(&path, ActionKind::Trash, fp)]),
            &MapGitProbe::default(),
            &rec,
            false,
        );
        assert_eq!(result.skipped[0].reason, copy::get().already_gone);

        let (path, fp) = fx.husk("stuck");
        let failing = FsReclaimer {
            fail: true,
            ..Default::default()
        };
        let result = run(
            &fx.plan(vec![action(&path, ActionKind::Trash, fp)]),
            &MapGitProbe::default(),
            &failing,
            false,
        );
        assert_eq!(result.errors.len(), 1);
        assert!(path.exists());

        let (path, mut fp) = fx.husk("noprune");
        let mut git = git_with(&path, worktree_facts(&path));
        git.prune_fail = true;
        let size = dir_size(&path).unwrap();
        fp.size_bytes = size.bytes;
        fp.mtime_ms = size.mtime_ms;
        let result = run(
            &fx.plan(vec![action(
                &path,
                ActionKind::GitWorktreeRemove,
                fp.clone(),
            )]),
            &git,
            &rec,
            false,
        );
        assert_eq!(result.applied.len(), 1);
        assert_eq!(result.errors.len(), 1);

        let (path, fp) = fx.husk("boom");
        fs::create_dir_all(path.join(".git")).unwrap();
        let boom = MapGitProbe {
            inspect_fail: true,
            ..Default::default()
        };
        let result = run(
            &fx.plan(vec![action(&path, ActionKind::GitWorktreeRemove, fp)]),
            &boom,
            &rec,
            false,
        );
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn worktree_without_checkout_path_prunes_husk_path() {
        let fx = Fx::new();
        let (path, mut fp) = fx.husk("bare-facts");
        let git = git_with(
            &path,
            GitFacts {
                is_worktree: true,
                ..Default::default()
            },
        );
        let size = dir_size(&path).unwrap();
        fp.size_bytes = size.bytes;
        fp.mtime_ms = size.mtime_ms;
        let rec = FsReclaimer::default();
        run(
            &fx.plan(vec![action(&path, ActionKind::GitWorktreeRemove, fp)]),
            &git,
            &rec,
            false,
        );
        // inspect_tree fills checkout, so the husk path is what gets pruned
        assert_eq!(
            git.pruned.lock().unwrap().as_slice(),
            std::slice::from_ref(&path)
        );
    }

    #[test]
    fn version_and_empty_plan_refused() {
        let fx = Fx::new();
        let mut plan = fx.plan(vec![]);
        let rec = FsReclaimer::default();
        let git = MapGitProbe::default();
        let opts = ApplyOptions {
            git: &git,
            reclaimer: &rec,
            processes: &DeadProcessProbe,
            force: false,
        };
        assert!(matches!(
            apply_or_refuse_empty_plan(&plan, &opts),
            Err(Error::Safety(_))
        ));
        plan.version = 1;
        assert!(matches!(apply(&plan, &opts), Err(Error::Version { .. })));
        let (path, fp) = fx.husk("ok");
        let good = fx.plan(vec![action(&path, ActionKind::Trash, fp)]);
        assert_eq!(
            apply_or_refuse_empty_plan(&good, &opts)
                .unwrap()
                .applied
                .len(),
            1
        );
    }
}
