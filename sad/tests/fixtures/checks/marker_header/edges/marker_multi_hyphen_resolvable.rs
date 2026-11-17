//! Purpose: provide a marker whose id carries two hyphens.
//! Responsibilities: let the resolver test the whole hyphenated id.
//! Rationale: truncating the id at the second hyphen invents a different id.

// see sad-lcy-extra: cap rule comes from review
pub fn cap() -> usize {
    1
}