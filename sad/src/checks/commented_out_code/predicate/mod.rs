//! Purpose: decide whether a normalised comment-block body is a syntactic
//! unit — the deterministic predicate behind `commented_out_code`.
//! Responsibilities: enforce rules 1–5 (clean parse, non-empty root, closed
//! root-allow table, at least one code-shaped root child, deny outranking code
//! shape) and keep every language fact in the sibling `tables` module.
//! Separator bodies with no alphanumeric character are rejected by the caller
//! before parsing, so a ruler never reaches this module.
//! Rationale: a regex cannot tell code from prose, so the predicate asks the
//! pinned grammar. Precision comes from the root shape: a declaration or a
//! control statement is code, while a root made only of bare expression
//! statements (calls, assignments, new-expressions, formulas, annotations) is
//! the observed field false positive and stays prose. The tables are
//! transcribed against the workspace-pinned grammars; unknown kinds are absent
//! from every table and therefore fail closed to prose, giving a grammar bump
//! exactly one audit point.

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
    children.iter().any(|child| code_shaped(*child, table))
        && !children.iter().any(|child| denied(*child, table))
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

/// Rule 4: at least one root child is a declaration or a control statement.
///
/// Rust wraps control expressions in `expression_statement`, so that payload
/// is unwrapped before the lookup; Python and TypeScript name their control
/// statements directly at the root. Bare payloads — calls, assignments,
/// new-expressions, formulas, annotations — are absent from `code_shape` and
/// therefore never make a block code.
fn code_shaped(node: Node<'_>, table: &Tables) -> bool {
    if table.code_shape.contains(&node.kind()) {
        return true;
    }
    node.kind() == "expression_statement"
        && node
            .named_child(0)
            .is_some_and(|inner| table.code_shape.contains(&inner.kind()))
}

/// Rule 5: a denied root child outranks any code shape the tree contains.
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
    fn declaration_and_control_shapes_fire() {
        for body in ["fn f() {}", "let x = 1;", "use a::b;", "struct S;"] {
            assert!(
                fires(Language::Rust, body),
                "Rust {body:?} is a declaration and must fire"
            );
        }
        for body in ["if x { f(); }", "for x in xs { f(x); }"] {
            assert!(
                fires(Language::Rust, body),
                "Rust {body:?} is a control statement and must fire"
            );
        }
        for body in ["for i in xs:\n    f(i)", "import os", "def f():\n    pass"] {
            assert!(fires(Language::Python, body), "Python {body:?} must fire");
        }
        for body in [
            "const x = await load();",
            "function f() {}",
            "if (x) { f(); }",
        ] {
            assert!(
                fires(Language::TypeScript, body),
                "TypeScript {body:?} must fire"
            );
        }
    }

    #[test]
    fn bare_expression_payloads_stay_prose() {
        for body in [
            "f();",
            "x = 1;",
            "e.g. g();",
            "E = m * c^2",
            "return x;",
            "println!(\"x\");",
        ] {
            assert!(
                !fires(Language::Rust, body),
                "Rust {body:?} is a bare expression and must stay prose"
            );
        }
        for body in ["x = f()", "x = 1", "return x", "raise ValueError()", "5"] {
            assert!(
                !fires(Language::Python, body),
                "Python {body:?} is a bare expression and must stay prose"
            );
        }
        for body in ["f();", "new Foo();", "x = 1;", "throw x;", "foo"] {
            assert!(
                !fires(Language::TypeScript, body),
                "TypeScript {body:?} is a bare expression and must stay prose"
            );
        }
    }

    #[test]
    fn root_deny_outranks_code_shape() {
        assert!(
            !fires(Language::Rust, "fn f() {}\nfn g();"),
            "a bodyless signature outranks the sibling function item"
        );
        assert!(
            !fires(Language::TypeScript, "function f() {}\nNote: retry();"),
            "a labeled statement outranks the sibling declaration"
        );
        assert!(
            !fires(Language::Python, "def f():\n    pass\nx: int"),
            "annotation-only assignment outranks the sibling definition"
        );
        assert!(
            !fires(Language::Python, "list[str] | None"),
            "a type expression is not code-shaped"
        );
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
