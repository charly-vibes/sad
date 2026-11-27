//! Purpose: handlers for the CLI subcommands outside the check pipeline —
//! version, doctor, feedback, and completions.
//! Responsibilities: build the version and doctor JSON envelopes, wrap the
//! feedback arguments, render the human doctor report, and print shell
//! completions raw.
//! Rationale: [`crate::cli::run`] must stay a thin dispatcher, so these
//! self-contained command handlers live in a sibling module; completions and
//! the human doctor exit the process directly, and every JSON path still
//! emits the single suite envelope the callers check `ok` first.

use std::path::PathBuf;

use clap::CommandFactory;
use genesis::envelope::{Envelope, EnvelopeKind, RemediationEntry};
use genesis::guide::{Output, Verbosity};
use serde::Serialize;

use crate::envelope::spec_error_envelope;

/// The `sad check` flags.
///
/// `--all` analyzes every file instead of only changed ones; `--adopt`
/// adopts existing documentation instead of reporting it and implies
/// `--all`; `--strict` fails on any finding, not only error-severity
/// findings; `--config PATH` points at an explicit `sad.toml`, defaulting
/// to `./sad.toml` when present.
#[derive(Debug, clap::Args)]
pub struct CheckArgs {
    /// Analyze every file instead of only changed ones.
    #[arg(long)]
    pub all: bool,
    /// Adopt existing documentation instead of reporting it; implies
    /// `--all` so adoption always sees the whole tree.
    #[arg(long)]
    pub adopt: bool,
    /// Fail on any finding, not only error-severity findings.
    #[arg(long)]
    pub strict: bool,
    /// Explicit `sad.toml` path; defaults to `./sad.toml` when present.
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,
}

/// Data payload of the `sad version` envelope.
#[derive(Debug, Serialize)]
struct VersionData {
    name: &'static str,
    version: &'static str,
    status: &'static str,
}

fn version_envelope() -> Envelope<VersionData> {
    Envelope::success(
        env!("CARGO_PKG_VERSION"),
        EnvelopeKind::Version,
        VersionData {
            name: "sad",
            version: env!("CARGO_PKG_VERSION"),
            status: "active",
        },
        vec![],
        vec![],
    )
}

/// The `sad version` envelope serialized as a JSON value.
pub(crate) fn version_envelope_value() -> serde_json::Result<serde_json::Value> {
    serde_json::to_value(version_envelope())
}

/// The `sad doctor --json` envelope: structured repository diagnostics for
/// the cwd.
///
/// The report is the same one the human path wraps, so the two modes can
/// never disagree.
pub(crate) fn doctor_envelope(fix: bool) -> serde_json::Result<serde_json::Value> {
    let repo = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let report = crate::doctor::run(&crate::doctor::DoctorArgs { fix }, &repo);
    serde_json::to_value(crate::doctor::report_envelope(&report))
}

/// The `sad feedback` envelope: issue reporting for the parsed arguments,
/// run from the cwd.
pub(crate) fn feedback_envelope(
    kind: String,
    title: Option<String>,
    dry_run: bool,
    from_last_error: bool,
) -> serde_json::Result<serde_json::Value> {
    let repo = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let args = crate::feedback_cmd::FeedbackArgs {
        kind,
        title,
        dry_run,
        from_last_error,
    };
    Ok(crate::feedback_cmd::feedback_envelope(&args, &repo))
}

/// `sad completions <shell>`: print the raw shell script to stdout.
///
/// Completions are the one command whose success output is not an envelope,
/// so this leaves the process directly — exit 0 after the script, or the
/// suite envelope with non-empty remediation when the write fails.
pub fn run_completions(shell: clap_complete::Shell) -> ! {
    let mut cmd = crate::cli::Cli::command();
    match genesis::cli::generate_completions(&mut cmd, shell) {
        Ok(()) => std::process::exit(0),
        Err(err) => {
            let json = serde_json::to_string(&spec_error_envelope(
                "completions_error",
                &format!("failed to write shell completions: {err}"),
                "cli",
                vec![RemediationEntry {
                    command: "sad completions bash".into(),
                    description: "Ensure stdout is writable, then re-run with the wanted shell."
                        .into(),
                }],
            ))
            .expect("envelope serialization cannot fail");
            println!("{json}");
            std::process::exit(crate::cli::exit_code(&json));
        }
    }
}

/// `sad doctor` without `--json`: render the report through the genesis
/// guide.
///
/// The pass/warn/fail summary and every per-check message appear at the
/// requested verbosity, then the process exits with the report's own code.
/// The report is the same one the JSON path wraps, so the two modes can
/// never disagree.
pub fn run_doctor_human(fix: bool, verbosity: Verbosity) -> ! {
    let repo = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let report = crate::doctor::run(&crate::doctor::DoctorArgs { fix }, &repo);
    Output::success(&report)
        .print(verbosity, &mut std::io::stdout(), &mut std::io::stderr())
        .unwrap_or_else(|err| eprintln!("sad doctor: cannot write report: {err}"));
    std::process::exit(report.exit_code());
}
