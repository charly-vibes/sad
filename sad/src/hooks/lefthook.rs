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

use crate::hooks::{HooksInstall, HooksUninstall, StageOutcome, UninstallOutcome};

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

/// The sad-owned managed block identity, markers included: genesis's
/// idempotence matches this exact text, and the guard must know the same
/// markers to skip our own block when scanning for user-authored gates.
const MANAGED_NAME: &str = "sad:managed";
const MANAGED_START: &str = "# sad:managed:start";
const MANAGED_END: &str = "# sad:managed:end";

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
/// Wiring is per stage: a stage that fails leaves the stages wired before
/// it in place, and re-running after fixing the config completes the
/// wiring (`already_wired` outcomes). Genesis refuses to anchor or to
/// guess at unbalanced markers, and the pre-insertion guard runs before
/// any write.
pub fn install(start: &Path) -> Result<HooksInstall, HooksError> {
    let root = git_hooks::repo_root_from(start)?;
    let existing = config_text(&root);
    let pre_commit = stage_outcome(&root, existing.as_deref(), Stage::PreCommit)?;
    let pre_push = stage_outcome(&root, existing.as_deref(), Stage::PrePush)?;
    Ok(HooksInstall {
        pre_commit,
        pre_push,
    })
}

/// The outcome one pinned stage became, wiring the gate into it when the
/// existing config does not already run a user-authored gate.
///
/// Callers evaluate the stages in wiring order, so a stage that fails leaves
/// the stages wired before it in place.
fn stage_outcome(
    root: &Path,
    existing: Option<&str>,
    stage: Stage,
) -> Result<StageOutcome, HooksError> {
    if existing.is_some_and(|text| stage_runs_gate(text, stage.key())) {
        return Ok(StageOutcome::SkippedUserGate);
    }
    let block = BlockDef::with_markers(MANAGED_NAME, MANAGED_START, MANAGED_END);
    Ok(
        match git_hooks::lefthook::ensure_wired(root, stage, &block, SAD_HOOK_COMMAND)? {
            git_hooks::lefthook::WiredOutcome::Injected => StageOutcome::Injected,
            git_hooks::lefthook::WiredOutcome::AlreadyWired => StageOutcome::AlreadyWired,
        },
    )
}

/// Remove every sad-owned managed block from the repository containing
/// `start`, in both pinned stages, and report what each stage became.
///
/// Removal is marker-guarded: only lines closed by the exact
/// `sad:managed` markers disappear, foreign managed blocks survive
/// byte-for-byte, and a config with unbalanced markers is refused instead
/// of guessed at. A stage without the block is a no-op success
/// (`not_wired`), mirroring genesis lifecycle semantics.
pub fn uninstall(start: &Path) -> Result<HooksUninstall, HooksError> {
    let root = git_hooks::repo_root_from(start)?;
    let path = config_path(&root).ok_or_else(|| HooksError::Framework {
        message: format!(
            "no lefthook.yml under {}: sad hooks uninstall unwires the gate from lefthook and there is no config to unwire",
            root.display()
        ),
    })?;
    let text = read_config(&path)?;
    let stripped = strip_sad_blocks(&text)?;
    ensure_blocks_are_pinned(&text)?;
    let uninstall = HooksUninstall {
        pre_commit: uninstall_outcome(&text, Stage::PreCommit),
        pre_push: uninstall_outcome(&text, Stage::PrePush),
    };
    if stripped != text {
        std::fs::write(&path, &stripped).map_err(|error| HooksError::Io {
            path: path.clone(),
            message: format!("cannot write lefthook config: {error}"),
        })?;
    }
    Ok(uninstall)
}

/// The lefthook config text at `path`, refusing an unreadable file.
fn read_config(path: &Path) -> Result<String, HooksError> {
    std::fs::read_to_string(path).map_err(|error| HooksError::Io {
        path: path.to_path_buf(),
        message: format!("cannot read lefthook config: {error}"),
    })
}

/// What one pinned stage became: the stage unwired when it carries the
/// sad managed block, otherwise never wired.
fn uninstall_outcome(text: &str, stage: Stage) -> UninstallOutcome {
    if stage_has_block(text, stage.key()) {
        UninstallOutcome::Removed
    } else {
        UninstallOutcome::NotWired
    }
}

/// Strip every balanced `sad:managed` block, preserving every other byte.
///
/// Markers are matched exactly, so foreign managed blocks are never
/// touched. A nested start marker, a stray end marker, or an unterminated
/// block refuses the whole edit: uninstall never guesses where an
/// unterminated block ends.
fn strip_sad_blocks(text: &str) -> Result<String, HooksError> {
    let unbalanced = || HooksError::Framework {
        message: "unbalanced sad:managed markers in the lefthook config: sad hooks uninstall \
                  removes only complete marker-guarded blocks and refuses to guess"
            .into(),
    };
    let mut stripped = String::with_capacity(text.len());
    let mut in_block = false;
    for line in text.split_inclusive('\n') {
        match line.trim_end() {
            MANAGED_START if !in_block => in_block = true,
            MANAGED_START => return Err(unbalanced()),
            MANAGED_END if in_block => in_block = false,
            MANAGED_END => return Err(unbalanced()),
            _ if in_block => {}
            _ => stripped.push_str(line),
        }
    }
    if in_block {
        return Err(unbalanced());
    }
    Ok(stripped)
}

/// Refuse a config whose balanced sad block sits outside both pinned
/// stages.
///
/// Strip is global while outcomes are stage-scoped: a hand-edited block
/// outside pre-commit/pre-push would otherwise mutate the file while
/// every stage reports `not_wired`. Refusing mirrors the unbalanced
/// marker convention — uninstall never guesses ownership.
fn ensure_blocks_are_pinned(text: &str) -> Result<(), HooksError> {
    let staged = STAGES
        .iter()
        .filter(|stage| stage_has_block(text, stage.key()))
        .count();
    let total = text
        .lines()
        .filter(|line| line.trim_end() == MANAGED_START)
        .count();
    if staged == total {
        return Ok(());
    }
    Err(HooksError::Framework {
        message: "sad:managed block count does not match the pre-commit/pre-push stages: sad \
                  hooks uninstall unwires exactly one block per pinned stage and refuses to \
                  guess ownership"
            .into(),
    })
}

/// Whether the stage's section carries the sad managed block, matched as
/// an exact trimmed line.
///
/// That is the same predicate the strip and the guard use, so detection and
/// removal can never disagree, and the `sad doctor` wiring check reads
/// wiring the same way. A missing stage yields `false`, because
/// there is nothing to unwire.
pub(crate) fn stage_has_block(content: &str, stage_key: &str) -> bool {
    stage_section(content, stage_key)
        .is_some_and(|section| section.lines().any(|line| line.trim_end() == MANAGED_START))
}

/// The lefthook config genesis would wire, when one exists, as a path.
///
/// Mirrors genesis's private `config_path` precedence (`lefthook.yml` wins
/// over `lefthook.yaml`) so the guard, the injection, the unwiring, and
/// the `sad doctor` wiring check always read the same file.
pub(crate) fn config_path(root: &Path) -> Option<PathBuf> {
    ["lefthook.yml", "lefthook.yaml"]
        .iter()
        .map(|name| root.join(name))
        .find(|path| path.is_file())
}

/// The lefthook config text genesis would wire, when one exists.
pub(crate) fn config_text(root: &Path) -> Option<String> {
    config_path(root).and_then(|path| std::fs::read_to_string(path).ok())
}

/// Whether the stage's section already invokes the gate outside the
/// managed block (espectacular GH#47): genesis's idempotence matches only
/// the exact block text, so a user-authored `run: sad check` would gain a
/// duplicate without this scan.
///
/// The section spans the stage's column-0 `key:` line to the next
/// column-0 key; comment lines, including our own markers, do not end it,
/// so a gate below the block is still seen. Lines between the markers are
/// our own block and never count as user gates. A missing stage returns
/// false, because its wiring proceeds.
fn stage_runs_gate(content: &str, stage_key: &str) -> bool {
    let Some(section) = stage_section(content, stage_key) else {
        return false;
    };
    let mut in_block = false;
    for line in section.lines().skip(1) {
        if line.trim_end() == MANAGED_START {
            in_block = true;
            continue;
        }
        if in_block {
            in_block = line.trim_end() != MANAGED_END;
            continue;
        }
        if line.contains("sad check") {
            return true;
        }
    }
    false
}

/// The stage's section including its `key:` line: from the key to the
/// next column-0 key line.
///
/// Column-0 comments are not keys (mirrors genesis's
/// `is_column_zero_key`), so injected markers stay inside the section; a
/// missing stage yields `None`.
fn stage_section<'a>(content: &'a str, stage_key: &str) -> Option<&'a str> {
    let start = if content.starts_with(stage_key) {
        0
    } else {
        content.find(&format!("\n{stage_key}:"))? + 1
    };
    let rest = &content[start..];
    let mut end = rest.len();
    let mut scanned = 0;
    for line in rest.split_inclusive('\n').skip(1) {
        if is_column_zero_key(line) {
            end = scanned;
            break;
        }
        scanned += line.len();
    }
    Some(&rest[..end.min(rest.len())])
}

/// True for a column-0 YAML key: first char is not whitespace or `#`, and
/// the line is not an HTML-comment marker.
///
/// Mirrors genesis's `is_column_zero_key` so the guard reads sections
/// exactly as `ensure_wired` anchors them.
fn is_column_zero_key(line: &str) -> bool {
    matches!(
        line.chars().next(),
        Some(c) if c != ' ' && c != '\t' && c != '#' && c != '\n'
    ) && !line.starts_with("<!--")
}
