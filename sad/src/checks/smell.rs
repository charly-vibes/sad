//! Purpose: assign the deterministic smell and severity a shared post-pass
//! gives every prose-derived finding.
//! Responsibilities: group each analysed file's finding-eligible prose nodes
//! into maximal adjacent runs exactly as `commented_out_code` does, classify
//! each run once (banner, todo, apology, narration, unclassified), apply the
//! smell and severity to every member node's `comment_outside_channel`
//! finding, and reclassify separator-bodied `commented_out_code` findings as
//! banners without touching any finding id, line, symbol path, or text.
//! Rationale: a multi-line comment is one thought, so judging it per node
//! splits one narration across several findings; one post-pass over the runs
//! keeps the decision deterministic and leaves each finding's stable identity
//! intact.

use std::collections::BTreeSet;

use regex::Regex;

use crate::attribution::leading_marker_id;
use crate::checks::RepoContext;
use crate::config::Config;
use crate::model::{
    AnalyzedFile, AttributionClass, CommentNode, Finding, FindingKind, Severity, Smell, finding_id,
};
use crate::remediation;

/// Words too generic to prove a comment restates the next statement.
const STOPWORDS: &[&str] = &[
    "this", "that", "with", "from", "then", "when", "where", "which", "will", "have", "into",
    "over", "under", "about", "after", "before", "these", "those", "their", "there", "function",
    "value", "return", "create", "make", "call", "line", "the", "and", "for", "not", "are", "was",
    "were", "has", "had", "our", "your", "its", "per", "via", "each", "some", "such", "only",
    "also", "more", "most", "much", "many", "any", "all", "can", "may", "must", "should", "would",
    "could",
];

/// Config, compiled marker pattern, and ledger context for one classify run.
///
/// Grouped so the todo/apology predicates take one argument instead of three
/// parallel ones, and the marker pattern is compiled exactly once per run.
struct MarkerContext<'a> {
    cfg: &'a Config,
    marker_pattern: Option<Regex>,
    context: &'a RepoContext,
}

impl<'a> MarkerContext<'a> {
    fn new(cfg: &'a Config, context: &'a RepoContext) -> Self {
        Self {
            cfg,
            marker_pattern: Regex::new(&cfg.marker_pattern).ok(),
            context,
        }
    }
}

/// Classify every prose run and block finding once, in place.
///
/// Exempt files are skipped, the configured marker pattern is compiled once
/// for the whole run, and only `comment_outside_channel` and
/// `commented_out_code` findings are rewritten; structural findings keep
/// their emitted smell, severity, and remediation.
pub fn classify(
    analyzed: &[AnalyzedFile],
    findings: &mut [Finding],
    cfg: &Config,
    context: &RepoContext,
) {
    let marker = MarkerContext::new(cfg, context);
    for file in analyzed.iter().filter(|file| !file.exempt) {
        for run in prose_runs(&file.nodes) {
            let smell = smell_of(&file.nodes, &run, &marker);
            let ids = run_finding_ids(file, &run);
            apply_smell_to_run(findings, &ids, smell);
        }
    }
    classify_block_findings(findings);
}

/// Stamp one run's resolved smell, severity, and remediation on its members.
///
/// Membership is keyed on stable finding ids, not path+line: two single-node
/// runs can share a row, and a line-keyed stamp would let the last run's
/// verdict overwrite its neighbour's.
fn apply_smell_to_run(findings: &mut [Finding], ids: &BTreeSet<String>, smell: Smell) {
    let severity = severity_of(smell);
    for finding in findings.iter_mut().filter(|finding| {
        finding.check == FindingKind::CommentOutsideChannel && ids.contains(&finding.id)
    }) {
        finding.smell = Some(smell);
        finding.severity = severity;
        finding.remediation = remediation::for_smell(smell, &finding.path);
    }
}

/// Stable finding ids of one run's member nodes, for the stamp lookup.
fn run_finding_ids(file: &AnalyzedFile, run: &[usize]) -> BTreeSet<String> {
    run.iter()
        .map(|&index| {
            finding_id(
                &file.path,
                &file.nodes[index].symbol_path,
                &file.nodes[index].body,
            )
        })
        .collect()
}

/// Separator-bodied block findings become banners; every other block is kept.
fn classify_block_findings(findings: &mut [Finding]) {
    let blocks = findings
        .iter_mut()
        .filter(|finding| finding.check == FindingKind::CommentedOutCode);
    for finding in blocks.filter(|finding| banner(&finding.text)) {
        finding.smell = Some(Smell::Banner);
        finding.severity = Severity::Warning;
        finding.remediation = remediation::for_smell(Smell::Banner, &finding.path);
    }
}

/// Maximal runs of adjacent finding-eligible prose nodes, as node indices.
fn prose_runs(nodes: &[CommentNode]) -> Vec<Vec<usize>> {
    let mut runs = Vec::new();
    let mut current = Vec::new();
    for (index, node) in nodes.iter().enumerate() {
        if !eligible(node) {
            flush(&mut runs, &mut current);
            continue;
        }
        let joins = current
            .last()
            .is_some_and(|last| joins(&nodes[*last], node));
        if !joins {
            flush(&mut runs, &mut current);
        }
        current.push(index);
    }
    flush(&mut runs, &mut current);
    runs
}

/// Finding eligibility is exactly the outside-channel rule: prose with a body.
fn eligible(node: &CommentNode) -> bool {
    node.class == AttributionClass::Prose && !node.body.trim().is_empty()
}

/// No row gap and one enclosing symbol: `next.line <= prev.line + span`.
fn joins(previous: &CommentNode, next: &CommentNode) -> bool {
    previous.symbol_path == next.symbol_path
        && next.line <= previous.line + previous.text.lines().count()
}

fn flush(runs: &mut Vec<Vec<usize>>, current: &mut Vec<usize>) {
    if !current.is_empty() {
        runs.push(std::mem::take(current));
    }
}

fn smell_of(nodes: &[CommentNode], run: &[usize], marker: &MarkerContext<'_>) -> Smell {
    let body = run
        .iter()
        .map(|index| nodes[*index].body.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    if banner(&body) {
        return Smell::Banner;
    }
    if todo(&body, marker) {
        return Smell::Todo;
    }
    if apology(&body, marker) {
        return Smell::Apology;
    }
    let next = run
        .last()
        .and_then(|index| nodes[*index].next_code_line.as_deref());
    if narration(&body, next) {
        return Smell::Narration;
    }
    Smell::Unclassified
}

fn severity_of(smell: Smell) -> Severity {
    match smell {
        Smell::Narration | Smell::Apology | Smell::Todo | Smell::CommentedOutCode => {
            Severity::Error
        }
        Smell::Banner | Smell::DocCap | Smell::Unclassified => Severity::Warning,
    }
}

/// A body with no alphanumerics carries no information: separators and rules.
fn banner(body: &str) -> bool {
    let trimmed = body.trim();
    !trimmed.is_empty() && !trimmed.chars().any(char::is_alphanumeric)
}

fn todo(body: &str, marker: &MarkerContext<'_>) -> bool {
    let tagged = body
        .split(|next: char| !next.is_alphanumeric())
        .any(|word| matches!(word, "TODO" | "FIXME" | "XXX"));
    tagged && !has_resolving_marker(body, marker)
}

fn apology(body: &str, marker: &MarkerContext<'_>) -> bool {
    if has_resolving_marker(body, marker) {
        return false;
    }
    let lower = body.to_lowercase();
    if contains_phrase(&lower, "for now")
        || contains_phrase(&lower, "workaround")
        || contains_phrase(&lower, "legacy shim")
        || contains_phrase(&lower, "to be removed")
        || hack_before_colon(&lower)
    {
        return true;
    }
    let words = split_words(&lower);
    words.windows(2).any(|pair| {
        pair[0] == "temporary"
            && matches!(pair[1], "fix" | "hack" | "workaround" | "shim" | "patch")
    })
}

/// Whether any marker-shaped `prefix` + id in `body` resolves against a ledger.
fn has_resolving_marker(body: &str, marker: &MarkerContext<'_>) -> bool {
    let Some(regex) = marker.marker_pattern.as_ref() else {
        return false;
    };
    let prefix = marker.cfg.marker_prefix.as_str();
    if prefix.is_empty() || !marker.context.any_resolver_available() {
        return false;
    }
    body.match_indices(prefix).any(|(start, _)| {
        let boundary = body[..start]
            .chars()
            .next_back()
            .is_none_or(|previous| !previous.is_alphanumeric());
        boundary
            && leading_marker_id(&body[start..], prefix, regex)
                .is_some_and(|id| marker.context.resolves(id))
    })
}

/// Case-folded `phrase` bounded by non-alphanumerics at both ends.
fn contains_phrase(text: &str, phrase: &str) -> bool {
    text.match_indices(phrase).any(|(start, _)| {
        let before = text[..start].chars().next_back();
        let after = text[start + phrase.len()..].chars().next();
        before.is_none_or(|next| !next.is_alphanumeric())
            && after.is_none_or(|next| !next.is_alphanumeric())
    })
}

/// `hack` as a whole word, followed only by spaces and then a colon.
fn hack_before_colon(text: &str) -> bool {
    text.match_indices("hack").any(|(start, _)| {
        let before = text[..start].chars().next_back();
        if before.is_some_and(|next| next.is_alphanumeric()) {
            return false;
        }
        text[start + "hack".len()..].trim_start().starts_with(':')
    })
}

fn split_words(text: &str) -> Vec<&str> {
    text.split(|next: char| !next.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect()
}

fn narration(body: &str, next: Option<&str>) -> bool {
    let Some(statement) = next else {
        return false;
    };
    let comment_tokens = distinctive_tokens(body);
    if comment_tokens.is_empty() {
        return false;
    }
    let statement_tokens = distinctive_tokens(statement);
    let matches = comment_tokens
        .iter()
        .filter(|token| {
            statement_tokens
                .iter()
                .any(|other| same_token(token.as_str(), other.as_str()))
        })
        .count();
    if matches >= 2 {
        return true;
    }
    matches >= 1 && statement.contains('(') && sequencer(body)
}

/// Lowercased alphanumeric words of length at least four, minus stopwords.
fn distinctive_tokens(text: &str) -> BTreeSet<String> {
    text.split(|next: char| !next.is_alphanumeric())
        .filter(|word| word.chars().count() >= 4)
        .map(str::to_lowercase)
        .filter(|word| !STOPWORDS.contains(&word.as_str()))
        .collect()
}

/// Equal, or sharing a prefix of at least four characters.
fn same_token(left: &str, right: &str) -> bool {
    left == right
        || left
            .chars()
            .zip(right.chars())
            .take_while(|(left, right)| left == right)
            .count()
            >= 4
}

/// The run's first word, ignoring surrounding punctuation, is a sequencer.
fn sequencer(body: &str) -> bool {
    let Some(first) = body.split_whitespace().next() else {
        return false;
    };
    let word: String = first
        .chars()
        .skip_while(|next| !next.is_alphanumeric())
        .take_while(|next| next.is_alphanumeric())
        .collect::<String>()
        .to_lowercase();
    matches!(word.as_str(), "now" | "then" | "first" | "next" | "finally")
}
