//! Purpose: detect Python docstrings on modules, classes, and functions.
//! Responsibilities: find an item's first statement, accept string literals
//! only, and unwrap parenthesised and implicitly concatenated forms.
//! Rationale: CPython folds those forms into the item's `__doc__`, so
//! extraction must recognise them structurally rather than by text.

use tree_sitter::Node;

/// First non-comment named child, so a leading Python comment cannot hide a
/// module docstring.
fn python_first_statement<'tree>(node: Node<'tree>) -> Option<Node<'tree>> {
    (0..node.named_child_count())
        .filter_map(|index| node.named_child(index))
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
    let value = first.named_child(0)?;
    string_literal(value)
}

/// Unwrap `parenthesized_expression`/`concatenated_string` wrappers down to
/// the first string literal, whose `string_content` carries the body.
fn string_literal<'tree>(node: Node<'tree>) -> Option<Node<'tree>> {
    let mut current = node;
    loop {
        match current.kind() {
            "string" => return Some(current),
            "parenthesized_expression" | "concatenated_string" => {
                current = (0..current.named_child_count())
                    .filter_map(|index| current.named_child(index))
                    .find(|child| {
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
