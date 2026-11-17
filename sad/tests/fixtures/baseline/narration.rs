//! Purpose: hold exactly one narration comment inside a function body.
//! Responsibilities: keep the header and doc channels clean so only prose is
//! finding-eligible.
//! Rationale: the baseline tests need one stable, content-hashed finding id.

/// Returns the sum of the items.
pub fn total(items: &[i32]) -> i32 {
    // accumulate every item into the running total
    items.iter().sum()
}