//! Purpose: hold exactly one prose comment inside a named function.
//! Responsibilities: keep the header channel intact so only narration is flagged.
//! Rationale: the envelope test needs a stable symbol-scoped finding.

fn helper() {
    // sum every item through the iterator
    let total = items.iter().sum();
}