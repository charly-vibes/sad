//! Purpose: shared helpers for the integration-test binaries beside this
//! directory, split by the contract family they serve.
//! Responsibilities: hold the disposable-repository plumbing (`envelope`), the
//! baseline file plumbing (`baseline`), and the fixture analysis helpers
//! (`extract`) so no test binary duplicates them.
//! Rationale: each binary is its own crate, so sharing happens through a plain
//! module rather than a library; the split keeps the `TempRepo` shape each
//! contract expects while a binary only pays for the family it includes.

#![allow(dead_code)]

pub mod baseline;
pub mod envelope;
pub mod extract;
