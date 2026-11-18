//! Purpose: require the dogfood file header — the `missing_header` check that
//! keeps `Purpose`, `Responsibilities`, and `Rationale` in a file's first
//! comment/docstring block.
//! Responsibilities: no-op unless `require_header` and the `missing_header`
//! toggle are on, take the first contiguous comment block from the analyzed
//! node set (skipping leading licence/generated exemption nodes), test it for
//! all three keywords order-free as line-leading labels, and emit exactly one
//! file-scoped finding naming the missing keywords.
//! Rationale: the first block is the only header position; keywords appearing
//! in later blocks cannot retroactively satisfy it, and a keyword-based
//! order-free test keeps the mandated prose natural instead of dictating a
//! template. Two grouping rules keep that test honest: block membership is
//! measured by each node's *end* line (start line plus the newlines embedded
//! in its raw text), so a multi-line block comment and the line comment
//! directly beneath it are one block; and a keyword counts only when it opens
//! a line of the block (leading whitespace ignored) and is not followed by an
//! identifier character (ASCII alphanumeric or `_`), so `Purposeful` cannot
//! stand in for `Purpose` and a keyword the prose merely mentions inline —
//! say, in backticks — is not a header claim. A leading SPDX banner is
//! licence metadata, not prose, so Exemption-class nodes are skipped before
//! the block starts (the positional header channel alone is deliberately not
//! the predicate). The check is pure: no filesystem, git, or clock access.

use crate::config::Config;
use crate::model::{
    AnalyzedFile, AttributionClass, CommentNode, Finding, FindingKind, Severity, finding_id,
};
use crate::remediation;

/// Header keywords every first block must name, in canonical order.
const REQUIRED_KEYWORDS: [&str; 3] = ["Purpose", "Responsibilities", "Rationale"];

/// Report `file` when the header gate is on and its first block lacks a
/// required keyword.
///
/// An empty node set (or a set holding only exemption nodes) means an empty
/// first block, which fails the keyword test without panicking; the finding
/// stays file-scoped (`-` symbol path) and anchored to line 1, and its stable
/// id hashes the first block's body, or the empty string when there is no
/// block.
pub fn run(file: &AnalyzedFile, cfg: &Config) -> Vec<Finding> {
    if !cfg.require_header || !cfg.checks.missing_header {
        return Vec::new();
    }

    let block = first_block(&file.nodes);
    let body = block_body(block);
    let missing: Vec<&str> = REQUIRED_KEYWORDS
        .into_iter()
        .filter(|keyword| !keyword_present(&body, keyword))
        .collect();
    if missing.is_empty() {
        return Vec::new();
    }

    vec![Finding {
        check: FindingKind::MissingHeader,
        id: finding_id(&file.path, "-", &body),
        path: file.path.clone(),
        // A missing header is a property of the file's leading area, so the
        // finding stays anchored to line 1 even when the inspected block
        // starts below a skipped licence banner.
        line: 1,
        symbol_path: "-".to_string(),
        message: format!(
            "file header is missing required keyword(s): {}",
            missing.join(", ")
        ),
        text: body,
        severity: Severity::Error,
        smell: None,
        remediation: remediation::for_kind(FindingKind::MissingHeader, &file.path),
    }]
}

/// The file's first header block: the leading contiguous run of comment nodes
/// after any leading [`AttributionClass::Exemption`] nodes (licence banners,
/// generated-code markers) are skipped.
///
/// Two nodes are contiguous when the next one starts on the line after the
/// previous one *ends*; a multi-line block comment ends at
/// `line + text.matches('\n').count()`, so its later lines do not break the
/// block. Keywords after the first gap sit in a later block and cannot
/// satisfy the header requirement.
fn first_block(nodes: &[CommentNode]) -> &[CommentNode] {
    let Some(start) = nodes
        .iter()
        .position(|node| node.class != AttributionClass::Exemption)
    else {
        return &[];
    };

    let mut end = start + 1;
    while end < nodes.len() && adjacent(&nodes[end - 1], &nodes[end]) {
        end += 1;
    }
    &nodes[start..end]
}

/// Whether `next` begins on the source line after `previous` ends.
fn adjacent(previous: &CommentNode, next: &CommentNode) -> bool {
    next.line == end_line(previous) + 1
}

/// One-based source line on which `node` ends.
fn end_line(node: &CommentNode) -> usize {
    node.line + node.text.matches('\n').count()
}

/// Join one block's node bodies with `\n`, restoring the line structure a
/// per-line Rust doc block loses during extraction. Exemption nodes carried
/// inside the run contribute no prose, so their bodies are skipped.
fn block_body(block: &[CommentNode]) -> String {
    let mut body = String::new();
    let mut first = true;
    for node in block
        .iter()
        .filter(|node| node.class != AttributionClass::Exemption)
    {
        if !first {
            body.push('\n');
        }
        first = false;
        body.push_str(&node.body);
    }
    body
}

/// True when `body` opens one of its lines with `keyword`.
///
/// A header keyword is a label: it must be the first thing on a body line
/// (leading whitespace ignored) and must not be followed by an identifier
/// character (ASCII alphanumeric or `_`). Both halves matter: `Purposeful`,
/// `ResponsibilitiesMap`, and `RationaleFree` are not the keywords, and prose
/// that merely mentions a keyword inline (for example in backticks) is not a
/// header claim. The comparison stays case-sensitive.
fn keyword_present(body: &str, keyword: &str) -> bool {
    body.lines().any(|line| {
        let Some(rest) = line.trim_start().strip_prefix(keyword) else {
            return false;
        };
        !rest.chars().next().is_some_and(is_identifier_char)
    })
}

/// Character class that glues a keyword to an identifier: ASCII letters,
/// digits, and underscore.
fn is_identifier_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}
