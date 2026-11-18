//! Purpose: expose suppression debt as counted-but-exempt data and enforce the
//! optional per-file suppression cap.
//! Responsibilities: turn every suppression-class comment node into one
//! [`SuppressionEntry`] in source order (path, line, form, scoped check id),
//! and emit exactly one file-scoped `suppression_cap_exceeded` finding when a
//! file's debt exceeds `cfg.suppression_cap` and the check is enabled; the
//! finding's stable id hashes a sorted directive token list, never a line.
//! Rationale: suppression debt is exempt-but-counted, so both the debt ledger
//! and the cap breach must be pure functions of the analyzed file plus config;
//! one entry per directive keeps a missed directive or a silently disabled cap
//! visible instead of letting the gate be routed around.

use crate::config::Config;
use crate::model::{
    AnalyzedFile, AttributionClass, Finding, FindingKind, Severity, SuppressionEntry,
    SuppressionForm, finding_id,
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
/// debt equal to the cap is tolerated.
pub fn run(file: &AnalyzedFile, cfg: &Config) -> Vec<Finding> {
    let Some(cap) = cap_in_force(cfg) else {
        return Vec::new();
    };
    let debt = entries(file);
    if debt.len() <= cap {
        return Vec::new();
    }
    vec![cap_finding(file, &debt, cap)]
}

/// The cap to enforce, when the cap check is enabled and a cap is configured.
///
/// Both a disabled toggle and an absent cap resolve to "no cap in force", so
/// callers need not repeat those two guards.
fn cap_in_force(cfg: &Config) -> Option<usize> {
    if !cfg.checks.suppression_cap_exceeded {
        return None;
    }
    cfg.suppression_cap
}

/// The single file-scoped `suppression_cap_exceeded` finding for `debt`.
///
/// The finding is anchored at line 1 and carries `-` as its symbol path. Its
/// stable id hashes the sorted multiset of directive forms (each with its
/// scoped check id, duplicates preserved) and never a line number, so moving a
/// directive within the file — or inserting code above it — leaves the id
/// unchanged, while adding, removing, or rescoping a directive changes it.
fn cap_finding(file: &AnalyzedFile, debt: &[SuppressionEntry], cap: usize) -> Finding {
    // Hash input: sorted `form` / `form:check` items, duplicates preserved.
    // Lines stay out of the id and only annotate the human-readable `text`.
    let mut identity = debt.iter().map(identity_key).collect::<Vec<_>>();
    identity.sort_unstable();
    Finding {
        check: FindingKind::SuppressionCapExceeded,
        id: finding_id(&file.path, "-", &identity.join("|")),
        path: file.path.clone(),
        line: 1,
        symbol_path: "-".to_string(),
        message: format!(
            "{} carries {} suppression directives, exceeding the configured cap of {}",
            file.path,
            debt.len(),
            cap
        ),
        text: debt
            .iter()
            .map(|entry| format!("{}@{}", form_id(entry.form), entry.line))
            .collect::<Vec<_>>()
            .join(","),
        severity: Severity::Error,
        smell: None,
        remediation: remediation::for_kind(FindingKind::SuppressionCapExceeded, &file.path),
    }
}

/// One identity token for the cap finding's id: form plus scoped check id.
///
/// The line never enters the token, and tokens are not deduplicated, so two
/// identical directives still contribute two tokens to the hash.
fn identity_key(entry: &SuppressionEntry) -> String {
    match entry.check.as_deref() {
        Some(check) => format!("{}:{check}", form_id(entry.form)),
        None => form_id(entry.form).to_string(),
    }
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
            next_code_line: None,
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

    /// Three directives against a cap of two: one unambiguous breach, reused
    /// by the id-stability tests.
    fn over_cap_nodes() -> Vec<CommentNode> {
        vec![
            directive_node(3, SuppressionForm::Noqa, None),
            directive_node(7, SuppressionForm::Noqa, None),
            directive_node(
                11,
                SuppressionForm::SadIgnore,
                Some("comment_outside_channel"),
            ),
        ]
    }

    #[test]
    fn cap_finding_id_ignores_line_drift_and_directive_order() {
        let cfg = cap_config(2);
        let nodes = over_cap_nodes();
        let original = run(&file_with(nodes.clone()), &cfg);
        assert_eq!(original.len(), 1);

        let mut drifted = nodes.clone();
        for node in &mut drifted {
            node.line += 40;
        }
        let mut reordered = nodes;
        reordered.reverse();

        for (label, variant) in [("line drift", drifted), ("reordering", reordered)] {
            let findings = run(&file_with(variant), &cfg);
            assert_eq!(findings.len(), 1);
            assert_eq!(
                findings[0].id, original[0].id,
                "the file-scoped id must survive {label}"
            );
        }
    }

    #[test]
    fn cap_finding_id_changes_when_a_directive_is_rescoped_or_added() {
        let cfg = cap_config(2);
        let nodes = over_cap_nodes();
        let original = run(&file_with(nodes.clone()), &cfg);
        assert_eq!(original.len(), 1);

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
