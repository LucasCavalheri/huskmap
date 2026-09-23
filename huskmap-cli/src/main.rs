mod args;
mod commands;
mod home;
mod paint;
mod tui;
mod view;

use clap::FromArgMatches;

use args::{Cli, command_localized};

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    huskmap_core::copy::init();
    let cli = match Cli::from_arg_matches(&command_localized().get_matches()) {
        Ok(cli) => cli,
        Err(err) => err.exit(),
    };
    if let Some(lang) = &cli.lang
        && let Ok(locale) = huskmap_core::Locale::parse(lang)
    {
        huskmap_core::copy::set_locale(locale);
    }
    match commands::exec(cli.home, cli.command) {
        Ok(code) => {
            if code != 0 {
                std::process::exit(code);
            }
        }
        Err(err) => {
            eprintln!("{} {err}", huskmap_core::copy::get().error_grove);
            std::process::exit(1);
        }
    }
}
