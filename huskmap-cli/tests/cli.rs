use std::fs;
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_huskmap"))
}

#[test]
fn help_mentions_scan_and_voice() {
    let out = bin().arg("--help").output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success());
    assert!(text.contains("scan"));
    assert!(text.contains("doctor"));
    assert!(text.contains("plan"));
    assert!(text.contains("apply"));
    assert!(!text.to_ascii_lowercase().contains("agent-gc"));
    assert!(!text.to_ascii_lowercase().contains("cleaner"));
}

#[test]
fn scan_json_in_temp_home() {
    let tmp = tempfile::tempdir().unwrap();
    fs::create_dir_all(tmp.path().join("dev/app/node_modules")).unwrap();
    fs::write(tmp.path().join("dev/app/package.json"), "{}").unwrap();
    fs::write(tmp.path().join("dev/app/node_modules/x"), "hello-world").unwrap();
    let out = bin()
        .env("HUSKMAP_HOME", tmp.path())
        .args(["scan", "--json"])
        .arg(tmp.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("ballast") || stdout.contains("husks"));
}

#[test]
fn doctor_json() {
    let tmp = tempfile::tempdir().unwrap();
    let out = bin()
        .env("HUSKMAP_HOME", tmp.path())
        .args(["doctor", "--json"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("claude") || stdout.contains("roots"));
}

#[test]
fn apply_without_plan_fails() {
    let out = bin().arg("apply").output().unwrap();
    assert!(!out.status.success());
}
