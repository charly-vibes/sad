//! Purpose: wire the sad gate into a repository's lefthook config as a
//! managed block via genesis `ensure_wired`.
//! Responsibilities: resolve the repository root from a starting directory,
//! inject or verify the `sad:managed` block carrying the design-pinned
//! plain `sad check` gate in `pre-commit` and `pre-push`, skip a stage that
//! already runs the gate outside the block, and report per-stage outcomes
//! or a named failure — never creating a lefthook config.
//! Rationale: genesis passes the mechanics (column-0 anchor injection,
//! file-level idempotence) and sad owns the marker and the gate command
//! (boundary rule); `sad check` fails on error-severity findings only,
//! mirroring `pretender --mode gate`. A user-authored gate inside a stage
//! is invisible to genesis's file-level idempotence (espectacular GH#47),
//! so the per-stage scan here keeps a doubled gate off the adoption path.

use std::fmt;
use std::path::{Path, PathBuf};

use genesis::git_hooks::{self, lefthook::Stage};
use genesis::managed_block::BlockDef;

use crate::hooks::{HooksInstall, StageOutcome};

/// The managed block's entries, nested inside a stage's `commands:` mapping
/// at the commands-child indent (4 spaces).
///
/// The leading newline keeps the
/// start marker on its own line: `ensure_wired` concatenates marker plus
/// content verbatim, and without it the glued line would begin with `#`
/// and read as a YAML comment (genesis-au8 glue pattern).
const SAD_HOOK_COMMAND: &str = "\n    sad-check:\n      run: sad check\n";

/// The stages the gate wires, in wiring order.
const STAGES: [Stage; 2] = [Stage::PreCommit, Stage::PrePush];

/// Why `sad hooks install` refused to wire the gate.
///
#[derive(Debug)]
pub enum HooksError {
    /// The starting directory is not inside a git repository.
    NotInRepo {
        /// The directory the upward walk started from.
        start: PathBuf,
    },
    /// The repository has no lefthook config, or the config cannot be
    /// anchored — genesis reports the framework problem and never creates
    /// a config.
    Framework {
        /// Human-readable framework problem naming lefthook.
        message: String,
    },
    /// The filesystem failed while reading or writing the config.
    Io {
        /// Path the failed operation touched.
        path: PathBuf,
        /// Human-readable context.
        message: String,
    },
}

impl fmt::Display for HooksError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotInRepo { start } => write!(
                f,
                "not inside a git repository: {} has no .git entry in itself or any parent",
                start.display()
            ),
            Self::Framework { message } => write!(f, "{message}"),
            Self::Io { path, message } => write!(f, "{message}: {}", path.display()),
        }
    }
}

impl From<git_hooks::GitHooksError> for HooksError {
    fn from(error: git_hooks::GitHooksError) -> Self {
        match error {
            git_hooks::GitHooksError::NotInRepo { start } => Self::NotInRepo { start },
            git_hooks::GitHooksError::MissingLefthookConfig { root } => Self::Framework {
                message: format!(
                    "no lefthook.yml under {}: sad hooks install wires the gate into lefthook and never creates a config",
                    root.display()
                ),
            },
            git_hooks::GitHooksError::UnanchorableLefthookConfig { stage, message, .. } => {
                Self::Framework {
                    message: format!("lefthook {stage} stage: {message}"),
                }
            }
            git_hooks::GitHooksError::Io { path, message, .. } => Self::Io { path, message },
            other => Self::Framework {
                message: other.to_string(),
            },
        }
    }
}

/// Install or verify the sad gate in the repository containing `start`,
/// for both pinned stages, and report what each stage became.
///
/// Errors leave the config untouched: genesis refuses to anchor or to
/// guess at unbalanced markers, and the pre-insertion guard runs before
/// any write.
pub fn install(start: &Path) -> Result<HooksInstall, HooksError> {
    let root = git_hooks::repo_root_from(start)?;
    let existing = config_text(&root);
    let block = BlockDef::with_markers("sad:managed", "# sad:managed:start", "# sad:managed:end");
    let mut install = HooksInstall {
        pre_commit: StageOutcome::AlreadyWired,
        pre_push: StageOutcome::AlreadyWired,
    };
    for stage in STAGES {
        if existing
            .as_deref()
            .is_some_and(|text| stage_runs_gate(text, stage.key()))
        {
            set_outcome(&mut install, stage, StageOutcome::SkippedUserGate);
            continue;
        }
        let outcome =
            match git_hooks::lefthook::ensure_wired(&root, stage, &block, SAD_HOOK_COMMAND)? {
                git_hooks::lefthook::WiredOutcome::Injected => StageOutcome::Injected,
                git_hooks::lefthook::WiredOutcome::AlreadyWired => StageOutcome::AlreadyWired,
            };
        set_outcome(&mut install, stage, outcome);
    }
    Ok(install)
}

fn set_outcome(install: &mut HooksInstall, stage: Stage, outcome: StageOutcome) {
    match stage {
        Stage::PreCommit => install.pre_commit = outcome,
        Stage::PrePush => install.pre_push = outcome,
    }
}

/// The lefthook config text genesis would wire, when one exists.
///
/// Mirrors genesis's private `config_path` precedence (`lefthook.yml` wins
/// over `lefthook.yaml`) so the guard and the injection always read the
/// same file.
fn config_text(root: &Path) -> Option<String> {
    ["lefthook.yml", "lefthook.yaml"]
        .iter()
        .map(|name| root.join(name))
        .find(|path| path.is_file())
        .and_then(|path| std::fs::read_to_string(path).ok())
}

/// Whether the stage's section already invokes the gate (espectacular
/// GH#47): genesis's idempotence matches only the exact block text, so a
/// user-authored `run: sad check` would gain a duplicate without this
/// scan.
///
/// The section spans from the stage's column-0 key line to the next
/// non-blank column-0 line; a missing stage returns false, because its
/// wiring proceeds.
fn stage_runs_gate(content: &str, stage_key: &str) -> bool {
    let start = if content.starts_with(stage_key) {
        0
    } else {
        match content.find(&format!("\n{stage_key}:")) {
            Some(index) => index + 1,
            None => return false,
        }
    };
    for line in content[start..].lines().skip(1) {
        if !line.is_empty() && !line.starts_with([' ', '\t']) {
            break;
        }
        if line.contains("sad check") {
            return true;
        }
    }
    false
}
