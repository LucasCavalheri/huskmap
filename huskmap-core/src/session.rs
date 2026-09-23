//! What a running huskmap tells the rest of the machine, in `$XDG_STATE_HOME/huskmap/`.
//!
//! - `session.json`: the open window's phase and marks. Marks survive a restart or an update.
//! - `apply.lock`: held while husks move to the trash. Installers and a second huskmap wait
//!   for it; nobody interrupts an apply.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::clock::Millis;
use crate::error::Error;
use crate::ids::HuskId;

/// Is `pid` a live (non-zombie) process? Reads `<proc_root>/<pid>/stat`.
pub fn pid_alive(proc_root: &Path, pid: u32) -> bool {
    let Ok(stat) = std::fs::read_to_string(proc_root.join(pid.to_string()).join("stat")) else {
        return false;
    };
    // Field 3, after the parenthesized command name, is the state.
    stat.rsplit_once(") ")
        .and_then(|(_, rest)| rest.split_whitespace().next())
        .is_some_and(|state| state != "Z" && state != "X")
}

fn write_atomic(path: &Path, text: &str) -> Result<(), Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, text).map_err(|e| Error::io(&tmp, e))?;
    std::fs::rename(&tmp, path).map_err(|e| Error::io(path, e))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SessionPhase {
    #[default]
    Idle,
    Scanning,
    Ready,
    Applying,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Session {
    pub pid: u32,
    pub version: String,
    pub phase: SessionPhase,
    pub marked: Vec<HuskId>,
    pub forced: Vec<HuskId>,
    /// What the marks would free, for the installer's warning.
    pub marked_bytes: u64,
    pub updated_at_ms: Millis,
}

impl Session {
    pub fn path(state_dir: &Path) -> PathBuf {
        state_dir.join("session.json")
    }

    pub fn load(state_dir: &Path) -> Option<Self> {
        let text = std::fs::read_to_string(Self::path(state_dir)).ok()?;
        serde_json::from_str(&text).ok()
    }

    pub fn save(&self, state_dir: &Path) -> Result<(), Error> {
        write_atomic(&Self::path(state_dir), &serde_json::to_string_pretty(self)?)
    }

    /// Is the window that wrote this still open?
    pub fn is_live(&self, proc_root: &Path) -> bool {
        self.pid != 0 && pid_alive(proc_root, self.pid)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockInfo {
    pub pid: u32,
    pub husks: usize,
    pub bytes: u64,
    pub started_at_ms: Millis,
}

/// Held for the duration of an apply. Dropping it releases the lock.
#[derive(Debug)]
pub struct ApplyLock {
    path: PathBuf,
}

impl ApplyLock {
    pub fn path(state_dir: &Path) -> PathBuf {
        state_dir.join("apply.lock")
    }

    /// The live apply holding the lock, if any. A lock left by a dead process is stale.
    pub fn holder(state_dir: &Path, proc_root: &Path) -> Option<LockInfo> {
        let text = std::fs::read_to_string(Self::path(state_dir)).ok()?;
        let info: LockInfo = serde_json::from_str(&text).ok()?;
        pid_alive(proc_root, info.pid).then_some(info)
    }

    pub fn acquire(
        state_dir: &Path,
        proc_root: &Path,
        husks: usize,
        bytes: u64,
        now_ms: Millis,
    ) -> Result<Self, Error> {
        let path = Self::path(state_dir);
        std::fs::create_dir_all(state_dir).map_err(|e| Error::io(state_dir, e))?;
        if let Some(info) = Self::holder(state_dir, proc_root) {
            return Err(Error::safety(
                crate::copy::get()
                    .apply_busy
                    .replace("{pid}", &info.pid.to_string()),
            ));
        }
        // Stale or unreadable: clear it, then create exclusively so two racers cannot both win.
        let _ = std::fs::remove_file(&path);
        let info = LockInfo {
            pid: std::process::id(),
            husks,
            bytes,
            started_at_ms: now_ms,
        };
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| Error::io(&path, e))?;
        file.write_all(serde_json::to_string(&info)?.as_bytes())
            .map_err(|e| Error::io(&path, e))?;
        Ok(Self { path })
    }
}

impl Drop for ApplyLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::HuskKind;
    use std::fs;

    fn fake_proc(root: &Path, pid: u32, state: &str) {
        fs::create_dir_all(root.join(pid.to_string())).unwrap();
        fs::write(
            root.join(pid.to_string()).join("stat"),
            format!("{pid} (husk map) {state} 1 2 3"),
        )
        .unwrap();
    }

    #[test]
    fn pid_liveness_from_stat() {
        let tmp = tempfile::tempdir().unwrap();
        fake_proc(tmp.path(), 10, "S");
        fake_proc(tmp.path(), 11, "Z");
        fs::create_dir_all(tmp.path().join("12")).unwrap();
        fs::write(tmp.path().join("12/stat"), "garbage").unwrap();
        assert!(pid_alive(tmp.path(), 10));
        assert!(!pid_alive(tmp.path(), 11), "zombies are dead");
        assert!(!pid_alive(tmp.path(), 12));
        assert!(!pid_alive(tmp.path(), 13));
        assert!(pid_alive(Path::new("/proc"), std::process::id()));
    }

    #[test]
    fn session_roundtrip_and_liveness() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(Session::load(tmp.path()).is_none());
        let s = Session {
            pid: std::process::id(),
            version: "0.1.0".into(),
            phase: SessionPhase::Ready,
            marked: vec![HuskId::new(HuskKind::Cache, "/c")],
            forced: vec![],
            marked_bytes: 9,
            updated_at_ms: 1,
        };
        s.save(&tmp.path().join("deep")).unwrap();
        let back = Session::load(&tmp.path().join("deep")).unwrap();
        assert_eq!(back, s);
        assert!(back.is_live(Path::new("/proc")));
        assert!(!Session::default().is_live(Path::new("/proc")));
        fs::write(Session::path(tmp.path()), "nope").unwrap();
        assert!(Session::load(tmp.path()).is_none());
        let blocked = tmp.path().join("file");
        fs::write(&blocked, "x").unwrap();
        assert!(s.save(&blocked).is_err());
    }

    #[test]
    fn apply_lock_is_exclusive_and_self_cleaning() {
        let tmp = tempfile::tempdir().unwrap();
        let state = tmp.path().join("state");
        let proc = Path::new("/proc");
        let lock = ApplyLock::acquire(&state, proc, 3, 100, 7).unwrap();
        let held = ApplyLock::holder(&state, proc).unwrap();
        assert_eq!(
            (held.pid, held.husks, held.bytes),
            (std::process::id(), 3, 100)
        );
        let err = ApplyLock::acquire(&state, proc, 1, 1, 8).unwrap_err();
        assert!(matches!(err, Error::Safety(_)));
        drop(lock);
        assert!(ApplyLock::holder(&state, proc).is_none());
        assert!(!ApplyLock::path(&state).exists());
        // a lock from a dead pid is stale and gets replaced
        let fake = tmp.path().join("proc");
        fs::create_dir_all(&fake).unwrap();
        fs::write(
            ApplyLock::path(&state),
            r#"{"pid":999999,"husks":1,"bytes":1,"started_at_ms":1}"#,
        )
        .unwrap();
        assert!(ApplyLock::holder(&state, &fake).is_none());
        let again = ApplyLock::acquire(&state, &fake, 2, 2, 9).unwrap();
        drop(again);
        // unwritable state dir
        let blocked = tmp.path().join("blocked");
        fs::write(&blocked, "x").unwrap();
        assert!(ApplyLock::acquire(&blocked, proc, 1, 1, 1).is_err());
    }
}
