use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::error::Error;

pub trait Reclaimer: Send + Sync {
    fn send_to_trash(&self, path: &Path) -> Result<(), Error>;
}

/// Production reclaimer. Never silent-unlinks; trash failure is an error.
#[derive(Debug, Default, Clone, Copy)]
pub struct TrashReclaimer;

impl Reclaimer for TrashReclaimer {
    fn send_to_trash(&self, path: &Path) -> Result<(), Error> {
        trash::delete(path).map_err(|err| Error::Trash {
            path: path.to_path_buf(),
            message: err.to_string(),
        })
    }
}

/// Test reclaimer that actually deletes in a temp tree so tests assert observable state.
#[derive(Debug, Default)]
pub struct FsReclaimer {
    pub trashed: Mutex<Vec<PathBuf>>,
    pub fail: bool,
}

impl Reclaimer for FsReclaimer {
    fn send_to_trash(&self, path: &Path) -> Result<(), Error> {
        if self.fail {
            return Err(Error::Trash {
                path: path.to_path_buf(),
                message: "forced fail".into(),
            });
        }
        if path.is_dir() {
            std::fs::remove_dir_all(path).map_err(|e| Error::io(path, e))?;
        } else if path.exists() {
            std::fs::remove_file(path).map_err(|e| Error::io(path, e))?;
        }
        if let Ok(mut g) = self.trashed.lock() {
            g.push(path.to_path_buf());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn fs_reclaimer_removes_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("husk");
        fs::create_dir_all(dir.join("a")).unwrap();
        fs::write(dir.join("a/f"), "x").unwrap();
        let rec = FsReclaimer::default();
        rec.send_to_trash(&dir).unwrap();
        assert!(!dir.exists());
        assert_eq!(rec.trashed.lock().unwrap().len(), 1);
    }

    #[test]
    fn fs_reclaimer_removes_file() {
        let tmp = tempfile::tempdir().unwrap();
        let f = tmp.path().join("x");
        fs::write(&f, "1").unwrap();
        FsReclaimer::default().send_to_trash(&f).unwrap();
        assert!(!f.exists());
    }

    #[test]
    fn fs_reclaimer_can_fail() {
        let rec = FsReclaimer {
            fail: true,
            ..Default::default()
        };
        let err = rec.send_to_trash(Path::new("/x")).unwrap_err();
        assert!(matches!(err, Error::Trash { .. }));
    }

    #[test]
    fn missing_dir_is_ok_for_fs() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("gone");
        FsReclaimer::default().send_to_trash(&missing).unwrap();
    }
}
