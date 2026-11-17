//! Purpose: hold comment-looking text inside string literals.
//! Responsibilities: prove literals never yield comment nodes.
//! Rationale: nodes come from the grammar, not from scanning bytes.

pub fn literals() -> (&'static str, &'static str) {
    let s = "// not a comment";
    let block = "/* also not a comment */";
    // real narration
    (s, block)
}