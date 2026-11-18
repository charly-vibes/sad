//! Purpose: turn a raw comment or doc-string node into the normalised body the
//! checks read.
//! Responsibilities: classify comment kinds, strip line/block markers, remove
//! block-comment gutters and the delimiter newline, and extract Python
//! docstring text.
//! Rationale: body text must be identical regardless of comment syntax so
//! attribution and the gate checks never depend on delimiters.

use tree_sitter::Node;

use crate::model::{CommentKind, Language};

pub(crate) fn comment_kind(language: Language, node: Node<'_>, raw: &str) -> CommentKind {
    match language {
        Language::Rust => {
            if node.child_by_field_name("doc").is_some() {
                CommentKind::Doc
            } else if node.kind() == "line_comment" {
                CommentKind::Line
            } else {
                CommentKind::Block
            }
        }
        Language::Python => CommentKind::Line,
        Language::TypeScript => {
            if raw.starts_with("//") {
                CommentKind::Line
            } else if raw.starts_with("/**") {
                CommentKind::Doc
            } else {
                CommentKind::Block
            }
        }
    }
}

pub(crate) fn comment_body(
    language: Language,
    kind: CommentKind,
    node: Node<'_>,
    raw: &str,
    source: &str,
) -> String {
    match language {
        Language::Rust => strip_rust_marker(raw),
        Language::Python => match kind {
            CommentKind::Doc => python_docstring_body(node, raw, source),
            _ => strip_hash_marker(raw),
        },
        Language::TypeScript => strip_typescript_marker(raw),
    }
}

fn strip_rust_marker(raw: &str) -> String {
    let text = raw.trim_end_matches(['\r', '\n']);
    // `////` is the grammar's plain `//` comment whose text begins with `//`:
    // strip exactly the first two slashes so a ruler keeps its extra slashes.
    if let Some(rest) = text.strip_prefix("////") {
        return strip_one_space(rest).trim_end().to_string();
    }
    for marker in ["//!", "///", "//"] {
        if let Some(rest) = text.strip_prefix(marker) {
            return strip_one_space(rest).trim_end().to_string();
        }
    }
    for opener in ["/*!", "/**", "/*"] {
        if let Some(rest) = text.strip_prefix(opener) {
            let inner = rest.strip_suffix("*/").unwrap_or(rest);
            return normalise_block_body(inner);
        }
    }
    text.to_string()
}

/// Normalise a block-comment body: the caller removes the opener (`/*`,
/// `/**`, `/*!`) and the closing `*/`, this removes the delimiter newline and
/// the per-line `*` gutter.
///
/// Every line loses trailing whitespace, leading whitespace, exactly one
/// leading `*`, and one optional space after it. Leading and trailing empty
/// lines are dropped and the rest is joined with `\n`, so a single-line block
/// yields its trimmed content without a `*` and a starred multi-line block
/// yields the logical documentation text.
fn normalise_block_body(inner: &str) -> String {
    let lines: Vec<&str> = inner
        .split('\n')
        .map(|line| {
            let line = line.trim_end();
            let line = line.trim_start();
            let line = line.strip_prefix('*').unwrap_or(line);
            line.strip_prefix(' ').unwrap_or(line)
        })
        .collect();
    let start = lines
        .iter()
        .position(|line| !line.is_empty())
        .unwrap_or(lines.len());
    let end = lines
        .iter()
        .rposition(|line| !line.is_empty())
        .map_or(start, |index| index + 1);
    lines[start..end].join("\n")
}

fn strip_hash_marker(raw: &str) -> String {
    let text = raw.trim_end_matches(['\r', '\n']);
    strip_one_space(text.strip_prefix('#').unwrap_or(text))
        .trim_end()
        .to_string()
}

fn strip_typescript_marker(raw: &str) -> String {
    let text = raw.trim_end_matches(['\r', '\n']);
    if let Some(rest) = text.strip_prefix("//") {
        return strip_one_space(rest).trim_end().to_string();
    }
    for opener in ["/**", "/*"] {
        if let Some(rest) = text.strip_prefix(opener) {
            let inner = rest.strip_suffix("*/").unwrap_or(rest);
            return normalise_block_body(inner);
        }
    }
    text.to_string()
}

fn strip_one_space(text: &str) -> &str {
    text.strip_prefix(' ').unwrap_or(text)
}

pub(crate) fn python_docstring_body(node: Node<'_>, raw: &str, source: &str) -> String {
    let mut cursor = node.walk();
    if let Some(content) = node
        .named_children(&mut cursor)
        .find(|child| child.kind() == "string_content")
    {
        return source
            .get(content.byte_range())
            .unwrap_or("")
            .trim()
            .to_string();
    }
    raw.trim()
        .trim_start_matches(['r', 'u', 'b', 'f'])
        .trim_matches(['"', '\''])
        .trim()
        .to_string()
}
