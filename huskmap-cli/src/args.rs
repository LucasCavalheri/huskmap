use std::path::PathBuf;

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use huskmap_core::copy;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum PresetArg {
    Safe,
    AgentOnly,
    Older,
}

impl PresetArg {
    pub fn as_core(self, days: u64) -> huskmap_core::PlanPreset {
        match self {
            Self::Safe => huskmap_core::PlanPreset::Safe,
            Self::AgentOnly => huskmap_core::PlanPreset::AgentOnly,
            Self::Older => huskmap_core::PlanPreset::Older { days },
        }
    }
}

#[derive(Debug, Parser)]
#[command(
    name = copy::BINARY,
    version,
    about = copy::ABOUT,
    long_about = None,
    after_help = copy::SPLASH
)]
pub struct Cli {
    /// Hidden override so tests never touch a real home.
    #[arg(long, env = "HUSKMAP_HOME", hide = true, global = true)]
    pub home: Option<PathBuf>,

    /// Interface language: en or pt-br
    #[arg(long, env = "HUSKMAP_LANG", global = true, value_name = "LANG")]
    pub lang: Option<String>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

pub fn command_localized() -> clap::Command {
    let deck = copy::get();
    let mut cmd = Cli::command().about(deck.about).after_help(deck.splash);
    for (name, about) in [
        ("scan", deck.help_scan),
        ("doctor", deck.help_doctor),
        ("plan", deck.help_plan),
        ("apply", deck.help_apply),
        ("map", deck.help_map),
        ("gui", deck.help_gui),
        ("update", deck.help_update),
    ] {
        cmd = cmd.mut_subcommand(name, |sub| sub.about(about));
    }
    cmd.mut_arg("lang", |arg| arg.help(deck.help_lang))
}

/// `512`, `10K`, `500M`, `2G`, `1.5G`.
pub fn parse_size(raw: &str) -> Result<u64, String> {
    let t = raw.trim().to_ascii_uppercase();
    let t = t.strip_suffix('B').unwrap_or(&t);
    let (num, mul) = match t.chars().last() {
        Some('K') => (&t[..t.len() - 1], 1u64 << 10),
        Some('M') => (&t[..t.len() - 1], 1 << 20),
        Some('G') => (&t[..t.len() - 1], 1 << 30),
        Some('T') => (&t[..t.len() - 1], 1 << 40),
        _ => (t, 1),
    };
    let value: f64 = num
        .trim()
        .parse()
        .map_err(|_| format!("not a size: {raw}"))?;
    if value < 0.0 || !value.is_finite() {
        return Err(format!("not a size: {raw}"));
    }
    Ok((value * mul as f64) as u64)
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Walk known agent roots and map husks
    Scan {
        paths: Vec<PathBuf>,
        #[arg(long)]
        json: bool,
        /// Minimum size, e.g. 500M, 2G, or plain bytes
        #[arg(long, default_value = "0", value_parser = parse_size)]
        min_size: u64,
        /// worktree | ballast | toolchain | afterimage | cache | debris
        #[arg(long)]
        category: Option<String>,
        /// Print every husk instead of the heaviest per kind
        #[arg(long)]
        all: bool,
    },
    /// Check whether the grove can be sounded
    Doctor {
        #[arg(long)]
        json: bool,
    },
    /// Dry-run a reclaim plan from a scan report
    Plan {
        #[arg(long, value_enum, default_value_t = PresetArg::Safe)]
        preset: PresetArg,
        #[arg(long, default_value_t = huskmap_core::DEFAULT_OLDER_DAYS)]
        older_than: u64,
        #[arg(long)]
        from: Option<PathBuf>,
        #[arg(short, long)]
        out: Option<PathBuf>,
        /// Plan exactly these husk paths instead of a preset
        #[arg(long, value_name = "PATH")]
        only: Vec<PathBuf>,
        /// Let --only take dirty, stranded or locked worktrees. Occupied ones never go.
        #[arg(long)]
        force: bool,
        #[arg(long)]
        json: bool,
    },
    /// Apply a saved plan after re-checking facts
    Apply {
        #[arg(long)]
        plan: PathBuf,
        #[arg(long)]
        force: bool,
        #[arg(long)]
        json: bool,
    },
    /// Open a TUI of the last scan report
    Map {
        #[arg(long)]
        from: Option<PathBuf>,
    },
    /// Open the desktop map
    Gui,
    /// Check for and install a newer huskmap
    Update {
        /// Only say whether one exists
        #[arg(long)]
        check: bool,
        /// Do not ask before installing
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Print a shell completion script (bash, zsh, fish, elvish, powershell)
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
    /// Print the man page (roff) to stdout
    #[command(hide = true)]
    Man,
    /// Machine-readable state for installers: open window, marks, running apply
    #[command(hide = true)]
    Status,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_scan_json() {
        let cli = Cli::parse_from(["huskmap", "scan", "--json", "--min-size", "10", "/tmp"]);
        match cli.command {
            Some(Command::Scan {
                json,
                min_size,
                paths,
                ..
            }) => {
                assert!(json);
                assert_eq!(min_size, 10);
                assert_eq!(paths, vec![PathBuf::from("/tmp")]);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn apply_requires_plan() {
        let err = Cli::try_parse_from(["huskmap", "apply"]).unwrap_err();
        assert!(err.to_string().contains("--plan") || err.to_string().contains("required"));
    }

    #[test]
    fn no_args_is_ok() {
        let cli = Cli::parse_from(["huskmap"]);
        assert!(cli.command.is_none());
    }

    #[test]
    fn help_uses_product_voice() {
        let mut cmd = huskmap_core::copy::with_locale(huskmap_core::Locale::En, command_localized);
        let mut buf = Vec::new();
        cmd.write_help(&mut buf).unwrap();
        let help = String::from_utf8(buf).unwrap();
        assert!(huskmap_core::copy::asserts_voice(&help));
        assert!(help.contains("scan"));
        assert!(help.contains("doctor"));
        assert!(!help.to_ascii_lowercase().contains("agent-gc"));
    }

    #[test]
    fn help_pt_br() {
        let mut cmd =
            huskmap_core::copy::with_locale(huskmap_core::Locale::PtBr, command_localized);
        let mut buf = Vec::new();
        cmd.write_help(&mut buf).unwrap();
        let help = String::from_utf8(buf).unwrap();
        assert!(help.contains("Encontra e libera"));
        assert!(huskmap_core::copy::asserts_voice(&help));
    }

    #[test]
    fn size_parser() {
        assert_eq!(parse_size("0"), Ok(0));
        assert_eq!(parse_size("512"), Ok(512));
        assert_eq!(parse_size("10k"), Ok(10 * 1024));
        assert_eq!(parse_size("500M"), Ok(500 << 20));
        assert_eq!(parse_size("1.5GB"), Ok(3 << 29));
        assert_eq!(parse_size("2T"), Ok(2 << 40));
        assert!(parse_size("lots").is_err());
        assert!(parse_size("-1").is_err());
        let cli = Cli::parse_from(["huskmap", "scan", "--min-size", "1G"]);
        assert!(matches!(cli.command, Some(Command::Scan { min_size, .. }) if min_size == 1 << 30));
    }

    #[test]
    fn plan_only_and_force() {
        let cli = Cli::parse_from(["huskmap", "plan", "--only", "/a", "--only", "/b", "--force"]);
        match cli.command {
            Some(Command::Plan { only, force, .. }) => {
                assert_eq!(only.len(), 2);
                assert!(force);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn update_flags() {
        let cli = Cli::parse_from(["huskmap", "update", "--check"]);
        assert!(matches!(
            cli.command,
            Some(Command::Update {
                check: true,
                yes: false
            })
        ));
        let cli = Cli::parse_from(["huskmap", "update", "-y"]);
        assert!(matches!(
            cli.command,
            Some(Command::Update {
                check: false,
                yes: true
            })
        ));
    }

    #[test]
    fn completions_and_man_parse() {
        let cli = Cli::parse_from(["huskmap", "completions", "zsh"]);
        assert!(matches!(
            cli.command,
            Some(Command::Completions {
                shell: clap_complete::Shell::Zsh
            })
        ));
        assert!(matches!(
            Cli::parse_from(["huskmap", "man"]).command,
            Some(Command::Man)
        ));
        assert!(Cli::try_parse_from(["huskmap", "completions", "tcsh"]).is_err());
    }

    #[test]
    fn parses_lang() {
        let cli = Cli::parse_from(["huskmap", "--lang", "pt-br", "doctor"]);
        assert_eq!(cli.lang.as_deref(), Some("pt-br"));
    }

    #[test]
    fn preset_mapping() {
        assert!(matches!(
            PresetArg::Safe.as_core(30),
            huskmap_core::PlanPreset::Safe
        ));
        assert!(matches!(
            PresetArg::AgentOnly.as_core(30),
            huskmap_core::PlanPreset::AgentOnly
        ));
        assert_eq!(
            PresetArg::Older.as_core(7),
            huskmap_core::PlanPreset::Older { days: 7 }
        );
    }

    #[test]
    fn doctor_and_plan_and_map() {
        let _ = Cli::parse_from(["huskmap", "doctor", "--json"]);
        let _ = Cli::parse_from(["huskmap", "plan", "--preset", "agent-only", "--json"]);
        let _ = Cli::parse_from(["huskmap", "map"]);
        let _ = Cli::parse_from(["huskmap", "gui"]);
    }
}
