use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc;

use rayon::prelude::*;
use walkdir::WalkDir;

use crate::classify::{Classification, ClassifyContext, HuskSeed, classify_path};
use crate::clock::{Clock, Millis, SystemClock};
use crate::domain::{
    Afterimage, AgentKind, Ballast, Ecosystem, Grove, Husk, HuskKind, RECENT_MS, REPORT_VERSION,
    Risk, ScanReport, Totals, Ward,
};
use crate::error::Error;
use crate::git::{Git2Probe, GitMarker, GitProbe, git_marker, inspect_tree};
use crate::ids::HuskId;
use crate::process::{HolderIndex, ProcProcessProbe, ProcessProbe};
use crate::roots::{Dirs, ScanRootKind, agent_homes, dedupe_nested, existing_roots};
use crate::safety::{canonicalize_lossy, file_name_str, is_within_roots};
use crate::secrets::guarding_secrets;
use crate::size::dir_size;

pub struct ScanOptions {
    /// Explicit roots. When empty, the known table is used.
    pub extra_roots: Vec<PathBuf>,
    pub min_size: u64,
    pub category: Option<HuskKind>,
    pub home: PathBuf,
    pub dirs: Dirs,
    pub git: Arc<dyn GitProbe>,
    pub processes: Arc<dyn ProcessProbe>,
    pub clock: Arc<dyn Clock>,
}

impl ScanOptions {
    /// XDG defaults for `home`, ignoring the environment. Views use [`ScanOptions::for_dirs`].
    pub fn for_home(home: PathBuf) -> Self {
        Self::for_dirs(Dirs::for_home(home))
    }

    pub fn for_dirs(dirs: Dirs) -> Self {
        Self {
            extra_roots: vec![],
            min_size: 0,
            category: None,
            home: dirs.home.clone(),
            dirs,
            git: Arc::new(Git2Probe),
            processes: Arc::new(ProcProcessProbe::default()),
            clock: Arc::new(SystemClock),
        }
    }

    pub fn scan_roots(&self) -> Vec<ScanRoot> {
        if self.extra_roots.is_empty() {
            return existing_roots(&self.dirs)
                .into_iter()
                .map(|r| ScanRoot {
                    path: r.path,
                    label: r.label.to_string(),
                    kind: r.kind,
                })
                .collect();
        }
        dedupe_nested(
            self.extra_roots
                .iter()
                .filter(|p| p.is_dir())
                .cloned()
                .collect(),
        )
        .into_iter()
        .map(|path| ScanRoot {
            label: path.display().to_string(),
            path,
            kind: ScanRootKind::ProjectHome,
        })
        .collect()
    }

    pub fn roots(&self) -> Vec<PathBuf> {
        self.scan_roots().into_iter().map(|r| r.path).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanRoot {
    pub path: PathBuf,
    pub label: String,
    pub kind: ScanRootKind,
}

/// Progress for live views.
#[derive(Debug, Clone, PartialEq)]
pub enum ScanEvent {
    /// About to walk a root.
    Walking {
        root: PathBuf,
        index: usize,
        total: usize,
    },
    /// Walk done; `pending` husks are being weighed.
    Weighing { pending: usize },
    /// One husk fully weighed and judged.
    Found(Box<Husk>),
}

struct Pending {
    path: PathBuf,
    seed: HuskSeed,
    outside: bool,
}

fn toolchain_seed(root: &ScanRoot) -> Option<HuskSeed> {
    let deck = crate::copy::get();
    let (kind, eco, agent, note): (HuskKind, Option<Ecosystem>, Option<AgentKind>, String) =
        match root.kind {
            ScanRootKind::Toolchain(eco) => (
                HuskKind::Toolchain,
                Some(eco),
                None,
                deck.note_toolchain.replace("{tool}", &root.label),
            ),
            ScanRootKind::AgentCache(agent) => (
                HuskKind::Cache,
                None,
                Some(agent),
                deck.note_cache.to_string(),
            ),
            ScanRootKind::AgentHome(_) | ScanRootKind::ProjectHome => return None,
        };
    Some(HuskSeed {
        kind,
        agent,
        ballast: None,
        ecosystem: eco,
        marker: None,
        risk: Risk::Safe,
        reclaimable: true,
        notes: vec![note],
        skip_children: true,
        no_git: false,
    })
}

/// Claude Code names a project dir after its cwd with every non-alphanumeric byte as `-`.
pub fn claude_project_key(cwd: &Path) -> String {
    cwd.to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// Find the directory a Claude project key was made from, walking the filesystem by prefix.
pub fn resolve_claude_origin(key: &str, fs_root: &Path) -> Option<PathBuf> {
    fn go(dir: &Path, key: &str, budget: &mut usize) -> Option<PathBuf> {
        if claude_project_key(dir) == key {
            return Some(dir.to_path_buf());
        }
        let entries = std::fs::read_dir(dir).ok()?;
        for entry in entries.flatten() {
            if *budget == 0 {
                return None;
            }
            *budget -= 1;
            let child = entry.path();
            if !entry.file_type().is_ok_and(|t| t.is_dir()) {
                continue;
            }
            let k = claude_project_key(&child);
            let fits = key.starts_with(&k)
                && (key.len() == k.len() || key.as_bytes().get(k.len()) == Some(&b'-'));
            if fits && let Some(found) = go(&child, key, budget) {
                return Some(found);
            }
        }
        None
    }
    let mut budget = 20_000;
    go(fs_root, key, &mut budget)
}

struct Judge<'a> {
    opts: &'a ScanOptions,
    holders: &'a HolderIndex,
    roots: &'a [PathBuf],
    now: Millis,
}

impl Judge<'_> {
    fn weigh(&self, p: Pending) -> Result<Husk, String> {
        let size = dir_size(&p.path).map_err(|e| format!("size {}: {e}", p.path.display()))?;
        let seed = p.seed;
        let kind = seed.kind;
        let mut husk = Husk {
            id: HuskId::new(kind, p.path.clone()),
            kind,
            path: p.path,
            size_bytes: size.bytes,
            mtime_ms: size.mtime_ms,
            risk: seed.risk,
            reclaimable: seed.reclaimable,
            notes: seed.notes,
            agent: seed.agent,
            ballast: seed.ballast,
            ecosystem: seed.ecosystem,
            git: None,
            grove: None,
            parent: None,
            contains_secrets: false,
            live: false,
            wards: vec![],
            project: None,
        };
        if p.outside || !is_within_roots(&husk.path, self.roots) {
            husk.ward(Ward::OutsideRoots);
        }
        let inside = self.holders.inside(&husk.path);
        if !inside.is_empty() {
            husk.ward(Ward::Occupied { holders: inside });
        }
        let minutes = husk
            .age_ms(self.now)
            .filter(|age| *age < RECENT_MS)
            .map(|age| age / 60_000);
        match kind {
            HuskKind::Worktree => self.judge_worktree(&mut husk, seed.no_git),
            HuskKind::Ballast => self.judge_ballast(&mut husk),
            HuskKind::Afterimage => self.judge_afterimage(&mut husk, minutes.is_some()),
            HuskKind::Toolchain | HuskKind::Cache | HuskKind::Debris => {}
        }
        if !guarding_secrets(kind, &husk.path, &size.secrets, husk.git.as_ref()).is_empty() {
            husk.contains_secrets = true;
            husk.ward(Ward::Secrets);
        }
        if let Some(minutes) = minutes
            && matches!(kind, HuskKind::Worktree | HuskKind::Afterimage)
        {
            husk.ward(Ward::Warm { minutes });
        }
        Ok(husk)
    }

    fn judge_worktree(&self, husk: &mut Husk, no_git: bool) {
        let facts = if no_git {
            None
        } else {
            match inspect_tree(self.opts.git.as_ref(), &husk.path) {
                Ok(f) => f,
                Err(err) => {
                    husk.notes.push(format!("git: {err}"));
                    None
                }
            }
        };
        let Some(facts) = facts else {
            husk.ward(Ward::NoGit);
            return;
        };
        if facts.is_primary {
            husk.ward(Ward::Primary);
        }
        if facts.dirty {
            husk.ward(Ward::Dirty {
                files: facts.dirty_files.max(1),
            });
        }
        if facts.stranded_commits > 0 {
            husk.ward(Ward::Stranded {
                commits: facts.stranded_commits,
            });
        }
        if facts.locked {
            husk.ward(Ward::Locked {
                reason: facts.lock_reason.clone(),
            });
        }
        husk.grove = Some(facts.common_dir.clone());
        husk.git = Some(facts);
    }

    fn judge_ballast(&self, husk: &mut Husk) {
        let Some(project) = husk.path.parent().map(Path::to_path_buf) else {
            return;
        };
        husk.grove = self.opts.git.discover_common_dir(&project);
        let around = self.holders.around(&husk.path, &project);
        if !around.is_empty() {
            husk.ward(Ward::NeighborBusy { holders: around });
        }
    }

    fn judge_afterimage(&self, husk: &mut Husk, warm: bool) {
        let parent = husk.path.parent().and_then(file_name_str).unwrap_or("");
        let name = file_name_str(&husk.path).unwrap_or("").to_string();
        let agent_holders = || self.holders.all().filter(|h| h.agent.is_some());
        let (owners, origin): (Vec<_>, Option<Option<PathBuf>>) = match (husk.agent, parent) {
            (Some(AgentKind::Claude), "projects") => {
                let owners = agent_holders()
                    .filter(|h| claude_project_key(&h.cwd) == name)
                    .cloned()
                    .collect();
                let fs_root = self.opts.home.ancestors().last().unwrap_or(Path::new("/"));
                let origin = resolve_claude_origin(&name, fs_root);
                husk.project = origin.clone();
                (owners, Some(origin))
            }
            (Some(AgentKind::Grok), "sessions") => match percent_decode(&name) {
                Some(origin) => {
                    let want = canonicalize_lossy(&origin);
                    let owners = agent_holders()
                        .filter(|h| canonicalize_lossy(&h.cwd) == want)
                        .cloned()
                        .collect();
                    husk.project = Some(origin.clone());
                    (owners, Some(origin.is_dir().then_some(origin)))
                }
                None => (vec![], None),
            },
            _ => (vec![], None),
        };
        if !owners.is_empty() {
            husk.ward(Ward::Occupied { holders: owners });
        } else if let Some(None) = origin {
            husk.ward(Ward::Orphaned { origin: None });
        }
        if warm {
            // A session written minutes ago is probably still being written.
            husk.live = true;
            husk.reclaimable = false;
        }
    }
}

/// `%2Fhome%2Fme` → `/home/me`. `None` unless it decodes to an absolute path.
pub fn percent_decode(raw: &str) -> Option<PathBuf> {
    if !raw.contains('%') {
        return None;
    }
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    let path = PathBuf::from(String::from_utf8(out).ok()?);
    path.is_absolute().then_some(path)
}

fn link_parents(husks: &mut [Husk]) {
    let mut by_path: Vec<(PathBuf, HuskId)> = husks
        .iter()
        .map(|h| (h.path.clone(), h.id.clone()))
        .collect();
    by_path.sort();
    for husk in husks.iter_mut() {
        husk.parent = husk.path.ancestors().skip(1).find_map(|a| {
            by_path
                .binary_search_by(|(p, _)| p.as_path().cmp(a))
                .ok()
                .map(|i| by_path[i].1.clone())
        });
    }
}

fn has_reclaimable_ancestor(husk: &Husk, by_id: &BTreeMap<HuskId, &Husk>) -> bool {
    let mut cur = husk.parent.as_ref();
    while let Some(id) = cur {
        match by_id.get(id) {
            Some(parent) if parent.reclaimable => return true,
            Some(parent) => cur = parent.parent.as_ref(),
            None => break,
        }
    }
    false
}

fn grove_name(common: &Path) -> String {
    let base = if file_name_str(common) == Some(".git") {
        common.parent().unwrap_or(common)
    } else {
        common
    };
    file_name_str(base).unwrap_or("repo").to_string()
}

fn build_groves(husks: &[Husk]) -> Vec<Grove> {
    let mut map: BTreeMap<PathBuf, Grove> = BTreeMap::new();
    for husk in husks {
        let Some(key) = husk.grove.clone() else {
            continue;
        };
        let grove = map.entry(key.clone()).or_insert_with(|| Grove {
            name: grove_name(&key),
            primary: (file_name_str(&key) == Some(".git"))
                .then(|| key.parent().map(Path::to_path_buf))
                .flatten(),
            id: key,
            worktrees: vec![],
            husks: vec![],
            bytes: 0,
        });
        grove.husks.push(husk.id.clone());
        if husk.kind == HuskKind::Worktree || husk.parent.is_none() {
            grove.bytes += husk.size_bytes;
        }
        if husk.kind == HuskKind::Worktree && !husk.git.as_ref().is_some_and(|g| g.is_primary) {
            grove.worktrees.push(husk.path.clone());
        }
    }
    map.into_values().collect()
}

fn build_afterimages(husks: &[Husk]) -> Vec<Afterimage> {
    husks
        .iter()
        .filter(|h| h.kind == HuskKind::Afterimage)
        .map(|h| Afterimage {
            husk_id: h.id.clone(),
            agent: h.agent.unwrap_or(AgentKind::Unknown),
            live: h.live,
        })
        .collect()
}

fn build_ballast(husks: &[Husk]) -> Vec<Ballast> {
    husks
        .iter()
        .filter(|h| h.kind == HuskKind::Ballast)
        .filter_map(|h| {
            let kind = h.ballast?;
            let duplicates = match &h.grove {
                Some(grove) => husks
                    .iter()
                    .filter(|o| {
                        o.id != h.id && o.ballast == Some(kind) && o.grove.as_ref() == Some(grove)
                    })
                    .map(|o| o.path.clone())
                    .collect(),
                None => vec![],
            };
            Some(Ballast {
                husk_id: h.id.clone(),
                kind,
                marker: h.path.parent().map(Path::to_path_buf).unwrap_or_default(),
                duplicates,
            })
        })
        .collect()
}

fn totals(husks: &[Husk], groves: &[Grove]) -> Totals {
    let by_id: BTreeMap<HuskId, &Husk> = husks.iter().map(|h| (h.id.clone(), h)).collect();
    Totals {
        husk_count: husks.len(),
        grove_count: groves.len(),
        bytes_seen: husks
            .iter()
            .filter(|h| h.parent.is_none())
            .map(|h| h.size_bytes)
            .sum(),
        bytes_reclaimable: husks
            .iter()
            .filter(|h| h.reclaimable && !has_reclaimable_ancestor(h, &by_id))
            .map(|h| h.size_bytes)
            .sum(),
        high_risk: husks
            .iter()
            .filter(|h| matches!(h.risk, Risk::Dangerous | Risk::Forbidden))
            .count(),
        occupied: husks.iter().filter(|h| h.has_ward("occupied")).count(),
        alarms: husks.iter().filter(|h| h.is_alarm()).count(),
    }
}

pub fn scan(opts: &ScanOptions) -> Result<ScanReport, Error> {
    scan_with_progress(opts, |_| {})
}

/// Walk, weigh, judge. `on_event` runs on the calling thread.
pub fn scan_with_progress<F>(opts: &ScanOptions, mut on_event: F) -> Result<ScanReport, Error>
where
    F: FnMut(&ScanEvent),
{
    let now = opts.clock.now_ms();
    let scan_roots = opts.scan_roots();
    let roots: Vec<PathBuf> = scan_roots.iter().map(|r| r.path.clone()).collect();
    let homes = agent_homes(&opts.dirs);
    let ctx = ClassifyContext {
        home: &opts.home,
        agent_homes: &homes,
    };
    let holders = HolderIndex::new(opts.processes.snapshot(), &opts.home);
    let mut warnings: Vec<String> = Vec::new();
    let mut pending: Vec<Pending> = Vec::new();
    let mut seen: BTreeSet<PathBuf> = BTreeSet::new();

    let mut walks: Vec<(PathBuf, bool)> = Vec::new();
    for root in &scan_roots {
        if let Some(seed) = toolchain_seed(root) {
            if seen.insert(root.path.clone()) {
                pending.push(Pending {
                    path: root.path.clone(),
                    seed,
                    outside: false,
                });
            }
        } else {
            walks.push((root.path.clone(), false));
        }
    }

    let mut i = 0;
    while i < walks.len() {
        let (root, outside) = walks[i].clone();
        on_event(&ScanEvent::Walking {
            root: root.clone(),
            index: i,
            total: walks.len(),
        });
        i += 1;
        let mut it = WalkDir::new(&root).follow_links(false).into_iter();
        while let Some(entry) = it.next() {
            let Ok(entry) = entry else {
                continue;
            };
            let is_dir = entry.file_type().is_dir();
            let path = entry.path();
            match classify_path(path, &ctx) {
                Classification::Skip => {
                    if is_dir {
                        it.skip_current_dir();
                    }
                }
                Classification::Continue => {
                    if is_dir && git_marker(path) == GitMarker::Primary {
                        match opts.git.list_worktree_paths(path) {
                            Ok(wts) => {
                                for wt in wts {
                                    let known = walks.iter().any(|(w, _)| w == &wt);
                                    if !known && wt.is_dir() && !is_within_roots(&wt, &roots) {
                                        walks.push((wt, true));
                                    }
                                }
                            }
                            Err(err) => warnings.push(format!("git worktrees: {err}")),
                        }
                    }
                }
                Classification::Husk(seed) => {
                    let skip = seed.skip_children && is_dir;
                    if seen.insert(canonicalize_lossy(path)) {
                        pending.push(Pending {
                            path: path.to_path_buf(),
                            seed,
                            outside,
                        });
                    }
                    if skip {
                        it.skip_current_dir();
                    }
                }
            }
        }
    }

    on_event(&ScanEvent::Weighing {
        pending: pending.len(),
    });
    let judge = Judge {
        opts,
        holders: &holders,
        roots: &roots,
        now,
    };
    let mut husks: Vec<Husk> = Vec::with_capacity(pending.len());
    let (tx, rx) = mpsc::channel::<Result<Husk, String>>();
    std::thread::scope(|s| {
        let judge = &judge;
        s.spawn(move || {
            pending.into_par_iter().for_each_with(tx, |tx, p| {
                let _ = tx.send(judge.weigh(p));
            });
        });
        for result in rx {
            match result {
                Ok(husk) => {
                    on_event(&ScanEvent::Found(Box::new(husk.clone())));
                    husks.push(husk);
                }
                Err(w) => warnings.push(w),
            }
        }
    });

    husks.retain(|h| h.size_bytes >= opts.min_size);
    if let Some(cat) = opts.category {
        husks.retain(|h| h.kind == cat);
    }
    husks.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes).then(a.path.cmp(&b.path)));
    link_parents(&mut husks);

    let groves = build_groves(&husks);
    let afterimages = build_afterimages(&husks);
    let ballast = build_ballast(&husks);
    let totals = totals(&husks, &groves);
    warnings.sort();
    warnings.dedup();

    Ok(ScanReport {
        version: REPORT_VERSION,
        scanned_at_ms: now,
        roots,
        home: opts.home.clone(),
        husks,
        groves,
        afterimages,
        ballast,
        totals,
        warnings,
    })
}
