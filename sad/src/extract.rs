//! Purpose: parse Rust, Python, and TypeScript sources with pinned
//! tree-sitter grammars and extract every comment/doc-string node with its
//! channel attribution, body, line, and symbol path.
//! Responsibilities: own the public entry points ([`language_for_path`],
//! [`analyze_file`], [`Analyzer`]), configure one parser per language and one
//! marker matcher per run, walk the parse tree (never scan text), skip the
//! grammar's `doc_comment` children, detect the leading header run by byte
//! span, and mark whole-file exemptions (a generated-code marker block or
//! exemption-only comments).
//! Rationale: structural extraction is the single source of truth for every
//! gate finding; keeping it parse-tree based makes string literals immune and
//! keeps line attribution independent of regexes or byte heuristics. Bodies,
//! symbol resolution, and docstring detection live in submodules so each stays
//! small enough to audit on its own.

use std::borrow::Cow;
use std::path::Path;

use tree_sitter::Node;

use crate::attribution::{
    MarkerMatcher, classify, has_generated_marker, is_coding_cookie, is_exemption, is_shebang,
};
use crate::model::{
    AnalyzedFile, AttributionClass, CommentKind, CommentNode, Language, SuppressionDirective,
};

pub(crate) use self::analyzer::parser_for;
pub use self::analyzer::{Analyzer, analyze_file};

use self::bodies::{comment_body, comment_channel, python_docstring_body};
use self::docstrings::python_docstring;
use self::symbols::symbol_path;

mod analyzer;
mod bodies;
mod docstrings;
mod symbols;

/// Map a path to a supported v1 language by extension.
pub fn language_for_path(path: &Path) -> Option<Language> {
    match path.extension()?.to_str()? {
        "rs" => Some(Language::Rust),
        "py" | "pyi" => Some(Language::Python),
        "ts" | "js" | "mjs" => Some(Language::TypeScript),
        _ => None,
    }
}

/// Normalise CRLF and lone-CR line endings so line attribution is stable.
fn normalise_source(source: &str) -> Cow<'_, str> {
    if source.contains('\r') {
        Cow::Owned(source.replace("\r\n", "\n").replace('\r', "\n"))
    } else {
        Cow::Borrowed(source)
    }
}

/// Everything the tree walk needs beyond the parse tree itself.
struct WalkContext<'a> {
    language: Language,
    source: &'a str,
    marker_prefix: &'a str,
    markers: &'a MarkerMatcher,
    header_span: Option<(usize, usize)>,
    module_doc_start: Option<usize>,
}

/// One node's extraction: the node to record (the comment itself, or the
/// docstring's string literal), its normalised body, and the verdict.
struct Extracted<'tree> {
    node: Node<'tree>,
    kind: CommentKind,
    body: String,
    class: AttributionClass,
    directive: Option<SuppressionDirective>,
}

/// Walk the parse tree and return the extracted nodes in source order.
fn collect_nodes(root: Node<'_>, context: WalkContext<'_>) -> Vec<CommentNode> {
    let mut extractor = Extractor {
        context,
        nodes: Vec::new(),
    };
    extractor.walk(root);
    extractor.nodes.sort_by_key(|(start_byte, _)| *start_byte);
    extractor.nodes.into_iter().map(|(_, node)| node).collect()
}

fn empty_file(rel_path: &str, language: Language) -> AnalyzedFile {
    AnalyzedFile {
        path: rel_path.to_string(),
        language,
        nodes: Vec::new(),
        exempt: false,
        header: false,
    }
}

fn next_code_line(node: Node<'_>, source: &str) -> Option<String> {
    let mut sibling = node.next_named_sibling();
    while let Some(current) = sibling {
        let comment_like = matches!(
            current.kind(),
            "line_comment" | "block_comment" | "comment" | "doc_comment"
        );
        if !comment_like {
            let rest = source.get(current.start_byte()..)?;
            let line = rest.lines().find(|line| !line.trim().is_empty())?;
            return Some(line.trim().to_string());
        }
        sibling = current.next_named_sibling();
    }
    None
}

struct Extractor<'a> {
    /// Everything the walk reads per node.
    context: WalkContext<'a>,
    /// `(start byte, node)` pairs sorted into source order at the end.
    nodes: Vec<(usize, CommentNode)>,
}

impl<'a> Extractor<'a> {
    /// Visits every node in pre-order without recursing.
    ///
    /// The recursion this loop replaced overflowed the stack on pathological
    /// ASTs: a few tens of thousands of nested parens parse fine and then
    /// the per-node frames exhaust the thread stack. Nodes are sorted into
    /// source order at the end of [`collect_nodes`], so visit order carries
    /// no observable meaning beyond docstring `is_module` flags, which the
    /// node kinds themselves determine.
    fn walk(&mut self, root: Node<'_>) {
        let mut pending = vec![root];
        while let Some(node) = pending.pop() {
            if let Some(extracted) = self.extract_node(node) {
                self.nodes.push(self.finish(extracted));
            }
            let mut cursor = node.walk();
            let children: Vec<_> = node.children(&mut cursor).collect();
            pending.extend(children.into_iter().rev());
        }
    }

    /// Which node kinds each language extracts, matched as a pure classifier:
    /// comments in every language, plus the Python docstring of a module,
    /// class, or function.
    fn extract_node<'tree>(&self, node: Node<'tree>) -> Option<Extracted<'tree>> {
        match self.context.language {
            Language::Rust if matches!(node.kind(), "line_comment" | "block_comment") => {
                self.extract_comment(node)
            }
            Language::Python if node.kind() == "comment" => self.extract_comment(node),
            Language::Python
                if matches!(
                    node.kind(),
                    "module" | "class_definition" | "function_definition"
                ) =>
            {
                python_docstring(node)
                    .map(|docstring| self.extract_docstring(docstring, node.kind() == "module"))
            }
            Language::TypeScript if node.kind() == "comment" => self.extract_comment(node),
            _ => None,
        }
    }

    /// Everything attribution decides for one comment node.
    fn extract_comment<'tree>(&self, node: Node<'tree>) -> Option<Extracted<'tree>> {
        let channel = comment_channel(self.context.language, node, self.context.source);
        let body = comment_body(&channel, node, self.context.source);
        let (class, directive) = if let Some(class) = self.file_metadata_class(node, &body) {
            (class, None)
        } else {
            let is_header = self.is_header_position(node);
            classify(
                &body,
                channel.kind,
                is_header,
                self.context.marker_prefix,
                self.context.markers,
            )
        };
        Some(Extracted {
            node,
            kind: channel.kind,
            body,
            class,
            directive,
        })
    }

    /// File-metadata lines are exemptions, never finding-eligible prose.
    ///
    /// A Python shebang is file metadata only on source row 0 and a PEP 263
    /// coding cookie only on rows 0-1; the same text later in the file is
    /// ordinary prose, so the positional gate lives here and `classify` stays
    /// a pure function of the body. Exemption is exempt-but-observable:
    /// [`crate::exemptions::summary`] still counts these nodes.
    fn file_metadata_class(&self, node: Node<'_>, body: &str) -> Option<AttributionClass> {
        if self.context.language != Language::Python {
            return None;
        }
        let row = node.start_position().row;
        let metadata = (row == 0 && is_shebang(body)) || (row <= 1 && is_coding_cookie(body));
        metadata.then_some(AttributionClass::Exemption)
    }

    /// Everything attribution decides for one Python docstring literal.
    ///
    /// `is_module` marks the module docstring, whose channel is the file
    /// header; class and function docstrings are ordinary docs.
    fn extract_docstring<'tree>(
        &self,
        docstring: Node<'tree>,
        is_module: bool,
    ) -> Extracted<'tree> {
        let raw = self
            .context
            .source
            .get(docstring.byte_range())
            .unwrap_or("");
        let body = python_docstring_body(docstring, raw, self.context.source);
        let (class, directive) = classify(
            &body,
            CommentKind::Doc,
            is_module,
            self.context.marker_prefix,
            self.context.markers,
        );
        Extracted {
            node: docstring,
            kind: CommentKind::Doc,
            body,
            class,
            directive,
        }
    }

    /// Record one extraction with its line, symbol path, and next code line.
    fn finish(&self, extracted: Extracted<'_>) -> (usize, CommentNode) {
        let node = extracted.node;
        let next_code_line = match extracted.kind {
            CommentKind::Doc => None,
            CommentKind::Line | CommentKind::Block => next_code_line(node, self.context.source),
        };
        let text = self
            .context
            .source
            .get(node.byte_range())
            .unwrap_or("")
            .trim_end_matches(['\r', '\n'])
            .to_string();
        (
            node.start_byte(),
            CommentNode {
                line: node.start_position().row + 1,
                kind: extracted.kind,
                class: extracted.class,
                symbol_path: symbol_path(node, self.context.language, self.context.source),
                body: extracted.body,
                text,
                directive: extracted.directive,
                next_code_line,
            },
        )
    }

    fn is_header_position(&self, node: Node<'_>) -> bool {
        match self.context.language {
            // Only an inner doc comment inside the leading inner-doc run is
            // the header channel: `mod inner { //! docs }` documents a module,
            // not the file.
            Language::Rust => {
                node.child_by_field_name("inner").is_some() && self.in_header_span(node)
            }
            Language::Python => self
                .context
                .module_doc_start
                .is_some_and(|start| node.start_byte() == start),
            Language::TypeScript => self.in_header_span(node),
        }
    }

    fn in_header_span(&self, node: Node<'_>) -> bool {
        self.context
            .header_span
            .is_some_and(|(start, end)| (start..end).contains(&node.start_byte()))
    }
}

/// Detect the file header channel and its exact byte span.
///
/// Returns `(file has header, header span, module docstring start)`. The span
/// is the leading header-channel run: the maximal prefix of inner-doc comments
/// for Rust (after any leading `shebang`/`attribute_item` children, which are
/// legal before a `//!` block), the first block comment for TypeScript, and
/// nothing for Python, whose header is the module docstring rather than a
/// comment node.
fn detect_header(
    language: Language,
    root: Node<'_>,
    source: &str,
) -> (bool, Option<(usize, usize)>, Option<usize>) {
    match language {
        Language::Rust => {
            let header_span = rust_header_span(root);
            (header_span.is_some(), header_span, None)
        }
        Language::Python => {
            let module_doc_start = python_docstring(root).map(|node| node.start_byte());
            (module_doc_start.is_some(), None, module_doc_start)
        }
        Language::TypeScript => {
            let header_span = root.named_child(0).and_then(|first| {
                (first.kind() == "comment"
                    && source
                        .get(first.byte_range())
                        .is_some_and(|text| text.starts_with("/*")))
                .then_some((first.start_byte(), first.end_byte()))
            });
            (header_span.is_some(), header_span, None)
        }
    }
}

/// Byte span of the leading inner-doc run, or `None` when the file does not
/// open with one.
fn rust_header_span(root: Node<'_>) -> Option<(usize, usize)> {
    let mut cursor = root.walk();
    let mut children = root.named_children(&mut cursor).peekable();
    // `#!/usr/bin/env …` and `#![…]` are named nodes that may precede the
    // header block; they are not comments and must not hide it.
    while children
        .peek()
        .is_some_and(|child| matches!(child.kind(), "shebang" | "attribute_item"))
    {
        children.next();
    }
    let first = children.next()?;
    if !is_inner_doc(first) {
        return None;
    }
    let mut span = (first.start_byte(), first.end_byte());
    for child in children {
        if !is_inner_doc(child) {
            break;
        }
        span.1 = child.end_byte();
    }
    Some(span)
}

/// True for the grammar's inner doc comments (`//!` and `/*! */`).
fn is_inner_doc(node: Node<'_>) -> bool {
    matches!(node.kind(), "line_comment" | "block_comment")
        && node.child_by_field_name("inner").is_some()
}

/// Whole-file exemptions are exempt-but-observable: the file's leading comment
/// block names a generated-code marker (`@generated` at the start of a line of
/// its normalised body), or every comment in the file is licence/generated
/// text. Header-position licence blocks keep the header channel, so they count
/// here through [`is_exemption`] rather than the exemption class. One prose,
/// doc, marker, or suppression node means the remaining exemptions are
/// node-level only.
fn compute_exempt(nodes: &[CommentNode]) -> bool {
    if nodes.is_empty() {
        return false;
    }
    if leading_block(nodes)
        .iter()
        .any(|node| has_generated_marker(&node.body))
    {
        return true;
    }
    nodes.iter().all(|node| {
        node.class == AttributionClass::Exemption
            || (node.class == AttributionClass::Header && is_exemption(&node.body))
    })
}

/// The leading comment block: nodes from the first non-empty comment, extended
/// while each next node starts on the line after the previous one ends.
fn leading_block(nodes: &[CommentNode]) -> &[CommentNode] {
    let Some(start) = nodes.iter().position(|node| !node.body.trim().is_empty()) else {
        return &[];
    };
    let mut end = start + 1;
    while end < nodes.len() {
        let previous = &nodes[end - 1];
        let previous_end = previous.line + previous.text.matches('\n').count();
        if nodes[end].line == previous_end + 1 {
            end += 1;
        } else {
            break;
        }
    }
    &nodes[start..end]
}
