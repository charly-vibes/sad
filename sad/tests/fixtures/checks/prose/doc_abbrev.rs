//! Purpose: document helpers whose summaries carry abbreviations, version
//! tokens, and inline code.
//! Responsibilities: exercise the periods that the sentence boundary rule
//! must ignore.
//! Rationale: abbreviation and code-span periods are not sentence ends, even
//! when an uppercase word follows them.

/// Parse e.g. v0.1 manifests from Node.js
pub fn parse_legacy() {}

/// Parse `a.b` values.
pub fn parse_dotted() {}

/// Resolve i.e. Absolute paths only.
pub fn resolve_absolute() {}