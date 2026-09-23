//! Shell: window, state, background scan/apply, keyboard. Rules live in `view_model` and core.

use freya::prelude::*;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use huskmap_core::session::{ApplyLock, Session};
use huskmap_core::settings::{Settings, update_check_disabled_by_env};
use huskmap_core::update::{self, Https, InstallKind, Release, SystemHost};
use huskmap_core::{
    ApplyOptions, ApplyResult, Clock, Dirs, Git2Probe, HuskId, Plan, ProcProcessProbe, ScanEvent,
    ScanOptions, ScanReport, SystemClock, TrashReclaimer, apply, copy, scan_with_progress,
};

use crate::components::chrome::{StatusBar, TopBar};
use crate::components::confirm::Confirm;
use crate::components::drawer::Drawer;
use crate::components::empty;
use crate::components::guide::GuideModal;
use crate::components::ledger::Ledger;
use crate::components::map::{HuskMap, sector_labels, tone_legend};
use crate::components::rail::Rail;
use crate::components::update::UpdateModal;
use crate::fonts::FONTS;
use crate::icons::Brand;
use crate::theme;
use crate::view_model::{AppState, Intent, Phase, ViewMode};

/// Wayland app id and X11 class. Must match `packaging/lucas.cavalheri.huskmap.desktop`.
pub const APP_ID: &str = "lucas.cavalheri.huskmap";

/// The desktop app. `initial` lets tests and snapshots start from any state.
#[derive(Clone)]
pub struct HuskmapApp {
    pub initial: AppState,
    pub dirs: Dirs,
    /// Run real scans and applies. Snapshots turn this off.
    pub live: bool,
}

enum Msg {
    Event(ScanEvent),
    Done(Box<ScanReport>),
    Failed(String),
    Applied(ApplyResult),
    UpdateFound(Box<Release>),
    UpdateReady(PathBuf),
    UpdateFailed(String),
}

fn start_scan(mut state: State<AppState>, dirs: Dirs) {
    state.write().begin_scan();
    let (tx, rx) = async_channel::unbounded::<Msg>();
    let home = dirs.home.clone();
    std::thread::spawn(move || {
        let opts = ScanOptions::for_dirs(dirs.clone());
        let result = scan_with_progress(&opts, |e| {
            let _ = tx.send_blocking(Msg::Event(e.clone()));
        });
        let msg = match result {
            Ok(report) => {
                if let Err(err) = report.save(&dirs.last_report()) {
                    tracing::warn!(error = %err, "could not cache the report");
                }
                Msg::Done(Box::new(report))
            }
            Err(err) => Msg::Failed(err.to_string()),
        };
        let _ = tx.send_blocking(msg);
    });
    spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let mut s = state.write();
            match msg {
                Msg::Event(e) => s.on_event(&e, &home),
                Msg::Done(report) => s.land(*report),
                Msg::Failed(err) => s.fail(err),
                _ => {}
            }
        }
    });
}

/// Hand a directory to the desktop's file manager.
pub fn open_folder(path: &std::path::Path) -> std::io::Result<()> {
    std::process::Command::new("xdg-open")
        .arg(path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
}

/// Plain plan first, then the force-marked one with `force`. Results merge.
pub fn apply_both(
    plan: &Plan,
    forced: &Plan,
    git: &dyn huskmap_core::GitProbe,
    reclaimer: &dyn huskmap_core::Reclaimer,
    processes: &dyn huskmap_core::ProcessProbe,
) -> ApplyResult {
    let mut total = ApplyResult::default();
    for (p, force) in [(plan, false), (forced, true)] {
        if p.actions.is_empty() {
            continue;
        }
        match apply(
            p,
            &ApplyOptions {
                git,
                reclaimer,
                processes,
                force,
            },
        ) {
            Ok(r) => {
                total.applied.extend(r.applied);
                total.skipped.extend(r.skipped);
                total.errors.extend(r.errors);
            }
            Err(err) => total.errors.push(err.to_string()),
        }
    }
    total
}

fn start_apply(mut state: State<AppState>, dirs: Dirs) {
    let Some((plan, forced)): Option<(Plan, Plan)> = state
        .peek()
        .confirm
        .as_ref()
        .map(|c| (c.plan.clone(), c.forced_plan.clone()))
    else {
        return;
    };
    {
        let mut s = state.write();
        s.applying = true;
        s.confirm = None;
    }
    let (tx, rx) = async_channel::unbounded::<Msg>();
    let state_dir = dirs.huskmap_state();
    std::thread::spawn(move || {
        let husks = plan.actions.len() + forced.actions.len();
        let bytes = plan.reclaimable_bytes() + forced.reclaimable_bytes();
        let result = match ApplyLock::acquire(
            &state_dir,
            Path::new("/proc"),
            husks,
            bytes,
            SystemClock.now_ms(),
        ) {
            Ok(_lock) => apply_both(
                &plan,
                &forced,
                &Git2Probe,
                &TrashReclaimer,
                &ProcProcessProbe::default(),
            ),
            Err(err) => ApplyResult {
                errors: vec![err.to_string()],
                ..Default::default()
            },
        };
        let _ = tx.send_blocking(Msg::Applied(result));
    });
    spawn(async move {
        if let Ok(Msg::Applied(result)) = rx.recv().await {
            state.write().applied(&result);
            // Facts changed on disk; sound again so the map tells the truth.
            start_scan(state, dirs);
        }
    });
}

fn start_update_check(mut state: State<AppState>, dirs: Dirs) {
    let (tx, rx) = async_channel::unbounded::<Msg>();
    let config = dirs.config.clone();
    std::thread::spawn(move || {
        let found = update::check(&Https, update::CURRENT_VERSION);
        let _ = Settings::update(&config, |s| {
            s.last_update_check_ms = Some(SystemClock.now_ms())
        });
        match found {
            Ok(Some(release)) => {
                let _ = tx.send_blocking(Msg::UpdateFound(Box::new(release)));
            }
            Ok(None) => {}
            Err(err) => tracing::info!(error = %err, "update check failed"),
        }
    });
    let skipped = Settings::load(&dirs.config).skipped_version;
    spawn(async move {
        if let Ok(Msg::UpdateFound(release)) = rx.recv().await {
            state.write().offer_update(&release, skipped.as_deref());
        }
    });
}

/// Install the offered release the way this huskmap was installed, then come back.
fn start_update(mut state: State<AppState>, dirs: Dirs) {
    let Some(version) = state.peek().update.as_ref().map(|u| u.version.clone()) else {
        return;
    };
    state.write().updating = true;
    let (tx, rx) = async_channel::unbounded::<Msg>();
    std::thread::spawn(move || {
        let msg = (|| -> Result<PathBuf, huskmap_core::Error> {
            let release = update::check(&Https, update::CURRENT_VERSION)?
                .filter(|r| r.version == version)
                .ok_or_else(|| huskmap_core::Error::update("the offered release is gone"))?;
            let kind = update::detect_install_kind(&SystemHost);
            if let InstallKind::Unmanaged(_) = kind {
                return Err(huskmap_core::Error::update(copy::get().update_unmanaged));
            }
            let work = std::env::temp_dir().join(format!("huskmap-update-{}", std::process::id()));
            let exe = update::install(&release, &kind, &Https, &SystemHost, &work);
            let _ = std::fs::remove_dir_all(&work);
            exe
        })();
        let _ = tx.send_blocking(match msg {
            Ok(exe) => Msg::UpdateReady(exe),
            Err(err) => Msg::UpdateFailed(err.to_string()),
        });
    });
    let state_dir = dirs.huskmap_state();
    spawn(async move {
        match rx.recv().await {
            Ok(Msg::UpdateReady(exe)) => {
                // Marks are already in session.json; the new window picks them up.
                let snap = state
                    .peek()
                    .session(std::process::id(), SystemClock.now_ms());
                let _ = snap.save(&state_dir);
                match std::process::Command::new(&exe).arg("gui").spawn() {
                    Ok(_) => std::process::exit(0),
                    Err(err) => {
                        let mut s = state.write();
                        s.updating = false;
                        s.update_open = false;
                        s.status =
                            Some(copy::get().update_failed.replace("{err}", &err.to_string()));
                    }
                }
            }
            Ok(Msg::UpdateFailed(err)) => {
                let mut s = state.write();
                s.updating = false;
                s.status = Some(copy::get().update_failed.replace("{err}", &err));
            }
            _ => {}
        }
    });
}

type SessionKey = (
    Vec<HuskId>,
    Vec<HuskId>,
    huskmap_core::session::SessionPhase,
);

fn key_name(e: &Event<KeyboardEventData>) -> Option<String> {
    Some(match &e.key {
        Key::Named(NamedKey::Enter) => "enter".into(),
        Key::Named(NamedKey::Escape) => "escape".into(),
        Key::Named(NamedKey::Backspace) => "backspace".into(),
        Key::Named(NamedKey::Tab) => "tab".into(),
        Key::Named(NamedKey::ArrowDown) => "down".into(),
        Key::Named(NamedKey::ArrowUp) => "up".into(),
        Key::Named(NamedKey::ArrowLeft) => "left".into(),
        Key::Named(NamedKey::ArrowRight) => "right".into(),
        Key::Named(NamedKey::Home) => "home".into(),
        Key::Character(c) => c.to_string(),
        _ => return None,
    })
}

impl App for HuskmapApp {
    fn render(&self) -> impl IntoElement {
        let initial = self.initial.clone();
        let mut state = use_state(move || initial);
        let dirs = self.dirs.clone();
        let live = self.live;

        // Once per window: ask GitHub if the daily check is due.
        {
            let dirs = dirs.clone();
            use_hook(move || {
                let due = Settings::load(&dirs.config)
                    .update_check_due(SystemClock.now_ms(), update_check_disabled_by_env());
                if live && due {
                    start_update_check(state, dirs);
                }
            });
        }

        // Keep session.json in step with marks and phase, so installers can warn and marks survive.
        {
            let state_dir = dirs.huskmap_state();
            let last: Rc<RefCell<Option<SessionKey>>> = use_hook(|| Rc::new(RefCell::new(None)));
            use_side_effect(move || {
                if !live {
                    return;
                }
                let snap = state
                    .read()
                    .session(std::process::id(), SystemClock.now_ms());
                let key = (snap.marked.clone(), snap.forced.clone(), snap.phase);
                if last.borrow().as_ref() == Some(&key) {
                    return;
                }
                if let Err(err) = snap.save(&state_dir) {
                    tracing::warn!(error = %err, "could not write session.json");
                }
                *last.borrow_mut() = Some(key);
            });
        }

        // The guide opens by itself until it is closed once.
        {
            let config = dirs.config.clone();
            let was_open: Rc<RefCell<bool>> = use_hook(|| Rc::new(RefCell::new(false)));
            use_side_effect(move || {
                let open = state.read().guide_open;
                let closed_now = *was_open.borrow() && !open;
                *was_open.borrow_mut() = open;
                if live
                    && closed_now
                    && let Err(err) = Settings::update(&config, |s| s.guide_seen = true)
                {
                    tracing::warn!(error = %err, "could not remember the guide was read");
                }
            });
        }

        // Remember the theme pick.
        {
            let config = dirs.config.clone();
            let last: Rc<RefCell<Option<crate::theme::ThemeChoice>>> =
                use_hook(|| Rc::new(RefCell::new(None)));
            use_side_effect(move || {
                let theme = state.read().theme;
                let before = last.borrow_mut().replace(theme);
                if live
                    && before.is_some_and(|b| b != theme)
                    && let Err(err) =
                        Settings::update(&config, |s| s.theme = Some(theme.as_str().into()))
                {
                    tracing::warn!(error = %err, "could not remember the theme");
                }
            });
        }

        // Buttons ask; the shell does.
        {
            let dirs = dirs.clone();
            use_side_effect(move || {
                let request = state.read().request;
                if request.is_none() {
                    return;
                }
                let request = state.write().request.take();
                match request {
                    Some(Intent::Scan) if live && !state.peek().is_scanning() => {
                        start_scan(state, dirs.clone())
                    }
                    Some(Intent::Apply) if live => start_apply(state, dirs.clone()),
                    Some(Intent::Update) if live => start_update(state, dirs.clone()),
                    _ => {}
                }
            });
        }

        let on_key = {
            let dirs = dirs.clone();
            move |e: Event<KeyboardEventData>| {
                let Some(name) = key_name(&e) else { return };
                let intent = state.write().key(&name);
                match intent {
                    Intent::Scan if live => start_scan(state, dirs.clone()),
                    Intent::Apply if live => start_apply(state, dirs.clone()),
                    Intent::Update if live => start_update(state, dirs.clone()),
                    _ => {}
                }
            }
        };

        // Resolve the palette before anything reads a color. System follows the desktop.
        let system_light = *Platform::get().preferred_theme.read() == PreferredTheme::Light;
        let light = state.read().theme.is_light(system_light);
        theme::set_light(light);

        let s = state.read();
        let mode = s.mode;
        let phase = s.phase.clone();
        let nodes = s.nodes();
        let sectors = sector_labels(&s);
        let generation = s.generation;
        let scanning = s.is_scanning();
        let has_report = s.report.is_some();
        let empty_report = has_report && s.husks().is_empty();
        let drawer = s.drawer().map(|view| {
            let brand = s.husk(&view.id).and_then(Brand::for_husk);
            (view, brand)
        });
        let confirm = s.confirm.clone();
        let update_modal = s
            .update
            .clone()
            .filter(|_| s.update_open)
            .map(|offer| (offer, s.updating, s.applying));
        let guide = s.guide_open.then_some(s.guide_section);
        let (rows, bar, sort) = if mode == ViewMode::Ledger {
            (s.ledger(), Some(s.filter_bar()), s.sort)
        } else {
            (vec![], None, s.sort)
        };
        drop(s);

        let mut stage = rect()
            .content(Content::flex())
            .width(Size::flex(1.))
            .height(Size::fill())
            .child(TopBar { state });

        // `fill` (not `flex`): absolute overlays measure against this box.
        let mut content = rect().width(Size::fill()).height(Size::fill());
        match mode {
            ViewMode::Map => {
                content = content.child(HuskMap {
                    state,
                    nodes,
                    sectors,
                    generation,
                    scanning,
                });
                content = match (&phase, has_report) {
                    (Phase::Failed(msg), false) => content.child(empty::failed(state, msg)),
                    (Phase::Idle, false) => content.child(empty::idle(state)),
                    _ if empty_report && !scanning => content.child(empty::clean()),
                    _ => content,
                };
                content = content.child(
                    rect()
                        .position(Position::new_absolute().left(theme::GUTTER).bottom(20.))
                        .child(tone_legend()),
                );
            }
            ViewMode::Ledger => {
                if let Some(bar) = bar {
                    content = content.child(Ledger {
                        state,
                        rows,
                        bar,
                        sort,
                    });
                }
            }
        }
        if let Some((view, brand)) = drawer {
            content = content.child(
                rect()
                    .layer(Layer::OverlayLevel(2))
                    .position(Position::new_absolute().right(0.).top(0.))
                    .height(Size::percent(100.))
                    .child(Drawer { state, view, brand }),
            );
        }
        stage = stage.child(content);

        let mut root = rect()
            // A new key on a theme switch redraws every memoized component in the new colors.
            .key(if light { "light" } else { "dark" })
            .content(Content::flex())
            .expanded()
            .background(theme::pitch())
            .on_global_key_down(on_key)
            .child(
                rect()
                    .content(Content::flex())
                    .horizontal()
                    .width(Size::fill())
                    .height(Size::flex(1.))
                    .child(Rail { state })
                    .child(stage),
            )
            .child(StatusBar {
                state,
                config_dir: dirs.config.clone(),
            });
        if let Some(view) = confirm {
            root = root.child(Confirm { state, view });
        }
        if let Some(section) = guide {
            root = root.child(GuideModal { state, section });
        }
        if let Some((offer, updating, applying)) = update_modal {
            root = root.child(UpdateModal {
                state,
                offer,
                updating,
                applying,
                config_dir: dirs.config.clone(),
            });
        }
        root
    }
}

/// Start state: the cached report if there is one, then sound again for fresh facts.
pub fn initial_state(dirs: &Dirs) -> AppState {
    let mut state = match ScanReport::load(&dirs.last_report()) {
        Ok(report) => AppState::with_report(report),
        Err(_) => AppState::default(),
    };
    // Marks from the last window (or the one an update just closed) come back.
    if let Some(previous) = Session::load(&dirs.huskmap_state()) {
        state.restore(&previous);
    }
    state.request = Some(Intent::Scan);
    let settings = Settings::load(&dirs.config);
    state.guide_open = !settings.guide_seen;
    state.theme = settings
        .theme
        .as_deref()
        .map(crate::theme::ThemeChoice::parse)
        .unwrap_or_default();
    state
}

pub fn launch_app() {
    copy::init();
    let home = std::env::var_os("HUSKMAP_HOME")
        .or_else(|| std::env::var_os("HOME"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("/"));
    let dirs = if std::env::var_os("HUSKMAP_HOME").is_some() {
        Dirs::for_home(home)
    } else {
        Dirs::current(home)
    };
    let app = HuskmapApp {
        initial: initial_state(&dirs),
        dirs,
        live: true,
    };
    let mut config = LaunchConfig::new();
    for (name, bytes) in FONTS {
        config = config.with_font(*name, *bytes);
    }
    launch(
        config.with_default_font(theme::MONO_FACE).with_window(
            WindowConfig::new_app(app)
                .with_title(copy::get().map_title)
                .with_size(
                    f64::from(theme::WINDOW_WIDTH),
                    f64::from(theme::WINDOW_HEIGHT),
                )
                .with_min_size(
                    f64::from(theme::WINDOW_MIN_WIDTH),
                    f64::from(theme::WINDOW_MIN_HEIGHT),
                )
                .with_background(Color::from_rgb(
                    theme::pitch().0,
                    theme::pitch().1,
                    theme::pitch().2,
                )),
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use huskmap_core::{
        ActionKind, DeadProcessProbe, Fingerprint, FsReclaimer, HuskId, HuskKind, MapGitProbe,
        PLAN_VERSION, PlanAction, PlanPreset, dir_size,
    };

    fn plan_for(root: &std::path::Path, dir: &std::path::Path) -> Plan {
        let size = dir_size(dir).unwrap();
        Plan {
            version: PLAN_VERSION,
            created_at_ms: 1,
            preset: PlanPreset::Marked,
            roots: vec![root.to_path_buf()],
            actions: vec![PlanAction {
                husk_id: HuskId::new(HuskKind::Cache, dir),
                path: dir.to_path_buf(),
                kind: ActionKind::Trash,
                fingerprint: Fingerprint {
                    size_bytes: size.bytes,
                    mtime_ms: size.mtime_ms,
                    dirty: false,
                    unpushed: false,
                    is_primary: false,
                    contains_secrets: false,
                },
            }],
        }
    }

    #[test]
    fn apply_both_runs_plain_then_forced_and_merges() {
        let tmp = tempfile::tempdir().unwrap();
        let (a, b) = (tmp.path().join("a"), tmp.path().join("b"));
        for d in [&a, &b] {
            std::fs::create_dir_all(d).unwrap();
            std::fs::write(d.join("f"), "x").unwrap();
        }
        let rec = FsReclaimer::default();
        let git = MapGitProbe::default();
        let r = apply_both(
            &plan_for(tmp.path(), &a),
            &plan_for(tmp.path(), &b),
            &git,
            &rec,
            &DeadProcessProbe,
        );
        assert_eq!(r.applied.len(), 2);
        assert!(!a.exists() && !b.exists());
        let mut empty = plan_for(tmp.path(), tmp.path());
        empty.actions.clear();
        let mut bad = empty.clone();
        bad.version = 0;
        bad.actions = plan_for(tmp.path(), tmp.path()).actions;
        let r = apply_both(&empty, &bad, &git, &rec, &DeadProcessProbe);
        assert_eq!(r.errors.len(), 1, "a refused plan surfaces as an error");
    }

    #[test]
    fn initial_state_asks_for_a_scan() {
        let tmp = tempfile::tempdir().unwrap();
        let dirs = Dirs::for_home(tmp.path());
        assert_eq!(initial_state(&dirs).request, Some(Intent::Scan));
        assert!(
            initial_state(&dirs).guide_open,
            "first launch explains itself"
        );
        Settings::update(&dirs.config, |s| s.guide_seen = true).unwrap();
        assert!(!initial_state(&dirs).guide_open);
        Settings::update(&dirs.config, |s| s.theme = Some("light".into())).unwrap();
        assert_eq!(initial_state(&dirs).theme, crate::theme::ThemeChoice::Light);
        huskmap_core::ScanReport::empty(tmp.path().to_path_buf(), 1)
            .save(&dirs.last_report())
            .unwrap();
        let s = initial_state(&dirs);
        assert!(s.report.is_some());
        let id = HuskId::new(huskmap_core::HuskKind::Cache, "/c");
        Session {
            marked: vec![id.clone()],
            ..Default::default()
        }
        .save(&dirs.huskmap_state())
        .unwrap();
        // judged against the cached report, a mark for a husk that is gone is dropped
        assert!(initial_state(&dirs).marked.is_empty());
        // with no report yet, marks wait for the next scan to judge them
        std::fs::remove_file(dirs.last_report()).unwrap();
        let restored = initial_state(&dirs);
        assert!(restored.marked.contains(&id), "marks survive a restart");
    }
}
