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
use genesis::doctor::{CheckStatus, DoctorReport};
use genesis::envelope::{Envelope, EnvelopeKind, RemediationEntry};
use genesis::guide::Verbosity;
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

/// Handle `--version --json` before clap parses: clap intercepts
/// `--version` at the parser level, before global flags like `--json` are
/// visible, so the combo is checked on the raw argv. It prints the same
/// envelope `sad version --json` emits, so the two version surfaces never
/// disagree.
pub fn maybe_print_version_json() -> bool {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let wants_version = args.iter().any(|a| a == "--version" || a == "-V");
    let wants_json = args.iter().any(|a| a == "--json" || a == "-j");
    if !(wants_version && wants_json) {
        return false;
    }
    match version_envelope_value() {
        Ok(json) => println!("{json}"),
        Err(err) => eprintln!("sad: cannot serialize version envelope: {err}"),
    }
    true
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
/// run from the cwd. Serialization cannot fail — the payload carries plain
/// strings — so the value is returned directly.
pub fn feedback_command_envelope(
    kind: String,
    title: Option<String>,
    dry_run: bool,
    from_last_error: bool,
) -> serde_json::Value {
    let repo = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let args = crate::feedback_cmd::FeedbackArgs {
        kind,
        title,
        dry_run,
        from_last_error,
    };
    crate::feedback_cmd::feedback_envelope(&args, &repo)
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
            let message = format!("failed to write shell completions: {err}");
            let json = serde_json::to_string(&spec_error_envelope(
                "completions_error",
                &message,
                "cli",
                vec![RemediationEntry {
                    command: "sad completions bash".into(),
                    description: "Ensure stdout is writable, then re-run with the wanted shell."
                        .into(),
                }],
            ))
            .expect("envelope serialization cannot fail");
            println!("{json}");
            crate::feedback_cmd::record_last_error(
                "completions_error",
                Some("sad completions bash".into()),
                1,
            );
            std::process::exit(crate::cli::exit_code(&json));
        }
    }
}

/// `sad doctor` without `--json`: print a plain summary — one line per
/// check plus the pass/warn/fail counts — hidden at quiet verbosity, then
/// exit with the report's own code. The report is the same one the JSON
/// path wraps, so the two modes can never disagree.
pub fn run_doctor_human(fix: bool, verbosity: Verbosity) -> ! {
    let repo = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let report = crate::doctor::run(&crate::doctor::DoctorArgs { fix }, &repo);
    if verbosity.should_show(Verbosity::Normal as u8) {
        println!("{}", render_report(&report));
    }
    if report.summary.has_failures() {
        let footer = report.checks.iter().find_map(|check| check.fix.clone());
        crate::feedback_cmd::record_last_error("doctor", footer, report.exit_code());
    }
    std::process::exit(report.exit_code());
}

/// Plain-text rendering of the doctor report: one line per check plus the
/// pass/warn/fail counts, so the human path answers the same question the
/// JSON envelope's `data` answers without Rust debug noise.
fn render_report(report: &DoctorReport) -> String {
    let mut lines: Vec<String> = report
        .checks
        .iter()
        .map(|check| {
            let status = match check.status {
                CheckStatus::Pass => "pass",
                CheckStatus::Warn => "warn",
                CheckStatus::Fail => "fail",
            };
            match &check.fix {
                Some(fix) => format!("{status} {}: {} (fix: {fix})", check.name, check.message),
                None => format!("{status} {}: {}", check.name, check.message),
            }
        })
        .collect();
    let summary = &report.summary;
    lines.push(format!(
        "sad doctor: {} pass, {} warn, {} fail",
        summary.pass, summary.warn, summary.fail
    ));
    lines.join("\n")
}
