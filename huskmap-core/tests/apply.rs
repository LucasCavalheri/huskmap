mod common;

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use common::write;
use git2::{Repository, Signature};
use huskmap_core::*;

fn init(dir: &Path) -> Repository {
    fs::create_dir_all(dir).unwrap();
    let repo = Repository::init(dir).unwrap();
    let sig = Signature::now("huskmap", "huskmap@test").unwrap();
    fs::write(dir.join("README"), "x").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("README")).unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
        .unwrap();
    drop(tree);
    repo
}

/// Scan → mark → plan → apply against real git, in a temp home only.
#[test]
fn clean_worktree_goes_dirty_one_stays_and_git_forgets_the_gone() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    let app = home.join("dev/app");
    let repo = init(&app);
    let clean = home.join("dev/app-worktrees/clean");
    let dirty = home.join("dev/app-worktrees/dirty");
    fs::create_dir_all(clean.parent().unwrap()).unwrap();
    repo.worktree("clean", &clean, None).unwrap();
    repo.worktree("dirty", &dirty, None).unwrap();
    fs::write(dirty.join("unsaved.txt"), "mine").unwrap();
    write(&app.join("package.json"), "{}");
    write(&app.join("node_modules/a/index.js"), "x");
    fs::write(app.join(".gitignore"), "node_modules\npackage.json\n").unwrap();

    let mut opts = ScanOptions::for_home(home.clone());
    opts.processes = Arc::new(DeadProcessProbe);
    let report = scan(&opts).unwrap();
    let marked: BTreeSet<HuskId> = report
        .husks
        .iter()
        .filter(|h| h.kind == HuskKind::Worktree || h.kind == HuskKind::Ballast)
        .map(|h| h.id.clone())
        .collect();
    assert_eq!(marked.len(), 3);

    let plan = plan_marked(&report, &marked, false, &SystemClock);
    let planned: Vec<_> = plan.actions.iter().map(|a| a.path.clone()).collect();
    assert_eq!(planned, vec![app.join("node_modules"), clean.clone()]);

    let rec = FsReclaimer::default();
    let result = apply(
        &plan,
        &ApplyOptions {
            git: &Git2Probe,
            reclaimer: &rec,
            processes: &DeadProcessProbe,
            force: false,
        },
    )
    .unwrap();
    assert!(result.is_clean(), "{:?}", result.errors);
    assert_eq!(result.applied.len(), 2);
    assert!(!clean.exists());
    assert!(!app.join("node_modules").exists());
    assert!(dirty.join("unsaved.txt").exists(), "dirty work untouched");
    let left = Git2Probe.list_worktree_paths(&app).unwrap();
    assert_eq!(left.len(), 1, "git forgot the trashed worktree");

    // force still cannot take a worktree someone is inside
    let report = scan(&opts).unwrap();
    let dirty_id = report
        .husks
        .iter()
        .find(|h| h.path == dirty)
        .unwrap()
        .id
        .clone();
    let forced = plan_marked(
        &report,
        &[dirty_id].into_iter().collect(),
        true,
        &SystemClock,
    );
    assert_eq!(forced.actions.len(), 1);
    let occupied = FixedProcessProbe(vec![Holder {
        pid: 3,
        name: "codex".into(),
        cwd: dirty.clone(),
        agent: Some(AgentKind::Codex),
    }]);
    let result = apply(
        &forced,
        &ApplyOptions {
            git: &Git2Probe,
            reclaimer: &rec,
            processes: &occupied,
            force: true,
        },
    )
    .unwrap();
    assert!(result.applied.is_empty());
    assert!(dirty.exists());
    // and with nobody inside, force takes it
    let result = apply(
        &forced,
        &ApplyOptions {
            git: &Git2Probe,
            reclaimer: &rec,
            processes: &DeadProcessProbe,
            force: true,
        },
    )
    .unwrap();
    assert_eq!(result.applied.len(), 1, "{:?}", result);
    assert!(!dirty.exists());
    assert!(Git2Probe.list_worktree_paths(&app).unwrap().is_empty());
}

#[test]
fn plan_goes_stale_when_the_husk_changes() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    write(&home.join("dev/p/package.json"), "{}");
    write(&home.join("dev/p/node_modules/a"), "1");
    let mut opts = ScanOptions::for_home(home.clone());
    opts.processes = Arc::new(DeadProcessProbe);
    let report = scan(&opts).unwrap();
    let plan = plan(&report, PlanPreset::Safe);
    assert_eq!(plan.actions.len(), 1);
    let saved = tmp.path().join("plan.json");
    plan.save(&saved).unwrap();
    write(&home.join("dev/p/node_modules/b"), "grew");
    let rec = FsReclaimer::default();
    let result = apply(
        &Plan::load(&saved).unwrap(),
        &ApplyOptions {
            git: &Git2Probe,
            reclaimer: &rec,
            processes: &DeadProcessProbe,
            force: true,
        },
    )
    .unwrap();
    assert!(result.applied.is_empty());
    assert!(result.skipped[0].reason.contains("drifted"));
    assert!(home.join("dev/p/node_modules").exists());
}
