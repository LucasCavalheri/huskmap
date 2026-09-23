//! Scan edges driven by fake probes: git that fails, locks, primaries, odd groves.
mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use common::write;
use huskmap_core::*;

fn opts(home: &Path, git: MapGitProbe) -> ScanOptions {
    let mut o = ScanOptions::for_home(home.to_path_buf());
    o.processes = Arc::new(DeadProcessProbe);
    o.clock = Arc::new(FrozenClock::from_millis(4_000_000_000_000));
    o.git = Arc::new(git);
    o
}

fn linked(dir: &Path) {
    fs::create_dir_all(dir).unwrap();
    fs::write(dir.join(".git"), "gitdir: /r/.git/worktrees/x\n").unwrap();
}

#[test]
fn git_errors_become_notes_and_no_git_wards() {
    let tmp = tempfile::tempdir().unwrap();
    let wt = tmp.path().join("dev/a-wt/x");
    linked(&wt);
    let git = MapGitProbe {
        inspect_fail: true,
        ..Default::default()
    };
    let report = scan(&opts(tmp.path(), git)).unwrap();
    let h = report.husks.iter().find(|h| h.path == wt).unwrap();
    assert!(h.notes.iter().any(|n| n.starts_with("git:")));
    assert!(h.has_ward("no_git"));
}

#[test]
fn locked_and_primary_clone_and_bare_grove() {
    let tmp = tempfile::tempdir().unwrap();
    let locked = tmp.path().join("dev/w/locked");
    linked(&locked);
    let clone = tmp.path().join("dev/agent-7");
    fs::create_dir_all(clone.join(".git")).unwrap();
    let facts = |primary: bool, common: &str| GitFacts {
        is_worktree: !primary,
        is_primary: primary,
        common_dir: PathBuf::from(common),
        locked: !primary,
        lock_reason: Some("agent".into()),
        ..Default::default()
    };
    let git = MapGitProbe {
        facts: [
            (locked.clone(), facts(false, "/srv/repos/app.git")),
            (clone.clone(), facts(true, "/x/agent-7/.git")),
        ]
        .into_iter()
        .collect(),
        ..Default::default()
    };
    let report = scan(&opts(tmp.path(), git)).unwrap();
    let l = report.husks.iter().find(|h| h.path == locked).unwrap();
    assert!(l.wards.contains(&Ward::Locked {
        reason: Some("agent".into())
    }));
    let c = report.husks.iter().find(|h| h.path == clone).unwrap();
    assert!(c.has_ward("primary"));
    assert_eq!(c.risk, Risk::Forbidden);
    let bare = report
        .groves
        .iter()
        .find(|g| g.id == Path::new("/srv/repos/app.git"))
        .unwrap();
    assert_eq!(bare.name, "app.git");
    assert!(bare.primary.is_none());
}

#[derive(Debug)]
struct ListFails;

impl GitProbe for ListFails {
    fn inspect(&self, _: &Path) -> Result<Option<GitFacts>, Error> {
        Ok(None)
    }
    fn list_worktree_paths(&self, _: &Path) -> Result<Vec<PathBuf>, Error> {
        Err(Error::git("boom"))
    }
    fn discover_common_dir(&self, _: &Path) -> Option<PathBuf> {
        None
    }
    fn prune_worktree(&self, _: &Path, _: &Path) -> Result<(), Error> {
        Ok(())
    }
}

#[test]
fn worktree_listing_error_is_a_warning() {
    let tmp = tempfile::tempdir().unwrap();
    fs::create_dir_all(tmp.path().join("dev/repo/.git")).unwrap();
    let mut o = opts(tmp.path(), MapGitProbe::default());
    o.git = Arc::new(ListFails);
    let report = scan(&o).unwrap();
    assert!(report.warnings.iter().any(|w| w.contains("boom")));
    assert!(ListFails.discover_common_dir(tmp.path()).is_none());
    assert!(ListFails.prune_worktree(tmp.path(), tmp.path()).is_ok());
}

#[test]
fn unreadable_husk_is_a_warning_not_a_crash() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::tempdir().unwrap();
    write(&tmp.path().join("dev/p/package.json"), "{}");
    let nm = tmp.path().join("dev/p/node_modules");
    write(&nm.join("a/x"), "1");
    let link = tmp.path().join("dev/p/node_modules/a/loop");
    std::os::unix::fs::symlink(&nm, &link).unwrap();
    fs::set_permissions(nm.join("a"), fs::Permissions::from_mode(0o000)).unwrap();
    let report = scan(&opts(tmp.path(), MapGitProbe::default()));
    fs::set_permissions(nm.join("a"), fs::Permissions::from_mode(0o755)).unwrap();
    let report = report.unwrap();
    assert_eq!(
        report.husks.len(),
        1,
        "sized what it could, skipped the rest"
    );
}

#[test]
fn grok_session_that_is_not_a_path_stays_plain() {
    let tmp = tempfile::tempdir().unwrap();
    write(&tmp.path().join(".grok/sessions/plain-name/s.jsonl"), "{}");
    let report = scan(&opts(tmp.path(), MapGitProbe::default())).unwrap();
    let s = report
        .husks
        .iter()
        .find(|h| h.kind == HuskKind::Afterimage)
        .unwrap();
    assert!(s.wards.is_empty(), "{:?}", s.wards);
}
