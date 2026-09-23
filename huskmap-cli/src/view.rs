use huskmap_core::{
    Husk, HuskId, HuskKind, Risk, ScanReport, Ward, copy, format_age, format_bytes, tilde,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HuskRow {
    pub id: HuskId,
    pub kind: HuskKind,
    pub kind_label: String,
    pub risk: Risk,
    pub size_bytes: u64,
    pub size: String,
    pub age: String,
    pub path: String,
    pub agent: Option<String>,
    pub branch: Option<String>,
    pub wards: Vec<String>,
    pub reclaimable: bool,
    pub alarm: bool,
    pub depth: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub kind: HuskKind,
    pub label: String,
    pub bytes: u64,
    pub count: usize,
    pub rows: Vec<HuskRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanView {
    pub title: String,
    pub subtitle: String,
    pub husk_count: usize,
    pub grove_count: usize,
    pub reclaimable_label: String,
    pub seen_label: String,
    pub bytes_reclaimable: u64,
    pub alarms: Vec<HuskRow>,
    pub sections: Vec<Section>,
    pub rows: Vec<HuskRow>,
    pub empty: bool,
    pub warnings: Vec<String>,
}

fn depth(husk: &Husk, report: &ScanReport) -> usize {
    let mut d = 0;
    let mut cur = husk.parent.as_ref();
    while let Some(id) = cur {
        d += 1;
        cur = report.husk(id).and_then(|p| p.parent.as_ref());
    }
    d
}

impl HuskRow {
    pub fn from_husk(husk: &Husk, report: &ScanReport) -> Self {
        let git = husk.git.as_ref();
        Self {
            id: husk.id.clone(),
            kind: husk.kind,
            kind_label: copy::kind_label(husk.kind, false).into(),
            risk: husk.risk,
            size_bytes: husk.size_bytes,
            size: format_bytes(husk.size_bytes),
            age: husk
                .age_ms(report.scanned_at_ms)
                .map(format_age)
                .unwrap_or_else(|| "·".into()),
            path: tilde(&husk.path, &report.home),
            agent: husk.agent.map(|a| a.as_str().to_string()),
            branch: git.map(|g| {
                g.branch
                    .clone()
                    .unwrap_or_else(|| copy::get().detached.to_string())
            }),
            wards: husk
                .wards
                .iter()
                .filter(|w| !matches!(w, Ward::Warm { .. }) || husk.kind == HuskKind::Worktree)
                .map(copy::ward_text)
                .collect(),
            reclaimable: husk.reclaimable,
            alarm: husk.is_alarm(),
            depth: depth(husk, report),
        }
    }
}

impl ScanView {
    pub fn from_report(report: &ScanReport) -> Self {
        let deck = copy::get();
        let mut rows: Vec<HuskRow> = report
            .husks
            .iter()
            .map(|h| HuskRow::from_husk(h, report))
            .collect();
        rows.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes).then(a.path.cmp(&b.path)));
        let alarms = rows.iter().filter(|r| r.alarm).cloned().collect();
        let sections = report
            .bytes_by_kind()
            .into_iter()
            .filter(|(_, _, count)| *count > 0)
            .map(|(kind, bytes, count)| Section {
                kind,
                label: copy::kind_label(kind, true).into(),
                bytes,
                count,
                rows: rows.iter().filter(|r| r.kind == kind).cloned().collect(),
            })
            .collect();
        Self {
            title: deck.map_title.into(),
            subtitle: deck.map_subtitle.into(),
            husk_count: report.totals.husk_count,
            grove_count: report.totals.grove_count,
            reclaimable_label: format!(
                "{} {}",
                format_bytes(report.totals.bytes_reclaimable),
                deck.reclaimable
            ),
            seen_label: deck
                .seen_of
                .replace("{total}", &format_bytes(report.totals.bytes_seen)),
            bytes_reclaimable: report.totals.bytes_reclaimable,
            empty: report.husks.is_empty(),
            warnings: report.warnings.clone(),
            alarms,
            sections,
            rows,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use huskmap_core::{AgentKind, GitFacts, Holder, ScanReport};
    use std::path::PathBuf;

    fn report() -> ScanReport {
        let mut r = ScanReport::empty(PathBuf::from("/h"), 10 * 86_400_000);
        let mut wt = Husk::bare(HuskKind::Worktree, "/h/dev/wt", 5 * 1024 * 1024);
        wt.mtime_ms = Some(9 * 86_400_000);
        wt.git = Some(GitFacts {
            branch: None,
            ..Default::default()
        });
        wt.ward(Ward::Occupied {
            holders: vec![Holder {
                pid: 7,
                name: "claude".into(),
                cwd: PathBuf::from("/h/dev/wt"),
                agent: Some(AgentKind::Claude),
            }],
        });
        wt.agent = Some(AgentKind::Claude);
        let mut nm = Husk::bare(HuskKind::Ballast, "/h/dev/wt/node_modules", 3 * 1024 * 1024);
        nm.parent = Some(wt.id.clone());
        let mut img = Husk::bare(HuskKind::Afterimage, "/h/.claude/projects/x", 2048);
        img.ward(Ward::Warm { minutes: 3 });
        let small = Husk::bare(HuskKind::Cache, "/h/.codex/cache", 10 * 1024 * 1024);
        r.husks = vec![wt, nm, img, small];
        r.totals.husk_count = 4;
        r.totals.bytes_seen = 15 * 1024 * 1024;
        r.totals.bytes_reclaimable = 10 * 1024 * 1024;
        r
    }

    #[test]
    fn rows_sort_by_bytes_not_by_label() {
        let v = ScanView::from_report(&report());
        let sizes: Vec<u64> = v.rows.iter().map(|r| r.size_bytes).collect();
        assert!(sizes.windows(2).all(|w| w[0] >= w[1]), "{sizes:?}");
        assert_eq!(v.rows[0].path, "~/.codex/cache");
    }

    #[test]
    fn alarms_sections_and_labels() {
        huskmap_core::copy::with_locale(huskmap_core::Locale::En, || {
            let v = ScanView::from_report(&report());
            assert_eq!(v.alarms.len(), 1);
            let wt = &v.alarms[0];
            assert_eq!(wt.branch.as_deref(), Some("no branch (detached)"));
            assert_eq!(wt.wards, vec!["claude · pid 7 is using this folder now"]);
            assert_eq!(wt.age, "1d");
            assert_eq!(wt.agent.as_deref(), Some("claude"));
            let nm = v.rows.iter().find(|r| r.kind == HuskKind::Ballast).unwrap();
            assert_eq!(nm.depth, 1);
            let img = v
                .rows
                .iter()
                .find(|r| r.kind == HuskKind::Afterimage)
                .unwrap();
            assert!(img.wards.is_empty(), "warm is noise outside worktrees");
            assert_eq!(img.age, "·");
            assert_eq!(v.sections.len(), 4);
            assert_eq!(v.sections[0].label, "Worktrees");
            assert_eq!(v.reclaimable_label, "10.0 MB can be freed");
            assert_eq!(v.seen_label, "of 15.0 MB found");
            assert!(!v.empty);
        });
    }

    #[test]
    fn empty_report() {
        let v = ScanView::from_report(&ScanReport::empty(PathBuf::from("/h"), 1));
        assert!(v.empty);
        assert!(v.sections.is_empty());
        assert!(v.alarms.is_empty());
    }
}
