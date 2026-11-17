//! Purpose: provide a marker citing an openspec capability name.
//! Responsibilities: let the openspec positive resolver satisfy the cite.
//! Rationale: a resolvable openspec id must never be reported as fabricated.

// see comment-gate: gate flow spec
pub fn gate() -> bool {
    true
}