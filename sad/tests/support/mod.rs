//! Purpose: shared helpers for the integration-test binaries beside this
//! directory, split by the contract family they serve.
//! Responsibilities: hold the disposable-repository plumbing (`envelope`), the
//! baseline file plumbing (`baseline`), the fixture analysis helpers
//! (`extract`, `prose`), and the commented-out-code fixture helpers
//! (`commented_out_code`) so no test binary duplicates them.
//! Rationale: each binary is its own crate, so sharing happens through a plain
//! module rather than a library; the split keeps the `TempRepo` shape each
//! contract expects while a binary only pays for the family it includes.

#![allow(dead_code)]

pub mod baseline;
pub mod commented_out_code;
pub mod envelope;
pub mod extract;
pub mod hooks;
pub mod prose;
pub mod scope_repo;
