use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use git2::{Repository, Status, StatusOptions, WorktreeLockStatus};

use crate::domain::GitFacts;
use crate::error::Error;
use crate::safety::{canonicalize_lossy, is_skip_dir_name};

/// Git access. Isolated so scans and applies can be tested without real repos.
pub trait GitProbe: Send + Sync {
    /// Facts for a checkout rooted exactly at `path`. `None` when it is not a checkout root.
    fn inspect(&self, path: &Path) -> Result<Option<GitFacts>, Error>;
    /// Linked worktree paths registered by the repo at `path`.
    fn list_worktree_paths(&self, path: &Path) -> Result<Vec<PathBuf>, Error>;
    /// Common git dir of the checkout containing `path`, searching upward.
    fn discover_common_dir(&self, path: &Path) -> Option<PathBuf>;
    /// Once a linked checkout is gone from disk, drop git's admin entry for it.
    fn prune_worktree(&self, common_dir: &Path, checkout: &Path) -> Result<(), Error>;
}

/// What a `.git` entry says about a directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitMarker {
    None,
    Primary,
    Linked,
    Submodule,
}

/// Read the `.git` entry without opening the repo.
pub fn git_marker(dir: &Path) -> GitMarker {
    let dot = dir.join(".git");
    if dot.is_dir() {
        return GitMarker::Primary;
    }
    let Ok(text) = std::fs::read_to_string(&dot) else {
        return GitMarker::None;
    };
    let Some(target) = text.trim().strip_prefix("gitdir:") else {
        return GitMarker::None;
    };
    let target = target.trim().replace('\\', "/");
    if target.contains("/worktrees/") {
        GitMarker::Linked
    } else if target.contains("/modules/") {
        GitMarker::Submodule
    } else {
        GitMarker::None
    }
}

/// Checkouts at `path` or, when `path` is a bare container (agent slot), up to `depth` below it.
pub fn find_checkouts(path: &Path, depth: usize) -> Vec<PathBuf> {
    match git_marker(path) {
        GitMarker::Primary | GitMarker::Linked => return vec![path.to_path_buf()],
        GitMarker::Submodule | GitMarker::None => {}
    }
    if depth == 0 {
        return vec![];
    }
    let Ok(entries) = std::fs::read_dir(path) else {
        return vec![];
    };
    let mut dirs: Vec<PathBuf> = entries
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| !is_skip_dir_name(n) && n != "node_modules" && n != "target")
        })
        .collect();
    dirs.sort();
    dirs.into_iter()
        .flat_map(|d| find_checkouts(&d, depth - 1))
        .collect()
}

/// Inspect every checkout at or under `path` and fold them into one set of facts.
pub fn inspect_tree(git: &dyn GitProbe, path: &Path) -> Result<Option<GitFacts>, Error> {
    let mut all = Vec::new();
    for checkout in find_checkouts(path, 3) {
        if let Some(mut facts) = git.inspect(&checkout)? {
            facts.checkout = checkout;
            all.push(facts);
        }
    }
    Ok(merge_facts(all))
}

/// Worst-case merge: dirty if any is dirty, primary if any is primary, counts add up.
pub fn merge_facts(mut all: Vec<GitFacts>) -> Option<GitFacts> {
    if all.len() <= 1 {
        return all.pop();
    }
    let mut iter = all.into_iter();
    let mut acc = iter.next()?;
    for f in iter {
        acc.is_primary |= f.is_primary;
        acc.is_worktree &= f.is_worktree;
        acc.dirty |= f.dirty;
        acc.unpushed |= f.unpushed;
        acc.locked |= f.locked;
        acc.dirty_files += f.dirty_files;
        acc.stranded_commits += f.stranded_commits;
        acc.last_commit_ms = acc.last_commit_ms.max(f.last_commit_ms);
        acc.lock_reason = acc.lock_reason.or(f.lock_reason);
        acc.upstream = acc.upstream.or(f.upstream);
    }
    Some(acc)
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Git2Probe;

fn branch_name(repo: &Repository) -> Option<String> {
    let head = repo.head().ok()?;
    if head.is_branch() {
        head.shorthand().ok().map(str::to_string)
    } else {
        None
    }
}

fn upstream_name(repo: &Repository) -> Option<String> {
    let head = repo.head().ok()?;
    if !head.is_branch() {
        return None;
    }
    let up = git2::Branch::wrap(head).upstream().ok()?;
    up.name().ok().flatten().map(str::to_string)
}

fn dirty_files(repo: &Repository) -> u32 {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .include_ignored(false)
        .recurse_untracked_dirs(false)
        .exclude_submodules(true);
    repo.statuses(Some(&mut opts))
        .map(|statuses| {
            statuses
                .iter()
                .filter(|s| s.status() != Status::CURRENT)
                .count() as u32
        })
        .unwrap_or(0)
}

/// Commits reachable from HEAD and from no other ref. Those vanish with a detached checkout
/// and live only on a branch nobody else has.
fn stranded_commits(repo: &Repository) -> u32 {
    let Ok(head) = repo.head() else {
        return 0;
    };
    let Some(oid) = head.target() else {
        return 0;
    };
    let own = if head.is_branch() {
        head.name().ok().map(str::to_string)
    } else {
        None
    };
    let Ok(mut walk) = repo.revwalk() else {
        return 0;
    };
    if walk.push(oid).is_err() {
        return 0;
    }
    if let Ok(refs) = repo.references() {
        for r in refs.flatten() {
            if r.name().is_ok_and(|n| Some(n) == own.as_deref()) {
                continue;
            }
            if let Ok(commit) = r.peel_to_commit() {
                let _ = walk.hide(commit.id());
            }
        }
    }
    walk.take(10_000).filter(Result::is_ok).count() as u32
}

fn head_commit(repo: &Repository) -> (Option<String>, Option<u64>) {
    let Ok(commit) = repo.head().and_then(|h| h.peel_to_commit()) else {
        return (None, None);
    };
    let summary = commit.summary().ok().flatten().map(str::to_string);
    let secs = commit.time().seconds();
    (summary, u64::try_from(secs).ok().map(|s| s * 1000))
}

fn lock_status(repo: &Repository) -> (bool, Option<String>) {
    if !repo.is_worktree() {
        return (false, None);
    }
    match git2::Worktree::open_from_repository(repo).map(|wt| wt.is_locked()) {
        Ok(Ok(WorktreeLockStatus::Locked(reason))) => (true, reason),
        _ => (false, None),
    }
}

fn facts_from_repo(path: &Path, repo: &Repository) -> GitFacts {
    let is_worktree = repo.is_worktree();
    let files = dirty_files(repo);
    let stranded = stranded_commits(repo);
    let (head_summary, last_commit_ms) = head_commit(repo);
    let (locked, lock_reason) = lock_status(repo);
    GitFacts {
        is_worktree,
        is_primary: !is_worktree,
        git_dir: repo.path().to_path_buf(),
        common_dir: canonicalize_lossy(repo.commondir()),
        checkout: path.to_path_buf(),
        branch: branch_name(repo),
        dirty: files > 0,
        unpushed: stranded > 0,
        locked,
        dirty_files: files,
        stranded_commits: stranded,
        upstream: upstream_name(repo),
        head_summary,
        last_commit_ms,
        lock_reason,
    }
}

impl GitProbe for Git2Probe {
    fn inspect(&self, path: &Path) -> Result<Option<GitFacts>, Error> {
        if git_marker(path) == GitMarker::None {
            return Ok(None);
        }
        let Ok(repo) = Repository::open(path) else {
            return Ok(None);
        };
        if repo.is_bare() {
            return Ok(None);
        }
        Ok(Some(facts_from_repo(path, &repo)))
    }

    fn list_worktree_paths(&self, path: &Path) -> Result<Vec<PathBuf>, Error> {
        let Ok(repo) = Repository::open(path) else {
            return Ok(vec![]);
        };
        let names = repo.worktrees()?;
        Ok(names
            .iter()
            .flatten()
            .flatten()
            .filter_map(|name| repo.find_worktree(name).ok())
            .map(|wt| wt.path().to_path_buf())
            .collect())
    }

    fn discover_common_dir(&self, path: &Path) -> Option<PathBuf> {
        let repo = Repository::discover(path).ok()?;
        Some(canonicalize_lossy(repo.commondir()))
    }

    fn prune_worktree(&self, common_dir: &Path, checkout: &Path) -> Result<(), Error> {
        let repo = Repository::open(common_dir)?;
        let want = canonicalize_lossy(checkout);
        for name in repo.worktrees()?.iter().flatten().flatten() {
            let Ok(wt) = repo.find_worktree(name) else {
                continue;
            };
            if canonicalize_lossy(wt.path()) != want && wt.path() != checkout {
                continue;
            }
            if wt.validate().is_ok() {
                return Err(Error::git("checkout still on disk; refusing to prune"));
            }
            wt.prune(None)?;
            return Ok(());
        }
        Ok(())
    }
}

/// Test double. Paths missing from `facts` are not checkouts.
#[derive(Debug, Default)]
pub struct MapGitProbe {
    pub facts: HashMap<PathBuf, GitFacts>,
    pub worktrees: HashMap<PathBuf, Vec<PathBuf>>,
    pub commons: HashMap<PathBuf, PathBuf>,
    pub inspect_fail: bool,
    pub prune_fail: bool,
    pub pruned: Mutex<Vec<PathBuf>>,
}

impl GitProbe for MapGitProbe {
    fn inspect(&self, path: &Path) -> Result<Option<GitFacts>, Error> {
        if self.inspect_fail {
            return Err(Error::git("forced inspect fail"));
        }
        Ok(self.facts.get(path).cloned())
    }

    fn list_worktree_paths(&self, path: &Path) -> Result<Vec<PathBuf>, Error> {
        Ok(self.worktrees.get(path).cloned().unwrap_or_default())
    }

    fn discover_common_dir(&self, path: &Path) -> Option<PathBuf> {
        path.ancestors().find_map(|p| self.commons.get(p)).cloned()
    }

    fn prune_worktree(&self, _common_dir: &Path, checkout: &Path) -> Result<(), Error> {
        if self.prune_fail {
            return Err(Error::git("forced prune fail"));
        }
        if let Ok(mut g) = self.pruned.lock() {
            g.push(checkout.to_path_buf());
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use git2::Signature;
    use std::fs;

    pub fn commit_file(repo: &Repository, dir: &Path, name: &str, body: &str, msg: &str) {
        let sig = Signature::now("huskmap", "huskmap@test").unwrap();
        fs::write(dir.join(name), body).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new(name)).unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let parents: Vec<git2::Commit> = repo
            .head()
            .ok()
            .and_then(|h| h.peel_to_commit().ok())
            .into_iter()
            .collect();
        let refs: Vec<&git2::Commit> = parents.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &refs)
            .unwrap();
    }

    pub fn init_repo(dir: &Path) -> Repository {
        fs::create_dir_all(dir).unwrap();
        let repo = Repository::init(dir).unwrap();
        commit_file(&repo, dir, "README", "x", "init");
        repo
    }

    #[test]
    fn markers() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        assert_eq!(git_marker(root), GitMarker::None);
        fs::create_dir_all(root.join("p/.git")).unwrap();
        assert_eq!(git_marker(&root.join("p")), GitMarker::Primary);
        let cases = [
            ("l", "gitdir: /r/.git/worktrees/l\n", GitMarker::Linked),
            ("w", "gitdir: C:\\r\\.git\\worktrees\\w", GitMarker::Linked),
            ("s", "gitdir: ../.git/modules/s", GitMarker::Submodule),
            ("o", "gitdir: /elsewhere", GitMarker::None),
            ("g", "garbage", GitMarker::None),
        ];
        for (name, body, want) in cases {
            fs::create_dir_all(root.join(name)).unwrap();
            fs::write(root.join(name).join(".git"), body).unwrap();
            assert_eq!(git_marker(&root.join(name)), want, "{name}");
        }
    }

    #[test]
    fn find_checkouts_descends_into_slots() {
        let tmp = tempfile::tempdir().unwrap();
        let slot = tmp.path().join("slot");
        fs::create_dir_all(slot.join("a/repo/.git")).unwrap();
        fs::create_dir_all(slot.join("b/.git")).unwrap();
        fs::create_dir_all(slot.join("node_modules/x/.git")).unwrap();
        fs::create_dir_all(slot.join("c/d/e/f/.git")).unwrap();
        fs::write(slot.join("file"), "x").unwrap();
        let found = find_checkouts(&slot, 3);
        assert_eq!(found, vec![slot.join("a/repo"), slot.join("b")]);
        assert!(find_checkouts(&tmp.path().join("missing"), 2).is_empty());
        assert_eq!(find_checkouts(&slot.join("b"), 0), vec![slot.join("b")]);
        assert!(find_checkouts(&slot.join("a"), 0).is_empty());
    }

    #[test]
    fn merge_is_worst_case() {
        assert!(merge_facts(vec![]).is_none());
        let clean = GitFacts {
            is_worktree: true,
            checkout: PathBuf::from("/a"),
            last_commit_ms: Some(5),
            ..Default::default()
        };
        assert_eq!(merge_facts(vec![clean.clone()]), Some(clean.clone()));
        let dirty = GitFacts {
            is_worktree: false,
            is_primary: true,
            dirty: true,
            unpushed: true,
            locked: true,
            dirty_files: 3,
            stranded_commits: 2,
            last_commit_ms: Some(9),
            lock_reason: Some("agent".into()),
            upstream: Some("origin/x".into()),
            ..Default::default()
        };
        let m = merge_facts(vec![clean, dirty]).unwrap();
        assert!(m.is_primary && m.dirty && m.unpushed && m.locked && !m.is_worktree);
        assert_eq!((m.dirty_files, m.stranded_commits), (3, 2));
        assert_eq!(m.last_commit_ms, Some(9));
        assert_eq!(m.lock_reason.as_deref(), Some("agent"));
        assert_eq!(m.upstream.as_deref(), Some("origin/x"));
        assert_eq!(m.checkout, PathBuf::from("/a"));
    }

    #[test]
    fn inspect_non_repo_and_bare_marker() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(Git2Probe.inspect(tmp.path()).unwrap().is_none());
        fs::create_dir_all(tmp.path().join("fake/.git")).unwrap();
        assert!(
            Git2Probe
                .inspect(&tmp.path().join("fake"))
                .unwrap()
                .is_none()
        );
        let bare = tmp.path().join("bare");
        Repository::init_bare(bare.join(".git")).unwrap();
        assert!(Git2Probe.inspect(&bare).unwrap().is_none());
    }

    #[test]
    fn primary_dirty_and_stranded() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("repo");
        let repo = init_repo(&dir);
        fs::write(dir.join("dirty.txt"), "nope").unwrap();
        let facts = Git2Probe.inspect(&dir).unwrap().unwrap();
        assert!(facts.is_primary && !facts.is_worktree);
        assert!(facts.dirty);
        assert_eq!(facts.dirty_files, 1);
        assert_eq!(facts.stranded_commits, 1, "single branch: only copy");
        assert!(facts.unpushed);
        assert_eq!(facts.head_summary.as_deref(), Some("init"));
        assert!(facts.last_commit_ms.is_some());
        assert!(facts.upstream.is_none());
        assert!(!facts.locked);
        assert_eq!(facts.checkout, dir);
        // a tag makes the commit safe elsewhere
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        repo.tag_lightweight("keep", head.as_object(), false)
            .unwrap();
        let facts = Git2Probe.inspect(&dir).unwrap().unwrap();
        assert_eq!(facts.stranded_commits, 0);
    }

    #[test]
    fn linked_worktree_merged_vs_stranded() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("repo");
        let wt_dir = tmp.path().join("wt");
        let repo = init_repo(&dir);
        repo.worktree("agent-one", &wt_dir, None).unwrap();
        let listed = Git2Probe.list_worktree_paths(&dir).unwrap();
        assert!(
            listed
                .iter()
                .any(|p| canonicalize_lossy(p) == canonicalize_lossy(&wt_dir))
        );
        let facts = Git2Probe.inspect(&wt_dir).unwrap().unwrap();
        assert!(facts.is_worktree && !facts.is_primary);
        assert_eq!(facts.branch.as_deref(), Some("agent-one"));
        assert_eq!(facts.stranded_commits, 0, "same commit as main");
        assert!(!facts.dirty);
        assert_eq!(facts.common_dir, canonicalize_lossy(&dir.join(".git")));

        let wt_repo = Repository::open(&wt_dir).unwrap();
        commit_file(&wt_repo, &wt_dir, "work.txt", "w", "agent work");
        commit_file(&wt_repo, &wt_dir, "more.txt", "m", "more work");
        let facts = Git2Probe.inspect(&wt_dir).unwrap().unwrap();
        assert_eq!(facts.stranded_commits, 2);
        assert!(facts.unpushed);
        assert_eq!(facts.head_summary.as_deref(), Some("more work"));

        // a remote-tracking ref containing the work clears it
        let head = wt_repo.head().unwrap().target().unwrap();
        repo.reference("refs/remotes/origin/agent-one", head, true, "push")
            .unwrap();
        let facts = Git2Probe.inspect(&wt_dir).unwrap().unwrap();
        assert_eq!(facts.stranded_commits, 0);

        assert_eq!(
            Git2Probe.discover_common_dir(&wt_dir.join("README")),
            Some(canonicalize_lossy(&dir.join(".git")))
        );
        assert!(Git2Probe.discover_common_dir(Path::new("/")).is_none());
    }

    #[test]
    fn upstream_and_detached_head() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("repo");
        let repo = init_repo(&dir);
        let head = repo.head().unwrap().target().unwrap();
        let branch = repo.head().unwrap().shorthand().unwrap().to_string();
        repo.remote("origin", "https://example.invalid/r.git")
            .unwrap();
        repo.reference(&format!("refs/remotes/origin/{branch}"), head, true, "x")
            .unwrap();
        let mut cfg = repo.config().unwrap();
        cfg.set_str(&format!("branch.{branch}.remote"), "origin")
            .unwrap();
        cfg.set_str(
            &format!("branch.{branch}.merge"),
            &format!("refs/heads/{branch}"),
        )
        .unwrap();
        let facts = Git2Probe.inspect(&dir).unwrap().unwrap();
        assert_eq!(facts.upstream, Some(format!("origin/{branch}")));
        assert_eq!(facts.stranded_commits, 0);

        repo.set_head_detached(head).unwrap();
        let facts = Git2Probe.inspect(&dir).unwrap().unwrap();
        assert!(facts.branch.is_none());
        assert!(facts.upstream.is_none());
        assert_eq!(facts.stranded_commits, 0);
    }

    #[test]
    fn empty_repo_has_no_head() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("empty");
        Repository::init(&dir).unwrap();
        let facts = Git2Probe.inspect(&dir).unwrap().unwrap();
        assert_eq!(facts.stranded_commits, 0);
        assert!(facts.head_summary.is_none());
        assert!(facts.branch.is_none());
    }

    #[test]
    fn locked_worktree_reason() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("repo");
        let wt_dir = tmp.path().join("wt");
        let repo = init_repo(&dir);
        let wt = repo.worktree("locked", &wt_dir, None).unwrap();
        wt.lock(Some("agent at work")).unwrap();
        let facts = Git2Probe.inspect(&wt_dir).unwrap().unwrap();
        assert!(facts.locked);
        assert_eq!(facts.lock_reason.as_deref(), Some("agent at work"));
    }

    #[test]
    fn prune_after_checkout_is_gone() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("repo");
        let wt_dir = tmp.path().join("wt");
        let other = tmp.path().join("other");
        let repo = init_repo(&dir);
        repo.worktree("agent-two", &wt_dir, None).unwrap();
        repo.worktree("agent-three", &other, None).unwrap();
        let common = dir.join(".git");
        let err = Git2Probe.prune_worktree(&common, &wt_dir).unwrap_err();
        assert!(matches!(err, Error::Git(_)));
        fs::remove_dir_all(&wt_dir).unwrap();
        Git2Probe.prune_worktree(&common, &wt_dir).unwrap();
        let left = Git2Probe.list_worktree_paths(&dir).unwrap();
        assert_eq!(left.len(), 1);
        // unknown checkout is a no-op
        Git2Probe
            .prune_worktree(&common, &tmp.path().join("never"))
            .unwrap();
        assert!(
            Git2Probe
                .prune_worktree(&tmp.path().join("nope"), &wt_dir)
                .is_err()
        );
        assert!(
            Git2Probe
                .list_worktree_paths(tmp.path())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn inspect_tree_merges_nested_slot() {
        let tmp = tempfile::tempdir().unwrap();
        let slot = tmp.path().join("slot");
        let a = slot.join("a/repo");
        let b = slot.join("b");
        fs::create_dir_all(a.join(".git")).unwrap();
        fs::create_dir_all(b.join(".git")).unwrap();
        let probe = MapGitProbe {
            facts: [
                (a.clone(), GitFacts::default()),
                (
                    b.clone(),
                    GitFacts {
                        dirty: true,
                        dirty_files: 4,
                        ..Default::default()
                    },
                ),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        };
        let merged = inspect_tree(&probe, &slot).unwrap().unwrap();
        assert!(merged.dirty);
        assert_eq!(merged.dirty_files, 4);
        assert_eq!(merged.checkout, a);
        assert!(
            inspect_tree(&probe, &tmp.path().join("none"))
                .unwrap()
                .is_none()
        );
        let failing = MapGitProbe {
            inspect_fail: true,
            ..Default::default()
        };
        assert!(inspect_tree(&failing, &slot).is_err());
    }

    #[test]
    fn map_probe() {
        let probe = MapGitProbe {
            commons: [(PathBuf::from("/r"), PathBuf::from("/r/.git"))]
                .into_iter()
                .collect(),
            ..Default::default()
        };
        assert!(probe.inspect(Path::new("/x")).unwrap().is_none());
        assert!(
            probe
                .list_worktree_paths(Path::new("/x"))
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            probe.discover_common_dir(Path::new("/r/a/b")),
            Some(PathBuf::from("/r/.git"))
        );
        assert!(probe.discover_common_dir(Path::new("/q")).is_none());
        probe
            .prune_worktree(Path::new("/r/.git"), Path::new("/w"))
            .unwrap();
        assert_eq!(probe.pruned.lock().unwrap().len(), 1);
        let failing = MapGitProbe {
            prune_fail: true,
            ..Default::default()
        };
        assert!(
            failing
                .prune_worktree(Path::new("/r"), Path::new("/w"))
                .is_err()
        );
    }
}
