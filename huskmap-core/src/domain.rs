use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::clock::Millis;
use crate::error::Error;
use crate::ids::HuskId;

pub const REPORT_VERSION: u32 = 2;
pub const PLAN_VERSION: u32 = 2;
pub const DEFAULT_OLDER_DAYS: u64 = 30;
/// A husk touched more recently than this is treated as warm.
pub const RECENT_MS: Millis = 30 * 60 * 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HuskKind {
    Worktree,
    Ballast,
    Toolchain,
    Afterimage,
    Cache,
    Debris,
}

impl HuskKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Worktree => "worktree",
            Self::Ballast => "ballast",
            Self::Toolchain => "toolchain",
            Self::Afterimage => "afterimage",
            Self::Cache => "cache",
            Self::Debris => "debris",
        }
    }

    pub fn all() -> [Self; 6] {
        [
            Self::Worktree,
            Self::Ballast,
            Self::Toolchain,
            Self::Afterimage,
            Self::Cache,
            Self::Debris,
        ]
    }
}

impl std::fmt::Display for HuskKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for HuskKind {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let wanted = s.trim().to_ascii_lowercase();
        Self::all()
            .into_iter()
            .find(|k| k.as_str() == wanted)
            .ok_or_else(|| Error::safety(format!("unknown category: {wanted}")))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Risk {
    Safe,
    Caution,
    Dangerous,
    Forbidden,
}

impl Risk {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Caution => "caution",
            Self::Dangerous => "dangerous",
            Self::Forbidden => "forbidden",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    Claude,
    Codex,
    Cursor,
    OpenCode,
    Gemini,
    Aider,
    Grok,
    Unknown,
}

impl AgentKind {
    pub const KNOWN: [Self; 7] = [
        Self::Claude,
        Self::Codex,
        Self::Cursor,
        Self::OpenCode,
        Self::Gemini,
        Self::Aider,
        Self::Grok,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Cursor => "cursor",
            Self::OpenCode => "opencode",
            Self::Gemini => "gemini",
            Self::Aider => "aider",
            Self::Grok => "grok",
            Self::Unknown => "unknown",
        }
    }

    /// Only dot-dirs count. A project named `codex` is not an agent home.
    pub fn from_path_component(name: &str) -> Option<Self> {
        let bare = name.strip_prefix('.')?;
        Self::KNOWN.into_iter().find(|a| a.as_str() == bare)
    }

    /// Match a process name or argv against agent binaries.
    pub fn from_process(name: &str, argv: &[String]) -> Option<Self> {
        let words = std::iter::once(name).chain(argv.iter().take(3).map(String::as_str));
        for word in words {
            let base = word
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or(word)
                .to_ascii_lowercase();
            let base = base.strip_suffix(".exe").unwrap_or(&base);
            if let Some(agent) = Self::KNOWN
                .into_iter()
                .find(|a| base == a.as_str() || base.starts_with(&format!("{}-", a.as_str())))
            {
                return Some(agent);
            }
        }
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ecosystem {
    Node,
    Python,
    Rust,
    Go,
    Browsers,
}

impl Ecosystem {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Node => "node",
            Self::Python => "python",
            Self::Rust => "rust",
            Self::Go => "go",
            Self::Browsers => "browsers",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BallastKind {
    NodeModules,
    RustTarget,
    Venv,
    Next,
    Turbo,
    Nuxt,
    SvelteKit,
    Parcel,
    PyTooling,
    Tox,
}

impl BallastKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NodeModules => "node_modules",
            Self::RustTarget => "target",
            Self::Venv => "venv",
            Self::Next => "next",
            Self::Turbo => "turbo",
            Self::Nuxt => "nuxt",
            Self::SvelteKit => "svelte-kit",
            Self::Parcel => "parcel-cache",
            Self::PyTooling => "py-tooling",
            Self::Tox => "tox",
        }
    }

    pub fn ecosystem(self) -> Ecosystem {
        match self {
            Self::NodeModules
            | Self::Next
            | Self::Turbo
            | Self::Nuxt
            | Self::SvelteKit
            | Self::Parcel => Ecosystem::Node,
            Self::RustTarget => Ecosystem::Rust,
            Self::Venv | Self::PyTooling | Self::Tox => Ecosystem::Python,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct GitFacts {
    pub is_worktree: bool,
    pub is_primary: bool,
    pub git_dir: PathBuf,
    pub common_dir: PathBuf,
    /// The checkout root these facts were read from. Differs from the husk path for agent slots.
    #[serde(default)]
    pub checkout: PathBuf,
    pub branch: Option<String>,
    pub dirty: bool,
    /// Commits reachable from HEAD and from no other branch, remote, or tag.
    pub unpushed: bool,
    pub locked: bool,
    #[serde(default)]
    pub dirty_files: u32,
    #[serde(default)]
    pub stranded_commits: u32,
    #[serde(default)]
    pub upstream: Option<String>,
    #[serde(default)]
    pub head_summary: Option<String>,
    #[serde(default)]
    pub last_commit_ms: Option<Millis>,
    #[serde(default)]
    pub lock_reason: Option<String>,
}

impl GitFacts {
    pub fn grove_key(&self) -> &Path {
        &self.common_dir
    }
}

/// A process whose working directory sits inside (or right above) a husk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Holder {
    pub pid: u32,
    pub name: String,
    pub cwd: PathBuf,
    pub agent: Option<AgentKind>,
}

impl Holder {
    pub fn label(&self) -> String {
        match self.agent {
            Some(agent) => format!("{} · pid {}", agent.as_str(), self.pid),
            None => format!("{} · pid {}", self.name, self.pid),
        }
    }
}

/// Structured reasons a husk is guarded. Views render these; core decides them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "ward", rename_all = "snake_case")]
pub enum Ward {
    /// A live process works inside. Never reclaimed, force or not.
    Occupied {
        holders: Vec<Holder>,
    },
    /// A process runs in the project that owns this ballast.
    NeighborBusy {
        holders: Vec<Holder>,
    },
    /// Uncommitted or untracked changes.
    Dirty {
        files: u32,
    },
    /// Commits that exist nowhere else.
    Stranded {
        commits: u32,
    },
    Locked {
        reason: Option<String>,
    },
    Primary,
    Secrets,
    OutsideRoots,
    /// Touched within [`RECENT_MS`].
    Warm {
        minutes: u64,
    },
    /// A worktree slot with no git checkout inside. Nothing vouches for the files.
    NoGit,
    /// The project this session belonged to is gone.
    Orphaned {
        origin: Option<PathBuf>,
    },
}

impl Ward {
    pub fn key(&self) -> &'static str {
        match self {
            Self::Occupied { .. } => "occupied",
            Self::NeighborBusy { .. } => "neighbor_busy",
            Self::Dirty { .. } => "dirty",
            Self::Stranded { .. } => "stranded",
            Self::Locked { .. } => "locked",
            Self::Primary => "primary",
            Self::Secrets => "secrets",
            Self::OutsideRoots => "outside_roots",
            Self::Warm { .. } => "warm",
            Self::NoGit => "no_git",
            Self::Orphaned { .. } => "orphaned",
        }
    }

    /// Wards that make a husk untouchable without an explicit force (or ever).
    pub fn blocks(&self) -> bool {
        matches!(
            self,
            Self::Occupied { .. }
                | Self::Dirty { .. }
                | Self::Stranded { .. }
                | Self::Locked { .. }
                | Self::Primary
                | Self::Secrets
                | Self::OutsideRoots
        )
    }

    /// Wards that force cannot lift.
    pub fn absolute(&self) -> bool {
        matches!(
            self,
            Self::Occupied { .. } | Self::Primary | Self::Secrets | Self::OutsideRoots
        )
    }

    /// Risk floor this ward imposes.
    pub fn risk(&self) -> Risk {
        match self {
            Self::Primary => Risk::Forbidden,
            Self::Occupied { .. } | Self::Dirty { .. } | Self::Secrets => Risk::Dangerous,
            Self::Stranded { .. }
            | Self::Locked { .. }
            | Self::NeighborBusy { .. }
            | Self::Warm { .. }
            | Self::NoGit
            | Self::OutsideRoots => Risk::Caution,
            Self::Orphaned { .. } => Risk::Safe,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Husk {
    pub id: HuskId,
    pub kind: HuskKind,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub mtime_ms: Option<Millis>,
    pub risk: Risk,
    pub reclaimable: bool,
    pub notes: Vec<String>,
    pub agent: Option<AgentKind>,
    pub ballast: Option<BallastKind>,
    #[serde(default)]
    pub ecosystem: Option<Ecosystem>,
    pub git: Option<GitFacts>,
    /// Repo common dir this husk belongs to, when it sits inside a git checkout.
    #[serde(default)]
    pub grove: Option<PathBuf>,
    pub parent: Option<HuskId>,
    pub contains_secrets: bool,
    pub live: bool,
    #[serde(default)]
    pub wards: Vec<Ward>,
    /// The project folder a session belongs to, decoded from its store name. It may be gone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<PathBuf>,
}

impl Husk {
    /// Bare husk with no facts. Scan fills the rest.
    pub fn bare(kind: HuskKind, path: impl Into<PathBuf>, size_bytes: u64) -> Self {
        let path = path.into();
        Self {
            id: HuskId::new(kind, path.clone()),
            kind,
            path,
            size_bytes,
            mtime_ms: None,
            risk: Risk::Safe,
            reclaimable: true,
            notes: vec![],
            agent: None,
            ballast: None,
            ecosystem: None,
            git: None,
            grove: None,
            parent: None,
            contains_secrets: false,
            live: false,
            wards: vec![],
            project: None,
        }
    }

    pub fn has_ward(&self, key: &str) -> bool {
        self.wards.iter().any(|w| w.key() == key)
    }

    pub fn holders(&self) -> Vec<&Holder> {
        self.wards
            .iter()
            .flat_map(|w| match w {
                Ward::Occupied { holders } | Ward::NeighborBusy { holders } => holders.iter(),
                _ => [].iter(),
            })
            .collect()
    }

    /// Worktrees and checkouts where someone is working or work would be lost.
    pub fn is_alarm(&self) -> bool {
        self.kind == HuskKind::Worktree
            && self.wards.iter().any(|w| {
                matches!(
                    w,
                    Ward::Occupied { .. } | Ward::Dirty { .. } | Ward::Stranded { .. }
                )
            })
    }

    pub fn age_ms(&self, now_ms: Millis) -> Option<Millis> {
        self.mtime_ms.map(|m| now_ms.saturating_sub(m))
    }

    /// Add a ward and let it raise risk / drop reclaimability.
    pub fn ward(&mut self, ward: Ward) {
        if self.wards.iter().any(|w| w.key() == ward.key()) {
            return;
        }
        if ward.risk() > self.risk {
            self.risk = ward.risk();
        }
        if ward.blocks() {
            self.reclaimable = false;
        }
        if matches!(ward, Ward::Occupied { .. }) {
            self.live = true;
        }
        self.wards.push(ward);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grove {
    pub id: PathBuf,
    #[serde(default)]
    pub name: String,
    pub primary: Option<PathBuf>,
    pub worktrees: Vec<PathBuf>,
    pub husks: Vec<HuskId>,
    #[serde(default)]
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Afterimage {
    pub husk_id: HuskId,
    pub agent: AgentKind,
    pub live: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ballast {
    pub husk_id: HuskId,
    pub kind: BallastKind,
    pub marker: PathBuf,
    /// Same kind of ballast in the same grove (sibling worktrees of one repo).
    pub duplicates: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Totals {
    pub husk_count: usize,
    pub grove_count: usize,
    pub bytes_seen: u64,
    pub bytes_reclaimable: u64,
    pub high_risk: usize,
    #[serde(default)]
    pub occupied: usize,
    #[serde(default)]
    pub alarms: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanReport {
    pub version: u32,
    pub scanned_at_ms: Millis,
    pub roots: Vec<PathBuf>,
    pub home: PathBuf,
    pub husks: Vec<Husk>,
    pub groves: Vec<Grove>,
    pub afterimages: Vec<Afterimage>,
    pub ballast: Vec<Ballast>,
    pub totals: Totals,
    pub warnings: Vec<String>,
}

impl ScanReport {
    pub fn empty(home: PathBuf, scanned_at_ms: Millis) -> Self {
        Self {
            version: REPORT_VERSION,
            scanned_at_ms,
            roots: vec![],
            home,
            husks: vec![],
            groves: vec![],
            afterimages: vec![],
            ballast: vec![],
            totals: Totals::default(),
            warnings: vec![],
        }
    }

    pub fn husk(&self, id: &HuskId) -> Option<&Husk> {
        self.husks.iter().find(|h| &h.id == id)
    }

    pub fn to_json(&self) -> Result<String, Error> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json(text: &str) -> Result<Self, Error> {
        let report: Self = serde_json::from_str(text)?;
        if report.version != REPORT_VERSION {
            return Err(Error::Version {
                kind: "report",
                found: report.version,
                want: REPORT_VERSION,
            });
        }
        Ok(report)
    }

    pub fn load(path: &Path) -> Result<Self, Error> {
        let text = std::fs::read_to_string(path).map_err(|e| Error::io(path, e))?;
        Self::from_json(&text)
    }

    pub fn save(&self, path: &Path) -> Result<(), Error> {
        write_json(path, &self.to_json()?)
    }

    /// Bytes per kind, counting only top-level husks of that kind so nesting is not doubled.
    pub fn bytes_by_kind(&self) -> Vec<(HuskKind, u64, usize)> {
        HuskKind::all()
            .into_iter()
            .map(|kind| {
                let of_kind: Vec<&Husk> = self.husks.iter().filter(|h| h.kind == kind).collect();
                let bytes = of_kind
                    .iter()
                    .filter(|h| {
                        h.parent
                            .as_ref()
                            .and_then(|p| self.husk(p))
                            .is_none_or(|p| p.kind != kind)
                    })
                    .map(|h| h.size_bytes)
                    .sum();
                (kind, bytes, of_kind.len())
            })
            .collect()
    }
}

fn write_json(path: &Path, text: &str) -> Result<(), Error> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
    }
    std::fs::write(path, text).map_err(|e| Error::io(path, e))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlanPreset {
    Safe,
    AgentOnly,
    Older {
        days: u64,
    },
    /// Exactly what the user marked. Still filtered by every safety rule.
    Marked,
}

impl PlanPreset {
    pub fn parse(name: &str) -> Result<Self, Error> {
        match name.trim().to_ascii_lowercase().as_str() {
            "safe" => Ok(Self::Safe),
            "agent-only" | "agent_only" | "agentonly" => Ok(Self::AgentOnly),
            "older" => Ok(Self::Older {
                days: DEFAULT_OLDER_DAYS,
            }),
            "marked" => Ok(Self::Marked),
            other => Err(Error::safety(format!("unknown preset: {other}"))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::AgentOnly => "agent-only",
            Self::Older { .. } => "older",
            Self::Marked => "marked",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    Trash,
    /// Trash the checkout, then prune git's admin entry for it.
    GitWorktreeRemove,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fingerprint {
    pub size_bytes: u64,
    pub mtime_ms: Option<Millis>,
    pub dirty: bool,
    pub unpushed: bool,
    pub is_primary: bool,
    pub contains_secrets: bool,
}

impl Fingerprint {
    pub fn from_husk(husk: &Husk) -> Self {
        Self {
            size_bytes: husk.size_bytes,
            mtime_ms: husk.mtime_ms,
            dirty: husk.git.as_ref().is_some_and(|g| g.dirty),
            unpushed: husk.git.as_ref().is_some_and(|g| g.unpushed),
            is_primary: husk.git.as_ref().is_some_and(|g| g.is_primary),
            contains_secrets: husk.contains_secrets,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanAction {
    pub husk_id: HuskId,
    pub path: PathBuf,
    pub kind: ActionKind,
    pub fingerprint: Fingerprint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    pub version: u32,
    pub created_at_ms: Millis,
    pub preset: PlanPreset,
    pub roots: Vec<PathBuf>,
    pub actions: Vec<PlanAction>,
}

impl Plan {
    pub fn to_json(&self) -> Result<String, Error> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json(text: &str) -> Result<Self, Error> {
        let plan: Self = serde_json::from_str(text)?;
        if plan.version != PLAN_VERSION {
            return Err(Error::Version {
                kind: "plan",
                found: plan.version,
                want: PLAN_VERSION,
            });
        }
        Ok(plan)
    }

    pub fn load(path: &Path) -> Result<Self, Error> {
        let text = std::fs::read_to_string(path).map_err(|e| Error::io(path, e))?;
        Self::from_json(&text)
    }

    pub fn save(&self, path: &Path) -> Result<(), Error> {
        write_json(path, &self.to_json()?)
    }

    pub fn reclaimable_bytes(&self) -> u64 {
        self.actions.iter().map(|a| a.fingerprint.size_bytes).sum()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppliedAction {
    pub path: PathBuf,
    pub kind: ActionKind,
    #[serde(default)]
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkippedAction {
    pub path: PathBuf,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ApplyResult {
    pub applied: Vec<AppliedAction>,
    pub skipped: Vec<SkippedAction>,
    pub errors: Vec<String>,
}

impl ApplyResult {
    pub fn is_clean(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn bytes_reclaimed(&self) -> u64 {
        self.applied.iter().map(|a| a.bytes).sum()
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

/// `~/dev/app` instead of `/home/me/dev/app`.
pub fn tilde(path: &Path, home: &Path) -> String {
    if home.as_os_str().is_empty() {
        return path.display().to_string();
    }
    match path.strip_prefix(home) {
        Ok(rest) if rest.as_os_str().is_empty() => "~".into(),
        Ok(rest) => format!("~{}{}", std::path::MAIN_SEPARATOR, rest.display()),
        Err(_) => path.display().to_string(),
    }
}

/// Compact age: `12m`, `5h`, `3d`, `7w`, `4mo`, `2y`.
pub fn format_age(ms: Millis) -> String {
    let minutes = ms / 60_000;
    let hours = minutes / 60;
    let days = hours / 24;
    if minutes < 60 {
        format!("{minutes}m")
    } else if hours < 24 {
        format!("{hours}h")
    } else if days < 14 {
        format!("{days}d")
    } else if days < 60 {
        format!("{}w", days / 7)
    } else if days < 730 {
        format!("{}mo", days / 30)
    } else {
        format!("{}y", days / 365)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_roundtrip() {
        for kind in HuskKind::all() {
            assert_eq!(HuskKind::from_str(kind.as_str()).unwrap(), kind);
            assert_eq!(kind.to_string(), kind.as_str());
        }
        assert!(HuskKind::from_str("nope").is_err());
        assert_eq!(HuskKind::from_str(" Ballast ").unwrap(), HuskKind::Ballast);
    }

    #[test]
    fn preset_parse() {
        let table = [
            ("safe", PlanPreset::Safe),
            ("agent-only", PlanPreset::AgentOnly),
            ("agent_only", PlanPreset::AgentOnly),
            (
                "older",
                PlanPreset::Older {
                    days: DEFAULT_OLDER_DAYS,
                },
            ),
            ("marked", PlanPreset::Marked),
        ];
        for (raw, want) in table {
            assert_eq!(PlanPreset::parse(raw).unwrap(), want);
        }
        assert!(PlanPreset::parse("gc").is_err());
        assert_eq!(PlanPreset::Safe.as_str(), "safe");
        assert_eq!(PlanPreset::AgentOnly.as_str(), "agent-only");
        assert_eq!(PlanPreset::Older { days: 7 }.as_str(), "older");
        assert_eq!(PlanPreset::Marked.as_str(), "marked");
    }

    #[test]
    fn agent_from_component_needs_dot() {
        assert_eq!(
            AgentKind::from_path_component(".claude"),
            Some(AgentKind::Claude)
        );
        assert_eq!(
            AgentKind::from_path_component(".grok"),
            Some(AgentKind::Grok)
        );
        assert_eq!(AgentKind::from_path_component("codex"), None);
        assert_eq!(AgentKind::from_path_component(".foo"), None);
        assert_eq!(AgentKind::from_path_component(".unknown"), None);
        for agent in AgentKind::KNOWN {
            let dotted = format!(".{}", agent.as_str());
            assert_eq!(AgentKind::from_path_component(&dotted), Some(agent));
        }
        assert_eq!(AgentKind::Unknown.as_str(), "unknown");
    }

    #[test]
    fn agent_from_process() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(
            AgentKind::from_process("claude", &[]),
            Some(AgentKind::Claude)
        );
        assert_eq!(
            AgentKind::from_process("node", &s(&["/usr/bin/node", "/opt/bin/codex"])),
            Some(AgentKind::Codex)
        );
        assert_eq!(
            AgentKind::from_process("Cursor.exe", &[]),
            Some(AgentKind::Cursor)
        );
        assert_eq!(
            AgentKind::from_process("gemini-cli", &[]),
            Some(AgentKind::Gemini)
        );
        assert_eq!(AgentKind::from_process("zsh", &s(&["-zsh"])), None);
        assert_eq!(
            AgentKind::from_process("claudette", &[]),
            None,
            "prefix without dash is not an agent"
        );
    }

    #[test]
    fn ballast_names_and_ecosystems() {
        let table = [
            (BallastKind::NodeModules, "node_modules", Ecosystem::Node),
            (BallastKind::RustTarget, "target", Ecosystem::Rust),
            (BallastKind::Venv, "venv", Ecosystem::Python),
            (BallastKind::Next, "next", Ecosystem::Node),
            (BallastKind::Turbo, "turbo", Ecosystem::Node),
            (BallastKind::Nuxt, "nuxt", Ecosystem::Node),
            (BallastKind::SvelteKit, "svelte-kit", Ecosystem::Node),
            (BallastKind::Parcel, "parcel-cache", Ecosystem::Node),
            (BallastKind::PyTooling, "py-tooling", Ecosystem::Python),
            (BallastKind::Tox, "tox", Ecosystem::Python),
        ];
        for (kind, name, eco) in table {
            assert_eq!(kind.as_str(), name);
            assert_eq!(kind.ecosystem(), eco);
        }
        for (eco, name) in [
            (Ecosystem::Node, "node"),
            (Ecosystem::Python, "python"),
            (Ecosystem::Rust, "rust"),
            (Ecosystem::Go, "go"),
            (Ecosystem::Browsers, "browsers"),
        ] {
            assert_eq!(eco.as_str(), name);
        }
    }

    #[test]
    fn format_bytes_table() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(format_bytes(1024u64.pow(4)), "1.0 TB");
        assert_eq!(format_bytes(1024u64.pow(5)), "1024.0 TB");
    }

    #[test]
    fn tilde_paths() {
        let home = Path::new("/home/me");
        assert_eq!(tilde(Path::new("/home/me"), home), "~");
        assert_eq!(
            tilde(Path::new("/home/me/dev/a"), home),
            format!("~{}dev/a", std::path::MAIN_SEPARATOR)
        );
        assert_eq!(tilde(Path::new("/opt/x"), home), "/opt/x");
        assert_eq!(
            tilde(Path::new("/opt/x"), Path::new("")),
            "/opt/x",
            "unknown home"
        );
    }

    #[test]
    fn format_age_table() {
        let min = 60_000;
        let day = 24 * 60 * min;
        assert_eq!(format_age(5 * min), "5m");
        assert_eq!(format_age(3 * 60 * min), "3h");
        assert_eq!(format_age(3 * day), "3d");
        assert_eq!(format_age(21 * day), "3w");
        assert_eq!(format_age(90 * day), "3mo");
        assert_eq!(format_age(800 * day), "2y");
    }

    fn report() -> ScanReport {
        ScanReport::empty(PathBuf::from("/tmp"), 1)
    }

    #[test]
    fn report_json_version_gate() {
        let json = report().to_json().unwrap();
        let back = ScanReport::from_json(&json).unwrap();
        assert_eq!(back.home, PathBuf::from("/tmp"));
        let bad = json.replacen("\"version\": 2", "\"version\": 9", 1);
        let err = ScanReport::from_json(&bad).unwrap_err();
        assert!(matches!(err, Error::Version { kind: "report", .. }));
    }

    #[test]
    fn plan_json_version_gate() {
        let plan = Plan {
            version: PLAN_VERSION,
            created_at_ms: 1,
            preset: PlanPreset::Safe,
            roots: vec![],
            actions: vec![],
        };
        let json = plan.to_json().unwrap();
        assert_eq!(Plan::from_json(&json).unwrap(), plan);
        let bad = json.replacen("\"version\": 2", "\"version\": 1", 1);
        let err = Plan::from_json(&bad).unwrap_err();
        assert!(matches!(err, Error::Version { kind: "plan", .. }));
    }

    #[test]
    fn save_and_load_roundtrip_including_bare_filename() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("deep/nested/report.json");
        report().save(&path).unwrap();
        assert_eq!(ScanReport::load(&path).unwrap(), report());
        let plan = Plan {
            version: PLAN_VERSION,
            created_at_ms: 1,
            preset: PlanPreset::Marked,
            roots: vec![],
            actions: vec![],
        };
        let plan_path = tmp.path().join("p/plan.json");
        plan.save(&plan_path).unwrap();
        assert_eq!(Plan::load(&plan_path).unwrap(), plan);
        assert!(ScanReport::load(&tmp.path().join("nope.json")).is_err());
        assert!(Plan::load(&tmp.path().join("nope.json")).is_err());
        let blocked = tmp.path().join("file");
        std::fs::write(&blocked, "x").unwrap();
        assert!(report().save(&blocked.join("child.json")).is_err());
    }

    #[test]
    fn risk_orders_and_names() {
        assert!(Risk::Safe < Risk::Caution);
        assert!(Risk::Caution < Risk::Dangerous);
        assert!(Risk::Dangerous < Risk::Forbidden);
        assert_eq!(Risk::Safe.as_str(), "safe");
        assert_eq!(Risk::Caution.as_str(), "caution");
        assert_eq!(Risk::Dangerous.as_str(), "dangerous");
        assert_eq!(Risk::Forbidden.as_str(), "forbidden");
    }

    #[test]
    fn apply_result_clean_and_bytes() {
        let mut ok = ApplyResult::default();
        assert!(ok.is_clean());
        ok.applied.push(AppliedAction {
            path: PathBuf::from("/a"),
            kind: ActionKind::Trash,
            bytes: 7,
        });
        assert_eq!(ok.bytes_reclaimed(), 7);
        ok.errors.push("no".into());
        assert!(!ok.is_clean());
    }

    fn holder(agent: Option<AgentKind>) -> Holder {
        Holder {
            pid: 42,
            name: "zsh".into(),
            cwd: PathBuf::from("/w"),
            agent,
        }
    }

    #[test]
    fn holder_labels() {
        assert_eq!(holder(Some(AgentKind::Claude)).label(), "claude · pid 42");
        assert_eq!(holder(None).label(), "zsh · pid 42");
    }

    #[test]
    fn wards_raise_risk_and_block() {
        let mut husk = Husk::bare(HuskKind::Worktree, "/w", 10);
        husk.ward(Ward::Warm { minutes: 3 });
        assert_eq!(husk.risk, Risk::Caution);
        assert!(husk.reclaimable);
        husk.ward(Ward::Occupied {
            holders: vec![holder(Some(AgentKind::Codex))],
        });
        assert_eq!(husk.risk, Risk::Dangerous);
        assert!(!husk.reclaimable);
        assert!(husk.live);
        assert!(husk.is_alarm());
        husk.ward(Ward::Occupied { holders: vec![] });
        assert_eq!(husk.wards.len(), 2, "duplicate ward keys collapse");
        assert_eq!(husk.holders().len(), 1);
        husk.ward(Ward::Stranded { commits: 1 });
        assert_eq!(husk.risk, Risk::Dangerous, "risk never drops");
        assert!(husk.has_ward("stranded"));
        assert!(!husk.has_ward("dirty"));
    }

    #[test]
    fn ward_table() {
        let all = [
            (
                Ward::Occupied { holders: vec![] },
                "occupied",
                true,
                true,
                Risk::Dangerous,
            ),
            (
                Ward::NeighborBusy { holders: vec![] },
                "neighbor_busy",
                false,
                false,
                Risk::Caution,
            ),
            (
                Ward::Dirty { files: 1 },
                "dirty",
                true,
                false,
                Risk::Dangerous,
            ),
            (
                Ward::Stranded { commits: 1 },
                "stranded",
                true,
                false,
                Risk::Caution,
            ),
            (
                Ward::Locked { reason: None },
                "locked",
                true,
                false,
                Risk::Caution,
            ),
            (Ward::Primary, "primary", true, true, Risk::Forbidden),
            (Ward::Secrets, "secrets", true, true, Risk::Dangerous),
            (
                Ward::OutsideRoots,
                "outside_roots",
                true,
                true,
                Risk::Caution,
            ),
            (
                Ward::Warm { minutes: 1 },
                "warm",
                false,
                false,
                Risk::Caution,
            ),
            (Ward::NoGit, "no_git", false, false, Risk::Caution),
            (
                Ward::Orphaned { origin: None },
                "orphaned",
                false,
                false,
                Risk::Safe,
            ),
        ];
        for (ward, key, blocks, absolute, risk) in all {
            assert_eq!(ward.key(), key);
            assert_eq!(ward.blocks(), blocks, "{key}");
            assert_eq!(ward.absolute(), absolute, "{key}");
            assert_eq!(ward.risk(), risk, "{key}");
        }
    }

    #[test]
    fn non_worktree_is_never_alarm() {
        let mut husk = Husk::bare(HuskKind::Ballast, "/b", 1);
        husk.ward(Ward::Dirty { files: 2 });
        assert!(!husk.is_alarm());
        let mut quiet = Husk::bare(HuskKind::Worktree, "/w", 1);
        quiet.ward(Ward::Warm { minutes: 2 });
        assert!(!quiet.is_alarm());
        assert_eq!(quiet.age_ms(10), None);
        quiet.mtime_ms = Some(4);
        assert_eq!(quiet.age_ms(10), Some(6));
        assert_eq!(quiet.age_ms(1), Some(0));
    }

    #[test]
    fn bytes_by_kind_skips_same_kind_nesting() {
        let mut r = report();
        let outer = Husk::bare(HuskKind::Worktree, "/w", 100);
        let mut inner = Husk::bare(HuskKind::Ballast, "/w/node_modules", 60);
        inner.parent = Some(outer.id.clone());
        let mut nested_wt = Husk::bare(HuskKind::Worktree, "/w/sub", 30);
        nested_wt.parent = Some(outer.id.clone());
        r.husks = vec![outer, inner, nested_wt];
        let by = r.bytes_by_kind();
        let get = |k| by.iter().find(|(kind, _, _)| *kind == k).unwrap();
        assert_eq!(get(HuskKind::Worktree).1, 100);
        assert_eq!(get(HuskKind::Worktree).2, 2);
        assert_eq!(get(HuskKind::Ballast).1, 60);
        assert_eq!(get(HuskKind::Debris).1, 0);
        assert!(
            r.husk(&HuskId::new(HuskKind::Ballast, "/w/node_modules"))
                .is_some()
        );
    }

    #[test]
    fn git_facts_grove_key() {
        let facts = GitFacts {
            common_dir: PathBuf::from("/repo/.git"),
            ..Default::default()
        };
        assert_eq!(facts.grove_key(), Path::new("/repo/.git"));
    }
}
