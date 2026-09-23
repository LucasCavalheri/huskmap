//! huskmap-core — scan, classify, size, risk, plan, apply.
//!
//! No UI. CLI and GUI are views over this crate.

mod apply;
mod classify;
mod clock;
pub mod copy;
mod doctor;
mod domain;
mod error;
mod git;
mod ids;
mod plan;
mod process;
mod reclaim;
mod roots;
mod safety;
mod scan;
mod secrets;
pub mod session;
pub mod settings;
mod size;
pub mod update;

pub use apply::{ApplyOptions, apply, apply_or_refuse_empty_plan};
pub use classify::{Classification, ClassifyContext, HuskSeed, agent_for_path, classify_path};
pub use clock::{Clock, FrozenClock, Millis, SystemClock, millis_of, system_time_from_millis};
pub use copy::{Deck, Locale};
pub use doctor::{DoctorReport, RootHealth, doctor, doctor_with, session_from};
pub use domain::{
    ActionKind, Afterimage, AgentKind, AppliedAction, ApplyResult, Ballast, BallastKind,
    DEFAULT_OLDER_DAYS, Ecosystem, Fingerprint, GitFacts, Grove, Holder, Husk, HuskKind,
    PLAN_VERSION, Plan, PlanAction, PlanPreset, RECENT_MS, REPORT_VERSION, Risk, ScanReport,
    SkippedAction, Totals, Ward, format_age, format_bytes, tilde,
};
pub use error::Error;
pub use git::{
    Git2Probe, GitMarker, GitProbe, MapGitProbe, find_checkouts, git_marker, inspect_tree,
    merge_facts,
};
pub use ids::HuskId;
pub use plan::{admissible, plan, plan_marked, plan_with_clock};
pub use process::{
    DeadProcessProbe, FixedProcessProbe, HolderIndex, ProcProcessProbe, ProcessProbe,
};
pub use reclaim::{FsReclaimer, Reclaimer, TrashReclaimer};
pub use roots::{
    Base, Dirs, ResolvedRoot, SCAN_ROOTS, ScanRootKind, ScanRootSpec, agent_homes, dedupe_nested,
    display_available, display_available_from, existing_roots, resolve_known_roots,
};
pub use safety::{
    canonicalize_lossy, file_name_str, is_forbidden_name, is_forbidden_path, is_skip_dir,
    is_skip_dir_name, is_within_roots, path_has_component,
};
pub use scan::{
    ScanEvent, ScanOptions, ScanRoot, claude_project_key, percent_decode, resolve_claude_origin,
    scan, scan_with_progress,
};
pub use secrets::guarding_secrets;
pub use size::{SizeOutcome, dir_size};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn report_and_plan_save_load_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        fs::create_dir_all(home.join("app")).unwrap();
        fs::write(home.join("app/package.json"), "{}").unwrap();
        fs::create_dir_all(home.join("app/node_modules")).unwrap();
        fs::write(home.join("app/node_modules/x"), "xx").unwrap();
        let mut opts = ScanOptions::for_home(home.to_path_buf());
        opts.extra_roots = vec![home.to_path_buf()];
        opts.git = std::sync::Arc::new(MapGitProbe::default());
        let report = scan(&opts).unwrap();
        let report_path = tmp.path().join("report.json");
        report.save(&report_path).unwrap();
        let loaded = ScanReport::load(&report_path).unwrap();
        assert_eq!(loaded.husks.len(), report.husks.len());

        let built = plan_with_clock(&loaded, PlanPreset::Safe, &FrozenClock::from_millis(9));
        let plan_path = tmp.path().join("nested/plan.json");
        built.save(&plan_path).unwrap();
        let loaded_plan = Plan::load(&plan_path).unwrap();
        assert_eq!(loaded_plan.actions.len(), built.actions.len());
    }

    #[test]
    fn load_missing_errors() {
        let err = ScanReport::load(Path::new("/no/such/huskmap-report.json")).unwrap_err();
        assert!(matches!(err, Error::Io { .. }));
        let err = Plan::load(Path::new("/no/such/huskmap-plan.json")).unwrap_err();
        assert!(matches!(err, Error::Io { .. }));
    }

    #[test]
    fn public_api_names_are_stable() {
        let _ = HuskKind::Worktree;
        let _ = HuskKind::Afterimage;
        let _ = HuskKind::Ballast;
        let _ = SCAN_ROOTS.len();
        assert!(copy::asserts_voice(copy::ABOUT));
    }
}
