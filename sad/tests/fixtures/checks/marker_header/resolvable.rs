//! Purpose: provide a resolvable traceability marker fixture.
//! Responsibilities: carry a default-prefix marker and a custom-prefix marker.
//! Rationale: resolver behavior and prefix behavior can share one parse-clean file.

// see sad-lcy: cap rule comes from review
pub fn cap() -> usize {
    1
}

// ticket sad-lcy: custom-prefix marker, prose until the prefix is configured
pub fn ticket_cap() -> usize {
    1
}