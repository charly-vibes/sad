// Note: this fixture deliberately starts with prose instead of a header.

pub fn first() -> bool {
    true
}

/// Purpose: name the first keyword in a doc block that is not the header.
/// Responsibilities: name the second keyword in a later doc block.
/// Rationale: the header check must inspect only the file's first block.
pub fn later() -> bool {
    true
}