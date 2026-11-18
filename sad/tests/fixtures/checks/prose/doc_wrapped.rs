//! Purpose: document a parser with one sentence wrapped over four doc lines.
//! Responsibilities: exercise the sentence-based summary cap against rendered
//! line count.
//! Rationale: a single sentence must pass no matter how the source wraps it.

/// Parses the config from the given text and returns the number of bytes
/// read from the underlying path, validating every key as it goes so a
/// malformed manifest is rejected before any state is mutated or any
/// output is emitted.
///
/// # Errors
///
/// Returns an error when the file cannot be read.
pub fn parse_config(text: &str) -> usize {
    text.len()
}