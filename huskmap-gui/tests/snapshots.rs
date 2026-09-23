//! Headless frame dumps of every screen, from a synthetic report.
//!
//! PNGs land in `$CARGO_TARGET_TMPDIR/snapshots/`. Set `HUSKMAP_SNAPSHOT_REPORT` to a report
//! JSON to render real data instead (never commit those frames).
#![cfg(feature = "desktop")]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use freya::prelude::{App, IntoElement};
use freya_testing::prelude::*;
use huskmap_core::{
    AgentKind, Dirs, Ecosystem, GitFacts, Grove, Holder, Husk, HuskKind, Locale, Risk, ScanReport,
    Ward, copy,
};
use huskmap_gui::app::HuskmapApp;
use huskmap_gui::fonts::FONTS;
use huskmap_gui::theme;
use huskmap_gui::view_model::{AppState, Phase, ViewMode};

const DAY: u64 = 86_400_000;
const NOW: u64 = 1_900_000_000_000;

fn husk(kind: HuskKind, path: &str, mb: u64, age_days: f64) -> Husk {
    let mut h = Husk::bare(kind, format!("/home/me/{path}"), mb * 1024 * 1024);
    h.mtime_ms = Some(NOW - (age_days * DAY as f64) as u64);
    if kind == HuskKind::Worktree || kind == HuskKind::Afterimage {
        h.risk = Risk::Caution;
    }
    h
}

fn wt(path: &str, branch: &str, mb: u64, age: f64) -> Husk {
    let mut h = husk(HuskKind::Worktree, path, mb, age);
    h.git = Some(GitFacts {
        is_worktree: true,
        branch: Some(branch.into()),
        head_summary: Some("wire the husk map".into()),
        common_dir: PathBuf::from("/home/me/dev/app/.git"),
        ..Default::default()
    });
    h.grove = Some(PathBuf::from("/home/me/dev/app/.git"));
    h
}

pub fn synthetic() -> ScanReport {
    let mut r = ScanReport::empty(PathBuf::from("/home/me"), NOW);
    let mut husks = vec![];
    let mut busy = wt(
        "dev/app-worktrees/feat-husk-map",
        "feat/husk-map",
        894,
        0.01,
    );
    busy.agent = Some(AgentKind::Claude);
    busy.ward(Ward::Occupied {
        holders: vec![Holder {
            pid: 4242,
            name: "claude".into(),
            cwd: busy.path.clone(),
            agent: Some(AgentKind::Claude),
        }],
    });
    husks.push(busy);
    let mut dirty = wt("dev/app-worktrees/fix-paridade", "fix/paridade", 300, 7.0);
    dirty.ward(Ward::Dirty { files: 5 });
    husks.push(dirty);
    let mut stranded = wt(".codex/worktrees/a1b2/api", "fix/nf-remessa", 42, 3.0);
    stranded.agent = Some(AgentKind::Codex);
    stranded.ward(Ward::Stranded { commits: 3 });
    husks.push(stranded);
    for (i, (b, age)) in [
        ("chore/bun-msw", 13.0),
        ("feat/render", 20.0),
        ("fix/vercel", 60.0),
        ("spike/old", 200.0),
    ]
    .iter()
    .enumerate()
    {
        husks.push(wt(
            &format!("dev/app-worktrees/w{i}"),
            b,
            300 + i as u64 * 60,
            *age,
        ));
    }
    for (i, age) in [0.5, 2.0, 9.0, 30.0, 45.0, 120.0, 300.0].iter().enumerate() {
        let mut nm = husk(
            HuskKind::Ballast,
            &format!("dev/p{i}/node_modules"),
            120 + i as u64 * 90,
            *age,
        );
        nm.ecosystem = Some(Ecosystem::Node);
        nm.ballast = Some(huskmap_core::BallastKind::NodeModules);
        husks.push(nm);
    }
    let mut target = husk(HuskKind::Ballast, "dev/huskmap/target", 6400, 0.0);
    target.ecosystem = Some(Ecosystem::Rust);
    husks.push(target);
    let mut venv = husk(HuskKind::Ballast, "dev/ml/.venv", 900, 40.0);
    venv.ecosystem = Some(Ecosystem::Python);
    husks.push(venv);
    for (p, mb, age, eco) in [
        (".cache/uv", 4300, 0.02, Ecosystem::Python),
        (".bun/install/cache", 3000, 0.8, Ecosystem::Node),
        (".npm/_cacache", 2500, 0.01, Ecosystem::Node),
        (".cargo/registry", 2000, 0.01, Ecosystem::Rust),
        (".cache/ms-playwright", 1900, 8.0, Ecosystem::Browsers),
    ] {
        let mut t = husk(HuskKind::Toolchain, p, mb, age);
        t.ecosystem = Some(eco);
        husks.push(t);
    }
    for i in 0..18 {
        let mut s = husk(
            HuskKind::Afterimage,
            &format!(".claude/projects/-home-me-dev-p{i}"),
            2 + i * 7,
            (i as f64 * 1.7).powf(1.6),
        );
        s.agent = Some(if i % 3 == 0 {
            AgentKind::Codex
        } else {
            AgentKind::Claude
        });
        if i % 5 == 0 {
            s.ward(Ward::Orphaned { origin: None });
        }
        husks.push(s);
    }
    for (p, mb, age) in [
        (".cache/codex-runtimes", 1600, 14.0),
        (".codex/cache", 26, 0.1),
        (".claude/paste-cache", 1, 0.2),
    ] {
        let mut c = husk(HuskKind::Cache, p, mb, age);
        c.agent = Some(AgentKind::Codex);
        husks.push(c);
    }
    husks.push(husk(HuskKind::Debris, ".claude/history.jsonl", 3, 0.1));
    husks.push(husk(HuskKind::Debris, ".codex/log/codex-tui.log", 12, 2.0));
    let seen: u64 = husks.iter().map(|h| h.size_bytes).sum();
    let rec: u64 = husks
        .iter()
        .filter(|h| h.reclaimable)
        .map(|h| h.size_bytes)
        .sum();
    r.totals.husk_count = husks.len();
    r.totals.bytes_seen = seen;
    r.totals.bytes_reclaimable = rec;
    r.husks = husks;
    r.groves = vec![Grove {
        id: PathBuf::from("/home/me/dev/app/.git"),
        name: "app".into(),
        primary: Some(PathBuf::from("/home/me/dev/app")),
        worktrees: vec![],
        husks: vec![],
        bytes: 0,
    }];
    r
}

fn report() -> ScanReport {
    std::env::var_os("HUSKMAP_SNAPSHOT_REPORT")
        .and_then(|p| ScanReport::load(Path::new(&p)).ok())
        .unwrap_or_else(synthetic)
}

fn out_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("snapshots");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// `HUSKMAP_SNAPSHOT_SCALE=2` renders retina frames (for the website).
fn scale() -> f64 {
    std::env::var("HUSKMAP_SNAPSHOT_SCALE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1.0)
}

fn render(name: &str, state: AppState, locale: Locale) -> PathBuf {
    copy::set_locale(locale);
    let app = HuskmapApp {
        initial: state,
        dirs: Dirs::for_home("/nonexistent-huskmap-home"),
        live: false,
    };
    let (mut t, _) = TestingRunner::new(
        move || app.render().into_element(),
        freya::prelude::Size2D::new(
            theme::WINDOW_WIDTH * scale() as f32,
            theme::WINDOW_HEIGHT * scale() as f32,
        ),
        |_| {},
        scale(),
    );
    let fonts: HashMap<&str, &[u8]> = FONTS
        .iter()
        .filter(|(_, b)| b.len() != 71592) // one face per alias; keep upright serif
        .map(|(n, b)| (*n, *b))
        .collect();
    t.set_fonts(fonts);
    t.set_default_fonts(&[theme::MONO_FACE.into()]);
    t.sync_and_update();
    t.poll(Duration::from_millis(16), Duration::from_millis(2600));
    t.sync_and_update();
    let path = out_dir().join(format!("{name}.png"));
    t.render_to_file(&path);
    path
}

fn png_ok(path: &Path) {
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(&bytes[1..4], b"PNG");
    assert!(bytes.len() > 40_000, "{} looks blank", path.display());
}

#[test]
fn frames() {
    let base = AppState::with_report(report());
    let map = render("01-map", base.clone(), Locale::En);
    png_ok(&map);

    let mut drawer = base.clone();
    let busy = drawer.alarms().first().map(|a| a.id.clone());
    drawer.selected = busy;
    drawer.drawer_open = true;
    png_ok(&render("02-drawer", drawer, Locale::PtBr));

    let mut guarded = base.clone();
    let dirty = guarded
        .husks()
        .iter()
        .find(|h| h.has_ward("dirty"))
        .map(|h| h.id.clone());
    guarded.selected = dirty.clone();
    guarded.drawer_open = true;
    png_ok(&render("02b-drawer-guarded", guarded.clone(), Locale::En));
    if let Some(id) = dirty {
        guarded.toggle_force(&id);
        guarded.drawer_open = false;
        guarded.open_confirm();
        assert!(
            guarded
                .confirm
                .as_ref()
                .is_some_and(|c| c.forced_note.is_some())
        );
        png_ok(&render("04b-confirm-forced", guarded, Locale::En));
    }

    let mut ledger = base.clone();
    ledger.mode = ViewMode::Ledger;
    png_ok(&render("03-ledger", ledger, Locale::En));

    let mut filtered = base.clone();
    filtered.mode = ViewMode::Ledger;
    copy::set_locale(Locale::PtBr);
    filtered.press_chip(huskmap_gui::view_model::Chip::Kind(
        huskmap_core::HuskKind::Ballast,
    ));
    filtered.press_chip(huskmap_gui::view_model::Chip::Age(Some(">7d")));
    filtered.sort = huskmap_gui::view_model::Sort {
        key: huskmap_gui::view_model::SortKey::Age,
        desc: true,
    };
    png_ok(&render("03b-ledger-filtered", filtered, Locale::PtBr));

    let mut guide = base.clone();
    guide.open_guide(2);
    png_ok(&render("10-guide-map", guide.clone(), Locale::PtBr));
    guide.open_guide(5);
    png_ok(&render("10b-guide-filters", guide.clone(), Locale::En));
    guide.open_guide(1);
    png_ok(&render("10c-guide-steps", guide, Locale::PtBr));

    let mut confirm = base.clone();
    let free: Vec<_> = confirm
        .visible()
        .iter()
        .filter(|h| huskmap_core::admissible(h, false))
        .take(4)
        .map(|h| h.id.clone())
        .collect();
    confirm.marked = free.into_iter().collect();
    confirm.open_confirm();
    assert!(confirm.confirm.is_some());
    png_ok(&render("04-confirm", confirm, Locale::PtBr));

    let mut upd = base.clone();
    upd.offer_update(
        &huskmap_core::update::Release {
            version: "0.2.0".into(),
            page: String::new(),
            notes: "- Force-mark guarded worktrees from the drawer\n- Marks survive restarts and updates\n- Grok sessions know their project".into(),
            assets: vec![],
        },
        None,
    );
    upd.update_open = true;
    png_ok(&render("08-update", upd, Locale::PtBr));

    png_ok(&render("05-idle", AppState::default(), Locale::En));

    let mut scanning = AppState::default();
    scanning.begin_scan();
    scanning.phase = Phase::Scanning {
        walking: Some("~/Documentos".into()),
        weighing: None,
    };
    scanning.live = report().husks.into_iter().take(20).collect();
    png_ok(&render("06-scanning", scanning, Locale::En));

    let mut failed = AppState::default();
    failed.fail("permission denied at ~/dev".into());
    png_ok(&render("07-failed", failed, Locale::En));
    copy::set_locale(Locale::En);
    eprintln!("snapshots: {}", out_dir().display());
}

/// The list keeps every row drawn while it scrolls (rows used to share one diff key).
#[test]
fn ledger_scrolls() {
    copy::set_locale(Locale::PtBr);
    let mut state = AppState::with_report(report());
    state.mode = ViewMode::Ledger;
    let app = HuskmapApp {
        initial: state,
        dirs: Dirs::for_home("/nonexistent-huskmap-home"),
        live: false,
    };
    let (mut t, _) = TestingRunner::new(
        move || app.render().into_element(),
        freya::prelude::Size2D::new(theme::WINDOW_WIDTH, theme::WINDOW_HEIGHT),
        |_| {},
        1.0,
    );
    let fonts: HashMap<&str, &[u8]> = FONTS
        .iter()
        .filter(|(_, b)| b.len() != 71592)
        .map(|(n, b)| (*n, *b))
        .collect();
    t.set_fonts(fonts);
    t.set_default_fonts(&[theme::MONO_FACE.into()]);
    t.sync_and_update();
    t.poll(Duration::from_millis(16), Duration::from_millis(400));
    for i in 0..3 {
        for _ in 0..4 {
            t.scroll((900.0, 600.0), (0.0, -61.0));
        }
        t.poll(Duration::from_millis(16), Duration::from_millis(100));
        let path = out_dir().join(format!("09-ledger-scrolled-{i}.png"));
        t.render_to_file(&path);
        png_ok(&path);
    }
    copy::set_locale(Locale::En);
}

/// Frames for the website, in both languages, from the synthetic report only.
/// `HUSKMAP_SNAPSHOT_SCALE=2 cargo test -p huskmap-gui --features desktop --test snapshots site_frames -- --ignored`
#[test]
#[ignore]
fn site_frames() {
    for (locale, tag) in [(Locale::En, "en"), (Locale::PtBr, "pt")] {
        copy::set_locale(locale);
        let base = AppState::with_report(synthetic());
        png_ok(&render(&format!("site-map-{tag}"), base.clone(), locale));

        let mut drawer = base.clone();
        drawer.selected = drawer.alarms().first().map(|a| a.id.clone());
        drawer.drawer_open = true;
        png_ok(&render(&format!("site-drawer-{tag}"), drawer, locale));

        let mut list = base.clone();
        list.mode = ViewMode::Ledger;
        copy::set_locale(locale);
        list.press_chip(huskmap_gui::view_model::Chip::Kind(
            huskmap_core::HuskKind::Ballast,
        ));
        list.press_chip(huskmap_gui::view_model::Chip::Kind(
            huskmap_core::HuskKind::Worktree,
        ));
        png_ok(&render(&format!("site-list-{tag}"), list, locale));

        let mut confirm = base.clone();
        let free: Vec<_> = confirm
            .visible()
            .iter()
            .filter(|h| huskmap_core::admissible(h, false))
            .take(5)
            .map(|h| h.id.clone())
            .collect();
        confirm.marked = free.into_iter().collect();
        confirm.open_confirm();
        png_ok(&render(&format!("site-confirm-{tag}"), confirm, locale));

        let mut guide = base.clone();
        guide.open_guide(2);
        png_ok(&render(&format!("site-guide-{tag}"), guide, locale));
    }
    copy::set_locale(Locale::En);
}
