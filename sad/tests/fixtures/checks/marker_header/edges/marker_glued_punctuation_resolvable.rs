//! Purpose: provide a marker cite whose id is followed by glued punctuation.
//! Responsibilities: let the resolver test the id the classifier extracted.
//! Rationale: attribution stops the id at the parenthesis, so the check must
//! judge `sad-lcy`, not the whole token `sad-lcy(sic)`.

// see sad-lcy(sic): the id ends at the parenthesis
pub fn cap() -> usize {
    1
}
