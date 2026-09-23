use std::fs;
use std::path::Path;

use walkdir::WalkDir;

use crate::clock::Millis;
use crate::error::Error;
use crate::safety::{is_forbidden_path, is_regenerable_dir_name, is_skip_dir};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SizeOutcome {
    pub bytes: u64,
    pub mtime_ms: Option<Millis>,
    pub contains_secrets: bool,
    /// Secret-looking files outside regenerable subtrees (`node_modules`, `target`, ...).
    pub secrets: Vec<std::path::PathBuf>,
    pub warnings: Vec<String>,
}

const MAX_SECRETS: usize = 64;

fn under_regenerable(root: &Path, p: &Path) -> bool {
    p.strip_prefix(root).is_ok_and(|rel| {
        rel.components()
            .filter_map(|c| c.as_os_str().to_str())
            .any(is_regenerable_dir_name)
    })
}

fn meta_mtime_ms(meta: &fs::Metadata) -> Option<Millis> {
    meta.modified().ok().map(crate::clock::millis_of)
}

/// Size a path without following symlinks. Secrets inside bump `contains_secrets`.
pub fn dir_size(path: &Path) -> Result<SizeOutcome, Error> {
    let meta = fs::symlink_metadata(path).map_err(|e| Error::io(path, e))?;
    if meta.file_type().is_file() || meta.file_type().is_symlink() {
        let secret = is_forbidden_path(path);
        return Ok(SizeOutcome {
            bytes: meta.len(),
            mtime_ms: meta_mtime_ms(&meta),
            contains_secrets: secret,
            secrets: if secret {
                vec![path.to_path_buf()]
            } else {
                vec![]
            },
            warnings: vec![],
        });
    }

    let mut bytes = 0u64;
    let mut mtime_ms = meta_mtime_ms(&meta);
    let mut secrets = Vec::new();

    let walker = WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| !is_skip_dir(entry.path()))
        .flatten();

    for entry in walker {
        let p = entry.path();
        if secrets.len() < MAX_SECRETS && is_forbidden_path(p) && !under_regenerable(path, p) {
            secrets.push(p.to_path_buf());
        }
        if let Ok(meta) = entry.metadata() {
            if meta.file_type().is_file() {
                bytes = bytes.saturating_add(meta.len());
            }
            if let Some(ms) = meta_mtime_ms(&meta) {
                mtime_ms = Some(mtime_ms.map_or(ms, |cur| cur.max(ms)));
            }
        }
    }

    Ok(SizeOutcome {
        bytes,
        mtime_ms,
        contains_secrets: !secrets.is_empty(),
        secrets,
        warnings: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn sizes_files_and_detects_secrets() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("husk");
        fs::create_dir_all(root.join("a")).unwrap();
        fs::write(root.join("a/one.txt"), "hello").unwrap();
        fs::write(root.join("a/.env"), "SECRET=1").unwrap();
        let out = dir_size(&root).unwrap();
        assert_eq!(out.bytes, 5 + 8);
        assert!(out.contains_secrets);
        assert!(out.mtime_ms.is_some());
    }

    #[test]
    fn secrets_inside_regenerable_subtrees_are_package_contents() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("wt");
        fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
        fs::write(root.join("node_modules/pkg/test.key"), "k").unwrap();
        let out = dir_size(&root).unwrap();
        assert!(!out.contains_secrets);
        fs::write(root.join(".env"), "S=1").unwrap();
        let out = dir_size(&root).unwrap();
        assert_eq!(out.secrets, vec![root.join(".env")]);
    }

    #[test]
    fn file_size() {
        let tmp = tempfile::tempdir().unwrap();
        let f = tmp.path().join("x");
        fs::write(&f, "abcd").unwrap();
        let out = dir_size(&f).unwrap();
        assert_eq!(out.bytes, 4);
        assert!(!out.contains_secrets);
    }

    #[test]
    fn missing_path_errors() {
        let err = dir_size(Path::new("/no/such/huskmap-path")).unwrap_err();
        assert!(matches!(err, Error::Io { .. }));
    }

    #[test]
    fn skips_git_dir_contents() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("repo");
        fs::create_dir_all(root.join(".git/objects")).unwrap();
        fs::write(root.join(".git/objects/pack"), "BIGPACKDATA").unwrap();
        fs::write(root.join("file"), "ok").unwrap();
        let out = dir_size(&root).unwrap();
        assert_eq!(out.bytes, 2);
        assert!(!out.contains_secrets);
    }

    #[test]
    fn forbidden_file_itself() {
        let tmp = tempfile::tempdir().unwrap();
        let f = tmp.path().join(".env");
        fs::write(&f, "x").unwrap();
        let out = dir_size(&f).unwrap();
        assert!(out.contains_secrets);
    }
}
