//! Purpose: `sad` — documentation gate CLI library root.
//! Responsibilities: expose the [`cli`] module (envelope dispatch), the
//! [`config`] module (`sad.toml` schema and loading), the [`baseline`] module
//! (`sad-baseline.json` adoption and shrink-only gate), the [`model`] data
//! contract, the [`extract`] tree-sitter frontends, [`attribution`] channel
//! classification, the [`checks`] gate checks, the [`check`] pipeline that
//! runs them over a tree, [`remediation`] fix guidance, [`envelope`]
//! (the genesis Check envelope), and [`render`] (text mode) to the thin
//! binary and to integration tests.
//! Rationale: keeping logic in the lib target lets `tests/` drive the exact
//! code path the binary runs, so CLI contract tests can't drift from reality.

pub mod attribution;
pub mod baseline;
pub mod check;
pub mod checks;
pub mod cli;
pub mod config;
pub mod envelope;
pub mod exemptions;
pub mod extract;
pub mod hooks;
pub mod model;
pub mod remediation;
pub mod render;
pub mod scope;
