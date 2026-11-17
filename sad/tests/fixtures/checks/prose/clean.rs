//! Purpose: hold only header, doc, marker, and suppression nodes.
//! Responsibilities: carry no prose anywhere in the file.
//! Rationale: a clean channel mix must produce no outside-channel finding.

/// Returns the sum of the items.
pub fn sum(items: &[i32]) -> i32 {
    // see sad-lcy: cap rule from review
    // sad:ignore comment_outside_channel reason here
    items.iter().sum()
}