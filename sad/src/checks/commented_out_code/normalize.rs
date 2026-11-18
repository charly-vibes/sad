//! Purpose: turn one block of comment nodes into the single normalised body
//! the commented-out-code predicate parses and the finding hashes.
//! Responsibilities: apply the fixed normalisation order — line endings and a
//! leading BOM, per-kind marker and gutter stripping, the newline join, the
//! common-prefix dedent, trailing-whitespace trimming, and blank-end trimming.
//! Rationale: identity must survive CRLF churn, block gutters, and ragged
//! indentation, so one implementation owns the order; splitting it across the
//! extractor and this check would let the two normalisations drift and make
//! finding IDs depend on which frontend produced the node.

use crate::model::{CommentKind, CommentNode, Language};

/// Normalise a block's nodes into one marker-free, dedented body.
///
/// `nodes` must already be the block's finding-eligible nodes in source
/// order. An empty result means every line was blank after stripping, so the
/// caller keeps the nodes as prose.
pub(super) fn normalise_block(language: Language, nodes: &[&CommentNode]) -> String {
    let mut joined = String::new();
    for (offset, node) in nodes.iter().enumerate() {
        if offset > 0 {
            joined.push('\n');
        }
        joined.push_str(&strip_marker(language, node.kind, &node.text));
    }
    finish(&joined)
}

/// Strip the comment syntax of one node, normalising line endings first.
fn strip_marker(language: Language, kind: CommentKind, raw: &str) -> String {
    let text = normalise_breaks(raw);
    match language {
        Language::Python => strip_hash(&text),
        Language::Rust | Language::TypeScript => match kind {
            CommentKind::Block => strip_block(&text),
            _ => strip_line(&text),
        },
    }
}

/// Collapse `\r\n` and lone `\r` to `\n` and drop one leading BOM.
fn normalise_breaks(raw: &str) -> String {
    let text = raw.strip_prefix('\u{feff}').unwrap_or(raw);
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// Strip `//` plus one following `/` or `!` and one space.
fn strip_line(raw: &str) -> String {
    let rest = raw.strip_prefix("//").unwrap_or(raw);
    let rest = rest
        .strip_prefix('/')
        .or_else(|| rest.strip_prefix('!'))
        .unwrap_or(rest);
    strip_one_space(rest).to_string()
}

/// Strip `/*` plus one following `*` or `!`, the closing `*/`, and gutters.
fn strip_block(raw: &str) -> String {
    let rest = raw.strip_prefix("/*").unwrap_or(raw);
    let rest = rest
        .strip_prefix('*')
        .or_else(|| rest.strip_prefix('!'))
        .unwrap_or(rest);
    let inner = rest.strip_suffix("*/").unwrap_or(rest);
    inner
        .split('\n')
        .map(strip_gutter)
        .collect::<Vec<String>>()
        .join("\n")
}

/// Remove one leading gutter `*` and the single space after it.
///
/// The `*` is only a gutter when what follows it is empty or starts with
/// whitespace, so a body line that legitimately opens with `*` keeps it.
fn strip_gutter(line: &str) -> String {
    let line = line.trim_start();
    let Some(rest) = line.strip_prefix('*') else {
        return line.to_string();
    };
    if rest.is_empty() || rest.starts_with(char::is_whitespace) {
        strip_one_space(rest).to_string()
    } else {
        line.to_string()
    }
}

/// Strip one Python `#` and one following space.
///
/// Rust and TypeScript never reach this branch, so an attribute such as
/// `#[test]` in their bodies is code, not a marker.
fn strip_hash(raw: &str) -> String {
    strip_one_space(raw.strip_prefix('#').unwrap_or(raw)).to_string()
}

fn strip_one_space(text: &str) -> &str {
    text.strip_prefix(' ').unwrap_or(text)
}

/// Dedent, trim trailing whitespace, and drop blank ends.
///
/// The surviving lines are joined with `\n`; a body with no non-blank line
/// becomes the empty string.
fn finish(joined: &str) -> String {
    let lines: Vec<&str> = joined.split('\n').collect();
    let indent = common_indent(&lines);
    let trimmed: Vec<&str> = lines
        .iter()
        .map(|line| line.strip_prefix(indent).unwrap_or(line).trim_end())
        .collect();
    let start = trimmed
        .iter()
        .position(|line| !line.is_empty())
        .unwrap_or(trimmed.len());
    let end = trimmed
        .iter()
        .rposition(|line| !line.is_empty())
        .map_or(start, |index| index + 1);
    trimmed[start..end].join("\n")
}

/// Longest whitespace prefix shared by every non-blank line.
fn common_indent<'a>(lines: &[&'a str]) -> &'a str {
    let mut indent: Option<&str> = None;
    for line in lines.iter().filter(|line| !line.trim().is_empty()) {
        let leading = &line[..line.len() - line.trim_start().len()];
        indent = Some(match indent {
            None => leading,
            Some(current) => shared_prefix(current, leading),
        });
    }
    indent.unwrap_or("")
}

fn shared_prefix<'a>(left: &'a str, right: &str) -> &'a str {
    let boundary = left
        .bytes()
        .zip(right.bytes())
        .position(|(a, b)| a != b)
        .unwrap_or(left.len().min(right.len()));
    &left[..boundary]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(language: Language, kind: CommentKind, text: &str) -> String {
        finish(&strip_marker(language, kind, text))
    }

    #[test]
    fn lone_carriage_return_and_bom_normalise_to_the_same_body() {
        let block = body(Language::Rust, CommentKind::Block, "/*\r * fn f() {}\r */");
        assert_eq!(block, "fn f() {}");
        let marked = body(Language::Python, CommentKind::Line, "\u{feff}# fn f() {}");
        assert_eq!(marked, "fn f() {}");
    }

    #[test]
    fn rust_and_typescript_keep_a_hash_in_the_body() {
        assert_eq!(
            body(Language::Rust, CommentKind::Line, "// #[test]"),
            "#[test]"
        );
        assert_eq!(
            body(Language::TypeScript, CommentKind::Line, "// #region"),
            "#region"
        );
    }

    #[test]
    fn block_gutters_and_common_indent_are_removed() {
        let text = "/*\n     *     let x = 1;\n     *     g(x);\n     */";
        assert_eq!(
            body(Language::Rust, CommentKind::Block, text),
            "let x = 1;\ng(x);"
        );
    }
}
