//! Purpose: report commented-out code as its own block-scoped finding —
//! `commented_out_code` — instead of one prose finding per comment node.
//! Responsibilities: group finding-eligible prose nodes into maximal adjacent
//! runs, normalise each run to one body, test the body against the pinned
//! grammar predicate, emit one finding anchored at the run's first node, and
//! return the node indices the group consumed so `comment_outside_channel`
//! stays silent for exactly those nodes.
//! Rationale: dead code was already a finding; only its identity and
//! remediation were wrong. Block scoping keeps a twelve-line dead block one
//! finding, and reusing the extractor's grammar wiring keeps the predicate on
//! the same pinned frontend that attributed the nodes. A predicate miss or a
//! disabled toggle leaves per-node prose untouched, so the class never buys
//! silence by guessing.

use tree_sitter::Parser;

use crate::config::Config;
use crate::extract::parser_for;
use crate::model::{
    AnalyzedFile, AttributionClass, CommentNode, Finding, FindingKind, Language, finding_id,
};
use crate::remediation;

use self::normalize::normalise_block;
use self::predicate::is_commented_out_code;

mod normalize;
mod predicate;

/// Message shared by every block finding.
///
/// Names the rule; never a required edit beyond deleting code that was
/// already decided against.
const MESSAGE: &str = "comment block parses as commented-out code; delete it and recover it \
                       from VCS history if it is still needed";

/// Reusable parse state for one gate run: one configured parser per language.
///
/// Constructed once per run so grammar setup is a per-run cost, and reused
/// across files because `Parser::parse` resets incremental state.
pub struct Checker {
    parsers: [Option<Parser>; 3],
}

impl Checker {
    /// Build the run's one-parser-per-language state.
    pub fn new() -> Self {
        Self {
            parsers: [
                parser_for(Language::Rust),
                parser_for(Language::Python),
                parser_for(Language::TypeScript),
            ],
        }
    }

    /// Report every matched block in `file` and return its consumed indices.
    ///
    /// An exempt file or a disabled toggle consumes nothing, so the caller
    /// falls back to per-node prose. A body that normalises to nothing or
    /// fails the predicate is left to `comment_outside_channel` untouched.
    pub fn run(&mut self, file: &AnalyzedFile, cfg: &Config) -> (Vec<Finding>, Vec<usize>) {
        if file.exempt || !cfg.checks.commented_out_code {
            return (Vec::new(), Vec::new());
        }
        let mut findings = Vec::new();
        let mut consumed = Vec::new();
        for block in blocks(&file.nodes) {
            let nodes: Vec<&CommentNode> = block.iter().map(|index| &file.nodes[*index]).collect();
            let body = normalise_block(file.language, &nodes);
            if body.is_empty() || !self.parses_as_code(file.language, &body) {
                continue;
            }
            findings.push(finding(file, nodes[0], body));
            consumed.extend(block);
        }
        (findings, consumed)
    }

    /// Parse `body` with the pinned frontend and apply the static predicate.
    fn parses_as_code(&mut self, language: Language, body: &str) -> bool {
        let Some(parser) = self.parsers[parser_slot(language)].as_mut() else {
            return false;
        };
        let Some(tree) = parser.parse(body, None) else {
            return false;
        };
        is_commented_out_code(tree.root_node(), language)
    }
}

impl Default for Checker {
    fn default() -> Self {
        Self::new()
    }
}

/// One-shot convenience wrapper; a whole run reuses [`Checker`] instead.
pub fn run(file: &AnalyzedFile, cfg: &Config) -> (Vec<Finding>, Vec<usize>) {
    Checker::new().run(file, cfg)
}

/// Index of `language` in [`Checker::parsers`].
fn parser_slot(language: Language) -> usize {
    match language {
        Language::Rust => 0,
        Language::Python => 1,
        Language::TypeScript => 2,
    }
}

fn finding(file: &AnalyzedFile, anchor: &CommentNode, body: String) -> Finding {
    Finding {
        check: FindingKind::CommentedOutCode,
        id: finding_id(&file.path, &anchor.symbol_path, &body),
        path: file.path.clone(),
        line: anchor.line,
        symbol_path: anchor.symbol_path.clone(),
        message: MESSAGE.to_string(),
        text: body,
        remediation: remediation::for_kind(FindingKind::CommentedOutCode, &file.path),
    }
}

/// Maximal runs of adjacent finding-eligible prose nodes, as node indices.
///
/// Any non-eligible node — code, a blank comment, doc, header, marker, or a
/// suppression directive — flushes the run, and a line gap or a different
/// symbol path starts a new one. Same-row nodes join because the run means
/// "no row gap", not "one node per row".
fn blocks(nodes: &[CommentNode]) -> Vec<Vec<usize>> {
    let mut blocks = Vec::new();
    let mut current: Vec<usize> = Vec::new();
    for (index, node) in nodes.iter().enumerate() {
        if !eligible(node) {
            flush(&mut blocks, &mut current);
            continue;
        }
        let joins = current
            .last()
            .is_some_and(|last| joins(&nodes[*last], node));
        if !joins {
            flush(&mut blocks, &mut current);
        }
        current.push(index);
    }
    flush(&mut blocks, &mut current);
    blocks
}

/// Finding eligibility is exactly the outside-channel rule: prose with a body.
fn eligible(node: &CommentNode) -> bool {
    node.class == AttributionClass::Prose && !node.body.trim().is_empty()
}

/// No row gap and one enclosing symbol: `next.line <= prev.line + span`.
fn joins(previous: &CommentNode, next: &CommentNode) -> bool {
    next.symbol_path == previous.symbol_path
        && next.line <= previous.line + previous.text.lines().count()
}

fn flush(blocks: &mut Vec<Vec<usize>>, current: &mut Vec<usize>) {
    if !current.is_empty() {
        blocks.push(std::mem::take(current));
    }
}
