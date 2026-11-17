//! Purpose: provide a version-number lookalike comment fixture.
//! Responsibilities: show that an id-shaped token without the prefix is prose.
//! Rationale: only a marker-prefixed leading token reaches the marker channel.

// requires serde-1 compat
pub fn requires() -> bool {
    true
}