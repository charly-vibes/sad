//! Purpose: provide a marker cite whose id ends in a trailing hyphen.
//! Responsibilities: let the resolver test the widened, trimmed id.
//! Rationale: attribution trims the trailing hyphen, so the check must judge
//! the same id instead of the literal token.

// see sad-lcy-: cap rule comes from review
pub fn cap() -> usize {
    1
}
