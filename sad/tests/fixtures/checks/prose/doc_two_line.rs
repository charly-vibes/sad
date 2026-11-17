//! Purpose: document a parser with a two-line summary and no blank separator.
//! Responsibilities: exercise the default summary cap of one line.
//! Rationale: a two-line lead must fail the cap on the first doc line.

/// Parses the config at `path`.
/// Fails on malformed input.
pub fn parse_config(text: &str) -> usize {
    text.len()
}