/* Purpose: keep a multi-line block comment and its adjacent line comment in
   Responsibilities: one header unit, with the third keyword on the next source line. */
// Rationale: a block comment's later lines still belong to the same first block.

pub fn multiline() -> bool {
    true
}