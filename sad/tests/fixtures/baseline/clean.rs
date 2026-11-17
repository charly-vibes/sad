//! Purpose: hold a header and a documented public item with no prose.
//! Responsibilities: stay finding-free so baseline counts are unambiguous.
//! Rationale: adopting a tree with a known count needs a clean companion file.

/// Returns the sum of the items.
pub fn sum(items: &[i32]) -> i32 {
    items.iter().sum()
}