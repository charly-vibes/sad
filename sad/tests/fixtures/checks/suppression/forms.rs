//! Purpose: exercise the Rust suppression directive forms.
//! Responsibilities: hold one eslint directive and one sad directive.
//! Rationale: every directive form must be counted exactly once.

/// Adds two numbers.
pub fn add(a: i32, b: i32) -> i32 {
    // eslint-disable-next-line
    a + b
}

/// Returns a constant.
pub fn constant() -> u8 {
    // sad:ignore comment_outside_channel reason here
    7
}