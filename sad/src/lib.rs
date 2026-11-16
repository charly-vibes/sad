//! Purpose: `sad` — documentation gate CLI library root.
//! Responsibilities: expose the [`cli`] module (envelope dispatch) to the
//! thin binary and to integration tests.
//! Rationale: keeping logic in the lib target lets `tests/` drive the exact
//! code path the binary runs, so CLI contract tests can't drift from reality.

pub mod cli;
