use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::git::{Git2Probe, GitProbe};
use crate::reclaim::{Reclaimer, TrashReclaimer};
use crate::roots::{Dirs, ScanRootKind, resolve_known_roots};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RootHealth {
    pub id: String,
    pub kind: String,
    pub path: PathBuf,
    pub exists: bool,
    pub readable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoctorReport {
    pub home: PathBuf,
    /// `wayland`, `x11` or `none`.
    pub session: String,
    pub proc_ok: bool,
    pub cache_dir: PathBuf,
    pub data_dir: PathBuf,
    pub state_dir: PathBuf,
    pub roots: Vec<RootHealth>,
    pub git_ok: bool,
    pub trash_ok: bool,
    pub notes: Vec<String>,
}

impl DoctorReport {
    pub fn ok(&self) -> bool {
        self.git_ok && self.proc_ok && self.roots.iter().any(|r| r.exists && r.readable)
    }

    pub fn to_json(&self) -> Result<String, Error> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

fn kind_label(kind: ScanRootKind) -> &'static str {
    match kind {
        ScanRootKind::AgentHome(_) => "agent",
        ScanRootKind::ProjectHome => "projects",
        ScanRootKind::Toolchain(_) => "toolchain",
        ScanRootKind::AgentCache(_) => "agent-cache",
    }
}

fn readable(path: &Path) -> bool {
    std::fs::read_dir(path).is_ok()
}

pub fn session_from(var: impl Fn(&str) -> bool) -> &'static str {
    if var("WAYLAND_DISPLAY") {
        "wayland"
    } else if var("DISPLAY") {
        "x11"
    } else {
        "none"
    }
}

pub fn doctor_with(
    dirs: &Dirs,
    proc_root: &Path,
    session: &str,
    git: &dyn GitProbe,
    trash: Option<&dyn Reclaimer>,
) -> DoctorReport {
    let roots = resolve_known_roots(dirs)
        .into_iter()
        .map(|r| RootHealth {
            id: r.spec_id.to_string(),
            kind: kind_label(r.kind).into(),
            readable: r.exists && readable(&r.path),
            path: r.path,
            exists: r.exists,
        })
        .collect::<Vec<_>>();
    let deck = crate::copy::get();
    let git_ok = git.inspect(&dirs.home).is_ok();
    let proc_ok = readable(proc_root);
    let mut notes = Vec::new();
    if !dirs.home.exists() {
        notes.push(deck.doctor_home_missing.into());
    }
    if !roots.iter().any(|r| r.exists) {
        notes.push(deck.doctor_no_roots.into());
    }
    if !proc_ok {
        notes.push(deck.doctor_no_proc.into());
    }
    if git_ok {
        notes.push(deck.doctor_git_ok.into());
    }
    DoctorReport {
        home: dirs.home.clone(),
        session: session.into(),
        proc_ok,
        cache_dir: dirs.cache.clone(),
        data_dir: dirs.data.clone(),
        state_dir: dirs.huskmap_state(),
        roots,
        git_ok,
        trash_ok: trash.is_some(),
        notes,
    }
}

pub fn doctor(dirs: &Dirs) -> DoctorReport {
    let session = session_from(|k| std::env::var_os(k).is_some_and(|v| !v.is_empty()));
    doctor_with(
        dirs,
        Path::new("/proc"),
        session,
        &Git2Probe,
        Some(&TrashReclaimer),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::MapGitProbe;
    use crate::reclaim::FsReclaimer;
    use std::fs;

    #[test]
    fn reports_existing_root() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join(".claude")).unwrap();
        let report = doctor_with(
            &Dirs::for_home(tmp.path()),
            tmp.path(),
            "wayland",
            &MapGitProbe::default(),
            Some(&FsReclaimer::default()),
        );
        assert!(report.git_ok && report.trash_ok && report.proc_ok);
        assert!(report.ok());
        let claude = report.roots.iter().find(|r| r.id == "claude").unwrap();
        assert!(claude.exists && claude.kind == "agent");
        assert!(report.roots.iter().any(|r| r.kind == "toolchain"));
        assert!(report.roots.iter().any(|r| r.kind == "projects"));
        assert!(report.roots.iter().any(|r| r.kind == "agent-cache"));
        assert!(report.to_json().unwrap().contains("claude"));
        assert_eq!(report.session, "wayland");
        assert!(report.state_dir.ends_with(".local/state/huskmap"));
    }

    #[test]
    fn missing_home_and_proc_not_ok() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("nope");
        let failing = MapGitProbe {
            inspect_fail: true,
            ..Default::default()
        };
        let report = doctor_with(&Dirs::for_home(&missing), &missing, "none", &failing, None);
        assert!(!report.ok());
        assert!(!report.trash_ok && !report.proc_ok && !report.git_ok);
        assert_eq!(report.notes.len(), 3);
    }

    #[test]
    fn sessions() {
        assert_eq!(session_from(|k| k == "WAYLAND_DISPLAY"), "wayland");
        assert_eq!(session_from(|k| k == "DISPLAY"), "x11");
        assert_eq!(session_from(|_| false), "none");
    }

    #[test]
    fn real_doctor_runs() {
        let tmp = tempfile::tempdir().unwrap();
        let report = doctor(&Dirs::for_home(tmp.path()));
        assert!(report.proc_ok, "/proc exists on Linux");
        assert!(report.trash_ok);
    }
}
