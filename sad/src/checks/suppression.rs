//! Purpose: expose suppression debt as counted-but-exempt data and enforce the
//! optional per-file suppression cap.
//! Responsibilities: turn every suppression-class comment node into one
//! [`SuppressionEntry`] in source order (path, line, form, scoped check id),
//! and emit exactly one file-scoped `suppression_cap_exceeded` finding when a
//! file's debt exceeds `cfg.suppression_cap` and the check is enabled.
//! Rationale: suppression debt is exempt-but-counted, so both the debt ledger
//! and the cap breach must be pure functions of the analyzed file plus config;
//! one entry per directive keeps a missed directive or a silently disabled cap
//! visible instead of letting the gate be routed around.

use crate::config::Config;
use crate::model::{
    AnalyzedFile, AttributionClass, Finding, FindingKind, SuppressionEntry, SuppressionForm,
    finding_id,
};
use crate::remediation;

/// One debt entry per suppression-class node, in source order.
///
/// Doc nodes are never suppression nodes (doc precedence wins during
/// attribution), so they contribute no entries. Nodes whose directive failed
/// to parse cannot name a form and are likewise not counted.
pub fn entries(file: &AnalyzedFile) -> Vec<SuppressionEntry> {
    file.nodes
        .iter()
        .filter(|node| node.class == AttributionClass::Suppression)
        .filter_map(|node| {
            node.directive.as_ref().map(|directive| SuppressionEntry {
                path: file.path.clone(),
                line: node.line,
                form: directive.form,
                check: directive.check.clone(),
            })
        })
        .collect()
}

/// Emit at most one `suppression_cap_exceeded` finding for `file`.
///
/// A missing cap (`None`) is the v0.1 default and emits nothing; a disabled
/// toggle emits nothing even on breach. A breach is strictly `debt > cap`, so
/// debt equal to the cap is tolerated. The finding is file-scoped: it carries
/// `-` as its symbol path and is anchored at line 1.
///
/// Its stable id hashes the sorted multiset of directive forms (each with its
/// scoped check id, duplicates preserved) and never a line number, so moving a
/// directive within the file — or inserting code above it — leaves the id
/// unchanged, while adding, removing, or rescoping a directive changes it.
pub fn run(file: &AnalyzedFile, cfg: &Config) -> Vec<Finding> {
    if !cfg.checks.suppression_cap_exceeded {
        return Vec::new();
    }
    let Some(cap) = cfg.suppression_cap else {
        return Vec::new();
    };

    let debt = entries(file);
    if debt.len() <= cap {
        return Vec::new();
    }

    // Hash input: sorted `form` / `form:check` items, duplicates preserved.
    // Lines stay out of the id and only annotate the human-readable `text`.
    let mut identity = debt
        .iter()
        .map(|entry| match entry.check.as_deref() {
            Some(check) => format!("{}:{check}", form_id(entry.form)),
            None => form_id(entry.form).to_string(),
        })
        .collect::<Vec<_>>();
    identity.sort_unstable();
    let identity = identity.join("|");
    let directives = debt
        .iter()
        .map(|entry| format!("{}@{}", form_id(entry.form), entry.line))
        .collect::<Vec<_>>()
        .join(",");
    let message = format!(
        "{} carries {} suppression directives, exceeding the configured cap of {}",
        file.path,
        debt.len(),
        cap
    );

    vec![Finding {
        check: FindingKind::SuppressionCapExceeded,
        id: finding_id(&file.path, "-", &identity),
        path: file.path.clone(),
        line: 1,
        symbol_path: "-".to_string(),
        message,
        text: directives,
        remediation: remediation::for_kind(FindingKind::SuppressionCapExceeded),
    }]
}

/// Snake-case label for a directive form, matching its serialised spelling.
///
/// `form_id_matches_the_serialised_spelling` keeps this table in lockstep with
/// the `snake_case` serde rename on [`SuppressionForm`], so the debt ledger and
/// the cap finding's identity cannot drift apart.
fn form_id(form: SuppressionForm) -> &'static str {
    match form {
        SuppressionForm::Noqa => "noqa",
        SuppressionForm::TypeIgnore => "type_ignore",
        SuppressionForm::TsIgnore => "ts_ignore",
        SuppressionForm::EslintDisable => "eslint_disable",
        SuppressionForm::SadIgnore => "sad_ignore",
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{CommentKind, CommentNode, Language, SuppressionDirective, SuppressionForm};

    use super::*;

    fn directive_node(line: usize, form: SuppressionForm, check: Option<&str>) -> CommentNode {
        CommentNode {
            line,
            kind: CommentKind::Line,
            class: AttributionClass::Suppression,
            symbol_path: "-".to_string(),
            body: form_id(form).to_string(),
            text: form_id(form).to_string(),
            directive: Some(SuppressionDirective {
                form,
                check: check.map(str::to_string),
                reason: None,
            }),
        }
    }

    fn file_with(nodes: Vec<CommentNode>) -> AnalyzedFile {
        AnalyzedFile {
            path: "src/debt.py".to_string(),
            language: Language::Python,
            nodes,
            exempt: false,
            header: false,
        }
    }

    fn cap_config(cap: usize) -> Config {
        Config {
            suppression_cap: Some(cap),
            ..Config::default()
        }
    }

    #[test]
    fn form_id_matches_the_serialised_spelling() {
        for form in [
            SuppressionForm::Noqa,
            SuppressionForm::TypeIgnore,
            SuppressionForm::TsIgnore,
            SuppressionForm::EslintDisable,
            SuppressionForm::SadIgnore,
        ] {
            assert_eq!(
                serde_json::to_string(&form).expect("SuppressionForm must serialise"),
                format!("\"{}\"", form_id(form))
            );
        }
    }

    #[test]
    fn cap_finding_id_ignores_line_drift_and_directive_order() {
        let cfg = cap_config(2);
        let nodes = vec![
            directive_node(3, SuppressionForm::Noqa, None),
            directive_node(7, SuppressionForm::Noqa, None),
            directive_node(
                11,
                SuppressionForm::SadIgnore,
                Some("comment_outside_channel"),
            ),
        ];
        let original = run(&file_with(nodes.clone()), &cfg);
        assert_eq!(original.len(), 1);

        let mut drifted = nodes.clone();
        for node in &mut drifted {
            node.line += 40;
        }
        let mut reordered = nodes.clone();
        reordered.reverse();

        for (label, variant) in [("line drift", drifted), ("reordering", reordered)] {
            let findings = run(&file_with(variant), &cfg);
            assert_eq!(findings.len(), 1);
            assert_eq!(
                findings[0].id, original[0].id,
                "the file-scoped id must survive {label}"
            );
        }

        let mut rescoped = nodes.clone();
        rescoped[2].directive.as_mut().expect("directive").check = Some("missing_header".into());
        let mut grown = nodes;
        grown.push(directive_node(12, SuppressionForm::TypeIgnore, None));

        for (label, variant) in [
            ("a rescoped directive", rescoped),
            ("an added directive", grown),
        ] {
            let findings = run(&file_with(variant), &cfg);
            assert_eq!(findings.len(), 1);
            assert_ne!(
                findings[0].id, original[0].id,
                "the id must change for {label}"
            );
        }
    }
}
