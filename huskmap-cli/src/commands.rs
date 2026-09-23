use std::collections::BTreeSet;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use huskmap_core::{
    ActionKind, ApplyOptions, Clock, Dirs, Git2Probe, HuskId, HuskKind, Plan, ProcProcessProbe,
    Risk, ScanEvent, ScanOptions, ScanReport, SystemClock, TrashReclaimer, apply,
    canonicalize_lossy, copy, doctor, format_bytes, plan_marked, plan_with_clock,
    scan_with_progress, session::ApplyLock, tilde,
};

use crate::args::{Command, PresetArg};
use crate::home::resolve_dirs;
use crate::paint::{Paint, Tone};
use crate::view::{HuskRow, ScanView};

const ROWS_PER_KIND: usize = 8;

pub fn exec(home: Option<PathBuf>, command: Option<Command>) -> anyhow::Result<i32> {
    let dirs = resolve_dirs(home)?;
    let home = &dirs;
    match command {
        None => default_launch(home),
        Some(Command::Scan {
            paths,
            json,
            min_size,
            category,
            all,
        }) => cmd_scan(home, paths, json, min_size, category, all),
        Some(Command::Doctor { json }) => cmd_doctor(home, json),
        Some(Command::Plan {
            preset,
            older_than,
            from,
            out,
            only,
            force,
            json,
        }) => cmd_plan(
            home,
            PlanArgs {
                preset,
                older_than,
                from,
                out,
                only,
                force,
                json,
            },
        ),
        Some(Command::Apply { plan, force, json }) => cmd_apply(home, &plan, force, json),
        Some(Command::Map { from }) => cmd_map(home, from),
        Some(Command::Gui) => launch_gui(),
        Some(Command::Update { check, yes }) => cmd_update(home, check, yes),
        Some(Command::Completions { shell }) => {
            write_completions(shell, &mut io::stdout().lock())?;
            Ok(0)
        }
        Some(Command::Status) => {
            write_status(home, Path::new("/proc"), &mut io::stdout().lock())?;
            Ok(0)
        }
        Some(Command::Man) => {
            write_man(&mut io::stdout().lock())?;
            Ok(0)
        }
    }
}

pub fn write_completions(shell: clap_complete::Shell, out: &mut impl Write) -> io::Result<()> {
    let mut cmd = crate::args::command_localized();
    clap_complete::generate(shell, &mut cmd, huskmap_core::copy::BINARY, out);
    Ok(())
}

pub fn write_man(out: &mut impl Write) -> io::Result<()> {
    clap_mangen::Man::new(crate::args::command_localized()).render(out)
}

fn default_launch(home: &Dirs) -> anyhow::Result<i32> {
    if cfg!(feature = "gui") && huskmap_core::display_available() {
        launch_gui()
    } else {
        cmd_scan(home, vec![], false, 0, None, false)
    }
}

fn launch_gui() -> anyhow::Result<i32> {
    #[cfg(feature = "gui")]
    {
        huskmap_gui::run();
        Ok(0)
    }
    #[cfg(not(feature = "gui"))]
    anyhow::bail!("{}", huskmap_core::copy::get().gui_missing)
}

fn scan_opts(
    home: &Dirs,
    paths: Vec<PathBuf>,
    min_size: u64,
    category: Option<String>,
) -> anyhow::Result<ScanOptions> {
    let category = category.map(|raw| HuskKind::from_str(&raw)).transpose()?;
    let mut opts = ScanOptions::for_dirs(home.clone());
    opts.extra_roots = paths;
    opts.min_size = min_size;
    opts.category = category;
    Ok(opts)
}

fn run_scan(opts: &ScanOptions, show_progress: bool) -> anyhow::Result<ScanReport> {
    let deck = copy::get();
    let mut err = io::stderr().lock();
    let report = scan_with_progress(opts, |event| {
        if !show_progress {
            return;
        }
        let line = match event {
            ScanEvent::Walking { root, .. } => {
                deck.walking.replace("{root}", &tilde(root, &opts.home))
            }
            ScanEvent::Weighing { pending } => deck.weighing.replace("{n}", &pending.to_string()),
            ScanEvent::Found(_) => return,
        };
        let _ = write!(err, "\r\x1b[2K  {line}");
        let _ = err.flush();
    })?;
    if show_progress {
        let _ = write!(err, "\r\x1b[2K");
    }
    Ok(report)
}

fn cmd_scan(
    home: &Dirs,
    paths: Vec<PathBuf>,
    json: bool,
    min_size: u64,
    category: Option<String>,
    all: bool,
) -> anyhow::Result<i32> {
    let opts = scan_opts(home, paths, min_size, category)?;
    let report = run_scan(&opts, !json && io::stderr().is_terminal())?;
    report.save(&home.last_report())?;
    if json {
        println!("{}", report.to_json()?);
        return Ok(0);
    }
    let paint = Paint::detect();
    print_scan_text(&mut io::stdout().lock(), &report, &paint, all)?;
    Ok(0)
}

fn middle_ellipsis(text: &str, max: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max || max < 5 {
        return text.to_string();
    }
    let keep = max - 1;
    let head = keep / 3;
    let tail = keep - head;
    let mut out: String = chars[..head].iter().collect();
    out.push('…');
    out.extend(&chars[chars.len() - tail..]);
    out
}

fn glyph(row: &HuskRow) -> (&'static str, Tone) {
    if row.reclaimable && row.risk == Risk::Safe {
        ("●", Tone::Verdigris)
    } else if row.reclaimable {
        ("◐", Tone::Amber)
    } else if matches!(row.risk, Risk::Dangerous | Risk::Forbidden) {
        ("■", Tone::Oxblood)
    } else {
        ("◆", Tone::Amber)
    }
}

fn write_row(out: &mut impl Write, row: &HuskRow, paint: &Paint) -> io::Result<()> {
    let (g, tone) = glyph(row);
    let indent = "  ".repeat(row.depth.min(3));
    let branch = row
        .branch
        .as_deref()
        .map(|b| format!("  {}", paint.tone(Tone::Copper, b)))
        .unwrap_or_default();
    writeln!(
        out,
        "  {} {:>9} {:>5}  {indent}{}{branch}",
        paint.tone(tone, g),
        paint.tone(Tone::Bone, &row.size),
        paint.tone(Tone::Ash, &row.age),
        paint.tone(Tone::Bone, &middle_ellipsis(&row.path, 72)),
    )?;
    for ward in &row.wards {
        writeln!(
            out,
            "  {:>17}  {indent}{}",
            "",
            paint.tone(tone, &format!("↳ {ward}"))
        )?;
    }
    Ok(())
}

pub fn print_scan_text(
    out: &mut impl Write,
    report: &ScanReport,
    paint: &Paint,
    all: bool,
) -> io::Result<()> {
    let view = ScanView::from_report(report);
    let deck = copy::get();
    writeln!(
        out,
        "{}  {}",
        paint.tone(Tone::Bone, &view.title),
        paint.tone(Tone::Copper, &view.subtitle)
    )?;
    if view.empty {
        writeln!(out, "{}", paint.tone(Tone::Ash, deck.empty_grove))?;
        return Ok(());
    }
    writeln!(
        out,
        "{}  {} · {} {} · {} {}",
        paint.tone(Tone::Amber, &view.reclaimable_label),
        paint.tone(Tone::Ash, &view.seen_label),
        view.husk_count,
        deck.husks,
        view.grove_count,
        deck.groves
    )?;
    writeln!(out)?;
    let title = paint.tone(Tone::Oxblood, &deck.alarms_title.to_uppercase());
    if view.alarms.is_empty() {
        writeln!(
            out,
            "  {title}  {}",
            paint.tone(Tone::Ash, deck.alarms_none)
        )?;
    } else {
        let count = deck
            .alarms_count
            .replace("{n}", &view.alarms.len().to_string());
        writeln!(out, "  {title}  {}", paint.tone(Tone::Amber, &count))?;
        for row in &view.alarms {
            write_row(out, row, paint)?;
        }
    }
    for section in &view.sections {
        writeln!(out)?;
        writeln!(
            out,
            "  {}  {} · {}",
            paint.tone(Tone::Copper, &section.label.to_uppercase()),
            section.count,
            format_bytes(section.bytes)
        )?;
        let limit = if all { usize::MAX } else { ROWS_PER_KIND };
        for row in section.rows.iter().take(limit) {
            write_row(out, row, paint)?;
        }
        if section.rows.len() > limit {
            writeln!(
                out,
                "  {:>17}  {}",
                "",
                paint.tone(
                    Tone::Ash,
                    &format!("+{} (--all)", section.rows.len() - limit)
                )
            )?;
        }
    }
    for w in view.warnings.iter().take(5) {
        writeln!(out, "  {}", paint.tone(Tone::Ash, &format!("! {w}")))?;
    }
    Ok(())
}

fn cmd_doctor(home: &Dirs, json: bool) -> anyhow::Result<i32> {
    let report = doctor(home);
    if json {
        println!("{}", report.to_json()?);
        return Ok(0);
    }
    let deck = copy::get();
    let paint = Paint::detect();
    println!("{}", paint.tone(Tone::Copper, deck.splash));
    let flag = |ok: bool| {
        if ok {
            paint.tone(Tone::Verdigris, "ok")
        } else {
            paint.tone(Tone::Oxblood, "no")
        }
    };
    println!("  session  {}", report.session);
    println!("  /proc    {}", flag(report.proc_ok));
    println!("  git      {}", flag(report.git_ok));
    println!("  trash    {}", flag(report.trash_ok));
    println!("  state    {}", tilde(&report.state_dir, &home.home));
    for root in report.roots.iter().filter(|r| r.exists) {
        let (mark, tone) = if root.exists {
            (deck.doctor_root_open, Tone::Verdigris)
        } else {
            (deck.doctor_root_dark, Tone::Ash)
        };
        println!(
            "  {}  {:<18} {}",
            paint.tone(tone, &format!("{mark:>6}")),
            root.id,
            tilde(&root.path, &home.home)
        );
    }
    if report.ok() {
        println!("{}", paint.tone(Tone::Bone, deck.doctor_ok));
    } else {
        println!("{}", paint.tone(Tone::Oxblood, deck.error_grove));
        for note in &report.notes {
            println!("  {note}");
        }
    }
    Ok(0)
}

fn load_report(home: &Dirs, from: Option<PathBuf>) -> anyhow::Result<ScanReport> {
    let path = from.unwrap_or_else(|| home.last_report());
    ScanReport::load(&path).map_err(|e| anyhow::anyhow!("{e}: {}", copy::get().no_report))
}

pub struct PlanArgs {
    pub preset: PresetArg,
    pub older_than: u64,
    pub from: Option<PathBuf>,
    pub out: Option<PathBuf>,
    pub only: Vec<PathBuf>,
    pub force: bool,
    pub json: bool,
}

fn resolve_only(report: &ScanReport, only: &[PathBuf]) -> anyhow::Result<BTreeSet<HuskId>> {
    only.iter()
        .map(|p| {
            let want = canonicalize_lossy(p);
            report
                .husks
                .iter()
                .find(|h| h.path == *p || canonicalize_lossy(&h.path) == want)
                .map(|h| h.id.clone())
                .ok_or_else(|| anyhow::anyhow!("{}: {}", p.display(), copy::get().search_none))
        })
        .collect()
}

fn cmd_plan(home: &Dirs, args: PlanArgs) -> anyhow::Result<i32> {
    let report = load_report(home, args.from)?;
    let built = if args.only.is_empty() {
        plan_with_clock(&report, args.preset.as_core(args.older_than), &SystemClock)
    } else {
        let marked = resolve_only(&report, &args.only)?;
        plan_marked(&report, &marked, args.force, &SystemClock)
    };
    if let Some(path) = args.out.as_ref() {
        built.save(path)?;
    }
    if args.json {
        println!("{}", built.to_json()?);
        return Ok(0);
    }
    let paint = Paint::detect();
    print_plan(&mut io::stdout().lock(), &built, &report.home, &paint)?;
    Ok(0)
}

pub fn print_plan(out: &mut impl Write, plan: &Plan, home: &Path, paint: &Paint) -> io::Result<()> {
    let deck = copy::get();
    writeln!(
        out,
        "{} {} · {} · {}",
        paint.tone(Tone::Copper, deck.plan_header),
        plan.preset.as_str(),
        plan.actions.len(),
        paint.tone(Tone::Amber, &format_bytes(plan.reclaimable_bytes()))
    )?;
    for action in &plan.actions {
        let kind = match action.kind {
            ActionKind::Trash => deck.action_trash,
            ActionKind::GitWorktreeRemove => deck.action_worktree_remove,
        };
        writeln!(
            out,
            "  {:<16} {:>9}  {}",
            paint.tone(Tone::Ash, kind),
            format_bytes(action.fingerprint.size_bytes),
            middle_ellipsis(&tilde(&action.path, home), 80)
        )?;
    }
    if plan.actions.is_empty() {
        writeln!(out, "  {}", paint.tone(Tone::Ash, deck.nothing_to_apply))?;
    }
    Ok(())
}

fn cmd_apply(home: &Dirs, plan_path: &Path, force: bool, json: bool) -> anyhow::Result<i32> {
    let plan = Plan::load(plan_path)?;
    // Held until this function returns; installers and other windows wait for it.
    let _lock = ApplyLock::acquire(
        &home.huskmap_state(),
        Path::new("/proc"),
        plan.actions.len(),
        plan.reclaimable_bytes(),
        SystemClock.now_ms(),
    )?;
    let result = apply(
        &plan,
        &ApplyOptions {
            git: &Git2Probe,
            reclaimer: &TrashReclaimer,
            processes: &ProcProcessProbe::default(),
            force,
        },
    )?;
    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        let deck = copy::get();
        let paint = Paint::detect();
        println!(
            "{}",
            paint.tone(
                Tone::Amber,
                &copy::format_apply_summary(
                    result.bytes_reclaimed(),
                    result.skipped.len(),
                    result.errors.len()
                )
            )
        );
        for skipped in &result.skipped {
            println!(
                "  {:<8} {}  {}",
                paint.tone(Tone::Ash, deck.apply_skip),
                skipped.path.display(),
                paint.tone(Tone::Amber, &skipped.reason)
            );
        }
        for err in &result.errors {
            println!("  {:<8} {err}", paint.tone(Tone::Oxblood, deck.apply_err));
        }
    }
    Ok(if result.is_clean() { 0 } else { 1 })
}

/// Ask once, on a terminal. Anything but yes is no.
fn confirm(question: &str) -> bool {
    if !io::stdin().is_terminal() {
        return false;
    }
    eprint!("{question} [y/N] ");
    let _ = io::stderr().flush();
    let mut line = String::new();
    io::stdin().read_line(&mut line).is_ok()
        && matches!(line.trim(), "y" | "Y" | "yes" | "s" | "sim")
}

fn cmd_update(home: &Dirs, check: bool, yes: bool) -> anyhow::Result<i32> {
    use huskmap_core::update::{self, Https, InstallKind, SystemHost};
    let deck = copy::get();
    let paint = Paint::detect();
    let found = update::check(&Https, update::CURRENT_VERSION);
    let _ = huskmap_core::settings::Settings::update(&home.config, |s| {
        s.last_update_check_ms = Some(SystemClock.now_ms());
    });
    let Some(release) = found? else {
        println!(
            "{}",
            paint.tone(
                Tone::Verdigris,
                &deck.up_to_date.replace("{v}", update::CURRENT_VERSION)
            )
        );
        return Ok(0);
    };
    println!(
        "{}",
        paint.tone(
            Tone::Amber,
            &deck.update_available.replace("{v}", &release.version)
        )
    );
    for line in release.summary(6) {
        println!("  {}", paint.tone(Tone::Ash, &line));
    }
    if !release.page.is_empty() {
        println!("  {}", paint.tone(Tone::Copper, &release.page));
    }
    if check {
        return Ok(0);
    }
    if let Some(lock) =
        huskmap_core::session::ApplyLock::holder(&home.huskmap_state(), Path::new("/proc"))
    {
        anyhow::bail!("{} (pid {})", deck.update_during_apply, lock.pid);
    }
    let kind = update::detect_install_kind(&SystemHost);
    if let InstallKind::Unmanaged(_) = kind {
        anyhow::bail!("{}", deck.update_unmanaged);
    }
    if !yes && !confirm(&format!("{} ({})?", deck.update_now, kind.label())) {
        return Ok(1);
    }
    println!("{}", deck.updating.replace("{v}", &release.version));
    let work = tempfile::tempdir()?;
    update::install(&release, &kind, &Https, &SystemHost, work.path())?;
    println!(
        "{}",
        paint.tone(
            Tone::Verdigris,
            &deck.update_done.replace("{v}", &release.version)
        )
    );
    Ok(0)
}

/// `key=value` lines. Stable: `install.sh` parses them.
pub fn write_status(home: &Dirs, proc_root: &Path, out: &mut impl Write) -> io::Result<()> {
    use huskmap_core::session::Session;
    let state = home.huskmap_state();
    let session = Session::load(&state).filter(|s| s.is_live(proc_root));
    let lock = ApplyLock::holder(&state, proc_root);
    writeln!(out, "version={}", huskmap_core::update::CURRENT_VERSION)?;
    writeln!(out, "window={}", session.as_ref().map_or(0, |s| s.pid))?;
    writeln!(
        out,
        "phase={}",
        session.as_ref().map_or("closed".to_string(), |s| {
            serde_json::to_value(s.phase)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_default()
        })
    )?;
    let saved = Session::load(&state).unwrap_or_default();
    writeln!(out, "marked={}", saved.marked.len() + saved.forced.len())?;
    writeln!(out, "marked_bytes={}", saved.marked_bytes)?;
    writeln!(out, "marked_human={}", format_bytes(saved.marked_bytes))?;
    writeln!(out, "applying={}", u8::from(lock.is_some()))?;
    writeln!(out, "apply_pid={}", lock.as_ref().map_or(0, |l| l.pid))?;
    writeln!(out, "apply_husks={}", lock.as_ref().map_or(0, |l| l.husks))?;
    Ok(())
}

fn cmd_map(home: &Dirs, from: Option<PathBuf>) -> anyhow::Result<i32> {
    let report = load_report(home, from)?;
    if io::stdout().is_terminal() {
        crate::tui::run_map(&report)?;
    } else {
        print_scan_text(&mut io::stdout().lock(), &report, &Paint::off(), true)?;
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use huskmap_core::{Husk, PlanPreset, Ward};
    use std::fs;

    fn seeded_home() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("dev/app/node_modules")).unwrap();
        fs::write(tmp.path().join("dev/app/package.json"), "{}").unwrap();
        fs::write(tmp.path().join("dev/app/node_modules/x"), "hello").unwrap();
        tmp
    }

    fn scan_into_cache(tmp: &tempfile::TempDir) {
        let code = cmd_scan(
            &Dirs::for_home(tmp.path()),
            vec![tmp.path().join("dev")],
            true,
            0,
            None,
            false,
        )
        .unwrap();
        assert_eq!(code, 0);
    }

    #[test]
    fn scan_json_writes_cache() {
        let tmp = seeded_home();
        scan_into_cache(&tmp);
        let cached = ScanReport::load(&Dirs::for_home(tmp.path()).last_report()).unwrap();
        assert_eq!(cached.husks.len(), 1);
    }

    #[test]
    fn doctor_json_and_text() {
        let tmp = seeded_home();
        assert_eq!(cmd_doctor(&Dirs::for_home(tmp.path()), true).unwrap(), 0);
        assert_eq!(cmd_doctor(&Dirs::for_home(tmp.path()), false).unwrap(), 0);
    }

    fn plan_args(only: Vec<PathBuf>, out: Option<PathBuf>) -> PlanArgs {
        PlanArgs {
            preset: PresetArg::Safe,
            older_than: 30,
            from: None,
            out,
            only,
            force: false,
            json: true,
        }
    }

    #[test]
    fn plan_preset_and_only() {
        let tmp = seeded_home();
        scan_into_cache(&tmp);
        let plan_path = tmp.path().join("plan.json");
        cmd_plan(
            &Dirs::for_home(tmp.path()),
            plan_args(vec![], Some(plan_path.clone())),
        )
        .unwrap();
        assert_eq!(Plan::load(&plan_path).unwrap().actions.len(), 1);
        let nm = tmp.path().join("dev/app/node_modules");
        cmd_plan(
            &Dirs::for_home(tmp.path()),
            plan_args(vec![nm], Some(plan_path.clone())),
        )
        .unwrap();
        let plan = Plan::load(&plan_path).unwrap();
        assert_eq!(plan.preset, PlanPreset::Marked);
        let err = cmd_plan(
            &Dirs::for_home(tmp.path()),
            plan_args(vec![tmp.path().join("nope")], None),
        )
        .unwrap_err();
        assert!(err.to_string().contains("nope"));
        let mut text = plan_args(vec![], None);
        text.json = false;
        cmd_plan(&Dirs::for_home(tmp.path()), text).unwrap();
    }

    #[test]
    fn plan_without_report_explains() {
        let tmp = tempfile::tempdir().unwrap();
        let err = cmd_plan(&Dirs::for_home(tmp.path()), plan_args(vec![], None)).unwrap_err();
        assert!(err.to_string().contains("huskmap scan"));
    }

    #[test]
    fn category_parse_error() {
        let tmp = seeded_home();
        let err = cmd_scan(
            &Dirs::for_home(tmp.path()),
            vec![],
            false,
            0,
            Some("nope".into()),
            false,
        )
        .unwrap_err();
        assert!(err.to_string().contains("unknown category"));
    }

    #[test]
    fn apply_missing_plan() {
        let err = cmd_apply(
            &Dirs::for_home("/nonexistent-huskmap"),
            Path::new("/no/such/plan.huskmap.json"),
            false,
            true,
        )
        .unwrap_err();
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn apply_empty_plan_is_clean() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("p.json");
        Plan {
            version: huskmap_core::PLAN_VERSION,
            created_at_ms: 1,
            preset: PlanPreset::Safe,
            roots: vec![],
            actions: vec![],
        }
        .save(&path)
        .unwrap();
        let dirs = Dirs::for_home(tmp.path());
        assert_eq!(cmd_apply(&dirs, &path, false, true).unwrap(), 0);
        assert_eq!(cmd_apply(&dirs, &path, false, false).unwrap(), 0);
        // a live apply elsewhere blocks this one
        let lock = ApplyLock::acquire(&dirs.huskmap_state(), Path::new("/proc"), 1, 1, 1).unwrap();
        assert!(cmd_apply(&dirs, &path, false, true).is_err());
        drop(lock);
        assert!(!confirm("never asked in tests"), "no terminal means no");
    }

    #[test]
    fn text_output_leads_with_alarms() {
        huskmap_core::copy::with_locale(huskmap_core::Locale::En, || {
            let mut report = ScanReport::empty(PathBuf::from("/h"), 1);
            let mut wt = Husk::bare(HuskKind::Worktree, "/h/dev/wt", 2048);
            wt.ward(Ward::Stranded { commits: 2 });
            let many: Vec<Husk> = (0..10)
                .map(|i| Husk::bare(HuskKind::Cache, format!("/h/.claude/cache{i}"), 10))
                .collect();
            report.husks = std::iter::once(wt).chain(many).collect();
            report.totals.husk_count = 11;
            let mut buf = Vec::new();
            print_scan_text(&mut buf, &report, &Paint::off(), false).unwrap();
            let text = String::from_utf8(buf).unwrap();
            let alarm = text.find("IN USE OR UNSAVED WORK").unwrap();
            let worktrees = text.find("WORKTREES").unwrap();
            assert!(alarm < worktrees);
            assert!(text.contains("↳ 2 commits exist only here"));
            assert!(text.contains("+2 (--all)"));
            assert!(!text.contains('\x1b'));
            let mut all = Vec::new();
            print_scan_text(&mut all, &report, &Paint::off(), true).unwrap();
            assert!(!String::from_utf8(all).unwrap().contains("--all"));
            let mut empty = Vec::new();
            print_scan_text(
                &mut empty,
                &ScanReport::empty(PathBuf::from("/h"), 1),
                &Paint::off(),
                false,
            )
            .unwrap();
            assert!(
                String::from_utf8(empty)
                    .unwrap()
                    .contains("Nothing scanned yet")
            );
        });
    }

    #[test]
    fn glyphs_and_ellipsis() {
        assert_eq!(middle_ellipsis("short", 10), "short");
        let long = "/home/me/Documentos/programacao/projeto-muito-longo/worktrees/x";
        let cut = middle_ellipsis(long, 24);
        assert_eq!(cut.chars().count(), 24);
        assert!(cut.contains('…') && cut.ends_with("/worktrees/x"));
        let mut report = ScanReport::empty(PathBuf::from("/h"), 1);
        let free = Husk::bare(HuskKind::Cache, "/h/a", 1);
        let mut caution = Husk::bare(HuskKind::Worktree, "/h/b", 1);
        caution.risk = Risk::Caution;
        let mut danger = Husk::bare(HuskKind::Worktree, "/h/c", 1);
        danger.ward(Ward::Dirty { files: 1 });
        let mut locked = Husk::bare(HuskKind::Worktree, "/h/d", 1);
        locked.ward(Ward::Locked { reason: None });
        report.husks = vec![free, caution, danger, locked];
        let glyphs: Vec<&str> = report
            .husks
            .iter()
            .map(|h| glyph(&HuskRow::from_husk(h, &report)).0)
            .collect();
        assert_eq!(glyphs, vec!["●", "◐", "■", "◆"]);
    }

    #[test]
    fn completions_and_man_render() {
        for shell in [
            clap_complete::Shell::Bash,
            clap_complete::Shell::Zsh,
            clap_complete::Shell::Fish,
        ] {
            let mut buf = Vec::new();
            write_completions(shell, &mut buf).unwrap();
            let text = String::from_utf8(buf).unwrap();
            assert!(
                text.contains("huskmap") && text.contains("scan"),
                "{shell:?}"
            );
        }
        let mut man = Vec::new();
        write_man(&mut man).unwrap();
        let man = String::from_utf8(man).unwrap();
        assert!(man.contains(".TH huskmap"));
        assert!(man.contains("doctor"));
    }

    #[test]
    fn status_lines_for_installers() {
        use huskmap_core::session::{Session, SessionPhase};
        let tmp = tempfile::tempdir().unwrap();
        let dirs = Dirs::for_home(tmp.path());
        let read = |proc: &Path| {
            let mut buf = Vec::new();
            write_status(&dirs, proc, &mut buf).unwrap();
            String::from_utf8(buf).unwrap()
        };
        let closed = read(Path::new("/proc"));
        assert!(
            closed.contains("window=0\nphase=closed\nmarked=0\n"),
            "{closed}"
        );
        assert!(closed.contains("applying=0"));
        Session {
            pid: std::process::id(),
            phase: SessionPhase::Scanning,
            marked: vec![HuskId::new(HuskKind::Cache, "/a")],
            marked_bytes: 2048,
            ..Default::default()
        }
        .save(&dirs.huskmap_state())
        .unwrap();
        let lock = ApplyLock::acquire(&dirs.huskmap_state(), Path::new("/proc"), 3, 9, 1).unwrap();
        let open = read(Path::new("/proc"));
        assert!(open.contains(&format!("window={}", std::process::id())));
        assert!(open.contains("phase=scanning"));
        assert!(open.contains("marked=1\nmarked_bytes=2048\nmarked_human=2.0 KB"));
        assert!(open.contains("applying=1") && open.contains("apply_husks=3"));
        drop(lock);
        // a dead window reads as closed but its marks are still reported
        let gone = read(&tmp.path().join("no-proc"));
        assert!(gone.contains("phase=closed") && gone.contains("marked=1"));
    }

    #[test]
    fn plan_text() {
        let mut buf = Vec::new();
        let plan = Plan {
            version: huskmap_core::PLAN_VERSION,
            created_at_ms: 1,
            preset: PlanPreset::Safe,
            roots: vec![],
            actions: vec![],
        };
        print_plan(&mut buf, &plan, Path::new("/h"), &Paint::off()).unwrap();
        assert!(String::from_utf8(buf).unwrap().contains("plan safe · 0"));
    }
}
