use std::path::{Path, PathBuf};

const FORBIDDEN_NAMES: &[&str] = &[
    ".env",
    ".env.local",
    ".env.production",
    ".env.development",
    "id_rsa",
    "id_dsa",
    "id_ecdsa",
    "id_ed25519",
    "authorized_keys",
    "known_hosts",
];

const FORBIDDEN_SUFFIXES: &[&str] = &[".pem", ".key"];

const SKIP_DIR_NAMES: &[&str] = &[".git", "$Recycle.Bin", "System Volume Information"];

const OS_VFS_NAMES: &[&str] = &["proc", "sys", "dev"];

/// Dirs rebuilt from a registry or a build. Secret-named files inside are package contents.
const REGENERABLE_DIRS: &[&str] = &[
    "node_modules",
    "target",
    ".venv",
    "venv",
    ".next",
    ".turbo",
    ".nuxt",
    ".output",
    ".svelte-kit",
    ".parcel-cache",
    ".tox",
    ".nox",
    ".mypy_cache",
    ".pytest_cache",
    ".ruff_cache",
    "site-packages",
];

pub fn is_regenerable_dir_name(name: &str) -> bool {
    REGENERABLE_DIRS.contains(&name)
}

pub fn file_name_str(path: &Path) -> Option<&str> {
    path.file_name().and_then(|n| n.to_str())
}

pub fn is_forbidden_name(name: &str) -> bool {
    if FORBIDDEN_NAMES.contains(&name) {
        return true;
    }
    FORBIDDEN_SUFFIXES.iter().any(|suffix| {
        name.ends_with(suffix) && name != "package-lock.json" && !name.ends_with(".lock")
    })
}

pub fn is_forbidden_path(path: &Path) -> bool {
    file_name_str(path).is_some_and(is_forbidden_name)
}

pub fn is_skip_dir_name(name: &str) -> bool {
    SKIP_DIR_NAMES.iter().any(|n| n.eq_ignore_ascii_case(name))
}

/// Skip `.git` anywhere, and OS virtual filesystems only at the volume root.
/// `~/dev` is a project home and must not be skipped.
pub fn is_skip_dir(path: &Path) -> bool {
    let Some(name) = file_name_str(path) else {
        return false;
    };
    if is_skip_dir_name(name) {
        return true;
    }
    if OS_VFS_NAMES.iter().any(|n| n.eq_ignore_ascii_case(name)) {
        return path.parent().is_some_and(|p| p.parent().is_none());
    }
    false
}

pub fn canonicalize_lossy(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

pub fn is_within_roots(path: &Path, roots: &[PathBuf]) -> bool {
    if roots.is_empty() {
        return false;
    }
    let canon = canonicalize_lossy(path);
    roots.iter().any(|root| {
        let root = canonicalize_lossy(root);
        canon == root || canon.starts_with(&root)
    })
}

pub fn path_has_component(path: &Path, name: &str) -> bool {
    path.components().any(|c| c.as_os_str() == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secrets_are_forbidden() {
        assert!(is_forbidden_path(Path::new("/proj/.env")));
        assert!(is_forbidden_path(Path::new("/home/me/.ssh/id_ed25519")));
        assert!(is_forbidden_path(Path::new("/x/secret.pem")));
        assert!(is_forbidden_path(Path::new("/x/tls.key")));
        assert!(!is_forbidden_path(Path::new("/x/Cargo.toml")));
        assert!(!is_forbidden_path(Path::new("/x/package-lock.json")));
    }

    #[test]
    fn skip_git_and_proc() {
        assert!(is_skip_dir_name(".git"));
        assert!(is_skip_dir(Path::new("/repo/.git")));
        assert!(is_skip_dir(Path::new("/proc")));
        assert!(is_skip_dir(Path::new("/dev")));
        assert!(!is_skip_dir(Path::new("/home/me/dev")));
        assert!(!is_skip_dir_name("src"));
        assert!(!is_skip_dir(Path::new("/home/me/src")));
    }

    #[test]
    fn within_roots_is_componentwise() {
        let roots = vec![PathBuf::from("/home/me/dev")];
        assert!(is_within_roots(Path::new("/home/me/dev/app"), &roots));
        assert!(!is_within_roots(Path::new("/home/me/devious"), &roots));
        assert!(!is_within_roots(Path::new("/home/me/dev/app"), &[]));
    }

    #[test]
    fn file_name_and_component() {
        assert_eq!(file_name_str(Path::new("/a/b")), Some("b"));
        assert!(path_has_component(
            Path::new("/home/me/.claude/x"),
            ".claude"
        ));
        assert!(!path_has_component(Path::new("/home/me/x"), ".claude"));
    }

    #[test]
    fn regenerable_names() {
        assert!(is_regenerable_dir_name("node_modules"));
        assert!(is_regenerable_dir_name(".venv"));
        assert!(!is_regenerable_dir_name("src"));
    }

    #[test]
    fn canonicalize_lossy_missing() {
        let p = Path::new("/definitely/not/here-huskmap");
        assert_eq!(canonicalize_lossy(p), p);
    }
}
