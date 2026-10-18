//! Thin binary: parse the command line, build the [`App`], run the terminal UI.

#![forbid(unsafe_code)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

use std::io::IsTerminal;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use gauss_trainer::app::App;
use gauss_trainer::cli::{self, CliError, Invocation};
use gauss_trainer::tui;
use gauss_trainer::ui::View;
use gauss_trainer::ui::palette::ColorDepth;

fn main() -> ExitCode {
    let args: Result<Vec<String>, CliError> = std::env::args_os()
        .skip(1)
        .map(|a| a.into_string().map_err(|_| CliError::NotUnicode))
        .collect();
    let invocation = match args.and_then(cli::parse_args) {
        Ok(invocation) => invocation,
        Err(e) => {
            eprintln!("Fehler: {e}");
            return ExitCode::from(2);
        }
    };

    match invocation {
        Invocation::Help => {
            print!("{}", cli::HELP);
            ExitCode::SUCCESS
        }
        Invocation::Version => {
            println!("gauss-trainer {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Invocation::Run(options) => {
            if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
                eprintln!("Fehler: gauss-trainer braucht ein interaktives Terminal.");
                return ExitCode::FAILURE;
            }
            let mut config = options.config;
            if options.seed.is_none() {
                config.seed = fresh_seed();
            }
            let depth = options.depth.unwrap_or_else(|| {
                ColorDepth::from_colorterm(std::env::var("COLORTERM").ok().as_deref())
            });
            let view = View::new(depth, options.brackets);
            match tui::run(App::new(config), view) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("Fehler im Terminal: {e}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}

/// A seed from the wall clock, used when `--seed` is not given.
fn fresh_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        // Truncation is fine: only the low bits vary between runs.
        .map_or(1, |d| d.as_nanos() as u64)
}
