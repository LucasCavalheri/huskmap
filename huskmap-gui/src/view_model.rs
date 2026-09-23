//! Pure state for the desktop map. No Freya types: every rule here is testable without a window.

use std::collections::BTreeSet;
use std::f32::consts::TAU;

use huskmap_core::{
    AgentKind, ApplyResult, Clock, Husk, HuskId, HuskKind, Locale, Millis, Plan, Risk, ScanEvent,
    ScanReport, Ward, admissible, copy, format_age, format_bytes, plan_marked, tilde,
};

use crate::query::{self, Facts, Key, Query, Status};

const DAY_MS: f32 = 86_400_000.0;
/// Radius where the youngest husks sit, as a fraction of the map radius.
pub const INNER_R: f32 = 0.2;
pub const OUTER_R: f32 = 0.96;
const SECTOR_GAP: f32 = 0.07;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    Map,
    Ledger,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Phase {
    /// Nothing sounded yet.
    #[default]
    Idle,
    Scanning {
        walking: Option<String>,
        weighing: Option<usize>,
    },
    Ready,
    Failed(String),
}

/// How a husk reads at a glance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tone {
    /// Reclaimable and safe.
    Free,
    /// Reclaimable, but look first.
    Caution,
    /// Guarded; force can lift it.
    Guarded,
    /// Occupied, primary, secrets, outside roots.
    Untouchable,
}

impl Tone {
    pub fn of(husk: &Husk) -> Self {
        if husk.wards.iter().any(Ward::absolute) || husk.risk == Risk::Forbidden || husk.live {
            Self::Untouchable
        } else if !husk.reclaimable || husk.risk == Risk::Dangerous {
            Self::Guarded
        } else if husk.risk == Risk::Caution {
            Self::Caution
        } else {
            Self::Free
        }
    }
}

/// Kind order around the dial, clockwise from twelve o'clock.
pub const DIAL: [HuskKind; 6] = [
    HuskKind::Worktree,
    HuskKind::Ballast,
    HuskKind::Toolchain,
    HuskKind::Afterimage,
    HuskKind::Cache,
    HuskKind::Debris,
];

#[derive(Debug, Clone, PartialEq)]
pub struct Sector {
    pub kind: HuskKind,
    /// Radians, 0 = twelve o'clock, clockwise.
    pub start: f32,
    pub sweep: f32,
}

impl Sector {
    pub fn mid(&self) -> f32 {
        self.start + self.sweep / 2.0
    }
}

pub fn sectors() -> Vec<Sector> {
    let sweep = TAU / DIAL.len() as f32;
    DIAL.iter()
        .enumerate()
        .map(|(i, kind)| Sector {
            kind: *kind,
            start: i as f32 * sweep,
            sweep,
        })
        .collect()
}

/// FNV-1a over the path, mapped to [0, 1). Stable across runs so the map never reshuffles.
pub fn hash01(text: &str) -> f32 {
    let mut h: u32 = 0x811c_9dc5;
    for b in text.bytes() {
        h ^= u32::from(b);
        h = h.wrapping_mul(0x0100_0193);
    }
    // murmur3 finalizer: near-identical paths must still land far apart
    h ^= h >> 16;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2_ae35);
    h ^= h >> 16;
    (h >> 8) as f32 / (1u32 << 24) as f32
}

/// Age in days → radius fraction. Log of hours: the last hour hugs the core, a year is the rim.
pub fn age_radius(age_days: f32) -> f32 {
    let hours = age_days.max(0.0) * 24.0;
    let t = (hours.ln_1p() / (365.0f32 * 24.0).ln_1p()).clamp(0.0, 1.0);
    INNER_R + (OUTER_R - INNER_R) * t
}

/// Rings drawn on the dial: (label, radius fraction).
pub fn rings() -> Vec<(&'static str, f32)> {
    let d = copy::get();
    vec![
        (d.ring_today, age_radius(1.0)),
        (d.ring_week, age_radius(7.0)),
        (d.ring_month, age_radius(30.0)),
        (d.ring_older, age_radius(365.0)),
    ]
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapNode {
    pub id: HuskId,
    pub kind: HuskKind,
    pub angle: f32,
    pub radius: f32,
    /// 0..1 relative weight, sqrt-scaled.
    pub weight: f32,
    pub tone: Tone,
    pub occupied: bool,
    pub alarm: bool,
    pub marked: bool,
    pub forced: bool,
    pub selected: bool,
    /// Heaviest first.
    pub rank: usize,
    pub name: String,
    pub size_label: String,
}

impl MapNode {
    /// Unit-circle position, y down. Multiply by the map radius and add the center.
    pub fn xy(&self) -> (f32, f32) {
        polar(self.angle, self.radius)
    }
}

pub fn polar(angle: f32, radius: f32) -> (f32, f32) {
    (radius * angle.sin(), -radius * angle.cos())
}

fn segment(p: &std::path::Path) -> Option<String> {
    p.file_name().map(|n| n.to_string_lossy().into_owned())
}

fn digits(s: &str, n: usize) -> bool {
    s.len() == n && s.bytes().all(|b| b.is_ascii_digit())
}

/// What people remember a husk by: the branch for worktrees, the project for sessions,
/// `parent/node_modules` for dependencies, the date for dated session folders.
pub fn short_name(husk: &Husk, home: &std::path::Path) -> String {
    if let Some(branch) = husk.git.as_ref().and_then(|g| g.branch.clone()) {
        return branch;
    }
    if let Some(project) = &husk.project {
        if !home.as_os_str().is_empty() && project == home {
            return "~".into();
        }
        if let Some(name) = segment(project) {
            return name;
        }
    }
    let name = segment(&husk.path).unwrap_or_default();
    if husk.kind == HuskKind::Afterimage {
        // Reports from before `project` existed: decode Grok's `%2F` names here.
        if let Some(project) = huskmap_core::percent_decode(&name) {
            if !home.as_os_str().is_empty() && project == home {
                return "~".into();
            }
            if let Some(n) = segment(&project) {
                return n;
            }
        }
        // Claude project keys: `-home-me-dev-app` reads as `dev-app` without the home part.
        let home_key = format!("{}-", huskmap_core::claude_project_key(home));
        if name.starts_with('-')
            && name.len() > home_key.len()
            && !home.as_os_str().is_empty()
            && let Some(rest) = name.strip_prefix(&home_key)
        {
            return rest.to_string();
        }
        // Codex shards sessions by date: `2026/09/06` → `2026-09-06`.
        let up: Vec<String> = husk
            .path
            .ancestors()
            .skip(1)
            .take(2)
            .filter_map(segment)
            .collect();
        match up.as_slice() {
            [m, y, ..] if digits(&name, 2) && digits(m, 2) && digits(y, 4) => {
                return format!("{y}-{m}-{name}");
            }
            [y, ..] if digits(&name, 2) && digits(y, 4) => return format!("{y}-{name}"),
            _ => {}
        }
    }
    match husk.kind {
        HuskKind::Ballast | HuskKind::Toolchain => {
            let parent = husk
                .path
                .parent()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            format!("{parent}/{name}")
        }
        _ => name,
    }
}

/// How the list is ordered. The map always ranks by weight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortKey {
    #[default]
    Size,
    Age,
    Name,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sort {
    pub key: SortKey,
    /// Size: heaviest first. Age: oldest first. Name: Z to A.
    pub desc: bool,
}

impl Default for Sort {
    fn default() -> Self {
        Self {
            key: SortKey::Size,
            desc: true,
        }
    }
}

/// What pressing a filter chip does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chip {
    Kind(HuskKind),
    Status(Status),
    Agent(AgentKind),
    /// `None` clears the size filter.
    Size(Option<&'static str>),
    Age(Option<&'static str>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChipView {
    pub chip: Chip,
    pub label: String,
    /// Items with this value, over everything scanned. `None` for size and age.
    pub count: Option<usize>,
    pub active: bool,
    /// Tone swatch for status chips.
    pub tone: Option<Tone>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FilterBar {
    pub kinds: Vec<ChipView>,
    pub status: Vec<ChipView>,
    pub agents: Vec<ChipView>,
    pub sizes: Vec<ChipView>,
    pub ages: Vec<ChipView>,
    /// "12 of 190 items · 3.4 GB"
    pub summary: String,
    pub filtered: bool,
    /// Marks every visible husk that may go; `None` when none may.
    pub mark_all: Option<(String, bool)>,
    /// Status, agent, size and age chips are showing.
    pub open: bool,
    /// Filters set outside the type row, so a closed bar still says something is on.
    pub hidden_active: usize,
}

pub const SIZE_STEPS: [&str; 3] = [">10mb", ">100mb", ">1gb"];
pub const AGE_STEPS: [&str; 4] = [">1d", ">7d", ">30d", ">90d"];

#[derive(Debug, Clone, PartialEq)]
pub struct LegendRow {
    pub kind: HuskKind,
    pub label: String,
    pub bytes: u64,
    pub bytes_label: String,
    pub count: usize,
    /// Share of all bytes seen, 0..1.
    pub share: f32,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AlarmRow {
    pub id: HuskId,
    pub title: String,
    pub path: String,
    pub lines: Vec<(Ward, String)>,
    pub agent: Option<AgentKind>,
    pub tone: Tone,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LedgerRow {
    pub id: HuskId,
    pub kind: HuskKind,
    pub tone: Tone,
    pub size_label: String,
    pub age_label: String,
    pub name: String,
    pub path: String,
    pub agent: Option<AgentKind>,
    /// Agent, tool or ecosystem mark.
    pub brand: Option<crate::icons::Brand>,
    pub first_ward: Option<String>,
    pub ward_count: usize,
    pub marked: bool,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DrawerView {
    pub id: HuskId,
    pub kind: HuskKind,
    pub kind_label: String,
    pub tone: Tone,
    pub title: String,
    pub path: String,
    pub size_label: String,
    pub age_label: String,
    pub agent: Option<AgentKind>,
    /// (label, value) facts, git first.
    pub facts: Vec<(String, String)>,
    pub wards: Vec<(Ward, String)>,
    pub notes: Vec<String>,
    pub verdict: String,
    pub marked: bool,
    pub can_mark: bool,
    /// Guarded, but force may take it (dirty, stranded, locked).
    pub can_force: bool,
    pub forced: bool,
    /// Absolute path, for "open folder" and "copy path".
    pub full_path: std::path::PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConfirmView {
    pub title: String,
    pub body: String,
    pub bytes: u64,
    pub bytes_label: String,
    pub paths: Vec<String>,
    pub more: usize,
    pub guarded_note: Option<String>,
    /// Husks the person chose to force, with a warning of their own.
    pub forced_note: Option<String>,
    pub plan: Plan,
    /// Applied with `force`; only the husks the person force-marked.
    pub forced_plan: Plan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Intent {
    #[default]
    None,
    Scan,
    Apply,
    Update,
}

/// A newer release, as the map shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateOffer {
    pub version: String,
    pub notes: Vec<String>,
    pub page: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AppState {
    pub report: Option<ScanReport>,
    /// Husks streamed in during a scan, before the report lands.
    pub live: Vec<Husk>,
    pub phase: Phase,
    pub mode: ViewMode,
    pub selected: Option<HuskId>,
    pub marked: BTreeSet<HuskId>,
    /// Guarded husks marked with force. Never occupied, primary or secret ones.
    pub forced: BTreeSet<HuskId>,
    /// Search words and filters, in the query language of [`crate::query`].
    pub search: String,
    pub searching: bool,
    pub sort: Sort,
    pub guide_open: bool,
    pub guide_section: usize,
    /// System, light or dark. The window resolves System against the desktop.
    pub theme: crate::theme::ThemeChoice,
    /// The list shows only the type chips until this opens the rest.
    pub filters_open: bool,
    pub drawer_open: bool,
    pub confirm: Option<ConfirmView>,
    pub status: Option<String>,
    /// Bumped every time a report lands, to restart reveal motion.
    pub generation: u64,
    pub last_reclaimed: u64,
    /// A button asked the shell for something only the shell can do.
    pub request: Option<Intent>,
    /// Husks are moving to the trash right now.
    pub applying: bool,
    pub update: Option<UpdateOffer>,
    pub update_open: bool,
    pub updating: bool,
}

impl AppState {
    pub fn with_report(report: ScanReport) -> Self {
        let mut s = Self::default();
        s.land(report);
        s
    }

    pub fn husks(&self) -> &[Husk] {
        // A rescan keeps the last report on screen; only a first scan streams live husks.
        match &self.report {
            Some(r) => &r.husks,
            None => &self.live,
        }
    }

    pub fn now_ms(&self) -> Millis {
        self.report
            .as_ref()
            .map(|r| r.scanned_at_ms)
            .unwrap_or_else(|| huskmap_core::SystemClock.now_ms())
    }

    fn home(&self) -> std::path::PathBuf {
        self.report
            .as_ref()
            .map(|r| r.home.clone())
            .unwrap_or_default()
    }

    pub fn husk(&self, id: &HuskId) -> Option<&Husk> {
        self.husks().iter().find(|h| &h.id == id)
    }

    pub fn is_scanning(&self) -> bool {
        matches!(self.phase, Phase::Scanning { .. })
    }

    pub fn is_marked(&self, id: &HuskId) -> bool {
        self.marked.contains(id) || self.forced.contains(id)
    }

    fn matches(&self, query: &Query, husk: &Husk, home: &std::path::Path, now: Millis) -> bool {
        if query.is_empty() {
            return true;
        }
        let name = short_name(husk, home);
        query.matches(
            husk,
            &Facts {
                tone: Tone::of(husk),
                marked: self.is_marked(&husk.id),
                now,
                name: &name,
            },
        )
    }

    /// Filtered and sorted. This is the j/k order in both views.
    pub fn visible(&self) -> Vec<&Husk> {
        let query = Query::parse(&self.search);
        let home = self.home();
        let now = self.now_ms();
        let mut v: Vec<&Husk> = self
            .husks()
            .iter()
            .filter(|h| self.matches(&query, h, &home, now))
            .collect();
        let by_size =
            |a: &&Husk, b: &&Husk| b.size_bytes.cmp(&a.size_bytes).then(a.path.cmp(&b.path));
        match self.sort.key {
            SortKey::Size => v.sort_by(by_size),
            SortKey::Age => v.sort_by_key(|h| std::cmp::Reverse(h.age_ms(now).unwrap_or(u64::MAX))),
            SortKey::Name => v.sort_by_cached_key(|h| short_name(h, &home).to_lowercase()),
        }
        let natural_desc = self.sort.key != SortKey::Name;
        if self.sort.desc != natural_desc {
            v.reverse();
        }
        v
    }

    /// Heaviest first, whatever the list is sorted by.
    fn by_weight(&self) -> Vec<&Husk> {
        let mut v = self.visible();
        v.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes).then(a.path.cmp(&b.path)));
        v
    }

    pub fn nodes(&self) -> Vec<MapNode> {
        let visible = self.by_weight();
        let home = self.home();
        let now = self.now_ms();
        let max = visible.first().map(|h| h.size_bytes).unwrap_or(1).max(1) as f32;
        let sectors = sectors();
        visible
            .iter()
            .enumerate()
            .map(|(rank, h)| {
                let sector = sectors
                    .iter()
                    .find(|s| s.kind == h.kind)
                    .unwrap_or(&sectors[0]);
                let span = sector.sweep * (1.0 - SECTOR_GAP * 2.0);
                let angle = sector.start
                    + sector.sweep * SECTOR_GAP
                    + hash01(&h.path.to_string_lossy()) * span;
                let age_days = h.age_ms(now).map(|a| a as f32 / DAY_MS).unwrap_or(365.0);
                let jitter = (hash01(&h.id.to_string()) - 0.5) * 0.05;
                MapNode {
                    id: h.id.clone(),
                    kind: h.kind,
                    angle,
                    radius: (age_radius(age_days) + jitter).clamp(INNER_R * 0.8, OUTER_R),
                    weight: (h.size_bytes as f32 / max).sqrt().clamp(0.0, 1.0),
                    tone: Tone::of(h),
                    occupied: h.has_ward("occupied"),
                    alarm: h.is_alarm(),
                    marked: self.is_marked(&h.id),
                    forced: self.forced.contains(&h.id),
                    selected: self.selected.as_ref() == Some(&h.id),
                    rank,
                    name: short_name(h, &home),
                    size_label: format_bytes(h.size_bytes),
                }
            })
            .collect()
    }

    /// Nearest node to a unit-circle point, within `reach` (unit radius).
    pub fn pick(&self, x: f32, y: f32, reach: f32) -> Option<HuskId> {
        self.nodes()
            .into_iter()
            .map(|n| {
                let (nx, ny) = n.xy();
                let d = ((nx - x).powi(2) + (ny - y).powi(2)).sqrt();
                (d - n.weight * 0.03, n.id)
            })
            .filter(|(d, _)| *d <= reach)
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, id)| id)
    }

    pub fn totals(&self) -> (u64, u64) {
        match &self.report {
            Some(r) => (r.totals.bytes_reclaimable, r.totals.bytes_seen),
            None => {
                let top = self.live.iter().filter(|h| {
                    !self
                        .live
                        .iter()
                        .any(|o| o.path != h.path && h.path.starts_with(&o.path))
                });
                top.fold((0, 0), |(rec, seen), h| {
                    (
                        rec + if h.reclaimable { h.size_bytes } else { 0 },
                        seen + h.size_bytes,
                    )
                })
            }
        }
    }

    pub fn legend(&self) -> Vec<LegendRow> {
        let husks = self.husks();
        let (_, seen) = self.totals();
        let kinds = Query::parse(&self.search).kinds();
        DIAL.iter()
            .map(|kind| {
                let of: Vec<&Husk> = husks.iter().filter(|h| h.kind == *kind).collect();
                let bytes: u64 = of
                    .iter()
                    .filter(|h| {
                        h.parent
                            .as_ref()
                            .and_then(|p| husks.iter().find(|o| &o.id == p))
                            .is_none_or(|p| p.kind != *kind)
                    })
                    .map(|h| h.size_bytes)
                    .sum();
                LegendRow {
                    kind: *kind,
                    label: copy::kind_label(*kind, true).into(),
                    bytes,
                    bytes_label: format_bytes(bytes),
                    count: of.len(),
                    share: if seen == 0 {
                        0.0
                    } else {
                        (bytes as f32 / seen as f32).min(1.0)
                    },
                    active: kinds.as_ref().is_none_or(|k| k.contains(kind)),
                }
            })
            .collect()
    }

    pub fn alarms(&self) -> Vec<AlarmRow> {
        let home = self.home();
        let mut rows: Vec<AlarmRow> = self
            .husks()
            .iter()
            .filter(|h| h.is_alarm())
            .map(|h| AlarmRow {
                id: h.id.clone(),
                title: short_name(h, &home),
                path: tilde(&h.path, &home),
                lines: h
                    .wards
                    .iter()
                    .filter(|w| {
                        matches!(
                            w,
                            Ward::Occupied { .. } | Ward::Dirty { .. } | Ward::Stranded { .. }
                        )
                    })
                    .map(|w| (w.clone(), copy::ward_text(w)))
                    .collect(),
                agent: h.agent.or_else(|| h.holders().iter().find_map(|x| x.agent)),
                tone: Tone::of(h),
            })
            .collect();
        rows.sort_by(|a, b| b.tone.cmp(&a.tone).then(a.title.cmp(&b.title)));
        rows
    }

    /// Agents seen, with how many husks they left.
    pub fn agents(&self) -> Vec<(AgentKind, usize, u64)> {
        let mut out: Vec<(AgentKind, usize, u64)> = Vec::new();
        for h in self.husks() {
            let Some(agent) = h.agent else { continue };
            match out.iter_mut().find(|(a, _, _)| *a == agent) {
                Some(row) => {
                    row.1 += 1;
                    row.2 += if h.parent.is_none() { h.size_bytes } else { 0 };
                }
                None => out.push((agent, 1, if h.parent.is_none() { h.size_bytes } else { 0 })),
            }
        }
        out.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
        out
    }

    pub fn ledger(&self) -> Vec<LedgerRow> {
        let home = self.home();
        let now = self.now_ms();
        self.visible()
            .into_iter()
            .map(|h| {
                let wards: Vec<&Ward> = h
                    .wards
                    .iter()
                    .filter(|w| !matches!(w, Ward::Warm { .. }))
                    .collect();
                LedgerRow {
                    id: h.id.clone(),
                    kind: h.kind,
                    tone: Tone::of(h),
                    size_label: format_bytes(h.size_bytes),
                    age_label: h.age_ms(now).map(format_age).unwrap_or_else(|| "·".into()),
                    name: short_name(h, &home),
                    path: tilde(&h.path, &home),
                    agent: h.agent,
                    brand: crate::icons::Brand::for_husk(h),
                    first_ward: wards.first().map(|w| copy::ward_text(w)),
                    ward_count: wards.len(),
                    marked: self.is_marked(&h.id),
                    selected: self.selected.as_ref() == Some(&h.id),
                }
            })
            .collect()
    }

    pub fn drawer(&self) -> Option<DrawerView> {
        if !self.drawer_open {
            return None;
        }
        let husk = self.husk(self.selected.as_ref()?)?;
        let d = copy::get();
        let home = self.home();
        let mut facts: Vec<(String, String)> = Vec::new();
        if let Some(g) = husk.git.as_ref() {
            facts.push((
                d.detail_branch.into(),
                g.branch.clone().unwrap_or_else(|| d.detached.into()),
            ));
            if let Some(head) = &g.head_summary {
                facts.push((d.detail_head.into(), head.clone()));
            }
            facts.push((
                d.detail_upstream.into(),
                g.upstream.clone().unwrap_or_else(|| d.no_upstream.into()),
            ));
            if let Some(name) = self
                .report
                .as_ref()
                .and_then(|r| r.groves.iter().find(|gr| gr.id == g.common_dir))
                .map(|gr| gr.name.clone())
            {
                facts.push((d.detail_grove.into(), name));
            }
            facts.push((
                d.detail_work.into(),
                if g.stranded_commits > 0 {
                    d.work_stranded
                        .replace("{n}", &g.stranded_commits.to_string())
                } else {
                    d.work_safe.into()
                },
            ));
        }
        if let Some(project) = &husk.project {
            facts.push((d.detail_project.into(), tilde(project, &home)));
        }
        if let Some(agent) = husk.agent {
            facts.push((d.detail_agent.into(), agent.as_str().into()));
        }
        if let Some(dups) = self
            .report
            .as_ref()
            .and_then(|r| r.ballast.iter().find(|b| b.husk_id == husk.id))
            .filter(|b| !b.duplicates.is_empty())
        {
            facts.push((
                d.kind_ballast.1.into(),
                d.detail_duplicates
                    .replace("{n}", &dups.duplicates.len().to_string()),
            ));
        }
        let tone = Tone::of(husk);
        let verdict = match tone {
            Tone::Free | Tone::Caution => d.verdict_free,
            Tone::Guarded => d.verdict_guarded,
            Tone::Untouchable => d.verdict_absolute,
        };
        Some(DrawerView {
            id: husk.id.clone(),
            kind: husk.kind,
            kind_label: copy::kind_label(husk.kind, false).into(),
            tone,
            title: short_name(husk, &home),
            path: tilde(&husk.path, &home),
            size_label: format_bytes(husk.size_bytes),
            age_label: husk
                .age_ms(self.now_ms())
                .map(|a| format!("{} {}", format_age(a), d.ago))
                .unwrap_or_else(|| "·".into()),
            agent: husk.agent,
            facts,
            wards: husk
                .wards
                .iter()
                .map(|w| (w.clone(), copy::ward_text(w)))
                .collect(),
            notes: husk
                .notes
                .iter()
                .map(|n| copy::relocalize_note(n))
                .collect(),
            verdict: verdict.into(),
            marked: self.marked.contains(&husk.id),
            can_mark: admissible(husk, false),
            can_force: !admissible(husk, false) && admissible(husk, true),
            forced: self.forced.contains(&husk.id),
            full_path: husk.path.clone(),
        })
    }

    /// (marked count, bytes that would actually go).
    pub fn marked_summary(&self) -> (usize, u64) {
        let bytes = self.plan().reclaimable_bytes() + self.forced_plan().reclaimable_bytes();
        (self.marked.len() + self.forced.len(), bytes)
    }

    fn working_report(&self) -> ScanReport {
        self.report.clone().unwrap_or_else(|| {
            let mut r = ScanReport::empty(Default::default(), 0);
            r.husks = self.live.clone();
            r
        })
    }

    pub fn plan(&self) -> Plan {
        plan_marked(
            &self.working_report(),
            &self.marked,
            false,
            &huskmap_core::SystemClock,
        )
    }

    pub fn forced_plan(&self) -> Plan {
        plan_marked(
            &self.working_report(),
            &self.forced,
            true,
            &huskmap_core::SystemClock,
        )
    }

    /// What the installer and the next launch need to know.
    pub fn session(&self, pid: u32, now_ms: Millis) -> huskmap_core::session::Session {
        use huskmap_core::session::SessionPhase;
        huskmap_core::session::Session {
            pid,
            version: huskmap_core::update::CURRENT_VERSION.into(),
            phase: if self.applying {
                SessionPhase::Applying
            } else if self.is_scanning() {
                SessionPhase::Scanning
            } else if self.report.is_some() {
                SessionPhase::Ready
            } else {
                SessionPhase::Idle
            },
            marked: self.marked.iter().cloned().collect(),
            forced: self.forced.iter().cloned().collect(),
            marked_bytes: self.marked_summary().1,
            updated_at_ms: now_ms,
        }
    }

    /// Bring marks back from a previous window. Landing a report drops any that are gone.
    pub fn restore(&mut self, session: &huskmap_core::session::Session) {
        self.marked.extend(session.marked.iter().cloned());
        self.forced.extend(session.forced.iter().cloned());
        if let Some(report) = self.report.as_ref() {
            let ids: BTreeSet<&HuskId> = report.husks.iter().map(|h| &h.id).collect();
            self.marked.retain(|id| ids.contains(id));
            self.forced.retain(|id| ids.contains(id));
        }
    }

    pub fn offer_update(&mut self, release: &huskmap_core::update::Release, skipped: Option<&str>) {
        if skipped == Some(release.version.as_str()) {
            return;
        }
        self.update = Some(UpdateOffer {
            version: release.version.clone(),
            notes: release.summary(5),
            page: release.page.clone(),
        });
    }

    // ---- mutations ----

    pub fn select(&mut self, id: Option<HuskId>) {
        self.selected = id;
    }

    fn step(&mut self, delta: isize) {
        let order: Vec<HuskId> = self.visible().into_iter().map(|h| h.id.clone()).collect();
        if order.is_empty() {
            self.selected = None;
            return;
        }
        let at = self
            .selected
            .as_ref()
            .and_then(|s| order.iter().position(|id| id == s));
        let next = match at {
            None => 0,
            Some(i) => (i as isize + delta).clamp(0, order.len() as isize - 1) as usize,
        };
        self.selected = Some(order[next].clone());
    }

    pub fn toggle_mark(&mut self, id: &HuskId) {
        let Some(husk) = self.husk(id) else { return };
        if !admissible(husk, false) {
            self.status = Some(copy::get().verdict_guarded.into());
            return;
        }
        if !self.marked.remove(id) {
            self.marked.insert(id.clone());
        }
    }

    /// Mark a guarded husk for a forced send. Untouchable ones still refuse.
    pub fn toggle_force(&mut self, id: &HuskId) {
        let Some(husk) = self.husk(id) else { return };
        if admissible(husk, false) {
            self.toggle_mark(id);
            return;
        }
        if !admissible(husk, true) {
            self.status = Some(copy::get().verdict_absolute.into());
            return;
        }
        if !self.forced.remove(id) {
            self.forced.insert(id.clone());
        }
    }

    /// Show only `kind`, or everything again when that is already the view.
    pub fn set_filter(&mut self, kind: HuskKind) {
        let locale = copy::current_locale();
        self.search = query::only_kind(&self.search, kind, locale, copy::get().query_keys[0]);
        self.reselect();
    }

    /// Keep the selection inside what is visible.
    fn reselect(&mut self) {
        let visible = self.visible();
        if self
            .selected
            .as_ref()
            .is_none_or(|s| !visible.iter().any(|h| &h.id == s))
        {
            self.selected = visible.first().map(|h| h.id.clone());
        }
    }

    pub fn press_chip(&mut self, chip: Chip) {
        let locale = copy::current_locale();
        let keys = copy::get().query_keys;
        self.search = match chip {
            Chip::Kind(k) => query::toggle_value(
                &self.search,
                Key::Kind,
                query::kind_token(k, locale),
                keys[0],
            ),
            Chip::Status(st) => {
                query::toggle_value(&self.search, Key::Status, st.token(locale), keys[1])
            }
            Chip::Agent(a) => query::toggle_value(&self.search, Key::Agent, a.as_str(), keys[2]),
            Chip::Size(v) => query::set_single(&self.search, Key::Size, v, keys[3]),
            Chip::Age(v) => query::set_single(&self.search, Key::Age, v, keys[4]),
        };
        self.reselect();
    }

    pub fn clear_filters(&mut self) {
        self.search.clear();
        self.reselect();
    }

    pub fn toggle_sort(&mut self, key: SortKey) {
        self.sort = if self.sort.key == key {
            Sort {
                key,
                desc: !self.sort.desc,
            }
        } else {
            Sort {
                key,
                desc: key != SortKey::Name,
            }
        };
    }

    /// Visible husks that may go without force.
    fn markable(&self) -> Vec<HuskId> {
        self.visible()
            .into_iter()
            .filter(|h| admissible(h, false))
            .map(|h| h.id.clone())
            .collect()
    }

    /// Mark every visible husk that may go. If they are all marked already, unmark them.
    pub fn mark_visible(&mut self) {
        let ids = self.markable();
        if ids.iter().all(|id| self.marked.contains(id)) {
            for id in &ids {
                self.marked.remove(id);
            }
        } else {
            self.marked.extend(ids);
        }
    }

    pub fn filter_bar(&self) -> FilterBar {
        let d = copy::get();
        let husks = self.husks();
        let text = &self.search;
        let chip = |chip: Chip, label: String, count: Option<usize>, active: bool| ChipView {
            chip,
            label,
            count,
            active,
            tone: match chip {
                Chip::Status(Status::Free) => Some(Tone::Free),
                Chip::Status(Status::Caution) => Some(Tone::Caution),
                Chip::Status(Status::Guarded) => Some(Tone::Guarded),
                Chip::Status(Status::Untouchable) => Some(Tone::Untouchable),
                _ => None,
            },
        };
        let kinds = DIAL
            .iter()
            .map(|k| {
                chip(
                    Chip::Kind(*k),
                    copy::kind_label(*k, true).into(),
                    Some(husks.iter().filter(|h| h.kind == *k).count()),
                    query::has_value(text, Key::Kind, query::kind_token(*k, Locale::En)),
                )
            })
            .collect();
        let home = self.home();
        let now = self.now_ms();
        let count_status = |st: Status| {
            let q = Query::parse(&format!("is:{}", st.token(Locale::En)));
            husks
                .iter()
                .filter(|h| self.matches(&q, h, &home, now))
                .count()
        };
        let status = [
            (Status::Free, d.tone_free),
            (Status::Caution, d.tone_caution),
            (Status::Guarded, d.tone_guarded),
            (Status::Untouchable, d.tone_untouchable),
            (Status::Occupied, d.flag_occupied),
            (Status::Dirty, d.flag_dirty),
            (Status::Stranded, d.flag_stranded),
            (Status::Secrets, d.flag_secrets),
            (Status::Orphaned, d.flag_orphaned),
            (Status::Marked, d.flag_marked),
        ]
        .into_iter()
        .map(|(st, label)| {
            chip(
                Chip::Status(st),
                label.into(),
                Some(count_status(st)),
                query::has_value(text, Key::Status, st.token(Locale::En)),
            )
        })
        .filter(|c| c.count != Some(0) || c.active)
        .collect();
        let agents = self
            .agents()
            .into_iter()
            .map(|(a, n, _)| {
                chip(
                    Chip::Agent(a),
                    a.as_str().into(),
                    Some(n),
                    query::has_value(text, Key::Agent, a.as_str()),
                )
            })
            .collect();
        let any_size = !Query::parse(text)
            .terms
            .iter()
            .any(|t| !t.negated && matches!(t.field, query::Field::Size(..)));
        let any_age = !Query::parse(text)
            .terms
            .iter()
            .any(|t| !t.negated && matches!(t.field, query::Field::Age(..)));
        let mut sizes = vec![chip(Chip::Size(None), d.filter_any.into(), None, any_size)];
        for step in SIZE_STEPS {
            let label = format!(
                "> {}",
                query::parse_size(step).map_or(String::new(), |(_, b)| format_bytes(b))
            )
            .replace(".0 ", " ");
            sizes.push(chip(
                Chip::Size(Some(step)),
                label,
                None,
                query::has_value(text, Key::Size, step),
            ));
        }
        let mut ages = vec![chip(Chip::Age(None), d.filter_any.into(), None, any_age)];
        for (step, label) in
            AGE_STEPS
                .iter()
                .zip([d.age_day, d.age_week, d.age_month, d.age_quarter])
        {
            ages.push(chip(
                Chip::Age(Some(step)),
                format!("> {label}"),
                None,
                query::has_value(text, Key::Age, step),
            ));
        }
        let visible = self.visible();
        let bytes: u64 = visible
            .iter()
            .filter(|h| {
                !visible
                    .iter()
                    .any(|o| o.path != h.path && h.path.starts_with(&o.path))
            })
            .map(|h| h.size_bytes)
            .sum();
        let markable = self.markable();
        let all_marked = !markable.is_empty() && markable.iter().all(|id| self.marked.contains(id));
        FilterBar {
            kinds,
            status,
            agents,
            sizes,
            ages,
            summary: d
                .filter_count
                .replace("{n}", &visible.len().to_string())
                .replace("{total}", &husks.len().to_string())
                .replace("{bytes}", &format_bytes(bytes)),
            filtered: !Query::parse(text).is_empty(),
            open: self.filters_open,
            hidden_active: Query::parse(text)
                .terms
                .iter()
                .filter(|t| !matches!(t.field, query::Field::Kind(_)))
                .count(),
            mark_all: (!markable.is_empty()).then(|| {
                let n = markable.len().to_string();
                if all_marked {
                    (d.unmark_visible.replace("{n}", &n), true)
                } else {
                    (d.mark_visible.replace("{n}", &n), false)
                }
            }),
        }
    }

    pub fn open_guide(&mut self, section: usize) {
        self.guide_open = true;
        self.guide_section = section.min(copy::get().guide.sections.len().saturating_sub(1));
    }

    pub fn open_confirm(&mut self) {
        if self.applying {
            return;
        }
        if self.is_scanning() {
            self.status = Some(copy::get().wait_scan.into());
            return;
        }
        let plan = self.plan();
        let forced_plan = self.forced_plan();
        let total = plan.actions.len() + forced_plan.actions.len();
        if total == 0 {
            self.status = Some(copy::get().nothing_to_apply.into());
            return;
        }
        let d = copy::get();
        let home = self.home();
        let bytes = plan.reclaimable_bytes() + forced_plan.reclaimable_bytes();
        let guarded = self.marked.len().saturating_sub(plan.actions.len());
        let paths: Vec<String> = forced_plan
            .actions
            .iter()
            .chain(&plan.actions)
            .take(6)
            .map(|a| tilde(&a.path, &home))
            .collect();
        let forced_n = forced_plan.actions.len();
        self.confirm = Some(ConfirmView {
            title: if total == 1 {
                d.apply_title_one.to_string()
            } else {
                d.apply_title.replace("{n}", &total.to_string())
            },
            body: d.apply_confirm.replace("{bytes}", &format_bytes(bytes)),
            bytes,
            bytes_label: format_bytes(bytes),
            more: total.saturating_sub(paths.len()),
            paths,
            guarded_note: (guarded > 0)
                .then(|| d.apply_guarded_note.replace("{n}", &guarded.to_string())),
            forced_note: (forced_n > 0)
                .then(|| d.forced_note.replace("{n}", &forced_n.to_string())),
            plan,
            forced_plan,
        });
    }

    pub fn begin_scan(&mut self) {
        self.phase = Phase::Scanning {
            walking: None,
            weighing: None,
        };
        self.live.clear();
        self.confirm = None;
    }

    pub fn on_event(&mut self, event: &ScanEvent, home: &std::path::Path) {
        match event {
            ScanEvent::Walking { root, .. } => {
                self.phase = Phase::Scanning {
                    walking: Some(tilde(root, home)),
                    weighing: None,
                };
            }
            ScanEvent::Weighing { pending } => {
                self.phase = Phase::Scanning {
                    walking: None,
                    weighing: Some(*pending),
                };
            }
            ScanEvent::Found(h) => self.live.push((**h).clone()),
        }
    }

    pub fn land(&mut self, report: ScanReport) {
        let ids: BTreeSet<HuskId> = report.husks.iter().map(|h| h.id.clone()).collect();
        self.marked.retain(|id| ids.contains(id));
        self.forced.retain(|id| ids.contains(id));
        if self.selected.as_ref().is_some_and(|s| !ids.contains(s)) {
            self.selected = None;
            self.drawer_open = false;
        }
        self.phase = Phase::Ready;
        self.live.clear();
        self.report = Some(report);
        self.generation += 1;
        if self.selected.is_none() {
            self.selected = self.visible().first().map(|h| h.id.clone());
        }
    }

    pub fn fail(&mut self, err: String) {
        self.phase = Phase::Failed(err);
    }

    pub fn applied(&mut self, result: &ApplyResult) {
        self.confirm = None;
        self.applying = false;
        self.last_reclaimed = result.bytes_reclaimed();
        let gone: BTreeSet<_> = result.applied.iter().map(|a| a.path.clone()).collect();
        self.marked.retain(|id| !gone.contains(&id.path));
        self.forced.retain(|id| !gone.contains(&id.path));
        self.status = Some(copy::format_apply_summary(
            result.bytes_reclaimed(),
            result.skipped.len(),
            result.errors.len(),
        ));
    }

    /// Keyboard. Returns what the shell must do beyond state changes.
    pub fn key(&mut self, key: &str) -> Intent {
        if self.searching {
            match key {
                // Stop typing, keep the filter. A second escape clears it.
                "escape" | "enter" => self.searching = false,
                "backspace" => {
                    self.search.pop();
                }
                k if k.chars().count() == 1 => self.search.push_str(k),
                _ => {}
            }
            self.reselect();
            return Intent::None;
        }
        if self.guide_open {
            let last = copy::get().guide.sections.len().saturating_sub(1);
            match key {
                "escape" | "?" | "enter" | "q" => self.guide_open = false,
                "j" | "down" | "right" => self.guide_section = (self.guide_section + 1).min(last),
                "k" | "up" | "left" => self.guide_section = self.guide_section.saturating_sub(1),
                k if k.len() == 1 && k.chars().all(|c| c.is_ascii_digit()) => {
                    let n = k.parse::<usize>().unwrap_or(1).max(1);
                    self.guide_section = (n - 1).min(last);
                }
                _ => {}
            }
            return Intent::None;
        }
        if self.update_open {
            return match key {
                "escape" | "n" => {
                    self.update_open = false;
                    Intent::None
                }
                "enter" | "y" if !self.updating && !self.applying => Intent::Update,
                _ => Intent::None,
            };
        }
        if self.confirm.is_some() {
            return match key {
                "escape" | "n" => {
                    self.confirm = None;
                    Intent::None
                }
                "enter" | "y" => Intent::Apply,
                _ => Intent::None,
            };
        }
        match key {
            "j" | "down" => self.step(1),
            "k" | "up" => self.step(-1),
            "g" | "home" => {
                self.selected = None;
                self.step(0);
            }
            "enter" => self.drawer_open = self.selected.is_some(),
            "escape" => {
                if self.drawer_open {
                    self.drawer_open = false;
                } else if !self.search.is_empty() {
                    self.clear_filters();
                }
            }
            "x" => {
                if let Some(id) = self.selected.clone() {
                    self.toggle_mark(&id);
                }
            }
            "X" => {
                if let Some(id) = self.selected.clone() {
                    self.toggle_force(&id);
                }
            }
            "a" => self.open_confirm(),
            // Keep what is there: chips and typed filters add up.
            "/" => self.searching = true,
            "?" => self.open_guide(0),
            "t" => self.theme = self.theme.next(),
            "tab" => {
                self.mode = match self.mode {
                    ViewMode::Map => ViewMode::Ledger,
                    ViewMode::Ledger => ViewMode::Map,
                }
            }
            "s" if !self.is_scanning() => return Intent::Scan,
            "u" if self.update.is_some() => self.update_open = true,
            k if k.len() == 1 && ('1'..='6').contains(&k.chars().next().unwrap_or('0')) => {
                let i = k.parse::<usize>().unwrap_or(1) - 1;
                if let Some(kind) = DIAL.get(i).copied() {
                    self.set_filter(kind);
                }
            }
            _ => {}
        }
        Intent::None
    }

    pub fn status_line(&self) -> String {
        let d = copy::get();
        match &self.phase {
            Phase::Scanning {
                walking: Some(root),
                ..
            } => d.walking.replace("{root}", root),
            Phase::Scanning {
                weighing: Some(n), ..
            } => d.weighing.replace("{n}", &n.to_string()),
            Phase::Scanning { .. } => d.scanning.into(),
            Phase::Failed(err) => format!("{} {err}", d.error_grove),
            _ if self.applying => d.applying_now.into(),
            _ if self.updating => d.updating.replace(
                "{v}",
                self.update.as_ref().map_or("", |u| u.version.as_str()),
            ),
            Phase::Idle => d.scan_hint.into(),
            Phase::Ready => self.status.clone().unwrap_or_else(|| {
                let husks = self.husks().len();
                let groves = self.report.as_ref().map(|r| r.groves.len()).unwrap_or(0);
                format!("{husks} {} · {groves} {}", d.husks, d.groves)
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use huskmap_core::{AgentKind, GitFacts, Holder, Locale, ScanReport, copy::with_locale};
    use std::path::{Path, PathBuf};

    const NOW: Millis = 400 * 86_400_000;

    fn husk(kind: HuskKind, path: &str, bytes: u64, age_days: u64) -> Husk {
        let mut h = Husk::bare(kind, path, bytes);
        h.mtime_ms = Some(NOW - age_days * 86_400_000);
        h
    }

    fn report() -> ScanReport {
        let mut r = ScanReport::empty(PathBuf::from("/h"), NOW);
        let mut busy = husk(HuskKind::Worktree, "/h/dev/app-wt/busy", 900, 0);
        busy.git = Some(GitFacts {
            branch: Some("feat/busy".into()),
            head_summary: Some("wip".into()),
            common_dir: PathBuf::from("/h/dev/app/.git"),
            ..Default::default()
        });
        busy.agent = Some(AgentKind::Claude);
        busy.ward(Ward::Occupied {
            holders: vec![Holder {
                pid: 1,
                name: "claude".into(),
                cwd: PathBuf::from("/h/dev/app-wt/busy"),
                agent: Some(AgentKind::Claude),
            }],
        });
        let mut dirty = husk(HuskKind::Worktree, "/h/dev/app-wt/dirty", 500, 3);
        dirty.git = Some(GitFacts::default());
        dirty.ward(Ward::Dirty { files: 2 });
        let mut clean = husk(HuskKind::Worktree, "/h/dev/app-wt/clean", 400, 40);
        clean.risk = Risk::Caution;
        clean.git = Some(GitFacts {
            branch: Some("fix/clean".into()),
            ..Default::default()
        });
        let mut nm = husk(
            HuskKind::Ballast,
            "/h/dev/app-wt/clean/node_modules",
            300,
            40,
        );
        nm.parent = Some(clean.id.clone());
        let npm = husk(HuskKind::Toolchain, "/h/.npm/_cacache", 1000, 2);
        let mut sess = husk(HuskKind::Afterimage, "/h/.claude/projects/x", 10, 200);
        sess.agent = Some(AgentKind::Claude);
        sess.risk = Risk::Caution;
        r.husks = vec![npm, busy, dirty, clean, nm, sess];
        r.totals.bytes_seen = 2810;
        r.totals.bytes_reclaimable = 1710;
        r.groves = vec![huskmap_core::Grove {
            id: PathBuf::from("/h/dev/app/.git"),
            name: "app".into(),
            primary: None,
            worktrees: vec![],
            husks: vec![],
            bytes: 0,
        }];
        r
    }

    fn id(s: &AppState, path: &str) -> HuskId {
        s.husks()
            .iter()
            .find(|h| h.path == std::path::Path::new(path))
            .unwrap()
            .id
            .clone()
    }

    #[test]
    fn tones() {
        let r = report();
        let tones: Vec<Tone> = r.husks.iter().map(Tone::of).collect();
        assert_eq!(
            tones,
            vec![
                Tone::Free,
                Tone::Untouchable,
                Tone::Guarded,
                Tone::Caution,
                Tone::Free,
                Tone::Caution
            ]
        );
        let mut forbidden = Husk::bare(HuskKind::Worktree, "/p", 1);
        forbidden.risk = Risk::Forbidden;
        assert_eq!(Tone::of(&forbidden), Tone::Untouchable);
    }

    #[test]
    fn geometry_is_stable_and_bounded() {
        let s = sectors();
        assert_eq!(s.len(), 6);
        assert!((s[5].start + s[5].sweep - TAU).abs() < 1e-4);
        assert!((s[0].mid() - TAU / 12.0).abs() < 1e-4);
        assert_eq!(hash01("a"), hash01("a"));
        assert_ne!(hash01("a"), hash01("b"));
        let spread: Vec<f32> = (0..20)
            .map(|i| hash01(&format!("/h/.claude/projects/p{i}")))
            .collect();
        let (lo, hi) = spread
            .iter()
            .fold((1f32, 0f32), |(l, h), x| (l.min(*x), h.max(*x)));
        assert!(
            hi - lo > 0.7,
            "similar paths spread over the sector: {spread:?}"
        );
        assert!((0.0..1.0).contains(&hash01("anything at all")));
        assert!((age_radius(0.0) - INNER_R).abs() < 1e-6);
        assert!((age_radius(10_000.0) - OUTER_R).abs() < 1e-6);
        assert!(age_radius(7.0) < age_radius(30.0));
        let (x, y) = polar(0.0, 1.0);
        assert!(
            x.abs() < 1e-6 && (y + 1.0).abs() < 1e-6,
            "twelve o'clock is up"
        );
        with_locale(Locale::En, || assert_eq!(rings()[0].0, "1 day"));
    }

    #[test]
    fn nodes_sit_in_their_sector_and_weigh_by_bytes() {
        let s = AppState::with_report(report());
        let nodes = s.nodes();
        let sectors = sectors();
        for n in &nodes {
            let sec = sectors.iter().find(|x| x.kind == n.kind).unwrap();
            assert!(
                n.angle >= sec.start && n.angle <= sec.start + sec.sweep,
                "{n:?}"
            );
            assert!(n.radius >= INNER_R * 0.8 && n.radius <= OUTER_R);
        }
        assert_eq!(nodes[0].rank, 0);
        assert_eq!(nodes[0].weight, 1.0);
        assert_eq!(nodes[0].kind, HuskKind::Toolchain);
        let busy = nodes.iter().find(|n| n.occupied).unwrap();
        assert!(busy.alarm && busy.tone == Tone::Untouchable);
        assert_eq!(busy.name, "feat/busy");
        let young = nodes.iter().find(|n| n.name == "feat/busy").unwrap();
        let old = nodes.iter().find(|n| n.name == "x").unwrap();
        assert!(young.radius < old.radius, "older drifts outward");
        // pick finds the node under the point
        let (x, y) = young.xy();
        assert_eq!(s.pick(x, y, 0.05), Some(young.id.clone()));
        assert_eq!(s.pick(5.0, 5.0, 0.05), None);
    }

    #[test]
    fn short_names() {
        let r = report();
        let home = PathBuf::from("/h");
        let names: Vec<String> = r.husks.iter().map(|h| short_name(h, &home)).collect();
        assert_eq!(names[0], ".npm/_cacache");
        assert_eq!(names[1], "feat/busy");
        assert_eq!(names[2], "dirty");
        assert_eq!(names[4], "clean/node_modules");
    }

    #[test]
    fn legend_alarms_agents() {
        with_locale(Locale::En, || {
            let s = AppState::with_report(report());
            let legend = s.legend();
            assert_eq!(legend.len(), 6);
            let wt = &legend[0];
            assert_eq!(
                (wt.label.as_str(), wt.count, wt.bytes),
                ("Worktrees", 3, 1800)
            );
            assert!(legend.iter().all(|l| l.active));
            let alarms = s.alarms();
            assert_eq!(alarms.len(), 2);
            assert_eq!(alarms[0].title, "feat/busy", "untouchable first");
            assert_eq!(alarms[0].agent, Some(AgentKind::Claude));
            assert!(alarms[0].lines[0].1.contains("claude · pid 1"));
            assert_eq!(alarms[1].lines[0].1, "2 uncommitted changes");
            assert_eq!(alarms[0].path, "~/dev/app-wt/busy");
            let agents = s.agents();
            assert_eq!(agents[0].0, AgentKind::Claude);
            assert_eq!(agents[0].1, 2);
            assert_eq!(s.totals(), (1710, 2810));
        });
    }

    #[test]
    fn filter_search_and_ledger() {
        with_locale(Locale::En, || {
            let mut s = AppState::with_report(report());
            s.set_filter(HuskKind::Worktree);
            assert_eq!(s.search, "kind:worktree");
            assert_eq!(s.visible().len(), 3);
            assert!(!s.legend()[1].active);
            s.set_filter(HuskKind::Worktree);
            assert!(s.search.is_empty(), "same kind toggles off");
            s.search = "claude".into();
            assert_eq!(s.visible().len(), 2, "agent name matches");
            s.search = "fix/".into();
            assert_eq!(s.visible().len(), 1, "branch matches");
            s.search = "node modules".into();
            assert_eq!(s.visible().len(), 1, "every word must match somewhere");
            s.search = "node zzz".into();
            assert_eq!(s.visible().len(), 0);
            s.search.clear();
            let ledger = s.ledger();
            assert_eq!(ledger.len(), 6);
            assert_eq!(ledger[0].name, ".npm/_cacache");
            assert_eq!(ledger[0].brand, Some(crate::icons::Brand::Npm));
            assert_eq!(ledger.iter().filter(|r| r.selected).count(), 1);
            let busy = ledger.iter().find(|r| r.name == "feat/busy").unwrap();
            assert_eq!(busy.ward_count, 1);
            assert!(
                busy.first_ward
                    .as_deref()
                    .unwrap()
                    .contains("using this folder now")
            );
            assert_eq!(busy.age_label, "0m");
        });
    }

    #[test]
    fn drawer_facts_and_verdicts() {
        with_locale(Locale::En, || {
            let mut s = AppState::with_report(report());
            assert!(s.drawer().is_none());
            s.selected = Some(id(&s, "/h/dev/app-wt/busy"));
            s.drawer_open = true;
            let d = s.drawer().unwrap();
            assert_eq!(d.verdict, "Blocked. huskmap will not touch it.");
            assert!(!d.can_mark);
            let keys: Vec<&str> = d.facts.iter().map(|(k, _)| k.as_str()).collect();
            assert_eq!(
                keys,
                vec![
                    "branch",
                    "last commit",
                    "remote branch",
                    "repo",
                    "commits",
                    "agent"
                ]
            );
            assert_eq!(d.facts[4].1, "every commit is also on another branch");
            assert!(!d.can_force, "occupied is never forceable");
            assert_eq!(d.facts[2].1, "never pushed");
            assert_eq!(d.age_label, "0m ago");
            s.selected = Some(id(&s, "/h/dev/app-wt/dirty"));
            let d = s.drawer().unwrap();
            assert_eq!(d.verdict, "Protected. You can still mark it anyway.");
            assert!(d.can_force && !d.forced);
            assert_eq!(d.facts[0].1, "no branch (detached)");
            s.selected = Some(id(&s, "/h/.npm/_cacache"));
            let d = s.drawer().unwrap();
            assert_eq!(d.verdict, "Safe to remove.");
            assert!(d.can_mark && d.facts.is_empty());
            s.selected = None;
            assert!(s.drawer().is_none());
        });
    }

    #[test]
    fn marking_and_confirm() {
        with_locale(Locale::En, || {
            let mut s = AppState::with_report(report());
            s.open_confirm();
            assert!(s.confirm.is_none());
            assert_eq!(s.status.as_deref(), Some("Nothing marked can be removed."));
            let busy = id(&s, "/h/dev/app-wt/busy");
            s.toggle_mark(&busy);
            assert!(s.marked.is_empty(), "untouchable cannot be marked");
            let npm = id(&s, "/h/.npm/_cacache");
            let clean = id(&s, "/h/dev/app-wt/clean");
            s.toggle_mark(&npm);
            s.toggle_mark(&clean);
            assert_eq!(s.marked_summary(), (2, 1400));
            s.toggle_mark(&clean);
            assert_eq!(s.marked.len(), 1);
            s.toggle_mark(&clean);
            s.toggle_mark(&HuskId::new(HuskKind::Cache, "/nope"));
            s.open_confirm();
            let c = s.confirm.clone().unwrap();
            assert_eq!(c.title, "Move 2 items to the trash?");
            assert_eq!(c.bytes, 1400);
            assert_eq!(c.paths, vec!["~/.npm/_cacache", "~/dev/app-wt/clean"]);
            assert!(c.guarded_note.is_none());
            assert!(c.body.contains("1.4 KB"));
            // nodes reflect marks
            assert!(s.nodes().iter().filter(|n| n.marked).count() == 2);
        });
    }

    #[test]
    fn forcing_guarded_husks() {
        with_locale(Locale::En, || {
            let mut s = AppState::with_report(report());
            let dirty = id(&s, "/h/dev/app-wt/dirty");
            let busy = id(&s, "/h/dev/app-wt/busy");
            let npm = id(&s, "/h/.npm/_cacache");
            s.toggle_force(&busy);
            assert!(s.forced.is_empty());
            assert_eq!(
                s.status.as_deref(),
                Some("Blocked. huskmap will not touch it.")
            );
            s.toggle_force(&npm);
            assert!(
                s.marked.contains(&npm),
                "free husks force-mark as plain marks"
            );
            s.selected = Some(dirty.clone());
            s.key("X");
            assert!(s.forced.contains(&dirty));
            assert_eq!(s.marked_summary(), (2, 1500));
            assert!(s.nodes().iter().any(|n| n.forced));
            s.open_confirm();
            let c = s.confirm.clone().unwrap();
            assert_eq!(c.title, "Move 2 items to the trash?");
            assert_eq!(c.forced_plan.actions.len(), 1);
            assert_eq!(c.plan.actions.len(), 1);
            assert!(c.forced_note.unwrap().starts_with("Marked anyway: 1."));
            assert_eq!(
                c.paths[0], "~/dev/app-wt/dirty",
                "forced ones are listed first"
            );
            s.key("escape");
            s.key("X");
            assert!(s.forced.is_empty(), "X toggles");
            s.forced.insert(dirty.clone());
            s.land(report());
            assert!(s.forced.contains(&dirty));
            s.applied(&ApplyResult {
                applied: vec![huskmap_core::AppliedAction {
                    path: PathBuf::from("/h/dev/app-wt/dirty"),
                    kind: huskmap_core::ActionKind::GitWorktreeRemove,
                    bytes: 500,
                }],
                ..Default::default()
            });
            assert!(s.forced.is_empty());
        });
    }

    #[test]
    fn update_offer_modal_and_session() {
        with_locale(Locale::En, || {
            let mut s = AppState::with_report(report());
            assert_eq!(s.key("u"), Intent::None);
            assert!(!s.update_open, "nothing to offer yet");
            let release = huskmap_core::update::Release {
                version: "9.9.9".into(),
                page: "https://x".into(),
                notes: "- faster\n- braver".into(),
                assets: vec![],
            };
            s.offer_update(&release, Some("9.9.9"));
            assert!(s.update.is_none(), "skipped versions stay quiet");
            s.offer_update(&release, None);
            assert_eq!(s.update.as_ref().unwrap().notes, vec!["faster", "braver"]);
            s.key("u");
            assert!(s.update_open);
            assert_eq!(s.key("j"), Intent::None, "modal swallows keys");
            assert_eq!(s.key("enter"), Intent::Update);
            s.applying = true;
            assert_eq!(s.key("enter"), Intent::None, "never during an apply");
            s.applying = false;
            s.updating = true;
            assert_eq!(s.status_line(), "Updating to 9.9.9");
            assert_eq!(s.key("y"), Intent::None);
            s.key("escape");
            assert!(!s.update_open);

            let npm = id(&s, "/h/.npm/_cacache");
            let dirty = id(&s, "/h/dev/app-wt/dirty");
            s.toggle_mark(&npm);
            s.toggle_force(&dirty);
            let snap = s.session(42, 7);
            assert_eq!(snap.phase, huskmap_core::session::SessionPhase::Ready);
            assert_eq!(
                (snap.marked.len(), snap.forced.len(), snap.marked_bytes),
                (1, 1, 1500)
            );
            let mut fresh = AppState::with_report(report());
            let mut stale = snap.clone();
            stale.marked.push(HuskId::new(HuskKind::Cache, "/gone"));
            fresh.restore(&stale);
            assert_eq!(fresh.marked.len(), 1, "unknown husks are dropped");
            assert_eq!(fresh.forced.len(), 1);
            let mut empty = AppState::default();
            empty.restore(&stale);
            assert_eq!(empty.marked.len(), 2, "kept until a report can judge them");
            assert_eq!(
                empty.session(1, 1).phase,
                huskmap_core::session::SessionPhase::Idle
            );
            empty.begin_scan();
            assert_eq!(
                empty.session(1, 1).phase,
                huskmap_core::session::SessionPhase::Scanning
            );
            empty.applying = true;
            assert_eq!(
                empty.session(1, 1).phase,
                huskmap_core::session::SessionPhase::Applying
            );
            empty.marked.insert(npm.clone());
            empty.open_confirm();
            assert!(empty.confirm.is_none(), "no second apply while one runs");
        });
    }

    #[test]
    fn keyboard_drives_everything() {
        with_locale(Locale::En, || {
            let mut s = AppState::with_report(report());
            let order: Vec<HuskId> = s.visible().iter().map(|h| h.id.clone()).collect();
            assert_eq!(s.selected.as_ref(), Some(&order[0]));
            s.key("j");
            s.key("down");
            assert_eq!(s.selected.as_ref(), Some(&order[2]));
            s.key("k");
            assert_eq!(s.selected.as_ref(), Some(&order[1]));
            for _ in 0..20 {
                s.key("j");
            }
            assert_eq!(s.selected.as_ref(), order.last());
            s.key("g");
            assert_eq!(s.selected.as_ref(), Some(&order[0]));
            s.key("enter");
            assert!(s.drawer_open);
            s.key("x");
            assert_eq!(s.marked.len(), 1);
            s.key("escape");
            assert!(!s.drawer_open);
            s.key("tab");
            assert_eq!(s.mode, ViewMode::Ledger);
            s.key("tab");
            assert_eq!(s.mode, ViewMode::Map);
            s.key("1");
            assert_eq!(s.search, "kind:worktree");
            s.key("escape");
            assert!(s.search.is_empty());
            // search mode swallows letters
            s.key("/");
            for k in ["f", "i", "x", "q"] {
                s.key(k);
            }
            assert_eq!(s.search, "fixq");
            s.key("backspace");
            assert_eq!(s.search, "fix");
            s.key("left");
            s.key("enter");
            assert!(!s.searching);
            assert_eq!(s.visible().len(), 1);
            assert_eq!(s.selected.as_ref(), Some(&s.visible()[0].id));
            s.key("/");
            s.key("escape");
            assert_eq!(s.search, "fix", "escape stops typing, keeps the filter");
            s.key("escape");
            assert!(s.search.is_empty(), "a second escape clears it");
            assert_eq!(s.key("s"), Intent::Scan);
            s.key("a");
            assert!(s.confirm.is_some());
            assert_eq!(s.key("j"), Intent::None);
            assert_eq!(s.key("y"), Intent::Apply);
            s.key("n");
            assert!(s.confirm.is_none());
            s.key("a");
            s.key("escape");
            assert!(s.confirm.is_none());
            s.key("a");
            assert_eq!(s.key("enter"), Intent::Apply);
            assert_eq!(s.key("?"), Intent::None);
            s.confirm = None;
            s.key("t");
            assert_eq!(s.theme, crate::theme::ThemeChoice::Light);
            s.key("t");
            assert_eq!(s.theme, crate::theme::ThemeChoice::Dark);
        });
    }

    #[test]
    fn scan_lifecycle_streams_then_lands() {
        with_locale(Locale::En, || {
            let mut s = AppState::default();
            assert_eq!(
                s.status_line(),
                "Scanning only reads. Nothing is removed until you confirm."
            );
            s.begin_scan();
            assert!(s.is_scanning());
            assert_eq!(s.key("s"), Intent::None, "no double scan");
            assert_eq!(s.status_line(), "Scanning");
            let home = PathBuf::from("/h");
            s.on_event(
                &ScanEvent::Walking {
                    root: PathBuf::from("/h/dev"),
                    index: 0,
                    total: 1,
                },
                &home,
            );
            assert_eq!(s.status_line(), "Reading ~/dev");
            s.on_event(&ScanEvent::Weighing { pending: 3 }, &home);
            assert_eq!(s.status_line(), "Measuring 3 items");
            let mut outer = Husk::bare(HuskKind::Worktree, "/h/w", 100);
            outer.reclaimable = false;
            s.on_event(&ScanEvent::Found(Box::new(outer)), &home);
            s.on_event(
                &ScanEvent::Found(Box::new(Husk::bare(
                    HuskKind::Ballast,
                    "/h/w/node_modules",
                    60,
                ))),
                &home,
            );
            assert_eq!(s.husks().len(), 2);
            assert_eq!(s.totals(), (0, 100), "nested bytes not doubled while live");
            assert_eq!(s.nodes().len(), 2);
            s.land(report());
            assert_eq!(s.phase, Phase::Ready);
            // a rescan keeps the old map on screen and refuses to send
            s.begin_scan();
            assert_eq!(s.husks().len(), 6);
            assert_eq!(s.totals(), (1710, 2810));
            s.marked.insert(s.husks()[0].id.clone());
            s.open_confirm();
            assert!(s.confirm.is_none());
            assert_eq!(
                s.status.as_deref(),
                Some("The scan is still running. Wait for it to finish.")
            );
            s.land(report());
            assert_eq!(s.generation, 2);
            assert_eq!(
                s.status_line(),
                "The scan is still running. Wait for it to finish.",
                "messages outlive a rescan"
            );
            s.status = None;
            assert!(s.live.is_empty());
            assert_eq!(s.status_line(), "6 items · 1 repos");
            s.fail("boom".into());
            assert!(s.status_line().contains("boom"));
        });
    }

    #[test]
    fn landing_keeps_valid_marks_and_apply_reports() {
        with_locale(Locale::En, || {
            let mut s = AppState::with_report(report());
            let npm = id(&s, "/h/.npm/_cacache");
            s.toggle_mark(&npm);
            s.marked.insert(HuskId::new(HuskKind::Cache, "/gone"));
            s.selected = Some(HuskId::new(HuskKind::Cache, "/gone"));
            s.drawer_open = true;
            s.land(report());
            assert_eq!(s.marked.len(), 1);
            assert!(!s.drawer_open);
            assert_eq!(
                s.selected.as_ref().unwrap().path,
                PathBuf::from("/h/.npm/_cacache")
            );
            let result = ApplyResult {
                applied: vec![huskmap_core::AppliedAction {
                    path: PathBuf::from("/h/.npm/_cacache"),
                    kind: huskmap_core::ActionKind::Trash,
                    bytes: 1000,
                }],
                skipped: vec![],
                errors: vec![],
            };
            s.applied(&result);
            assert!(s.marked.is_empty());
            assert_eq!(s.last_reclaimed, 1000);
            assert_eq!(s.status_line(), "1000 B freed · 0 skipped · 0 errors");
            let empty = AppState::default();
            assert!(empty.plan().actions.is_empty());
            assert!(empty.now_ms() > 0);
        });
    }

    #[test]
    fn sorting_the_list() {
        with_locale(Locale::En, || {
            let mut s = AppState::with_report(report());
            let names = |s: &AppState| -> Vec<String> {
                s.visible()
                    .iter()
                    .map(|h| short_name(h, Path::new("/h")))
                    .collect()
            };
            assert_eq!(names(&s)[0], ".npm/_cacache", "heaviest first by default");
            s.toggle_sort(SortKey::Size);
            assert_eq!(names(&s)[0], "x", "lightest first");
            s.toggle_sort(SortKey::Age);
            assert_eq!(names(&s)[0], "x", "oldest first");
            s.toggle_sort(SortKey::Age);
            assert_eq!(names(&s)[0], "feat/busy", "newest first");
            s.toggle_sort(SortKey::Name);
            assert!(!s.sort.desc);
            assert_eq!(names(&s)[0], ".npm/_cacache");
            assert_eq!(names(&s).last().map(String::as_str), Some("x"));
            s.toggle_sort(SortKey::Name);
            assert_eq!(names(&s)[0], "x");
            assert_eq!(
                s.nodes()[0].kind,
                HuskKind::Toolchain,
                "the map ranks by weight"
            );
        });
    }

    #[test]
    fn chips_filter_and_mark_what_is_visible() {
        with_locale(Locale::PtBr, || {
            let mut s = AppState::with_report(report());
            let bar = s.filter_bar();
            assert!(!bar.filtered);
            assert_eq!(bar.kinds.len(), 6);
            assert_eq!(bar.kinds[0].count, Some(3));
            assert_eq!(bar.summary, "6 de 6 itens · 2.7 KB");
            assert!(bar.sizes[0].active && bar.ages[0].active);
            assert_eq!(bar.sizes[3].label, "> 1 GB");
            assert_eq!(bar.ages[2].label, "> 1 semana");
            assert!(bar.status.iter().all(|c| c.count != Some(0)));
            assert_eq!(bar.agents[0].label, "claude");
            s.press_chip(Chip::Kind(HuskKind::Worktree));
            s.press_chip(Chip::Kind(HuskKind::Ballast));
            assert_eq!(s.search, "tipo:worktree,deps");
            assert_eq!(s.visible().len(), 4);
            s.press_chip(Chip::Status(Status::Free));
            assert_eq!(s.search, "tipo:worktree,deps status:livre");
            let bar = s.filter_bar();
            assert!(!bar.open, "only the type row shows at first");
            assert_eq!(
                bar.hidden_active, 1,
                "the closed bar still says a filter is on"
            );
            s.filters_open = true;
            assert!(s.filter_bar().open);
            assert_eq!(s.visible().len(), 1, "only the node_modules is free");
            let bar = s.filter_bar();
            assert!(bar.filtered && bar.kinds[0].active && !bar.kinds[2].active);
            assert_eq!(
                bar.mark_all,
                Some(("Marcar 1 que podem sair".into(), false))
            );
            s.mark_visible();
            assert_eq!(s.marked.len(), 1);
            assert!(s.filter_bar().mark_all.unwrap().1);
            s.mark_visible();
            assert!(s.marked.is_empty(), "a second press unmarks");
            s.press_chip(Chip::Status(Status::Free));
            s.press_chip(Chip::Size(Some(">100mb")));
            assert!(s.visible().is_empty());
            assert!(s.filter_bar().sizes[2].active);
            s.press_chip(Chip::Size(None));
            s.press_chip(Chip::Age(Some(">30d")));
            assert_eq!(s.visible().len(), 2, "clean worktree and its node_modules");
            s.press_chip(Chip::Agent(AgentKind::Claude));
            assert!(s.visible().is_empty());
            assert!(s.filter_bar().mark_all.is_none());
            s.clear_filters();
            assert!(s.search.is_empty());
            assert_eq!(s.visible().len(), 6);
            s.search = "status:marcado".into();
            assert!(s.visible().is_empty());
            let npm = id(&s, "/h/.npm/_cacache");
            s.marked.insert(npm);
            assert_eq!(s.visible().len(), 1);
        });
    }

    #[test]
    fn guide_opens_and_pages() {
        with_locale(Locale::En, || {
            let mut s = AppState::with_report(report());
            s.key("?");
            assert!(s.guide_open);
            assert_eq!(s.key("j"), Intent::None);
            assert_eq!(s.guide_section, 1);
            assert!(
                s.selected.is_some() && !s.drawer_open,
                "keys stay in the guide"
            );
            s.key("k");
            s.key("k");
            assert_eq!(s.guide_section, 0);
            s.key("9");
            assert_eq!(s.guide_section, copy::get().guide.sections.len() - 1);
            s.key("right");
            assert_eq!(s.guide_section, copy::get().guide.sections.len() - 1);
            s.key("3");
            assert_eq!(s.guide_section, 2);
            s.key("x");
            s.key("escape");
            assert!(!s.guide_open);
            assert!(s.marked.is_empty());
            s.open_guide(99);
            assert_eq!(s.guide_section, copy::get().guide.sections.len() - 1);
        });
    }

    #[test]
    fn session_names_read_like_projects() {
        let home = PathBuf::from("/home/me");
        let mut grok = Husk::bare(
            HuskKind::Afterimage,
            "/home/me/.grok/sessions/%2Fhome%2Fme%2Fdev%2Fpixeiro",
            1,
        );
        grok.project = Some(PathBuf::from("/home/me/dev/pixeiro"));
        assert_eq!(short_name(&grok, &home), "pixeiro");
        grok.project = Some(home.clone());
        assert_eq!(short_name(&grok, &home), "~");
        grok.project = None;
        assert_eq!(
            short_name(&grok, &home),
            "pixeiro",
            "old reports decode the name"
        );
        grok.project = Some(PathBuf::from("/"));
        grok.path = PathBuf::from("/home/me/.grok/sessions/%2F");
        assert_eq!(short_name(&grok, &home), "%2F");
        grok.path = PathBuf::from("/home/me/.grok/sessions/%2Fhome%2Fme");
        grok.project = None;
        assert_eq!(short_name(&grok, &home), "~");
        let claude = Husk::bare(
            HuskKind::Afterimage,
            "/home/me/.claude/projects/-home-me-dev-new-app",
            1,
        );
        assert_eq!(short_name(&claude, &home), "dev-new-app");
        assert_eq!(
            short_name(&claude, Path::new("")),
            "-home-me-dev-new-app",
            "no home, no stripping"
        );
        let day = Husk::bare(
            HuskKind::Afterimage,
            "/home/me/.codex/sessions/2026/09/06",
            1,
        );
        assert_eq!(short_name(&day, &home), "2026-09-06");
        let month = Husk::bare(HuskKind::Afterimage, "/home/me/.codex/sessions/2026/09", 1);
        assert_eq!(short_name(&month, &home), "2026-09");
        let plain = Husk::bare(HuskKind::Afterimage, "/home/me/.codex/sessions/ab", 1);
        assert_eq!(short_name(&plain, &home), "ab");
        let mut s = AppState::with_report(report());
        let mut with_project = husk(HuskKind::Afterimage, "/h/.grok/sessions/%2Fh%2Fp", 5, 1);
        with_project.project = Some(PathBuf::from("/h/dev/p"));
        s.report.as_mut().unwrap().husks.push(with_project.clone());
        s.selected = Some(with_project.id.clone());
        s.drawer_open = true;
        with_locale(Locale::En, || {
            let d = s.drawer().unwrap();
            assert_eq!(d.title, "p");
            assert!(d.facts.contains(&("project".into(), "~/dev/p".into())));
        });
    }
}
