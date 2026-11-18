//! Purpose: fixture and emission helpers shared by the contract-test binaries
//! for the block-scoped `commented_out_code` check.
//! Responsibilities: locate and analyse the check's fixtures, run the check
//! and the outside-channel fallback together, and read node and finding
//! projections back out of them.
//! Rationale: the block-emission and predicate contract tests observe the same
//! frontend, config, and exclusion plumbing, so the helpers live once beside
//! them instead of drifting between two copies.

use std::path::{Path, PathBuf};

use sad::checks::{commented_out_code, outside_channel};
use sad::config::Config;
use sad::extract::analyze_file;
use sad::model::{AnalyzedFile, AttributionClass, Finding, FindingKind};

pub fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/checks/commented_out_code")
        .join(name)
}

pub fn rel_path(name: &str) -> String {
    format!("tests/fixtures/checks/commented_out_code/{name}")
}

pub fn fixture(name: &str) -> AnalyzedFile {
    let source = std::fs::read_to_string(fixture_path(name))
        .unwrap_or_else(|err| panic!("fixture {name} must be readable: {err}"));
    source_at(name, &source)
}

pub fn source_at(name: &str, source: &str) -> AnalyzedFile {
    analyze_file(
        &fixture_path(name),
        &rel_path(name),
        source,
        &Config::default(),
    )
}

pub fn split(file: &AnalyzedFile, cfg: &Config) -> (Vec<Finding>, Vec<Finding>) {
    let (dead, consumed) = commented_out_code::run(file, cfg);
    let prose = outside_channel::run_excluding(file, cfg, &consumed);
    (dead, prose)
}

pub fn prose_nodes(file: &AnalyzedFile) -> Vec<usize> {
    file.nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.class == AttributionClass::Prose && !node.body.trim().is_empty())
        .map(|(index, _)| index)
        .collect()
}

pub fn bodies(findings: &[Finding]) -> Vec<&str> {
    findings
        .iter()
        .map(|finding| finding.text.as_str())
        .collect()
}

pub fn assert_kinds(findings: &[Finding], kind: FindingKind) {
    assert!(
        findings.iter().all(|finding| finding.check == kind),
        "every finding must be {kind:?}: {:?}",
        bodies(findings)
    );
}
