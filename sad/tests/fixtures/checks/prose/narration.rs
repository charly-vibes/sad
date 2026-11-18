//! Purpose: hold one narration comment inside a function body.
//! Responsibilities: keep the prose channel separate from the header and doc
//! channels.
//! Rationale: exactly one prose node makes the expected finding unambiguous.

/// Returns the sum of the items.
pub fn total(items: &[i32]) -> i32 {
    // sum every item through the iterator
    items.iter().sum()
}