//! Purpose: document a parser with a one-line summary and structured sections.
//! Responsibilities: show that a structured `# Errors` section is not part of
//! the summary lead.
//! Rationale: the summary cap counts lead lines, not section content.

/// Parses the config at `path`.
///
/// # Errors
///
/// Returns an error when the file cannot be read.
/// Returns an error when a key is unknown.
/// Returns an error when a value has the wrong type.
/// Returns an error when the marker pattern is invalid.
pub fn parse_config(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| err.to_string())
}