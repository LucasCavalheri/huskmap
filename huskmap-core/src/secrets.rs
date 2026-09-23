//! Which secret-looking files would actually be lost if a husk went away.

use std::path::{Path, PathBuf};

use crate::domain::{GitFacts, HuskKind};
use crate::safety::file_name_str;

fn primary_of(facts: &GitFacts) -> Option<PathBuf> {
    if !facts.is_worktree || file_name_str(&facts.common_dir) != Some(".git") {
        return None;
    }
    facts.common_dir.parent().map(Path::to_path_buf)
}

/// Same bytes at the same relative path in the primary checkout.
fn is_copy(secret: &Path, checkout: &Path, primary: &Path) -> bool {
    let Ok(rel) = secret.strip_prefix(checkout) else {
        return false;
    };
    match (std::fs::read(secret), std::fs::read(primary.join(rel))) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// Secrets that exist only inside this husk.
///
/// Regenerable husks (ballast, toolchain and agent caches) never guard: a `test.key` inside
/// `node_modules` is package content. A worktree's `.env` guards unless it is a byte-for-byte
/// copy of the primary checkout's `.env`, which is what agents usually do.
pub fn guarding_secrets(
    kind: HuskKind,
    root: &Path,
    found: &[PathBuf],
    facts: Option<&GitFacts>,
) -> Vec<PathBuf> {
    if matches!(
        kind,
        HuskKind::Ballast | HuskKind::Toolchain | HuskKind::Cache
    ) {
        return vec![];
    }
    let primary = facts.and_then(primary_of);
    let checkout = facts
        .map(|f| f.checkout.as_path())
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(root);
    found
        .iter()
        .filter(|s| !primary.as_ref().is_some_and(|p| is_copy(s, checkout, p)))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn copies_of_primary_do_not_guard() {
        let tmp = tempfile::tempdir().unwrap();
        let primary = tmp.path().join("app");
        let wt = tmp.path().join("wt");
        fs::create_dir_all(primary.join(".git")).unwrap();
        fs::create_dir_all(&wt).unwrap();
        fs::write(primary.join(".env"), "A=1").unwrap();
        fs::write(wt.join(".env"), "A=1").unwrap();
        fs::write(wt.join(".env.local"), "only here").unwrap();
        fs::write(primary.join("id_rsa"), "old").unwrap();
        fs::write(wt.join("id_rsa"), "new").unwrap();
        let facts = GitFacts {
            is_worktree: true,
            common_dir: primary.join(".git"),
            checkout: wt.clone(),
            ..Default::default()
        };
        let found = vec![wt.join(".env"), wt.join(".env.local"), wt.join("id_rsa")];
        let guard = guarding_secrets(HuskKind::Worktree, &wt, &found, Some(&facts));
        assert_eq!(guard, vec![wt.join(".env.local"), wt.join("id_rsa")]);
        // outside the checkout (agent slot root) always guards
        let stray = tmp.path().join("slot.env");
        assert_eq!(
            guarding_secrets(
                HuskKind::Worktree,
                &wt,
                std::slice::from_ref(&stray),
                Some(&facts)
            ),
            vec![stray]
        );
    }

    #[test]
    fn without_primary_everything_guards_and_regenerable_nothing() {
        let found = vec![PathBuf::from("/x/.env")];
        assert_eq!(
            guarding_secrets(HuskKind::Afterimage, Path::new("/x"), &found, None),
            found
        );
        let primary = GitFacts {
            is_primary: true,
            common_dir: PathBuf::from("/x/.git"),
            ..Default::default()
        };
        assert_eq!(
            guarding_secrets(HuskKind::Worktree, Path::new("/x"), &found, Some(&primary)),
            found
        );
        let bare = GitFacts {
            is_worktree: true,
            common_dir: PathBuf::from("/r/repo.git"),
            ..Default::default()
        };
        assert_eq!(
            guarding_secrets(HuskKind::Worktree, Path::new("/x"), &found, Some(&bare)).len(),
            1
        );
        for kind in [HuskKind::Ballast, HuskKind::Toolchain, HuskKind::Cache] {
            assert!(guarding_secrets(kind, Path::new("/x"), &found, None).is_empty());
        }
    }
}
