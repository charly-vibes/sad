//! Purpose: pin one node per attribution class in Rust.
//! Responsibilities: hold header, doc, marker, suppression, exemption, prose.
//! Rationale: attribution is structural; every comment form maps to one class.

/// Adds two numbers.
pub fn add(a: i32, b: i32) -> i32 {
    // see sad-lcy: cap rule from review
    // sad:ignore comment_outside_channel reason here
    a + b
}

// SPDX-License-Identifier: Apache-2.0

/// see sad-lcy: doc comments never become markers.
pub fn documented() -> &'static str {
    "documented"
}

// this function calculates the total
pub fn total(items: &[i32]) -> i32 {
    items.iter().sum()
}