//! Purpose: contract tests for file-metadata exemption — which leading comments
//! are file metadata or licence text rather than prose.
//! Responsibilities: pin the shebang and coding cookie as exemption-class rows,
//! keep a late shebang in prose, hold the whole-file exemption rule when
//! narration sits beside metadata, and pin the SPDX-and-copyright licence shape
//! against licence words that carry no marker.
//! Rationale: exemption is decided by comment position and shape alone, so the
//! tests drive `analyze_file` and the outside-channel check over the same
//! fixture sources the gate sees, with no hand-built nodes to drift.

mod support;

use sad::checks::outside_channel;
use sad::config::Config;
use sad::exemptions::summary;
use sad::model::AttributionClass;

use support::prose::{analyze, nodes_of};

const PYTHON_METADATA: &str = include_str!("fixtures/checks/prose/python_metadata.py");

#[test]
fn python_shebang_and_coding_cookie_are_file_metadata() {
    let cfg = Config::default();
    let file = analyze("python_metadata.py", PYTHON_METADATA, &cfg);

    let attribution: Vec<(usize, AttributionClass)> = file
        .nodes
        .iter()
        .map(|node| (node.line, node.class))
        .collect();
    assert_eq!(
        attribution,
        vec![
            (1, AttributionClass::Exemption),
            (2, AttributionClass::Exemption),
        ],
        "the shebang and the coding cookie are exemption-class nodes"
    );
    assert!(
        file.exempt,
        "a file whose only comments are file metadata is exempt"
    );

    let stats = summary(std::slice::from_ref(&file));
    assert_eq!(stats.files_exempt, 1);
    assert_eq!(stats.nodes_exempt, 2, "exempt-but-observable, like SPDX");
    assert!(outside_channel::run(&file, &cfg).is_empty());
}

#[test]
fn shebang_after_the_first_line_stays_prose() {
    let cfg = Config::default();
    let source = "VALUE = 1\n#!/usr/bin/env python3\n";
    let file = analyze("late_shebang.py", source, &cfg);

    let prose = nodes_of(&file, AttributionClass::Prose);
    assert_eq!(
        prose.len(),
        1,
        "only a row-0 shebang is file metadata; the late one is prose"
    );
    assert_eq!(prose[0].line, 2);

    let findings = outside_channel::run(&file, &cfg);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].line, 2);
}

#[test]
fn file_metadata_does_not_silence_other_prose() {
    let cfg = Config::default();
    let source =
        "#!/usr/bin/env python3\n# coding: utf-8\n\nVALUE = 1\n# sum every value in the table\n";
    let file = analyze("metadata_narration.py", source, &cfg);

    assert_eq!(
        nodes_of(&file, AttributionClass::Exemption).len(),
        2,
        "both metadata lines stay exemption-class"
    );
    assert!(
        !file.exempt,
        "the narration keeps the file out of whole-file exemption"
    );

    let prose = nodes_of(&file, AttributionClass::Prose);
    assert_eq!(prose.len(), 1);
    assert_eq!(prose[0].line, 5);

    let findings = outside_channel::run(&file, &cfg);
    assert_eq!(findings.len(), 1, "file metadata never silences prose");
    assert_eq!(findings[0].line, 5);
}

#[test]
fn copyright_line_beside_spdx_is_licence_text() {
    let cfg = Config::default();
    let source = "// SPDX-License-Identifier: MIT\n// Copyright 2026 Charly Vibes\n\npub fn licensed() -> u8 {\n    1\n}\n";
    let file = analyze("licence_spdx.rs", source, &cfg);

    assert_eq!(
        nodes_of(&file, AttributionClass::Exemption).len(),
        2,
        "the SPDX tag and the copyright line are both licence metadata"
    );
    assert!(
        file.exempt,
        "licence-only comments leave no prose debt in the file"
    );

    let stats = summary(std::slice::from_ref(&file));
    assert_eq!(stats.files_exempt, 1);
    assert_eq!(stats.nodes_exempt, 2);
    assert!(outside_channel::run(&file, &cfg).is_empty());
}

#[test]
fn licence_words_without_a_marker_stay_prose() {
    let cfg = Config::default();
    let source = "// Copyright is a legal concept, not a licence header\n// License negotiation happens elsewhere\n\npub fn plain() -> u8 {\n    1\n}\n";
    let file = analyze("licence_words.rs", source, &cfg);

    assert!(
        nodes_of(&file, AttributionClass::Exemption).is_empty(),
        "a bare `Copyright` word does not match the bounded licence shape"
    );
    assert_eq!(nodes_of(&file, AttributionClass::Prose).len(), 2);
    assert_eq!(
        outside_channel::run(&file, &cfg).len(),
        2,
        "licence-shaped words without a marker remain findings"
    );
}
