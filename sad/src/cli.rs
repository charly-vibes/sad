//! Purpose: clap CLI definition plus the single `run` dispatch that emits a
//! genesis suite JSON envelope for every sad command.
//! Responsibilities: parse arguments (global `--json`), route subcommands,
//! map errors to envelope errors, render text mode.
//! Rationale: the envelope is the *single* output format for all CLI commands
//! (callers check `ok` first); Invariant 3.2.5 forbids emitting an error
//! without non-empty remediation, so every error path carries a fix hint.
//! The tool is a deterministic gate: no network, no models, no randomness —
//! same input tree, same findings.

use clap::{Parser, Subcommand};
use genesis::envelope::{Envelope, EnvelopeKind, ErrorResult, RemediationEntry};
use serde::Serialize;

/// Top-level CLI: `sad [--json] <subcommand>`.
#[derive(Debug, Parser)]
#[command(
    name = "sad",
    version,
    about = "Documentation gate — code should speak for itself; prose means refactor"
)]
pub struct Cli {
    /// Emit the machine-readable suite envelope instead of a text rendering.
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Print tool identity as a suite envelope.
    Version,
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
            status: "scaffold",
        },
        vec![],
        vec![],
    )
}

/// The single dispatch: every command returns its envelope serialized to
/// JSON. Callers check `ok` first — the envelope is the only output format.
pub fn run(command: Command) -> String {
    let envelope = match command {
        Command::Version => version_envelope(),
    };
    serde_json::to_string(&envelope).expect("envelope serialization cannot fail")
}

/// Human-readable one-line message from a clap error:
/// - `DisplayHelpOnMissingArgumentOrSubcommand` (bare `sad`) renders the
///   about line first — that's the tool's purpose, not the mistake, so
///   substitute an explicit "subcommand required" message.
/// - Other kinds render `error: <summary>` then detail lines; the summary
///   alone is often useless, so take the first *paragraph* (up to the blank
///   line separating message from usage), newline-collapsed to one line.
fn clap_error_message(err: &clap::Error) -> String {
    if err.kind() == clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand {
        return "a subcommand is required (run sad --help)".to_string();
    }
    let rendered = err.to_string();
    let paragraph = rendered
        .split("\n\n")
        .next()
        .unwrap_or(&rendered)
        .trim_start_matches("error: ");
    paragraph.lines().collect::<Vec<_>>().join(" ")
}

/// Spec-traced error envelope: one construction point for every error
/// path, so Invariant 3.2.5 (non-empty remediation) holds by construction
/// — the `ErrorResult::new` constructor itself rejects an empty list.
fn spec_error_envelope(
    code: &str,
    message: &str,
    spec: Option<&str>,
    module: &str,
    remediation: Vec<RemediationEntry>,
) -> serde_json::Value {
    let error = ErrorResult::new(code, message, None, spec, Some(module), vec![], remediation)
        .expect("remediation is non-empty by construction (Invariant 3.2.5)");
    serde_json::to_value(Envelope::error(env!("CARGO_PKG_VERSION"), error, vec![]))
        .expect("envelope serialization cannot fail")
}

/// A bad invocation must still emit the suite envelope — never clap's
/// plain-text usage. JSON on stdout, exit non-zero (see [`exit_code`]).
pub fn argument_error_envelope(err: &clap::Error) -> String {
    let message = clap_error_message(err);
    serde_json::to_string(&spec_error_envelope(
        "argument_error",
        &message,
        None,
        "cli",
        vec![RemediationEntry {
            command: "sad --help".into(),
            description: "Run with --help for the accepted subcommands and flags.".into(),
        }],
    ))
    .expect("envelope serialization cannot fail")
}

/// Render an envelope JSON string as a short human-readable line.
///
/// Text mode is a convenience rendering of the same envelope — never a
/// second output format with different content.
pub fn render_text(json: &str) -> String {
    let v: serde_json::Value = serde_json::from_str(json).expect("run() always emits valid JSON");
    if v["ok"].as_bool().unwrap_or(false) {
        let name = v["data"]["name"].as_str().unwrap_or("sad");
        let version = v["data"]["version"].as_str().unwrap_or("?");
        let status = v["data"]["status"].as_str().unwrap_or("ok");
        format!("{name} {version} — {status}\n")
    } else {
        format!(
            "error: {}\n",
            v["data"]["message"].as_str().unwrap_or("unknown error")
        )
    }
}

/// Exit status mirrors the envelope: `ok:false` is never reported as
/// process success to machine consumers.
pub fn exit_code(json: &str) -> i32 {
    let v: serde_json::Value =
        serde_json::from_str(json).expect("sad always emits valid JSON envelopes");
    if v["ok"].as_bool().unwrap_or(false) {
        0
    } else {
        1
    }
}
