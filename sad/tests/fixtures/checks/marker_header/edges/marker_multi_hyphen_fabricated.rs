//! Purpose: provide a marker whose id carries two hyphens.
//! Responsibilities: let the resolver report the full hyphenated id.
//! Rationale: a truncated id would accidentally resolve against `sad-lcy`.

// see sad-lcy-extra: cap rule comes from review
pub fn cap() -> usize {
    1
}