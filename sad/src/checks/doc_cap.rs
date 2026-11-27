//! Purpose: flag documentation summaries whose lead exceeds the configured
//! `doc_summary_cap` — the `doc_summary_cap_exceeded` check.
//! Responsibilities: reassemble consecutive doc nodes that document the same
//! item (equal `symbol_path`) into one documentation block regardless of
//! blank source lines or attributes between them, count the sentences of the
//! summary lead (the non-empty lines before the first blank line, with
//! inline code, URLs, abbreviations, and version tokens masked), emit one
//! finding per block whose count exceeds the cap, anchored to the block's
//! first node and its stable (path, symbol, body) identity, and stay silent
//! for exempt files.
//! Rationale: the summary is what a reader sees before the first blank line,
//! so structured sections after it must never count against the cap, and a
//! single sentence wrapped over several lines must pass. A Rust `///` block
//! arrives as one node per line while a Python docstring arrives as a single
//! node, and rustdoc keeps one item's documentation together across blank
//! lines and intervening attributes, so the block is keyed on the documented
//! item rather than on source-line adjacency; independent blocks on different
//! items carry different symbols and never merge. Masking runs as ordered
//! passes — inline code, URLs, abbreviations, version tokens — each blanking
//! its regions so the next pass reads only still-unmasked text.

use crate::config::Config;
use crate::model::{
    AnalyzedFile, AttributionClass, CommentNode, Finding, FindingKind, Severity, Smell, finding_id,
};
use crate::remediation;

/// Run the documentation-summary cap over every doc block in `file`.
///
/// Pure: no filesystem, git, or clock access; empty bodies leave an empty
/// summary lead, an exempt file or a disabled toggle yields an empty vector,
/// and files without doc nodes produce no findings.
pub fn run(file: &AnalyzedFile, cfg: &Config) -> Vec<Finding> {
    if file.exempt || !cfg.checks.doc_summary_cap_exceeded {
        return Vec::new();
    }
    let cap = cfg.doc_summary_cap;
    doc_blocks(&file.nodes)
        .iter()
        .filter_map(|block| {
            let count = sentence_count(&summary_lead(block));
            (count > cap).then(|| block_finding(file, block, cap))
        })
        .collect()
}

/// Every documentation block in `nodes`, as the slices of consecutive
/// doc-class nodes sharing their first node's `symbol_path`.
///
/// A block starts at a doc node whose predecessor is not a doc node naming
/// the same item, so the blocks partition the doc nodes exactly as the
/// advancing start/end scan did.
fn doc_blocks(nodes: &[CommentNode]) -> Vec<&[CommentNode]> {
    nodes
        .iter()
        .enumerate()
        .filter(|&(index, node)| {
            node.class == AttributionClass::Doc
                && !(index > 0
                    && nodes[index - 1].class == AttributionClass::Doc
                    && nodes[index - 1].symbol_path == node.symbol_path)
        })
        .map(|(index, _)| &nodes[index..block_end(nodes, index)])
        .collect()
}

/// The `doc_summary_cap_exceeded` finding for one doc block whose summary
/// lead exceeds a cap of `cap`.
fn block_finding(file: &AnalyzedFile, block: &[CommentNode], cap: usize) -> Finding {
    let node = &block[0];
    let body = block_body(block);
    let count = sentence_count(&summary_lead(block));
    Finding {
        check: FindingKind::DocSummaryCapExceeded,
        id: finding_id(&file.path, &node.symbol_path, &body),
        path: file.path.clone(),
        line: node.line,
        symbol_path: node.symbol_path.clone(),
        message: format!(
            "doc summary lead spans {count} sentences, exceeding the {cap}-sentence doc_summary_cap"
        ),
        text: body,
        severity: Severity::Warning,
        smell: Some(Smell::DocCap),
        remediation: remediation::for_smell(Smell::DocCap, &file.path),
    }
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
    start
        + 1
        + nodes[start + 1..]
            .iter()
            .take_while(|node| {
                node.class == AttributionClass::Doc && node.symbol_path.as_str() == symbol
            })
            .count()
}

/// Join one doc block's node bodies with `\n`, restoring the line structure
/// that a per-line Rust doc block loses during extraction.
///
/// Only called when a finding is emitted, so the clean path allocates nothing.
fn block_body(block: &[CommentNode]) -> String {
    block
        .iter()
        .map(|node| node.body.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The summary lead of a doc block: the non-empty body lines before the first
/// blank line, trimmed and joined with single spaces.
///
/// A body with no blank line is all lead; an empty body has none. A line that
/// is only `*` after trimming counts as blank, so a block body that somehow
/// retains its block-comment gutter still ends its summary where a reader
/// would see the blank separator.
fn summary_lead(block: &[CommentNode]) -> String {
    block
        .iter()
        .flat_map(|node| node.body.split('\n'))
        .map(str::trim)
        .take_while(|line| !line.is_empty() && *line != "*")
        .collect::<Vec<_>>()
        .join(" ")
}

/// Number of sentences in the summary lead, with inline code spans, URLs,
/// abbreviations, and version tokens masked before any punctuation is read.
///
/// Terminal punctuation is a boundary only when whitespace plus an uppercase
/// character, digit, or opening quote/bracket follows, or when the lead ends;
/// a non-empty lead always counts at least one sentence.
fn sentence_count(lead: &str) -> usize {
    if lead.trim().is_empty() {
        return 0;
    }
    let chars = mask_lead(lead);
    let count = (0..chars.len())
        .filter(|&index| {
            sentence_punct(chars[index]) && (index == 0 || !sentence_punct(chars[index - 1]))
        })
        .filter(|&index| boundary_after(&chars, index))
        .count();
    count.max(1)
}

/// Whether `c` is a terminal punctuation character.
fn sentence_punct(c: char) -> bool {
    matches!(c, '.' | '!' | '?')
}

/// Whether the punctuation run opening at `index` ends a sentence.
///
/// A boundary is whitespace plus an uppercase character, digit, or opening
/// quote/bracket, or the end of the lead.
fn boundary_after(chars: &[char], index: usize) -> bool {
    let end = punctuation_run_end(chars, index);
    let next = next_non_whitespace(chars, end);
    next == chars.len() || (next > end && starts_sentence(chars[next]))
}

/// End of the punctuation run that begins at `start`.
fn punctuation_run_end(chars: &[char], start: usize) -> usize {
    start
        + chars[start..]
            .iter()
            .take_while(|c| sentence_punct(**c))
            .count()
}

/// Index of the first non-whitespace character at or after `start`, or the
/// length of `chars` when only whitespace follows.
fn next_non_whitespace(chars: &[char], start: usize) -> usize {
    start
        + chars[start..]
            .iter()
            .position(|c| !c.is_whitespace())
            .unwrap_or(chars.len() - start)
}

/// Whether `c` can open the sentence that follows a boundary.
fn starts_sentence(c: char) -> bool {
    c.is_uppercase()
        || c.is_ascii_digit()
        || matches!(
            c,
            '"' | '\'' | '`' | '(' | '[' | '{' | '“' | '‘' | '«' | '‹' | '„'
        )
}

/// Mask the spans whose internal punctuation must never end a sentence:
/// inline code, URLs, abbreviations, and version tokens, in that order.
fn mask_lead(lead: &str) -> Vec<char> {
    let mut chars: Vec<char> = lead.chars().collect();
    mask_code_spans(&mut chars);
    mask_urls(&mut chars);
    mask_abbreviations(&mut chars);
    mask_versions(&mut chars);
    chars
}

/// Scan `chars` left to right; `mask_at` blanks the span opening at each
/// position and returns the next index to scan.
fn mask_spans(chars: &mut [char], mask_at: impl Fn(&mut [char], usize) -> usize) {
    let mut index = 0;
    while index < chars.len() {
        index = mask_at(chars, index);
    }
}

/// Mask every inline code span, backticks included; an unmatched backtick
/// stays literal.
fn mask_code_spans(chars: &mut [char]) {
    mask_spans(chars, mask_code_span_at);
}

/// Mask the code span opening at `start` when a closing backtick follows, and
/// return the next index to scan.
fn mask_code_span_at(chars: &mut [char], start: usize) -> usize {
    if chars[start] != '`' {
        return start + 1;
    }
    let Some(end) = closing_backtick(chars, start) else {
        return start + 1;
    };
    chars[start..=end].fill(' ');
    end + 1
}

/// Index of the backtick that closes the code span opening at `start`.
fn closing_backtick(chars: &[char], start: usize) -> Option<usize> {
    let offset = chars[start + 1..].iter().position(|c| *c == '`')?;
    Some(start + 1 + offset)
}

/// Mask every `http://` or `https://` token up to the next whitespace.
fn mask_urls(chars: &mut [char]) {
    mask_spans(chars, mask_url_at);
}

/// Mask the URL starting at `start`, if any, and return the next index.
fn mask_url_at(chars: &mut [char], start: usize) -> usize {
    let Some(scheme) = scheme_len(chars, start) else {
        return start + 1;
    };
    let end = token_end(chars, start + scheme);
    chars[start..end].fill(' ');
    end
}

/// Length of the URL scheme at `start`, if the text opens with one.
fn scheme_len(chars: &[char], start: usize) -> Option<usize> {
    if at(chars, start, "https://") {
        Some(8)
    } else if at(chars, start, "http://") {
        Some(7)
    } else {
        None
    }
}

/// End of the non-whitespace token that begins at `start`.
fn token_end(chars: &[char], start: usize) -> usize {
    let mut end = start;
    while end < chars.len() && !chars[end].is_whitespace() {
        end += 1;
    }
    end
}

/// Abbreviation tokens whose trailing dot can never end a sentence.
const ABBREVIATIONS: [&str; 6] = ["e.g.", "i.e.", "etc.", "vs.", "cf.", "approx."];

/// Mask every known abbreviation token.
fn mask_abbreviations(chars: &mut [char]) {
    mask_spans(chars, mask_abbreviation_at);
}

/// Mask the abbreviation starting at `start`, if any, and return the next
/// index.
fn mask_abbreviation_at(chars: &mut [char], start: usize) -> usize {
    let Some(len) = abbreviation_len(chars, start) else {
        return start + 1;
    };
    chars[start..start + len].fill(' ');
    start + len
}

/// Length of the abbreviation token at `start`, if one begins there.
fn abbreviation_len(chars: &[char], start: usize) -> Option<usize> {
    for pattern in ABBREVIATIONS {
        if at(chars, start, pattern) {
            return Some(pattern.chars().count());
        }
    }
    None
}

/// Mask every version token such as `1.2` or `v3.0.1`.
fn mask_versions(chars: &mut [char]) {
    mask_spans(chars, mask_version_at);
}

/// Mask the version token starting at `start`, if any, and return the next
/// index.
fn mask_version_at(chars: &mut [char], start: usize) -> usize {
    let Some(end) = version_end(chars, start) else {
        return start + 1;
    };
    chars[start..end].fill(' ');
    end
}

/// End of the version token beginning at `start`, once it has at least one
/// dotted group.
fn version_end(chars: &[char], start: usize) -> Option<usize> {
    let mut end = digit_run(chars, version_stem(chars, start)?);
    let mut groups = 0;
    while dot_group_at(chars, end) {
        groups += 1;
        end = digit_run(chars, end + 1);
    }
    (groups > 0).then_some(end)
}

/// Index of the version token's first digit at `start`, allowing one `v`/`V`
/// prefix.
fn version_stem(chars: &[char], start: usize) -> Option<usize> {
    let prefixed = matches!(chars[start], 'v' | 'V') && digit_at(chars, start + 1);
    let stem = start + usize::from(prefixed);
    digit_at(chars, stem).then_some(stem)
}

/// Whether a `.<digit>` group opens at `index`.
fn dot_group_at(chars: &[char], index: usize) -> bool {
    chars.get(index) == Some(&'.') && digit_at(chars, index + 1)
}

/// End of the run of ASCII digits that begins at `start`.
fn digit_run(chars: &[char], start: usize) -> usize {
    let mut end = start;
    while digit_at(chars, end) {
        end += 1;
    }
    end
}

/// Whether `chars[index]` is an ASCII digit.
fn digit_at(chars: &[char], index: usize) -> bool {
    chars.get(index).is_some_and(char::is_ascii_digit)
}

/// Whether the ASCII `pattern` starts at `start` in `chars`.
fn at(chars: &[char], start: usize, pattern: &str) -> bool {
    pattern
        .chars()
        .enumerate()
        .all(|(offset, expected)| chars.get(start + offset) == Some(&expected))
}
