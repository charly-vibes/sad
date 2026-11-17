//! Purpose: pin CRLF normalisation.
//! Responsibilities: classify CRLF comments identically to LF comments.
//! Rationale: bodies must never leak carriage returns.

// see sad-lcy: cap rule from review

/// Adds two numbers.
pub fn add(a: i32, b: i32) -> i32 {
    // this function calculates the total
    a + b
}
