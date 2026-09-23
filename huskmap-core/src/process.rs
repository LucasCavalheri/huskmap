use std::path::{Path, PathBuf};

use crate::domain::{AgentKind, Holder};
use crate::safety::canonicalize_lossy;

/// Who is working where. Isolated so scans and applies can be tested with fakes.
pub trait ProcessProbe: Send + Sync {
    /// Every process with a known working directory, except huskmap itself.
    fn snapshot(&self) -> Vec<Holder>;
}

/// Sees nobody. For tests and `--no-processes`.
#[derive(Debug, Default, Clone, Copy)]
pub struct DeadProcessProbe;

impl ProcessProbe for DeadProcessProbe {
    fn snapshot(&self) -> Vec<Holder> {
        vec![]
    }
}

/// Fixed list of holders. Test double.
#[derive(Debug, Default, Clone)]
pub struct FixedProcessProbe(pub Vec<Holder>);

impl ProcessProbe for FixedProcessProbe {
    fn snapshot(&self) -> Vec<Holder> {
        self.0.clone()
    }
}

fn holder_from(pid: u32, me: u32, name: String, argv: &[String], cwd: PathBuf) -> Option<Holder> {
    if pid == me || cwd.as_os_str().is_empty() || cwd.parent().is_none() {
        return None;
    }
    let agent = AgentKind::from_process(&name, argv);
    Some(Holder {
        pid,
        name,
        cwd,
        agent,
    })
}

/// Production probe: reads `/proc/<pid>/{cmdline,comm,cwd}`. Tests point it at a fake tree,
/// where `cwd` may also be a plain text file.
#[derive(Debug, Clone)]
pub struct ProcProcessProbe {
    pub proc_root: PathBuf,
}

impl Default for ProcProcessProbe {
    fn default() -> Self {
        Self {
            proc_root: PathBuf::from("/proc"),
        }
    }
}

fn read_cwd(dir: &Path) -> Option<PathBuf> {
    let cwd = dir.join("cwd");
    if cwd.is_symlink() {
        return std::fs::read_link(&cwd).ok();
    }
    let text = std::fs::read_to_string(&cwd).ok()?;
    let trimmed = text.trim();
    (!trimmed.is_empty()).then(|| PathBuf::from(trimmed))
}

impl ProcessProbe for ProcProcessProbe {
    fn snapshot(&self) -> Vec<Holder> {
        let Ok(entries) = std::fs::read_dir(&self.proc_root) else {
            return vec![];
        };
        let me = std::process::id();
        let mut out = Vec::new();
        for entry in entries.flatten() {
            let dir = entry.path();
            let Some(pid) = dir
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.parse::<u32>().ok())
            else {
                continue;
            };
            let Some(cwd) = read_cwd(&dir) else {
                continue;
            };
            let argv: Vec<String> = std::fs::read(dir.join("cmdline"))
                .map(|bytes| {
                    bytes
                        .split(|b| *b == 0)
                        .filter(|s| !s.is_empty())
                        .map(|s| String::from_utf8_lossy(s).into_owned())
                        .collect()
                })
                .unwrap_or_default();
            let name = std::fs::read_to_string(dir.join("comm"))
                .map(|s| s.trim().to_string())
                .ok()
                .filter(|s| !s.is_empty())
                .or_else(|| argv.first().cloned())
                .unwrap_or_default();
            if let Some(h) = holder_from(pid, me, name, &argv, cwd) {
                out.push(h);
            }
        }
        out.sort_by_key(|h| h.pid);
        out
    }
}

/// Canonical cwd per holder, computed once per scan.
pub struct HolderIndex {
    entries: Vec<(PathBuf, Holder)>,
    home: PathBuf,
}

impl HolderIndex {
    pub fn new(holders: Vec<Holder>, home: &Path) -> Self {
        Self {
            entries: holders
                .into_iter()
                .map(|h| (canonicalize_lossy(&h.cwd), h))
                .collect(),
            home: canonicalize_lossy(home),
        }
    }

    pub fn all(&self) -> impl Iterator<Item = &Holder> {
        self.entries.iter().map(|(_, h)| h)
    }

    /// Processes working at or below `path`.
    pub fn inside(&self, path: &Path) -> Vec<Holder> {
        let path = canonicalize_lossy(path);
        self.entries
            .iter()
            .filter(|(cwd, _)| cwd.starts_with(&path))
            .map(|(_, h)| h.clone())
            .collect()
    }

    /// Processes working in `project` itself or in a directory under it,
    /// but not inside `husk`. Home and its ancestors never count.
    pub fn around(&self, husk: &Path, project: &Path) -> Vec<Holder> {
        let husk = canonicalize_lossy(husk);
        let project = canonicalize_lossy(project);
        if self.home.starts_with(&project) {
            return vec![];
        }
        self.entries
            .iter()
            .filter(|(cwd, _)| cwd.starts_with(&project) && !cwd.starts_with(&husk))
            .map(|(_, h)| h.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn dead_and_fixed_probes() {
        assert!(DeadProcessProbe.snapshot().is_empty());
        let h = Holder {
            pid: 1,
            name: "x".into(),
            cwd: PathBuf::from("/x"),
            agent: None,
        };
        assert_eq!(FixedProcessProbe(vec![h.clone()]).snapshot(), vec![h]);
    }

    #[test]
    fn real_proc_sees_processes_but_not_itself() {
        let holders = ProcProcessProbe::default().snapshot();
        assert!(holders.iter().all(|h| h.pid != std::process::id()));
        assert!(holders.windows(2).all(|w| w[0].pid <= w[1].pid));
    }

    #[test]
    fn holder_from_filters() {
        let me = 7;
        assert!(holder_from(7, me, "x".into(), &[], PathBuf::from("/a")).is_none());
        assert!(holder_from(8, me, "x".into(), &[], PathBuf::new()).is_none());
        assert!(holder_from(8, me, "x".into(), &[], PathBuf::from("/")).is_none());
        let h = holder_from(8, me, "claude".into(), &[], PathBuf::from("/a")).unwrap();
        assert_eq!(h.agent, Some(AgentKind::Claude));
    }

    #[test]
    fn proc_probe_reads_fixture_tree() {
        let tmp = tempfile::tempdir().unwrap();
        let proc = tmp.path().join("proc");
        let work = tmp.path().join("work");
        fs::create_dir_all(&work).unwrap();
        // text cwd + cmdline agent
        fs::create_dir_all(proc.join("4242")).unwrap();
        fs::write(proc.join("4242/cmdline"), b"node\0/opt/codex\0").unwrap();
        fs::write(proc.join("4242/cwd"), work.to_string_lossy().as_bytes()).unwrap();
        // comm name, symlink cwd
        fs::create_dir_all(proc.join("9")).unwrap();
        fs::write(proc.join("9/comm"), "vim\n").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&work, proc.join("9/cwd")).unwrap();
        #[cfg(not(unix))]
        fs::write(proc.join("9/cwd"), work.to_string_lossy().as_bytes()).unwrap();
        // noise: no cwd, empty cwd, non-numeric, plain file
        fs::create_dir_all(proc.join("10")).unwrap();
        fs::create_dir_all(proc.join("11")).unwrap();
        fs::write(proc.join("11/cwd"), "  ").unwrap();
        fs::create_dir_all(proc.join("self")).unwrap();
        fs::write(proc.join("version"), "x").unwrap();
        // empty comm falls back to argv[0], then to nothing
        fs::create_dir_all(proc.join("12")).unwrap();
        fs::write(proc.join("12/comm"), "\n").unwrap();
        fs::write(proc.join("12/cwd"), work.to_string_lossy().as_bytes()).unwrap();

        let holders = ProcProcessProbe { proc_root: proc }.snapshot();
        let pids: Vec<u32> = holders.iter().map(|h| h.pid).collect();
        assert_eq!(pids, vec![9, 12, 4242]);
        assert_eq!(holders[0].name, "vim");
        assert_eq!(holders[0].agent, None);
        assert_eq!(holders[1].name, "");
        assert_eq!(holders[2].agent, Some(AgentKind::Codex));
        assert_eq!(holders[2].name, "node");
    }

    #[test]
    fn proc_probe_missing_root_and_default() {
        let probe = ProcProcessProbe {
            proc_root: PathBuf::from("/definitely/missing-huskmap-proc"),
        };
        assert!(probe.snapshot().is_empty());
        assert_eq!(
            ProcProcessProbe::default().proc_root,
            PathBuf::from("/proc")
        );
    }

    fn at(cwd: &Path) -> Holder {
        Holder {
            pid: 1,
            name: "sh".into(),
            cwd: cwd.to_path_buf(),
            agent: None,
        }
    }

    #[test]
    fn index_inside_and_around() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let proj = home.join("proj");
        let nm = proj.join("node_modules");
        let deep = nm.join("pkg");
        let src = proj.join("src");
        for d in [&deep, &src] {
            fs::create_dir_all(d).unwrap();
        }
        let index = HolderIndex::new(vec![at(&deep), at(&src), at(&home)], &home);
        assert_eq!(index.inside(&nm).len(), 1);
        assert_eq!(index.inside(&proj).len(), 2);
        assert_eq!(
            index.around(&nm, &proj).len(),
            1,
            "src counts, deep does not"
        );
        assert!(index.around(&proj, &home).is_empty(), "home never counts");
        assert!(index.around(&home.join("x"), tmp.path()).is_empty());
    }
}
