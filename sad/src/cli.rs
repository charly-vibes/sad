//! Purpose: clap CLI definition plus the single `run` dispatch that emits a
//! genesis suite JSON envelope for every sad command.
//! Responsibilities: parse arguments (global `--json`), route subcommands
//! (`version`, `check`), resolve `sad.toml` for `check`, drive the check
//! pipeline over the working directory, map config and baseline gate errors
//! to envelope errors, and render text mode.
//! Rationale: the envelope is the *single* output format for all CLI commands
//! (callers check `ok` first); Invariant 3.2.5 forbids emitting an error
//! without non-empty remediation, so every error path carries a fix hint.
//! The tool is a deterministic gate: no network, no models, no randomness —
//! same input tree, same findings.

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use genesis::envelope::{Envelope, EnvelopeKind, ErrorResult, RemediationEntry};
use serde::Serialize;

use crate::config::{Config, ConfigError};
use crate::{baseline, check, envelope};

/// Auto-discovered config file name, relative to the working directory.
const DEFAULT_CONFIG_FILE: &str = "sad.toml";

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
    /// Analyze the working tree for documentation drift.
    Check {
        /// Analyze every file instead of only changed ones.
        #[arg(long)]
        all: bool,
        /// Adopt existing documentation instead of reporting it.
        #[arg(long)]
        adopt: bool,
        /// Explicit `sad.toml` path; defaults to `./sad.toml` when present.
        #[arg(long, value_name = "PATH")]
        config: Option<PathBuf>,
    },
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

/// Resolve the effective config: explicit `--config`, else `./sad.toml` when
/// present, else [`Config::default`]. Discovery attempts the read, treating
/// only `NotFound` as absent, so an unreadable `sad.toml` is an error rather
/// than a silent fallback to defaults. On failure the tuple carries the path
/// that should be named in the error envelope.
fn resolve_config(explicit: Option<&Path>) -> Result<Config, (PathBuf, ConfigError)> {
    if let Some(path) = explicit {
        return Config::load(path).map_err(|err| (path.to_path_buf(), err));
    }
    let discovered = PathBuf::from(DEFAULT_CONFIG_FILE);
    match Config::load_if_present(&discovered) {
        Ok(Some(config)) => Ok(config),
        Ok(None) => Ok(Config::default()),
        Err(err) => Err((discovered, err)),
    }
}

/// Error envelope for an unusable config; the message names the path and, for
/// semantic failures, the offending key.
fn config_error_envelope(path: &Path, err: &ConfigError) -> serde_json::Value {
    let message = match err {
        ConfigError::Io { message } => message.clone(),
        ConfigError::Parse { message } => {
            format!("failed to parse config `{}`: {message}", path.display())
        }
        ConfigError::InvalidValue { key, message } => {
            format!("invalid config `{}`: `{key}` {message}", path.display())
        }
    };
    spec_error_envelope(
        "config_error",
        &message,
        None,
        "config",
        vec![RemediationEntry {
            command: "sad --help".into(),
            description: "Fix the `sad.toml` path or contents, then re-run `sad check`.".into(),
        }],
    )
}

/// Error envelope for a baseline gate failure — corrupt, unreadable, or
/// growth-violating `sad-baseline.json`. The message names the artifact and
/// the remediation is non-empty by construction.
fn gate_failure_envelope(failure: &check::GateFailure) -> serde_json::Value {
    let remediation = match failure {
        check::GateFailure::Baseline(_) => vec![RemediationEntry {
            command: format!("rm {} && sad check --adopt", baseline::BASELINE_FILE),
            description: format!(
                "Fix or delete the unusable `{}`, then re-adopt the current findings; a corrupt \
                 baseline is never ignored.",
                baseline::BASELINE_FILE
            ),
        }],
        check::GateFailure::Growth { .. } => vec![RemediationEntry {
            command: "sad check --adopt".into(),
            description: format!(
                "Fix the remaining findings, then re-adopt so `{}` only shrinks; never add ids \
                 to grow it.",
                baseline::BASELINE_FILE
            ),
        }],
    };
    spec_error_envelope(
        failure.code(),
        &failure.to_string(),
        None,
        "baseline",
        remediation,
    )
}

/// The single dispatch: every command returns its envelope serialized to
/// JSON. Callers check `ok` first — the envelope is the only output format.
pub fn run(command: Command) -> String {
    let envelope = match command {
        Command::Version => serde_json::to_value(version_envelope()),
        Command::Check { all, adopt, config } => {
            match resolve_config(config.as_deref()) {
                Ok(config) => {
                    // The gate always analyses the working directory, which is
                    // what the CLI contract documents; an unavailable cwd
                    // degrades to a relative "." rather than panicking.
                    let repo = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                    match check::run(&repo, &config, all, adopt) {
                        Ok(report) => serde_json::to_value(envelope::check_envelope(report)),
                        Err(failure) => Ok(gate_failure_envelope(&failure)),
                    }
                }
                Err((path, err)) => Ok(config_error_envelope(&path, &err)),
            }
        }
    };
    serde_json::to_string(&envelope.expect("envelope serialization cannot fail"))
        .expect("envelope serialization cannot fail")
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
    if v["envelope_kind"] == "check" {
        let findings = v["data"]["stats"]["findings"].as_u64().unwrap_or(0);
        let files = v["data"]["stats"]["files_analyzed"].as_u64().unwrap_or(0);
        let verdict = if v["ok"].as_bool().unwrap_or(false) {
            "pass"
        } else {
            "fail"
        };
        return format!("sad check — {verdict}: {findings} findings in {files} files\n");
    }
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
