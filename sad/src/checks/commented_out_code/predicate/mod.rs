//! Purpose: decide whether a normalised comment-block body is a syntactic
//! unit — the deterministic predicate behind `commented_out_code`.
//! Responsibilities: enforce rules 1–5 (clean parse, non-empty root, closed
//! root-allow table, at least one shape node, root-deny outranking shape) and
//! keep every language fact in the sibling `tables` module.
//! Rationale: a regex cannot tell code from prose, so the predicate asks the
//! pinned grammar. The tables are transcribed against the workspace-pinned
//! grammars; unknown kinds are absent from every table and therefore fail
//! closed to prose, giving a grammar bump exactly one audit point.

use tree_sitter::Node;

use crate::model::Language;

use self::tables::{Tables, tables};

mod tables;

/// True when `root` satisfies every rule of the static predicate.
pub(super) fn is_commented_out_code(root: Node<'_>, language: Language) -> bool {
    if !clean_tree(root) {
        return false;
    }
    let children = root_children(root);
    if children.is_empty() {
        return false;
    }
    let table = tables(language);
    if !children
        .iter()
        .all(|child| table.root_allow.contains(&child.kind()))
    {
        return false;
    }
    has_shape(root, table) && !children.iter().any(|child| denied(*child, table))
}

/// Rule 1: no `ERROR` node and no node flagged missing anywhere in the tree.
fn clean_tree(root: Node<'_>) -> bool {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if node.is_error() || node.is_missing() {
            return false;
        }
        let mut cursor = node.walk();
        let children: Vec<Node<'_>> = node.children(&mut cursor).collect();
        stack.extend(children);
    }
    true
}

/// The root's named children with comment nodes ignored.
fn root_children<'tree>(root: Node<'tree>) -> Vec<Node<'tree>> {
    let mut cursor = root.walk();
    root.named_children(&mut cursor)
        .filter(|child| !is_comment(child.kind()))
        .collect()
}

fn is_comment(kind: &str) -> bool {
    matches!(kind, "line_comment" | "block_comment" | "comment")
}

/// Rule 4: some node in the tree carries a shape kind.
///
/// Argument-shaped keywords count only when they actually carry an argument,
/// so a bare `return` never makes a block code.
fn has_shape(root: Node<'_>, table: &Tables) -> bool {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if shape_node(node, table) {
            return true;
        }
        let mut cursor = node.walk();
        let children: Vec<Node<'_>> = node.named_children(&mut cursor).collect();
        stack.extend(children);
    }
    false
}

fn shape_node(node: Node<'_>, table: &Tables) -> bool {
    let kind = node.kind();
    if table.argument_shape.contains(&kind) {
        return node.named_child_count() > 0;
    }
    table.shape.contains(&kind)
}

/// Rule 5: a denied root child outranks any shape the tree contains.
fn denied(node: Node<'_>, table: &Tables) -> bool {
    if table.root_deny.contains(&node.kind()) {
        return true;
    }
    if node.kind() != "expression_statement" {
        return false;
    }
    let Some(inner) = node.named_child(0) else {
        return false;
    };
    table.payload_deny.contains(&inner.kind()) || annotation_only(inner)
}

/// Python `x: T` with no `right`: an annotation, never an assignment.
fn annotation_only(node: Node<'_>) -> bool {
    node.kind() == "assignment"
        && node.child_by_field_name("type").is_some()
        && node.child_by_field_name("right").is_none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::parser_for;

    fn fires(language: Language, body: &str) -> bool {
        let mut parser = parser_for(language).expect("grammar is pinned");
        let tree = parser
            .parse(body, None)
            .expect("tree-sitter parses the body");
        is_commented_out_code(tree.root_node(), language)
    }

    #[test]
    fn each_language_fires_on_item_and_statement_shapes() {
        for body in [
            "fn f() {}",
            "let x = 1;",
            "x = 1;",
            "f();",
            "use a::b;",
            "mod m {}",
        ] {
            assert!(fires(Language::Rust, body), "Rust {body:?} must fire");
        }
        for body in [
            "x = f()",
            "for i in xs:\n    f(i)",
            "import os",
            "def f():\n    pass",
        ] {
            assert!(fires(Language::Python, body), "Python {body:?} must fire");
        }
        for body in [
            "const x = await load();",
            "f();",
            "function f() {}",
            "new Foo();",
        ] {
            assert!(
                fires(Language::TypeScript, body),
                "TypeScript {body:?} must fire"
            );
        }
    }

    #[test]
    fn root_deny_outranks_shape() {
        assert!(fires(Language::Rust, "e.g. g();"), "the call shape fires");
        assert!(
            !fires(Language::Rust, "fn f();"),
            "a bodyless signature stays prose"
        );
        assert!(
            !fires(Language::TypeScript, "Note: retry();"),
            "a labeled statement stays prose despite its call payload"
        );
        assert!(
            !fires(Language::Python, "TODO: refactor"),
            "annotation-only assignment stays prose"
        );
        assert!(!fires(Language::Python, "list[str] | None"));
    }

    #[test]
    fn keyword_only_argument_shapes_do_not_count() {
        assert!(!fires(Language::Rust, "return;"));
        assert!(!fires(Language::Python, "return"));
        assert!(!fires(Language::TypeScript, "return;"));
        assert!(fires(Language::Rust, "return x;"));
        assert!(fires(Language::Python, "return x"));
        assert!(fires(Language::TypeScript, "throw x;"));
    }

    #[test]
    fn unknown_and_error_trees_fail_closed() {
        assert!(!fires(
            Language::Rust,
            "This function calculates the total."
        ));
        assert!(!fires(Language::Python, "5"));
        assert!(!fires(Language::TypeScript, "foo"));
    }
}
