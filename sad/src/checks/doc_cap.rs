//! Purpose: flag documentation summaries whose lead exceeds the configured
//! `doc_summary_cap` — the `doc_summary_cap_exceeded` check.
//! Responsibilities: reassemble consecutive doc nodes that document the same
//! item (equal `symbol_path`) into one documentation block regardless of
//! blank source lines or attributes between them, measure the summary lead
//! (the non-empty lines before the first blank line), emit one finding per
//! oversized block anchored to the block's first node and its stable
//! (path, symbol, body) identity, and stay silent for exempt files.
//! Rationale: the summary is what a reader sees before the first blank line,
//! so structured sections after it must never count against the cap. A Rust
//! `///` block arrives as one node per line while a Python docstring arrives
//! as a single node, and rustdoc keeps one item's documentation together
//! across blank lines and intervening attributes, so the block is keyed on
//! the documented item rather than on source-line adjacency; independent
//! blocks on different items carry different symbols and never merge.

use crate::config::Config;
use crate::model::{AnalyzedFile, AttributionClass, CommentNode, Finding, FindingKind, finding_id};
use crate::remediation;

/// Run the documentation-summary cap over every doc block in `file`.
///
/// Pure: no filesystem, git, or clock access; empty bodies count zero lead
/// lines, an exempt file or a disabled toggle yields an empty vector, and
/// files without doc nodes produce no findings.
pub fn run(file: &AnalyzedFile, cfg: &Config) -> Vec<Finding> {
    if file.exempt || !cfg.checks.doc_summary_cap_exceeded {
        return Vec::new();
    }
    let cap = cfg.doc_summary_cap;
    let nodes = &file.nodes;
    let mut findings = Vec::new();
    let mut start = 0;
    while start < nodes.len() {
        if nodes[start].class != AttributionClass::Doc {
            start += 1;
            continue;
        }
        let end = block_end(nodes, start);
        let block = &nodes[start..end];
        let lead = lead_lines(block);
        if lead > cap {
            let node = &block[0];
            let body = block_body(block);
            findings.push(Finding {
                check: FindingKind::DocSummaryCapExceeded,
                id: finding_id(&file.path, &node.symbol_path, &body),
                path: file.path.clone(),
                line: node.line,
                symbol_path: node.symbol_path.clone(),
                message: format!(
                    "doc summary lead spans {lead} lines, exceeding the {cap}-line doc_summary_cap"
                ),
                text: body,
                remediation: remediation::for_kind(FindingKind::DocSummaryCapExceeded),
            });
        }
        start = end;
    }
    findings
}

/// Exclusive end of the documentation block starting at `start`: the run of
/// consecutive doc-class nodes that share the first node's `symbol_path`.
///
/// The documented item — not the source-line number — defines the block, so
/// blank lines and attribute items between `///` lines neither split it nor
/// merge docs that document different items. `end` is always greater than
/// `start`, so the caller's scan always advances.
fn block_end(nodes: &[CommentNode], start: usize) -> usize {
    let symbol = nodes[start].symbol_path.as_str();
    let mut end = start + 1;
    while end < nodes.len()
        && nodes[end].class == AttributionClass::Doc
        && nodes[end].symbol_path.as_str() == symbol
    {
        end += 1;
    }
    end
}

/// Join one doc block's node bodies with `\n`, restoring the line structure
/// that a per-line Rust doc block loses during extraction.
///
/// Only called when a finding is emitted, so the clean path allocates nothing.
fn block_body(block: &[CommentNode]) -> String {
    let mut body = String::new();
    for (offset, node) in block.iter().enumerate() {
        if offset > 0 {
            body.push('\n');
        }
        body.push_str(&node.body);
    }
    body
}

/// Number of non-empty lines before the first blank line: the summary lead,
/// counted across the block's node bodies without joining them.
///
/// A body with no blank line is all lead; an empty body has none. A line that
/// is only `*` after trimming counts as blank, so a block body that somehow
/// retains its block-comment gutter still ends its summary where a reader
/// would see the blank separator.
fn lead_lines(block: &[CommentNode]) -> usize {
    let mut lead = 0;
    for node in block {
        for line in node.body.split('\n') {
            let line = line.trim();
            if line.is_empty() || line == "*" {
                return lead;
            }
            lead += 1;
        }
    }
    lead
}
