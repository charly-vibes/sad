//! Purpose: hold the hardcoded, per-kind remediation guidance every finding
//! carries in the check envelope.
//! Responsibilities: map each [`FindingKind`] to exactly one stable
//! [`RemediationEntry`] (an actionable command plus a human description), and
//! keep that guidance deterministic and free of runtime state.
//! Rationale: remediation is part of the machine contract — the spec requires
//! every finding to name a fix without prescribing a specific refactor — so
//! the text lives in one table here rather than being composed at each check
//! site; identical inputs must always produce identical commands.

use genesis::envelope::RemediationEntry;

use crate::model::FindingKind;

/// The single hardcoded remediation entry for `kind`.
///
/// Commands use `<path>` as the only placeholder (the anchor file the caller
/// substitutes); descriptions are per kind so a reader can tell the fixes
/// apart. No clock, network, configuration, or finding data is consulted.
pub fn for_kind(kind: FindingKind) -> RemediationEntry {
    let (command, description) = match kind {
        FindingKind::CommentOutsideChannel => (
            "$EDITOR <path>",
            "Move the comment into a documented channel (header, doc, marker, or \
             suppression) or delete it; code should speak for itself.",
        ),
        FindingKind::DocSummaryCapExceeded => (
            "$EDITOR <path>",
            "Shorten the summary lead so it fits the doc summary cap, and keep structured \
             sections after a blank line so they do not count against it.",
        ),
        FindingKind::UnresolvableMarker => (
            "$EDITOR <path>",
            "Resolve the cited id against the repository ledger (the beads issue or openspec \
             spec it names) or remove the citation.",
        ),
        FindingKind::MissingHeader => (
            "$EDITOR <path>",
            "Add a header to the file's first block naming Purpose, Responsibilities, and \
             Rationale.",
        ),
        FindingKind::SuppressionCapExceeded => (
            "$EDITOR <path>",
            "Reduce suppression debt by fixing the underlying finding or removing the \
             directive that hides it.",
        ),
    };
    RemediationEntry {
        command: command.to_string(),
        description: description.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_has_distinct_non_empty_guidance() {
        let kinds = [
            FindingKind::CommentOutsideChannel,
            FindingKind::DocSummaryCapExceeded,
            FindingKind::UnresolvableMarker,
            FindingKind::MissingHeader,
            FindingKind::SuppressionCapExceeded,
        ];
        let mut descriptions = std::collections::BTreeSet::new();
        for kind in kinds {
            let entry = for_kind(kind);
            assert!(!entry.command.trim().is_empty(), "{kind:?} needs a command");
            assert!(
                !entry.description.trim().is_empty(),
                "{kind:?} needs a description"
            );
            assert!(
                descriptions.insert(entry.description.clone()),
                "{kind:?} must not share another kind's description"
            );
            assert!(
                !entry
                    .command
                    .replace("<path>", "")
                    .chars()
                    .any(|character| character == '<' || character == '>'),
                "{kind:?} may only use a <path> placeholder"
            );
        }
    }
}
