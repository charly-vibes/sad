//! Purpose: fixture analysis helpers shared by the extraction contract tests.
//! Responsibilities: locate an attribution fixture, analyse it exactly as the
//! gate would, and read nodes and node attribution back out of the result.
//! Rationale: the class/body tests and the identity/path tests read the same
//! fixtures, so the analysis plumbing must not drift between them.

use std::path::{Path, PathBuf};

use sad::config::Config;
use sad::extract::analyze_file;
use sad::model::{AnalyzedFile, AttributionClass, CommentKind, CommentNode};

pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/attribution")
}

/// Analyses a fixture on disk exactly as the gate would.
#[track_caller]
pub fn analyze(group: &str, name: &str) -> AnalyzedFile {
    let path = fixtures_dir().join(group).join(name);
    let rel_path = format!("tests/fixtures/attribution/{group}/{name}");
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("fixture {} must be readable: {err}", path.display()));
    analyze_file(&path, &rel_path, &source, &Config::default())
}

#[track_caller]
pub fn node_at(file: &AnalyzedFile, line: usize) -> &CommentNode {
    file.nodes
        .iter()
        .find(|node| node.line == line)
        .unwrap_or_else(|| panic!("{} must carry a comment node at line {line}", file.path))
}

#[track_caller]
pub fn assert_nodes(file: &AnalyzedFile, expected: &[(usize, CommentKind, AttributionClass)]) {
    let actual: Vec<(usize, CommentKind, AttributionClass)> = file
        .nodes
        .iter()
        .map(|node| (node.line, node.kind, node.class))
        .collect();
    assert_eq!(
        actual.as_slice(),
        expected,
        "node attribution of {} must match, in source order",
        file.path
    );
}
