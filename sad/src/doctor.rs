//! Purpose: `sad doctor` — repository health diagnostics over the artifacts
//! sad adopts at runtime.
//! Responsibilities: implement the `config`, `lefthook-wiring`, and
//! `baseline` genesis `DoctorCheck`s over a repository root, run them
//! through `genesis::doctor::DoctorRunner` behind [`run`], and map the
//! report onto the suite envelope the CLI prints via [`report_envelope`].
//! Rationale: the doctor observes, it never re-implements: the config
//! check loads through [`Config::load_if_present`], the wiring check reuses
//! the lefthook marker helpers so it can never disagree with
//! `sad hooks install`, and the baseline check reuses [`Baseline::load`],
//! so a check verdict always answers the same question the commands it
//! mirrors answer. A missing optional artifact is a pass — absence is a
//! state, not an ailment; only a present-but-broken one fails.

use std::error::Error;
use std::path::Path;

use genesis::doctor::{DoctorCheck, DoctorReport, DoctorRunner};
use genesis::envelope::{Envelope, EnvelopeKind, HintEntry, Warning};
use genesis::git_hooks::lefthook::Stage;
use genesis::suite_linter::{LintResult, Severity};

use crate::baseline::{self, Baseline};
use crate::config::{CONFIG_MARKER, Config};
use crate::hooks::lefthook;

/// The stages the wiring check expects the gate in, in wiring order.
const WIRED_STAGES: [Stage; 2] = [Stage::PreCommit, Stage::PrePush];

/// Arguments of `sad doctor`.
///
pub struct DoctorArgs {
    /// Wire the missing `sad-check` managed blocks (`--fix`).
    pub fix: bool,
}

/// Check that `sad.toml`, when present, parses and validates.
///
/// A missing config is a pass: every tunable has a documented default.
/// A present but unusable one fails with sad's established config-error
/// remediation, and the message carries the offending key when the
/// failure is semantic.
struct ConfigCheck;

impl DoctorCheck for ConfigCheck {
    fn name(&self) -> &'static str {
        "config"
    }

    fn description(&self) -> &'static str {
        "Validate sad.toml"
    }

    fn run(&self, repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn Error>> {
        let path = repo_root.join(CONFIG_MARKER);
        match Config::load_if_present(&path) {
            Ok(None) | Ok(Some(_)) => Ok(vec![]),
            Err(err) => Ok(vec![LintResult::with_fix(
                format!("invalid `{CONFIG_MARKER}`: {err}"),
                Severity::Error,
                "sad --help",
            )]),
        }
    }
}

/// Check that the `sad-check` managed block is wired into both pinned
/// lefthook stages.
///
/// Detection goes through the same helpers `sad hooks install` and
/// `sad hooks uninstall` use, so the doctor reports exactly the wiring
/// those commands act on. No lefthook config is a pass: sad never creates
/// one, and a repository without lefthook has nothing to wire.
struct WiringCheck;

impl DoctorCheck for WiringCheck {
    fn name(&self) -> &'static str {
        "lefthook-wiring"
    }

    fn description(&self) -> &'static str {
        "Check that the sad-check managed block is wired into pre-commit and pre-push"
    }

    fn run(&self, repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn Error>> {
        let Some(path) = lefthook::config_path(repo_root) else {
            return Ok(vec![]);
        };
        let Some(text) = lefthook::config_text(repo_root) else {
            return Err(format!("cannot read lefthook config `{}`", path.display()).into());
        };
        let unwired: Vec<&str> = WIRED_STAGES
            .iter()
            .filter(|stage| !lefthook::stage_has_block(&text, stage.key()))
            .map(|stage| stage.key())
            .collect();
        if unwired.is_empty() {
            Ok(vec![])
        } else {
            Ok(vec![LintResult::with_fix(
                format!(
                    "no `sad-check` managed block in the {} stage(s) of `{}`",
                    unwired.join(", "),
                    path.display()
                ),
                Severity::Warning,
                "sad hooks install",
            )])
        }
    }

    fn auto_fixable(&self) -> bool {
        true
    }

    /// Wire the gate through the same installer `sad hooks install` runs,
    /// so a fix and the command it names cannot diverge.
    fn fix(&self, repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn Error>> {
        lefthook::install(repo_root)
            .map(|_| vec![])
            .map_err(|err| err.to_string().into())
    }
}

/// Check that `sad-baseline.json`, when present, parses and validates.
///
/// An absent baseline means nothing was adopted yet. A corrupt one fails:
/// the gate refuses to run over a baseline it cannot read, and the
/// remediation is the re-adoption invocation the baseline module documents.
struct BaselineCheck;

impl DoctorCheck for BaselineCheck {
    fn name(&self) -> &'static str {
        "baseline"
    }

    fn description(&self) -> &'static str {
        "Validate sad-baseline.json"
    }

    fn run(&self, repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn Error>> {
        let path = repo_root.join(baseline::BASELINE_FILE);
        match Baseline::load(&path) {
            Ok(None) | Ok(Some(_)) => Ok(vec![]),
            Err(err) => Ok(vec![LintResult::with_fix(
                err.to_string(),
                Severity::Error,
                format!("rm {} && sad check --all --adopt", baseline::BASELINE_FILE),
            )]),
        }
    }
}

/// Run the doctor checks over the repository at `root`.
///
/// With `args.fix`, the wiring check wires missing managed blocks through
/// the `sad hooks install` path; the other checks cannot self-fix and are
/// left advisory. The runner never returns an error for these checks —
/// every per-check failure is captured as a fail entry in the report.
pub fn run(args: &DoctorArgs, root: &Path) -> DoctorReport {
    DoctorRunner::new(vec![
        Box::new(ConfigCheck),
        Box::new(WiringCheck),
        Box::new(BaselineCheck),
    ])
    .with_tool_name("sad")
    .run(root, args.fix)
    .expect("DoctorRunner::run captures per-check errors and returns Ok")
}

/// Warnings for every check that reported an issue.
fn report_warnings(report: &DoctorReport) -> Vec<Warning> {
    report
        .checks
        .iter()
        .filter(|check| check.status.is_issue())
        .map(|check| Warning {
            rule_name: check.name.clone(),
            entity_id: None,
            message: check.message.clone(),
            suggested_remediation: check.fix.clone(),
        })
        .collect()
}

/// Hints carrying the runnable fix command of every check that has one.
fn report_hints(report: &DoctorReport) -> Vec<HintEntry> {
    report
        .checks
        .iter()
        .filter_map(|check| {
            check.fix.as_ref().map(|command| HintEntry {
                command: command.clone(),
                description: check.message.clone(),
            })
        })
        .collect()
}

/// Build the `sad doctor` suite envelope from a completed report.
///
/// The report is the `data` payload. Every non-pass check surfaces twice:
/// as a warning carrying its message and remediation, and as a hint
/// carrying the runnable fix command, so a machine consumer finds the
/// remediation either way. `ok` stays `true` while the report is
/// warning-only — a warn is advice, not a gate — and goes `false` only
/// when a check failed.
pub fn report_envelope(report: &DoctorReport) -> Envelope<&DoctorReport> {
    let mut envelope = Envelope::success(
        env!("CARGO_PKG_VERSION"),
        EnvelopeKind::Doctor,
        report,
        report_warnings(report),
        report_hints(report),
    );
    envelope.ok = !report.summary.has_failures();
    envelope
}
