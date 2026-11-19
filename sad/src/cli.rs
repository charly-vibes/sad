//! Purpose: clap CLI definition plus the single `run` dispatch that emits a
//! genesis suite JSON envelope for every sad command.
//! Responsibilities: parse arguments (global `--json`), route subcommands
//! (`version`, `check`), resolve `sad.toml` for `check`, drive the check
//! pipeline over the working directory, map config and baseline gate errors
//! to envelope errors, and expose text mode from [`crate::render`].
//! Rationale: the envelope is the *single* output format for all CLI commands
//! (callers check `ok` first); Invariant 3.2.5 forbids emitting an error
//! without non-empty remediation, so every error path carries a fix hint.
//! The check pipeline always analyses the working directory — an unavailable
//! cwd degrades to `.` rather than panicking — so the CLI contract holds even
//! there. The tool is a deterministic gate: no network, no models, no
//! randomness — same input tree, same findings. Text mode also explains an empty
//! attribution scope and every source the run could not read, so a green gate
//! that gated nothing never reads like a clean tree.

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use genesis::envelope::{Envelope, EnvelopeKind, ErrorResult, RemediationEntry};
use serde::Serialize;

use crate::baseline::{self, BaselineError};
use crate::config::{Config, ConfigError};
use crate::{check, envelope};

/// Auto-discovered config file name, relative to the working directory.
const DEFAULT_CONFIG_FILE: &str = "sad.toml";

/// Top-level CLI: `sad [--json] <subcommand>`.
///
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
    ///
    Version,
    /// Analyze the working tree for documentation drift.
    Check {
        /// Analyze every file instead of only changed ones.
        #[arg(long)]
        all: bool,
        /// Adopt existing documentation instead of reporting it; implies
        /// `--all` so adoption always sees the whole tree.
        #[arg(long)]
        adopt: bool,
        /// Fail on any finding, not only error-severity findings.
        #[arg(long)]
        strict: bool,
        /// Explicit `sad.toml` path; defaults to `./sad.toml` when present.
        #[arg(long, value_name = "PATH")]
        config: Option<PathBuf>,
    },
    /// Wire the sad gate into repository hook managers.
    Hooks {
        /// The hooks subcommand.
        #[command(subcommand)]
        command: HooksCommand,
    },
}

/// The `sad hooks` subcommands.
///
#[derive(Debug, Subcommand)]
pub enum HooksCommand {
    /// Install the sad gate into lefthook's pre-commit and pre-push stages.
    Install,
}

/// Data payload of the `sad version` envelope.
#[derive(Debug, Serialize)]
struct VersionData {
    name: &'static str,
    version: &'static str,
    status: &'static str,
}

/// Build the `sad hooks install` envelope: a successful command result
/// carrying the per-stage outcomes, never findings.
fn hooks_envelope(install: crate::hooks::HooksInstall) -> Envelope<crate::hooks::HooksInstall> {
    Envelope::success(
        env!("CARGO_PKG_VERSION"),
        EnvelopeKind::Ok,
        install,
        vec![],
        vec![],
    )
}

/// Spec-traced failure envelope for a refused hook installation; the
/// remediation names the concrete blocker (no repository, no lefthook
/// config, or an unusable config) per Invariant 3.2.5.
fn hooks_failure_envelope(error: &crate::hooks::lefthook::HooksError) -> serde_json::Value {
    use crate::hooks::lefthook::HooksError;
    let (message, remediation) = match error {
        HooksError::NotInRepo { .. } => (
            error.to_string(),
            RemediationEntry {
                command: "git init".into(),
                description:
                    "Hook wiring targets a git repository; run sad hooks install inside one.".into(),
            },
        ),
        HooksError::Framework { message } => (
            message.clone(),
            RemediationEntry {
                command: "lefthook init".into(),
                description:
                    "Create a lefthook.yml; sad wires managed blocks and never creates one.".into(),
            },
        ),
        HooksError::Io { .. } => (
            error.to_string(),
            RemediationEntry {
                command: "sad hooks install".into(),
                description: "Fix the filesystem error and re-run the installer.".into(),
            },
        ),
    };
    spec_error_envelope(
        "hooks_install_error",
        &message,
        None,
        "cli",
        vec![remediation],
    )
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
/// present, else [`Config::default`].
///
/// Discovery attempts the read, treating
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

/// Error envelope for a gate failure — a corrupt, unreadable, or
/// growth-violating `sad-baseline.json`, or a beads ledger that cannot judge
/// ids.
///
/// The message names the artifact and the remediation is non-empty by
/// construction; `config` is threaded through so a re-adoption hint repeats
/// the invocation the caller actually used.
fn gate_failure_envelope(
    failure: &envelope::GateFailure,
    config: Option<&Path>,
) -> serde_json::Value {
    let (module, remediation) = match failure {
        envelope::GateFailure::Baseline(error) => ("baseline", baseline_remediation(error, config)),
        envelope::GateFailure::Growth { .. } => ("baseline", growth_remediation(config)),
        envelope::GateFailure::Ledger { .. } => ("resolvers", ledger_remediation()),
    };
    spec_error_envelope(
        failure.code(),
        &failure.to_string(),
        None,
        module,
        remediation,
    )
}

/// The effective re-adoption invocation, including `--config` when supplied.
fn adopt_invocation(config: Option<&Path>) -> String {
    match config {
        Some(path) => format!("sad check --all --adopt --config {}", path.display()),
        None => "sad check --all --adopt".to_string(),
    }
}

/// Remediation for a typed baseline failure, chosen by what failed.
///
/// `Write` means the artifact is sound but unwritable; read/parse failures
/// mean the document itself must be fixed or deleted before re-adoption.
fn baseline_remediation(error: &BaselineError, config: Option<&Path>) -> Vec<RemediationEntry> {
    let adopt = adopt_invocation(config);
    match error {
        BaselineError::Write { .. } => vec![RemediationEntry {
            command: adopt.clone(),
            description: format!(
                "Ensure the `{}` path and its directory are writable, then re-run `{adopt}`.",
                baseline::BASELINE_FILE
            ),
        }],
        BaselineError::Read { .. } | BaselineError::Parse { .. } => vec![RemediationEntry {
            command: format!("rm {} && {adopt}", baseline::BASELINE_FILE),
            description: format!(
                "Fix or delete the unusable `{}`, then re-run `{adopt}`; a corrupt baseline is \
                 never ignored.",
                baseline::BASELINE_FILE
            ),
        }],
    }
}

/// Remediation for adoption growth: fix prose, then re-run the same adoption.
fn growth_remediation(config: Option<&Path>) -> Vec<RemediationEntry> {
    let command = adopt_invocation(config);
    vec![RemediationEntry {
        description: format!(
            "Fix the remaining findings, then re-run `{command}` so `{}` only shrinks; never \
             add ids to grow it.",
            baseline::BASELINE_FILE
        ),
        command,
    }]
}

/// Remediation for a broken beads ledger resolver source.
fn ledger_remediation() -> Vec<RemediationEntry> {
    vec![RemediationEntry {
        command: format!("bd export -o {}", check::BEADS_LEDGER),
        description: format!(
            "Restore or re-export `{}` so every line is one JSON record carrying an id, or turn \
             the resolver off with `[resolvers] beads = false` in `sad.toml`.",
            check::BEADS_LEDGER
        ),
    }]
}

/// The single dispatch: every command returns its envelope serialized to
/// JSON.
///
/// Callers check `ok` first — the envelope is the only output format.
pub fn run(command: Command) -> String {
    let envelope = match command {
        Command::Version => serde_json::to_value(version_envelope()),
        Command::Check {
            all,
            adopt,
            strict,
            config,
        } => match resolve_config(config.as_deref()) {
            Ok(resolved) => {
                let repo = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                match check::run(&repo, &resolved, all, adopt) {
                    Ok(report) => serde_json::to_value(envelope::check_envelope(report, strict)),
                    Err(failure) => Ok(gate_failure_envelope(&failure, config.as_deref())),
                }
            }
            Err((path, err)) => Ok(config_error_envelope(&path, &err)),
        },
        Command::Hooks {
            command: HooksCommand::Install,
        } => {
            let repo = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            match crate::hooks::lefthook::install(&repo) {
                Ok(install) => serde_json::to_value(hooks_envelope(install)),
                Err(error) => Ok(hooks_failure_envelope(&error)),
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
/// plain-text usage.
///
/// JSON on stdout, exit non-zero (see [`exit_code`]).
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

pub use crate::render::render_text;

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
