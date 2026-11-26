//! `rbit`: rebuild old software byte for byte with its original compiler,
//! and show why the result can be trusted.
#![expect(clippy::print_stdout, reason = "a command-line tool reports on stdout")]

mod binary;
mod build;
mod catalog;
mod cli;
mod digest;
mod fetch;
mod grind;
mod home;
mod import;
#[cfg(feature = "mcp")]
mod mcp;
mod project;
mod runner;
mod tools;
mod ui;
mod verify;

use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    let parsed = cli::Cli::parse();
    ui::set_quiet(parsed.quiet);
    let json = parsed.json;
    #[cfg(feature = "mcp")]
    if matches!(parsed.command, cli::Command::Mcp) {
        return match mcp::serve() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                ui::error(&format!("{error:#}"));
                ExitCode::FAILURE
            }
        };
    }
    match cli::run(parsed.command) {
        Ok(outcome) => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&outcome.json).unwrap_or_default()
                );
            } else if !outcome.text.is_empty() {
                println!("{}", outcome.text.trim_end());
            }
            if outcome.ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(error) => {
            if json {
                println!("{}", serde_json::json!({ "error": format!("{error:#}") }));
            }
            ui::error(&format!("{error:#}"));
            ExitCode::from(2)
        }
    }
}
