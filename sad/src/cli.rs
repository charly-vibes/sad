//! Purpose: clap CLI definition plus the single `run` dispatch that emits a
//! genesis suite JSON envelope for every sad command.
//! Responsibilities: parse arguments (global `--json`, guide verbosity),
//! route subcommands (`version`, `check`, `hooks`, `doctor`, `feedback`,
//! `completions`) — the non-check handlers live in [`crate::commands`] —
//! resolve `sad.toml` for `check`, drive the check
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
use genesis::envelope::RemediationEntry;
use genesis::guide::CliVerbosity;

use crate::envelope::spec_error_envelope;
use crate::hooks::{hooks_envelope, hooks_failure_envelope, hooks_uninstall_envelope};

use crate::baseline::{self, BaselineError};
use crate::config::{Config, ConfigError};
use crate::{check, commands, envelope};

pub use commands::{run_completions, run_doctor_human};

use commands::CheckArgs;

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

    /// Verbosity for guide-rendered human output (`-v`/`-vv`/`-vvv`, `-q`).
    #[command(flatten)]
    pub verbose: CliVerbosity,

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
        /// The check flags.
        #[command(flatten)]
        args: CheckArgs,
    },
    /// Wire the sad gate into repository hook managers.
    Hooks {
        /// The hooks subcommand.
        #[command(subcommand)]
        command: HooksCommand,
    },
    /// Run structured repository diagnostics, with optional auto-fix.
    Doctor {
        /// Wire missing `sad-check` managed blocks during the run.
        #[arg(long)]
        fix: bool,
    },
    /// Report an issue to the sad tracker.
    Feedback {
        /// Kind of issue (bug|feature|question|chore).
        kind: String,
        /// Override the issue title (wins over derived titles).
        #[arg(long)]
        title: Option<String>,
        /// Print the issue body and gh command without submitting.
        #[arg(long)]
        dry_run: bool,
        /// Auto-populate the body from the last failed sad command.
        #[arg(long)]
        from_last_error: bool,
    },
    /// Print shell completions for sad.
    Completions {
        /// Shell to generate completions for (bash, zsh, fish, powershell,
        /// elvish).
        shell: clap_complete::Shell,
    },
}

/// The `sad hooks` subcommands.
///
#[derive(Debug, Subcommand)]
pub enum HooksCommand {
    /// Install the sad gate into lefthook's pre-commit and pre-push stages.
    Install,
    /// Remove the sad gate from lefthook's pre-commit and pre-push stages.
    Uninstall,
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
    spec_error_envelope(failure.code(), &failure.to_string(), module, remediation)
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

/// The `sad check` envelope: resolve the config, run the pipeline over the
/// cwd, and map gate failures to error envelopes.
fn check_command_envelope(args: &CheckArgs) -> serde_json::Result<serde_json::Value> {
    match resolve_config(args.config.as_deref()) {
        Ok(resolved) => {
            let repo = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            match check::run(&repo, &resolved, args.all, args.adopt) {
                Ok(report) => serde_json::to_value(envelope::check_envelope(report, args.strict)),
                Err(failure) => Ok(gate_failure_envelope(&failure, args.config.as_deref())),
            }
        }
        Err((path, err)) => Ok(config_error_envelope(&path, &err)),
    }
}

/// The single dispatch: every command returns its envelope serialized to
/// JSON.
///
/// Callers check `ok` first — the envelope is the only output format.
pub fn run(command: Command) -> String {
    let envelope = match command {
        Command::Version => commands::version_envelope_value(),
        Command::Check { args } => check_command_envelope(&args),
        Command::Hooks { command } => hooks_command_envelope(command),
        Command::Doctor { fix } => commands::doctor_envelope(fix),
        Command::Feedback {
            kind,
            title,
            dry_run,
            from_last_error,
        } => commands::feedback_envelope(kind, title, dry_run, from_last_error),
        Command::Completions { shell } => run_completions(shell),
    };
    serde_json::to_string(&envelope.expect("envelope serialization cannot fail"))
        .expect("envelope serialization cannot fail")
}

/// Drive a `sad hooks` subcommand and serialize its outcome: the per-stage
/// envelope on success, the spec-traced refusal naming the subcommand
/// otherwise.
fn hooks_command_envelope(command: HooksCommand) -> serde_json::Result<serde_json::Value> {
    let repo = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    match command {
        HooksCommand::Install => match crate::hooks::lefthook::install(&repo) {
            Ok(install) => serde_json::to_value(hooks_envelope(install)),
            Err(error) => Ok(hooks_failure_envelope(
                &error,
                "hooks_install_error",
                "sad hooks install",
            )),
        },
        HooksCommand::Uninstall => match crate::hooks::lefthook::uninstall(&repo) {
            Ok(uninstall) => serde_json::to_value(hooks_uninstall_envelope(uninstall)),
            Err(error) => Ok(hooks_failure_envelope(
                &error,
                "hooks_uninstall_error",
                "sad hooks uninstall",
            )),
        },
    }
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

/// A bad invocation must still emit the suite envelope — never clap's
/// plain-text usage.
///
/// JSON on stdout, exit non-zero (see [`exit_code`]).
pub fn argument_error_envelope(err: &clap::Error) -> String {
    let message = clap_error_message(err);
    serde_json::to_string(&spec_error_envelope(
        "argument_error",
        &message,
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
