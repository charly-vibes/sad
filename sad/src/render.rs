//! Purpose: render an emitted envelope JSON string as the one-line text mode.
//! Responsibilities: pick the check or suite rendering, summarise the verdict
//! and counts, name the first finding with its remediation, explain an empty
//! attribution scope, and name every skipped source with its reason.
//! Rationale: text mode is a convenience view of the same envelope, never a
//! second output format with different content, so every branch here reads the
//! JSON the machine consumer reads. Keeping the renderers out of `cli.rs`
//! bounds that module's size and keeps dispatch separate from presentation.

use serde_json::Value;

/// Render an envelope JSON string as a short human-readable line.
///
/// Text mode is a convenience rendering of the same envelope — never a
/// second output format with different content.
pub fn render_text(json: &str) -> String {
    let v: Value = serde_json::from_str(json).expect("run() always emits valid JSON");
    if v["envelope_kind"] == "check" {
        return check_text(&v);
    }
    if v["ok"].as_bool().unwrap_or(false) {
        let name = v["data"]["name"].as_str().unwrap_or("sad");
        let version = v["data"]["version"].as_str().unwrap_or("?");
        let status = v["data"]["status"].as_str().unwrap_or("ok");
        format!("{name} {version} — {status}\n")
    } else {
        format!(
            "error: {}\n",
            v["data"]["message"].as_str().unwrap_or("unknown error")
        )
    }
}

/// One-line check summary: verdict, counts, first finding, and skip signals.
///
/// Includes the first finding as `path:line check_id` with its remediation
/// command when the envelope carries one, the attribution hint when nothing
/// was selected, and every skipped source with its reason.
fn check_text(v: &Value) -> String {
    let stats = &v["data"]["stats"];
    let findings = stats["findings"].as_u64().unwrap_or(0);
    let files = stats["files_analyzed"].as_u64().unwrap_or(0);
    let verdict = if v["ok"].as_bool().unwrap_or(false) {
        "pass"
    } else {
        "fail"
    };
    let mut line = format!(
        "sad check — {verdict}: {findings} {} in {files} {}",
        plural(findings, "finding"),
        plural(files, "file"),
    );
    if let Some(brief) = first_finding_brief(&v["data"]["findings"]) {
        line.push_str("; ");
        line.push_str(&brief);
    }
    let selected = stats["files_selected"].as_u64().unwrap_or(files);
    let unattributed = stats["files_unattributed"].as_u64().unwrap_or(0);
    if selected == 0 && unattributed > 0 {
        line.push_str(&format!(
            "; attribution scope excluded {} {}: commit with the configured trailer or run \
             `sad check --all`",
            unattributed,
            plural(unattributed, "file"),
        ));
    }
    if let Some(suffix) = skipped_suffix(&v["warnings"]) {
        line.push_str(&suffix);
    }
    line.push('\n');
    line
}

/// The skipped-source suffix, read from the `unreadable_source` warnings.
///
/// Text mode reads the machine contract rather than recomputing the skip set,
/// so it cannot drift from the JSON. `None` when nothing was skipped.
fn skipped_suffix(warnings: &Value) -> Option<String> {
    let mut skipped: Vec<String> = Vec::new();
    for warning in warnings.as_array()? {
        if warning["rule_name"] == "unreadable_source" {
            skipped.push(format!(
                "{} ({})",
                warning["entity_id"].as_str()?,
                warning["message"].as_str()?
            ));
        }
    }
    if skipped.is_empty() {
        return None;
    }
    let count = skipped.len() as u64;
    Some(format!(
        "; skipped {count} {}: {}",
        plural(count, "file"),
        skipped.join(", "),
    ))
}

/// `count` copies of `noun`, pluralised by appending `s`.
fn plural(count: u64, noun: &str) -> String {
    if count == 1 {
        noun.to_string()
    } else {
        format!("{noun}s")
    }
}

/// The first reported finding, with its remediation command when present.
///
/// Rendered as `path:line check_id`; `None` when there is no finding to name.
fn first_finding_brief(findings: &Value) -> Option<String> {
    let finding = findings.as_array()?.first()?;
    let path = finding["path"].as_str()?;
    let line = finding["line"].as_u64()?;
    let check = finding["check"].as_str()?;
    let mut brief = format!("{path}:{line} {check}");
    if let Some(command) = finding["remediation"]["command"]
        .as_str()
        .filter(|command| !command.is_empty())
    {
        brief.push_str(&format!(" (fix: {command})"));
    }
    Some(brief)
}
