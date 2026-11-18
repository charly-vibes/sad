//! Purpose: resolve the symbol path (`Foo::bar`) a comment or docstring
//! belongs to.
//! Responsibilities: join allowlisted symbol-bearing ancestors, resolve the
//! item an outer doc comment documents, and unwrap declaration wrappers such
//! as `export_statement` or `lexical_declaration`.
//! Rationale: symbolic attribution must use an explicit allowlist so member
//! reordering inside containers does not churn finding IDs.

use tree_sitter::Node;

use crate::model::Language;

/// Symbol path for a node: an outer doc comment carries the item it directly
/// documents, everything else carries its symbol-bearing ancestors; `-` when
/// the node sits outside every named item.
pub(crate) fn symbol_path(node: Node<'_>, language: Language, source: &str) -> String {
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
