//! Purpose: flag comment nodes attributed to the prose channel — commentary
//! that sits outside every documented channel (header, doc, marker,
//! suppression, exemption).
//! Responsibilities: emit one `comment_outside_channel` finding per
//! finding-eligible prose node — a body that is empty or whitespace-only is
//! not prose — anchored to the node's stable (path, symbol, body) identity
//! and carrying the node body as text, in source order, honouring the
//! `comment_outside_channel` toggle and staying silent for exempt files.
//! Rationale: narration inside a body is invisible to documentation readers;
//! keeping the rule purely structural means the check never guesses at intent
//! or prescribes a refactor, and reusing the node's own identity keeps
//! findings stable under line drift while staying sensitive to edits.

use crate::config::Config;
use crate::model::{AnalyzedFile, AttributionClass, Finding, FindingKind, finding_id};
use crate::remediation;

/// Message shared by every outside-channel finding: name the rule and the
/// documented channels, never a required edit.
const MESSAGE: &str = "comment is attributed to the prose channel, which is not one of the \
                       documented channels (header, doc, marker, suppression, exemption)";

/// Report every finding-eligible prose-class node in `file` as a
/// `comment_outside_channel` finding, in source order.
///
/// Pure: no filesystem, git, or clock access; an exempt file or a disabled
/// toggle yields an empty vector, and an unanalysed file (no nodes) can never
/// panic. A bare comment separator (`//`, `#`, an empty JSDoc) carries no
/// narration, so an empty or whitespace-only body is never a finding.
pub fn run(file: &AnalyzedFile, cfg: &Config) -> Vec<Finding> {
    if file.exempt || !cfg.checks.comment_outside_channel {
        return Vec::new();
    }
    file.nodes
        .iter()
        .filter(|node| node.class == AttributionClass::Prose && !node.body.trim().is_empty())
        .map(|node| Finding {
            check: FindingKind::CommentOutsideChannel,
            id: finding_id(&file.path, &node.symbol_path, &node.body),
            path: file.path.clone(),
            line: node.line,
            symbol_path: node.symbol_path.clone(),
            message: MESSAGE.to_string(),
            text: node.body.clone(),
            remediation: remediation::for_kind(FindingKind::CommentOutsideChannel, &file.path),
        })
        .collect()
}
