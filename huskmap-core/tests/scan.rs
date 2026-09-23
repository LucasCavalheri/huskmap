mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use common::{copy_dir, fixtures_root, write};
use git2::{Repository, Signature};
use huskmap_core::*;

const NOW: u64 = 1_900_000_000_000;

fn commit(repo: &Repository, dir: &Path, name: &str, msg: &str) {
    let sig = Signature::now("huskmap", "huskmap@test").unwrap();
    fs::write(dir.join(name), msg).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(name)).unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<&git2::Commit> = parent.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &parents)
        .unwrap();
}

struct Home {
    _tmp: tempfile::TempDir,
    home: PathBuf,
}

impl Home {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        fs::create_dir_all(&home).unwrap();
        Self { _tmp: tmp, home }
    }

    fn p(&self, rel: &str) -> PathBuf {
        self.home.join(rel)
    }

    fn opts(&self) -> ScanOptions {
        let mut opts = ScanOptions::for_home(self.home.clone());
        opts.processes = Arc::new(DeadProcessProbe);
        opts.clock = Arc::new(FrozenClock::from_millis(NOW));
        opts
    }

    /// dev/app (primary) + four linked worktrees in the layouts agents actually use.
    fn grove(&self) -> Repository {
        let app = self.p("dev/app");
        fs::create_dir_all(&app).unwrap();
        let repo = Repository::init(&app).unwrap();
        commit(&repo, &app, "README", "init");
        commit(&repo, &app, "package.json", "{}");
        write(
            &app.join("node_modules/left-pad/index.js"),
            "module.exports = 1",
        );
        fs::write(app.join(".gitignore"), "node_modules\n.claude\n").unwrap();
        commit(&repo, &app, ".gitignore", "node_modules\n.claude\n");

        for (name, path) in [
            ("clean", self.p("dev/app-worktrees/clean")),
            ("dirty", self.p("dev/app-worktrees/dirty")),
            ("stranded", self.p("dev/app-worktrees/stranded")),
            ("busy", self.p("dev/app/.claude/worktrees/busy")),
            ("codex", self.p(".codex/worktrees/a1b2/app")),
        ] {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            repo.worktree(name, &path, None).unwrap();
        }
        let wt = |rel: &str| self.p(rel);
        write(&wt("dev/app-worktrees/clean/node_modules/x/i.js"), "x");
        fs::write(wt("dev/app-worktrees/dirty/scratch.txt"), "unsaved").unwrap();
        let stranded = Repository::open(wt("dev/app-worktrees/stranded")).unwrap();
        commit(
            &stranded,
            &wt("dev/app-worktrees/stranded"),
            "work.txt",
            "agent work",
        );
        repo
    }
}

fn find<'a>(report: &'a ScanReport, path: &Path) -> &'a Husk {
    report
        .husks
        .iter()
        .find(|h| h.path == path)
        .unwrap_or_else(|| panic!("no husk at {}", path.display()))
}

#[test]
fn worktrees_are_judged_by_git_and_by_who_is_inside() {
    let h = Home::new();
    h.grove();
    let mut opts = h.opts();
    opts.processes = Arc::new(FixedProcessProbe(vec![Holder {
        pid: 4242,
        name: "claude".into(),
        cwd: h.p("dev/app/.claude/worktrees/busy/src"),
        agent: Some(AgentKind::Claude),
    }]));
    fs::create_dir_all(h.p("dev/app/.claude/worktrees/busy/src")).unwrap();
    let report = scan(&opts).unwrap();

    let clean = find(&report, &h.p("dev/app-worktrees/clean"));
    assert_eq!(clean.kind, HuskKind::Worktree);
    assert!(clean.reclaimable, "{:?}", clean.wards);
    assert_eq!(clean.risk, Risk::Caution);
    let facts = clean.git.as_ref().unwrap();
    assert_eq!(facts.branch.as_deref(), Some("clean"));
    assert_eq!(facts.stranded_commits, 0);
    assert!(!clean.is_alarm());

    let dirty = find(&report, &h.p("dev/app-worktrees/dirty"));
    assert!(dirty.has_ward("dirty"));
    assert!(!dirty.reclaimable);
    assert_eq!(dirty.risk, Risk::Dangerous);
    assert!(dirty.is_alarm());

    let stranded = find(&report, &h.p("dev/app-worktrees/stranded"));
    assert!(stranded.wards.contains(&Ward::Stranded { commits: 1 }));
    assert!(stranded.is_alarm());

    let busy = find(&report, &h.p("dev/app/.claude/worktrees/busy"));
    assert!(busy.has_ward("occupied"));
    assert!(busy.live);
    assert_eq!(busy.agent, Some(AgentKind::Claude));
    assert_eq!(busy.holders()[0].pid, 4242);

    let codex = find(&report, &h.p(".codex/worktrees/a1b2/app"));
    assert_eq!(codex.agent, Some(AgentKind::Codex));
    assert!(codex.reclaimable);

    // ballast inside a worktree is its own husk, parented to the worktree
    let nm = find(&report, &h.p("dev/app-worktrees/clean/node_modules"));
    assert_eq!(nm.parent.as_ref(), Some(&clean.id));
    // the primary checkout itself is never a husk, but its node_modules is
    assert!(report.husks.iter().all(|x| x.path != h.p("dev/app")));
    let primary_nm = find(&report, &h.p("dev/app/node_modules"));
    assert!(primary_nm.parent.is_none());

    // one grove for the whole repo, sibling node_modules are duplicates
    assert_eq!(report.groves.len(), 1);
    let grove = &report.groves[0];
    assert_eq!(grove.name, "app");
    assert_eq!(
        grove.primary.as_deref(),
        Some(canonicalize_lossy(&h.p("dev/app")).as_path())
    );
    assert_eq!(grove.worktrees.len(), 5);
    let dup = report
        .ballast
        .iter()
        .find(|b| b.husk_id == primary_nm.id)
        .unwrap();
    assert_eq!(
        dup.duplicates,
        vec![h.p("dev/app-worktrees/clean/node_modules")]
    );

    assert_eq!(report.totals.alarms, 3);
    assert_eq!(report.totals.occupied, 1);
    assert_eq!(report.version, REPORT_VERSION);
    assert_eq!(report.scanned_at_ms, NOW);
}

#[test]
fn worktree_env_guards_only_when_it_is_the_only_copy() {
    let h = Home::new();
    h.grove();
    fs::write(h.p("dev/app/.env"), "TOKEN=main").unwrap();
    fs::write(h.p("dev/app-worktrees/clean/.env"), "TOKEN=main").unwrap();
    let stranded = h.p("dev/app-worktrees/stranded");
    fs::write(stranded.join(".env"), "TOKEN=only-here").unwrap();
    fs::write(h.p("dev/app/.gitignore"), "node_modules\n.claude\n.env\n").unwrap();
    for wt in ["clean", "stranded"] {
        let ignore = h.p(&format!("dev/app-worktrees/{wt}/.gitignore"));
        fs::write(ignore, "node_modules\n.claude\n.env\n").unwrap();
    }
    let report = scan(&h.opts()).unwrap();
    let clean = find(&report, &h.p("dev/app-worktrees/clean"));
    assert!(!clean.has_ward("secrets"), "{:?}", clean.wards);
    let unique = find(&report, &stranded);
    assert!(unique.has_ward("secrets"));
    assert!(
        !admissible(unique, true),
        "force never takes a unique secret"
    );
}

#[test]
fn ballast_next_to_a_running_process_is_caution() {
    let h = Home::new();
    write(&h.p("dev/web/package.json"), "{}");
    write(&h.p("dev/web/node_modules/a/i.js"), "x");
    fs::create_dir_all(h.p("dev/web/src")).unwrap();
    let mut opts = h.opts();
    opts.processes = Arc::new(FixedProcessProbe(vec![Holder {
        pid: 9,
        name: "node".into(),
        cwd: h.p("dev/web"),
        agent: None,
    }]));
    let report = scan(&opts).unwrap();
    let nm = find(&report, &h.p("dev/web/node_modules"));
    assert!(nm.has_ward("neighbor_busy"));
    assert_eq!(nm.risk, Risk::Caution);
    assert!(nm.reclaimable, "manual mark still allowed");
    let safe = plan_with_clock(&report, PlanPreset::Safe, &FrozenClock::from_millis(NOW));
    assert!(safe.actions.is_empty(), "safe preset leaves it alone");
}

#[test]
fn fixtures_classify_by_marker_and_secrets_block() {
    let h = Home::new();
    let dev = h.p("dev");
    for name in [
        "node_app",
        "rust_workspace",
        "orphan_target",
        "venv_ok",
        "venv_bare",
        "next_app",
        "turbo_app",
    ] {
        copy_dir(&fixtures_root().join(name), &dev.join(name));
    }
    copy_dir(&fixtures_root().join("agent_home"), &h.home);
    write(&h.p("dev/rust_workspace/target/.env"), "SECRET=1");
    let report = scan(&h.opts()).unwrap();
    let kinds: Vec<(String, HuskKind)> = report
        .husks
        .iter()
        .map(|x| {
            (
                x.path.strip_prefix(&h.home).unwrap().display().to_string(),
                x.kind,
            )
        })
        .collect();
    let has = |rel: &str, kind| kinds.iter().any(|(p, k)| p == rel && *k == kind);
    assert!(has("dev/node_app/node_modules", HuskKind::Ballast));
    assert!(has("dev/venv_ok/.venv", HuskKind::Ballast));
    assert!(has("dev/next_app/.next", HuskKind::Ballast));
    assert!(has("dev/turbo_app/.turbo", HuskKind::Ballast));
    assert!(has(".claude/projects/hello", HuskKind::Afterimage));
    assert!(
        !kinds
            .iter()
            .any(|(p, _)| p.starts_with("dev/orphan_target"))
    );
    assert!(!kinds.iter().any(|(p, _)| p.starts_with("dev/venv_bare")));
    // a key-named file inside build output is package content, not a secret of yours
    let target = find(&report, &h.p("dev/rust_workspace/target"));
    assert!(!target.contains_secrets);
    assert!(target.reclaimable);
    // no-git slot in codex worktrees
    let slot = find(&report, &h.p(".codex/worktrees/wt1"));
    assert!(slot.has_ward("no_git"));
}

#[test]
fn toolchains_are_reported_whole() {
    let h = Home::new();
    write(&h.p(".npm/_cacache/index-v5/aa"), "npm");
    write(&h.p(".cargo/registry/cache/x.crate"), "crate");
    write(&h.p(".cache/uv/wheels/w"), "uv");
    write(&h.p(".cache/codex-runtimes/node/bin"), "rt");
    let report = scan(&h.opts()).unwrap();
    let npm = find(&report, &h.p(".npm/_cacache"));
    assert_eq!(npm.kind, HuskKind::Toolchain);
    assert_eq!(npm.ecosystem, Some(Ecosystem::Node));
    assert!(npm.notes[0].contains("npm"));
    assert_eq!(
        find(&report, &h.p(".cargo/registry")).ecosystem,
        Some(Ecosystem::Rust)
    );
    assert_eq!(
        find(&report, &h.p(".cache/uv")).ecosystem,
        Some(Ecosystem::Python)
    );
    let rt = find(&report, &h.p(".cache/codex-runtimes"));
    assert_eq!(
        (rt.kind, rt.agent),
        (HuskKind::Cache, Some(AgentKind::Codex))
    );
    assert_eq!(
        report.husks.iter().filter(|x| x.parent.is_some()).count(),
        0
    );
    assert!(report.roots.contains(&h.p(".npm/_cacache")));
    let plan = plan_with_clock(&report, PlanPreset::Safe, &FrozenClock::from_millis(NOW));
    assert_eq!(plan.actions.len(), 4);
}

#[test]
fn claude_sessions_know_their_project() {
    let h = Home::new();
    let project = h.p("dev/my app");
    fs::create_dir_all(&project).unwrap();
    let key = claude_project_key(&project);
    assert!(!key.contains(' ') && !key.contains('/'));
    write(&h.p(&format!(".claude/projects/{key}/s.jsonl")), "{}");
    let gone = claude_project_key(&h.p("dev/deleted-thing"));
    write(&h.p(&format!(".claude/projects/{gone}/s.jsonl")), "{}");
    let mut opts = h.opts();
    opts.processes = Arc::new(FixedProcessProbe(vec![Holder {
        pid: 1,
        name: "claude".into(),
        cwd: project.clone(),
        agent: Some(AgentKind::Claude),
    }]));
    let report = scan(&opts).unwrap();
    let live = find(&report, &h.p(&format!(".claude/projects/{key}")));
    assert!(live.has_ward("occupied"));
    assert!(!live.reclaimable);
    let orphan = find(&report, &h.p(&format!(".claude/projects/{gone}")));
    assert!(orphan.has_ward("orphaned"));
    assert!(orphan.reclaimable);
    assert_eq!(
        orphan.project, None,
        "a key that resolves nowhere names no project"
    );
    assert_eq!(
        live.project.as_deref().map(canonicalize_lossy),
        Some(canonicalize_lossy(&project))
    );
    assert_eq!(report.afterimages.len(), 2);
    assert_eq!(
        resolve_claude_origin(&key, Path::new("/")).map(|p| canonicalize_lossy(&p)),
        Some(canonicalize_lossy(&project))
    );
}

#[test]
fn grok_sessions_decode_their_origin() {
    let h = Home::new();
    let project = h.p("dev/proj");
    fs::create_dir_all(&project).unwrap();
    let enc = |p: &Path| p.to_string_lossy().replace('/', "%2F");
    write(
        &h.p(&format!(".grok/sessions/{}/a/s.jsonl", enc(&project))),
        "{}",
    );
    write(
        &h.p(&format!(
            ".grok/sessions/{}/a/s.jsonl",
            enc(&h.p("dev/gone"))
        )),
        "{}",
    );
    let mut opts = h.opts();
    opts.processes = Arc::new(FixedProcessProbe(vec![Holder {
        pid: 5,
        name: "grok".into(),
        cwd: project.clone(),
        agent: Some(AgentKind::Grok),
    }]));
    let report = scan(&opts).unwrap();
    let live = find(&report, &h.p(&format!(".grok/sessions/{}", enc(&project))));
    assert!(live.has_ward("occupied"));
    let gone = find(
        &report,
        &h.p(&format!(".grok/sessions/{}", enc(&h.p("dev/gone")))),
    );
    assert!(gone.has_ward("orphaned"));
    assert_eq!(
        gone.project,
        Some(h.p("dev/gone")),
        "decoded even when it is gone"
    );
    assert_eq!(live.project.as_deref(), Some(project.as_path()));
    assert_eq!(percent_decode("%2Fa%20b"), Some(PathBuf::from("/a b")));
    assert_eq!(percent_decode("plain"), None);
    assert_eq!(percent_decode("rel%2Fx"), None);
    assert_eq!(percent_decode("%zz"), None);
    assert_eq!(percent_decode("%2"), None);
    assert_eq!(percent_decode("%FF%FE"), None);
}

#[test]
fn warm_sessions_are_held() {
    let h = Home::new();
    write(&h.p(".codex/sessions/2026/09/23/rollout.jsonl"), "{}");
    let mut opts = h.opts();
    opts.clock = Arc::new(SystemClock);
    let report = scan(&opts).unwrap();
    let day = find(&report, &h.p(".codex/sessions/2026/09/23"));
    assert!(day.has_ward("warm"));
    assert!(day.live && !day.reclaimable);
}

#[test]
fn filters_explicit_roots_and_outside_worktrees() {
    let h = Home::new();
    let repo = h.grove();
    let outside = h._tmp.path().join("elsewhere/wt");
    fs::create_dir_all(outside.parent().unwrap()).unwrap();
    repo.worktree("outside", &outside, None).unwrap();
    let mut opts = h.opts();
    opts.extra_roots = vec![h.p("dev"), h.p("dev/app"), h.p("missing")];
    assert_eq!(opts.roots(), vec![h.p("dev")]);
    let report = scan(&opts).unwrap();
    let out = find(&report, &outside);
    assert!(out.has_ward("outside_roots"));
    assert!(!out.reclaimable);
    // linked worktrees outside the roots are shown, never reclaimable
    let codex = find(&report, &h.p(".codex/worktrees/a1b2/app"));
    assert!(codex.has_ward("outside_roots") && !codex.reclaimable);

    opts.category = Some(HuskKind::Ballast);
    let only = scan(&opts).unwrap();
    assert!(only.husks.iter().all(|x| x.kind == HuskKind::Ballast));
    assert!(only.husks.iter().all(|x| x.parent.is_none()));

    opts.category = None;
    opts.min_size = u64::MAX;
    assert!(scan(&opts).unwrap().husks.is_empty());
}

#[test]
fn progress_events_stream_every_husk() {
    let h = Home::new();
    h.grove();
    let mut walking = 0;
    let mut found = 0;
    let mut weighing = None;
    let report = scan_with_progress(&h.opts(), |e| match e {
        ScanEvent::Walking { .. } => walking += 1,
        ScanEvent::Weighing { pending } => weighing = Some(*pending),
        ScanEvent::Found(_) => found += 1,
    })
    .unwrap();
    assert!(walking >= 2);
    assert_eq!(found, report.husks.len());
    assert_eq!(weighing, Some(report.husks.len()));
    // sorted heaviest first
    assert!(
        report
            .husks
            .windows(2)
            .all(|w| w[0].size_bytes >= w[1].size_bytes)
    );
    let by = report.bytes_by_kind();
    assert!(
        by.iter()
            .any(|(k, b, _)| *k == HuskKind::Worktree && *b > 0)
    );
}

#[test]
fn empty_home_is_an_empty_report() {
    let h = Home::new();
    let report = scan(&h.opts()).unwrap();
    assert!(report.husks.is_empty());
    assert!(report.roots.is_empty());
    assert_eq!(report.totals, Totals::default());
}
