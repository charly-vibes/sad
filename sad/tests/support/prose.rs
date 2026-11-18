//! Purpose: fixture analysis helpers shared by the prose contract tests and the
//! file-metadata exemption contract tests.
//! Responsibilities: analyse a prose fixture exactly as the gate would — source
//! from the fixture directory, stable repo-relative path for finding ids — and
//! project the resulting nodes back out by attribution class.
//! Rationale: both binaries read the same fixtures through the same frontend, so
//! the plumbing lives once beside them instead of drifting between two copies.

use std::path::{Path, PathBuf};

use sad::config::Config;
use sad::extract::analyze_file;
use sad::model::{AnalyzedFile, AttributionClass, CommentNode};

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/checks/prose")
        .join(name)
}

/// Analyses a prose fixture exactly as the gate would: source from the fixture
/// file, stable repo-relative path for finding ids.
pub fn analyze(name: &str, source: &str, cfg: &Config) -> AnalyzedFile {
    let rel_path = format!("tests/fixtures/checks/prose/{name}");
    analyze_file(&fixture_path(name), &rel_path, source, cfg)
}

pub fn nodes_of(file: &AnalyzedFile, class: AttributionClass) -> Vec<&CommentNode> {
    file.nodes
        .iter()
        .filter(|node| node.class == class)
        .collect()
}
