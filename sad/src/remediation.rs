//! Purpose: hold the per-kind remediation guidance every finding carries in
//! the check envelope.
//! Responsibilities: map each [`FindingKind`] to exactly one stable
//! [`RemediationEntry`] (an actionable command plus a human description),
//! rendering the finding's repo-relative anchor path into the command so what
//! the envelope carries is runnable as emitted.
//! Rationale: remediation is part of the machine contract — the spec requires
//! every finding to name a fix without prescribing a specific refactor — so
//! the descriptions live in one table here rather than being composed at each
//! check site; identical inputs must always produce identical commands.

use genesis::envelope::RemediationEntry;

use crate::model::FindingKind;

/// The hardcoded remediation entry for `kind`, anchored to `path`.
///
/// `path` is the finding's repo-relative anchor, rendered into the command so
/// no placeholder survives into the envelope. Descriptions are per kind so a
/// reader can tell the fixes apart. No clock, network, configuration, or other
/// finding data is consulted.
pub fn for_kind(kind: FindingKind, path: &str) -> RemediationEntry {
    let command = match kind {
        FindingKind::CommentedOutCode => format!("git log -p --follow {path}"),
        _ => format!("$EDITOR {path}"),
    };
    RemediationEntry {
        command,
        description: description(kind).to_string(),
    }
}

/// The human description of what fixing `kind` means; never a specific
/// refactor, only the rule and the lawful outcomes.
fn description(kind: FindingKind) -> &'static str {
    match kind {
        FindingKind::CommentOutsideChannel => {
            "Move the comment into a documented channel (header, doc, marker, or \
             suppression) or delete it; code should speak for itself."
        }
        FindingKind::CommentedOutCode => {
            "Delete the commented-out code. If it is still needed, recover it from VCS history; \
             never restore it as a comment."
        }
        FindingKind::DocSummaryCapExceeded => {
            "Shorten the summary lead so it fits the doc summary cap, and keep structured \
             sections after a blank line so they do not count against it."
        }
        FindingKind::UnresolvableMarker => {
            "Resolve the cited id against the repository ledger (the beads issue or openspec \
             spec it names) or remove the citation."
        }
        FindingKind::MissingHeader => {
            "Add a header to the file's first block naming Purpose, Responsibilities, and \
             Rationale."
        }
        FindingKind::SuppressionCapExceeded => {
            "Reduce suppression debt by fixing the underlying finding or removing the \
             directive that hides it."
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commented_out_code_remediation_is_the_vcs_hatch() {
        let entry = for_kind(FindingKind::CommentedOutCode, "src/lib.rs");
        assert_eq!(entry.command, "git log -p --follow src/lib.rs");
        assert_eq!(
            entry.description,
            "Delete the commented-out code. If it is still needed, recover it from VCS \
             history; never restore it as a comment."
        );
    }

    #[test]
    fn every_kind_has_distinct_non_empty_guidance() {
        let kinds = [
            FindingKind::CommentOutsideChannel,
            FindingKind::CommentedOutCode,
            FindingKind::DocSummaryCapExceeded,
            FindingKind::UnresolvableMarker,
            FindingKind::MissingHeader,
            FindingKind::SuppressionCapExceeded,
        ];
        let mut descriptions = std::collections::BTreeSet::new();
        for kind in kinds {
            let entry = for_kind(kind, "src/lib.rs");
            let command = match kind {
                FindingKind::CommentedOutCode => "git log -p --follow src/lib.rs",
                _ => "$EDITOR src/lib.rs",
            };
            assert_eq!(
                entry.command, command,
                "{kind:?} must render the anchor path"
            );
            assert!(
                !entry.description.trim().is_empty(),
                "{kind:?} needs a description"
            );
            assert!(
                descriptions.insert(entry.description.clone()),
                "{kind:?} must not share another kind's description"
            );
        }
    }
}
