//! Purpose: detect Python docstrings on modules, classes, and functions.
//! Responsibilities: find an item's first statement, accept string literals
//! only, and unwrap parenthesised and implicitly concatenated forms.
//! Rationale: CPython folds those forms into the item's `__doc__`, so
//! extraction must recognise them structurally rather than by text.

use tree_sitter::Node;

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
pub(crate) fn python_docstring<'tree>(node: Node<'tree>) -> Option<Node<'tree>> {
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
