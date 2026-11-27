//! Purpose: thin binary entry point for `sad`.
//! Responsibilities: parse CLI args, print the emitted envelope (JSON mode)
//! or its text rendering (human mode), and exit non-zero when the envelope
//! reports failure — after a pre-parse `--version --json` check and any
//! raw-stdout command (completions, human doctor).
//! Rationale: all dispatch and envelope construction lives in [`sad::cli`] so
//! the binary layer stays thin and the logic stays unit-tested.

use clap::Parser;
use genesis::cli::maybe_print_version_json;
use sad::cli::{self, Cli, Command};

/// Run the `sad` command and exit with the envelope's status.
///
/// `--version --json` is delegated to `genesis::cli`: clap intercepts
/// `--version` at the parser level, before the global `--json` flag is
/// visible, so the combo is checked before parsing.
///
/// Human-mode doctor renders through the genesis guide instead of the text
/// rendering of the envelope; `--json` keeps the envelope path.
///
/// Exit status mirrors the envelope: ok:false is never reported as process
/// success to machine consumers.
fn main() {
    if maybe_print_version_json("sad", env!("CARGO_PKG_VERSION")) {
        return;
    }
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => handle_parse_error(err),
    };
    let json = match cli.command {
        Command::Doctor { fix } if !cli.json => cli::run_doctor_human(fix, cli.verbose.verbosity()),
        command => cli::run(command),
    };
    if cli.json {
        println!("{json}");
    } else {
        println!("{}", cli::render_text(&json));
    }
    std::process::exit(cli::exit_code(&json));
}

/// Handle a clap parse failure: help and version output print as clap text
/// and exit 0 — only real argument errors get the envelope treatment, so a
/// bad invocation still emits the suite envelope (JSON on stdout) and exits
/// non-zero, never clap's plain-text usage.
fn handle_parse_error(err: clap::error::Error) -> ! {
    if matches!(
        err.kind(),
        clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
    ) {
        err.print().expect("help/version output prints");
        std::process::exit(0);
    }
    let json = cli::argument_error_envelope(&err);
    println!("{json}");
    std::process::exit(cli::exit_code(&json));
}
