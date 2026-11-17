//! Purpose: parse Rust, Python, and TypeScript sources with pinned
//! tree-sitter grammars and extract every comment/doc-string node with its
//! channel attribution, body, line, and symbol path.
//! Responsibilities: normalise CRLF and block-comment gutters, walk the parse
//! tree (never scan text), skip the grammar's `doc_comment` children,
//! recognise Python docstrings, walk the allowlisted symbol-bearing ancestors
//! (including the item an outer doc comment documents) for symbol paths,
//! detect the leading header run by byte span, and mark whole-file exemptions
//! (a generated-code marker block or exemption-only comments).
//! Rationale: structural extraction is the single source of truth for every
//! gate finding; keeping it parse-tree based makes string literals immune and
//! keeps line attribution independent of regexes or byte heuristics.

use std::borrow::Cow;
use std::path::Path;

use tree_sitter::{Node, Parser};

use crate::attribution::{MarkerMatcher, classify, has_generated_marker, is_exemption};
use crate::config::Config;
use crate::model::{AnalyzedFile, AttributionClass, CommentKind, CommentNode, Language};

/// Map a path to a supported v1 language by extension.
pub fn language_for_path(path: &Path) -> Option<Language> {
    match path.extension()?.to_str()? {
        "rs" => Some(Language::Rust),
        "py" => Some(Language::Python),
        "ts" => Some(Language::TypeScript),
        _ => None,
    }
}

/// Analyse one source file exactly as the gate would.
///
/// `path` selects the grammar; `rel_path` is the stable path reported in the
/// returned [`AnalyzedFile`]. Unsupported extensions yield an empty analysis
/// with `Rust` as a neutral placeholder — callers filter by
/// [`language_for_path`] before relying on the result.
pub fn analyze_file(path: &Path, rel_path: &str, source: &str, cfg: &Config) -> AnalyzedFile {
    let Some(language) = language_for_path(path).or_else(|| language_for_path(Path::new(rel_path)))
    else {
        // Documented contract: no grammar is ever guessed for an unsupported
        // extension, so the result is empty rather than a wrong-grammar parse.
        return empty_file(rel_path, Language::Rust);
    };
    let normalised = if source.contains('\r') {
        Cow::Owned(source.replace("\r\n", "\n"))
    } else {
        Cow::Borrowed(source)
    };
    let source = normalised.as_ref();
    let markers = MarkerMatcher::new(&cfg.marker_pattern);

    let Some(mut parser) = parser_for(language) else {
        return empty_file(rel_path, language);
    };
    let Some(tree) = parser.parse(source, None) else {
        return empty_file(rel_path, language);
    };

    let root = tree.root_node();
    let (header, header_span, module_doc_start) = detect_header(language, root, source);

    let mut extractor = Extractor {
        language,
        source,
        marker_prefix: &cfg.marker_prefix,
        markers: &markers,
        header_span,
        module_doc_start,
        nodes: Vec::new(),
    };
    extractor.walk(root);
    extractor.nodes.sort_by_key(|(start_byte, _)| *start_byte);
    let nodes: Vec<CommentNode> = extractor.nodes.into_iter().map(|(_, node)| node).collect();
    let exempt = compute_exempt(&nodes);

    AnalyzedFile {
        path: rel_path.to_string(),
        language,
        nodes,
        exempt,
        header,
    }
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

fn parser_for(language: Language) -> Option<Parser> {
    let grammar: tree_sitter::Language = match language {
        Language::Rust => tree_sitter_rust::LANGUAGE.into(),
        Language::Python => tree_sitter_python::LANGUAGE.into(),
        Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
    };
    let mut parser = Parser::new();
    parser.set_language(&grammar).ok()?;
    Some(parser)
}

struct Extractor<'a> {
    language: Language,
    source: &'a str,
    marker_prefix: &'a str,
    markers: &'a MarkerMatcher,
    /// Byte span of the file's leading header-channel run, when present.
    header_span: Option<(usize, usize)>,
    /// Start byte of the Python module docstring, when present.
    module_doc_start: Option<usize>,
    /// `(start byte, node)` pairs sorted into source order at the end.
    nodes: Vec<(usize, CommentNode)>,
}

impl<'a> Extractor<'a> {
    fn walk(&mut self, node: Node<'_>) {
        match self.language {
            Language::Rust => {
                if matches!(node.kind(), "line_comment" | "block_comment") {
                    self.push_comment(node);
                }
            }
            Language::Python => {
                if node.kind() == "comment" {
                    self.push_comment(node);
                }
                if matches!(
                    node.kind(),
                    "module" | "class_definition" | "function_definition"
                ) && let Some(docstring) = python_docstring(node)
                {
                    self.push_docstring(docstring, node.kind() == "module");
                }
            }
            Language::TypeScript => {
                if node.kind() == "comment" {
                    self.push_comment(node);
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.walk(child);
        }
    }

    fn push_comment(&mut self, node: Node<'_>) {
        let source = self.source;
        let raw = source.get(node.byte_range()).unwrap_or("");
        let kind = comment_kind(self.language, node, raw);
        let body = comment_body(self.language, kind, node, raw, source);
        let is_header = self.is_header_position(node);
        let (class, directive) = classify(&body, kind, is_header, self.marker_prefix, self.markers);
        self.push(node, kind, class, body, raw, directive);
    }

    fn push_docstring(&mut self, node: Node<'_>, is_module: bool) {
        let source = self.source;
        let raw = source.get(node.byte_range()).unwrap_or("");
        let body = python_docstring_body(node, raw, source);
        let (class, directive) = classify(
            &body,
            CommentKind::Doc,
            is_module,
            self.marker_prefix,
            self.markers,
        );
        self.push(node, CommentKind::Doc, class, body, raw, directive);
    }

    fn push(
        &mut self,
        node: Node<'_>,
        kind: CommentKind,
        class: AttributionClass,
        body: String,
        raw: &str,
        directive: Option<crate::model::SuppressionDirective>,
    ) {
        let start_byte = node.start_byte();
        self.nodes.push((
            start_byte,
            CommentNode {
                line: node.start_position().row + 1,
                kind,
                class,
                symbol_path: symbol_path(node, self.language, self.source),
                body,
                text: raw.trim_end_matches(['\r', '\n']).to_string(),
                directive,
            },
        ));
    }

    fn is_header_position(&self, node: Node<'_>) -> bool {
        match self.language {
            // Only an inner doc comment inside the leading inner-doc run is
            // the header channel: `mod inner { //! docs }` documents a module,
            // not the file.
            Language::Rust => {
                node.child_by_field_name("inner").is_some() && self.in_header_span(node)
            }
            Language::Python => self
                .module_doc_start
                .is_some_and(|start| node.start_byte() == start),
            Language::TypeScript => self.in_header_span(node),
        }
    }

    fn in_header_span(&self, node: Node<'_>) -> bool {
        self.header_span
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

/// First non-comment named child, so a leading Python comment cannot hide a
/// module docstring.
fn python_first_statement<'tree>(node: Node<'tree>) -> Option<Node<'tree>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find(|child| child.kind() != "comment")
}

/// The docstring of a module/class/function, when its first statement is a
/// string literal.
///
/// Parenthesised literals and implicit concatenation (`("doc")`, `"a" "b"`)
/// are unwrapped to the first literal, because CPython folds both forms into
/// the item's `__doc__`.
fn python_docstring<'tree>(node: Node<'tree>) -> Option<Node<'tree>> {
    let first = if node.kind() == "module" {
        python_first_statement(node)?
    } else {
        let body = node.child_by_field_name("body")?;
        python_first_statement(body)?
    };
    if first.kind() != "expression_statement" {
        return None;
    }
    let mut cursor = first.walk();
    let value = first.named_children(&mut cursor).next()?;
    string_literal(value)
}

/// Unwrap `parenthesized_expression`/`concatenated_string` wrappers down to
/// the first string literal, whose `string_content` carries the body.
fn string_literal<'tree>(mut node: Node<'tree>) -> Option<Node<'tree>> {
    loop {
        match node.kind() {
            "string" => return Some(node),
            "parenthesized_expression" | "concatenated_string" => {
                let mut cursor = node.walk();
                node = node.named_children(&mut cursor).find(|child| {
                    matches!(
                        child.kind(),
                        "string" | "parenthesized_expression" | "concatenated_string"
                    )
                })?;
            }
            _ => return None,
        }
    }
}

fn comment_kind(language: Language, node: Node<'_>, raw: &str) -> CommentKind {
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

fn comment_body(
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

fn python_docstring_body(node: Node<'_>, raw: &str, source: &str) -> String {
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

/// Symbol path for a node: an outer doc comment carries the item it directly
/// documents, everything else carries its symbol-bearing ancestors; `-` when
/// the node sits outside every named item.
fn symbol_path(node: Node<'_>, language: Language, source: &str) -> String {
    if let Some(path) = documented_symbol_path(node, language, source) {
        return path;
    }
    join_chain(ancestor_names(node.parent(), language, source))
}

/// Ancestor chain joined by `::`, outermost first.
fn ancestor_names<'a>(from: Option<Node<'_>>, language: Language, source: &'a str) -> Vec<&'a str> {
    let mut names = Vec::new();
    let mut current = from;
    while let Some(node) = current {
        if let Some(name) = ancestor_name(node, language, source) {
            names.push(name);
        }
        current = node.parent();
    }
    names.reverse();
    names
}

fn join_chain(names: Vec<&str>) -> String {
    if names.is_empty() {
        "-".to_string()
    } else {
        names.join("::")
    }
}

/// Symbol path of the item an outer doc comment documents, when there is one.
///
/// A `///` line or `/** */` block above `Foo::bar` anchors to `Foo::bar`, not
/// to the `Foo` container it physically sits in; consecutive doc lines and any
/// attributes between the doc and the item are skipped. Inner docs and Python
/// docstrings keep the ancestor rule, because they already belong to their
/// item.
fn documented_symbol_path(node: Node<'_>, language: Language, source: &str) -> Option<String> {
    let is_outer_doc = match language {
        Language::Rust => node.child_by_field_name("outer").is_some(),
        Language::TypeScript => {
            node.kind() == "comment"
                && source
                    .get(node.byte_range())
                    .is_some_and(|raw| raw.starts_with("/**"))
        }
        Language::Python => false,
    };
    if !is_outer_doc {
        return None;
    }
    let target = symbol_target(next_item(node, language)?, language)?;
    let names = ancestor_names(Some(target), language, source);
    (!names.is_empty()).then(|| names.join("::"))
}

/// First sibling that is neither a comment nor an attribute: the item a doc
/// comment directly documents.
fn next_item<'tree>(node: Node<'tree>, language: Language) -> Option<Node<'tree>> {
    let mut current = node.next_named_sibling();
    while let Some(sibling) = current {
        let skip = match language {
            Language::Rust => matches!(
                sibling.kind(),
                "line_comment" | "block_comment" | "attribute_item"
            ),
            Language::TypeScript => matches!(sibling.kind(), "comment" | "decorator"),
            Language::Python => false,
        };
        if !skip {
            return Some(sibling);
        }
        current = sibling.next_named_sibling();
    }
    None
}

/// The symbol-bearing declaration a documented sibling introduces, unwrapping
/// declaration wrappers such as `export_statement` and `lexical_declaration`
/// so a JSDoc above `export const f = …` anchors to `f`.
fn symbol_target<'tree>(mut node: Node<'tree>, language: Language) -> Option<Node<'tree>> {
    for _ in 0..8 {
        if symbol_field(language, node.kind()).is_some() {
            return Some(node);
        }
        node = declaration_child(node, language)?;
    }
    None
}

fn declaration_child<'tree>(node: Node<'tree>, language: Language) -> Option<Node<'tree>> {
    if let Some(child) = node.child_by_field_name("declaration") {
        return Some(child);
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find(|child| symbol_field(language, child.kind()).is_some())
}

/// The field carrying the declared name of a symbol-bearing node kind.
///
/// The allowlist is deliberate: generic `name` sniffing would anchor a
/// comment inside a TypeScript `enum_body` to the first member, making finding
/// IDs churn when members are reordered. Containers that merely hold
/// declarations contribute nothing.
fn symbol_field(language: Language, kind: &str) -> Option<&'static str> {
    match language {
        Language::Rust => match kind {
            "impl_item" => Some("type"),
            "function_item" | "struct_item" | "enum_item" | "union_item" | "trait_item"
            | "mod_item" | "const_item" | "static_item" | "type_item" | "macro_definition" => {
                Some("name")
            }
            _ => None,
        },
        Language::Python => match kind {
            "function_definition" | "class_definition" => Some("name"),
            _ => None,
        },
        Language::TypeScript => match kind {
            "function_declaration"
            | "method_definition"
            | "class_declaration"
            | "interface_declaration"
            | "type_alias_declaration"
            | "enum_declaration"
            | "variable_declarator"
            | "module"
            | "internal_module" => Some("name"),
            _ => None,
        },
    }
}

fn ancestor_name<'a>(node: Node<'_>, language: Language, source: &'a str) -> Option<&'a str> {
    let field = symbol_field(language, node.kind())?;
    let child = node.child_by_field_name(field)?;
    let name = source.get(child.byte_range())?.trim();
    (!name.is_empty()).then_some(name)
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
