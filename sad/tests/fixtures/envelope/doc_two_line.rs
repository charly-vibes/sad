//! Purpose: hold a documentation summary longer than the one-line cap.
//! Responsibilities: keep the header block complete so only the doc cap fires.
//! Rationale: the exact rendered body must hash into the finding id.

/// First summary line.
/// Second summary line that exceeds the cap.
fn documented() -> u32 {
    1
}