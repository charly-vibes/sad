//! Purpose: render an emitted envelope JSON string as the one-line text mode.
//! Responsibilities: pick the check or suite rendering, summarise the verdict
//! and counts, name the first finding with its remediation, explain an empty
//! attribution scope, and name every skipped source with its reason and every
//! skipped source directory. The check line is assembled from a verdict
//! prefix, an optional first-finding clause, and the attribution and skip
//! suffixes, each built by its own helper.
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
        if let Some(line) = hooks_text(&v) {
            return line;
        }
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

/// One-line `hooks` summary: the command and each stage outcome.
///
/// The outcome vocabulary disambiguates the subcommand: `removed` and
/// `not_wired` belong to uninstall, the install outcomes to install —
/// text mode is a convenience rendering of the same envelope, so the
/// command name derives from the data instead of duplicating it.
fn hooks_text(v: &Value) -> Option<String> {
    let commit = v["data"]["pre_commit"].as_str()?;
    let push = v["data"]["pre_push"].as_str()?;
    let uninstall =
        matches!(commit, "removed" | "not_wired") || matches!(push, "removed" | "not_wired");
    let command = if uninstall {
        "sad hooks uninstall"
    } else {
        "sad hooks install"
    };
    Some(format!(
        "{command} — pre-commit: {commit}; pre-push: {push}\n"
    ))
}

/// One-line check summary: verdict, severity counts, first finding, and skip
/// signals.
///
/// Names the blocking findings apart from warnings once anything is reported,
/// includes the first finding as `path:line check_id` with its remediation
/// command when the run fails, the attribution hint when nothing was selected,
/// and every skipped source with its reason.
fn check_text(v: &Value) -> String {
    let stats = &v["data"]["stats"];
    let ok = v["ok"].as_bool().unwrap_or(false);
    let mut line = verdict_prefix(stats, ok);
    if let Some(clause) = first_finding_clause(&v["data"]["findings"], ok) {
        line.push_str(&clause);
    }
    if let Some(clause) = attribution_clause(stats) {
        line.push_str(&clause);
    }
    if let Some(suffix) = skipped_suffix(&v["warnings"], "unreadable_source", "file", true) {
        line.push_str(&suffix);
    }
    if let Some(suffix) = skipped_suffix(
        &v["warnings"],
        "skipped_source_directory",
        "directory",
        false,
    ) {
        line.push_str(&suffix);
    }
    line.push('\n');
    line
}

/// The attribution hint when nothing was selected but files went
/// unattributed; `None` when the attribution scope excluded nothing.
fn attribution_clause(stats: &Value) -> Option<String> {
    let files = stats["files_analyzed"].as_u64().unwrap_or(0);
    let selected = stats["files_selected"].as_u64().unwrap_or(files);
    let unattributed = stats["files_unattributed"].as_u64().unwrap_or(0);
    if selected == 0 && unattributed > 0 {
        Some(format!(
            "; attribution scope excluded {} {}: commit with the configured trailer or run \
             `sad check --all`",
            unattributed,
            plural(unattributed, "file"),
        ))
    } else {
        None
    }
}

/// The verdict and count prefix: pass or fail, the severity counts, and the
/// analysed-file total.
///
/// A run with no findings always reads as a plain pass, so warnings alone keep
/// the passing verdict and only failures name the blocking count.
fn verdict_prefix(stats: &Value, ok: bool) -> String {
    let findings = stats["findings"].as_u64().unwrap_or(0);
    let blocking = stats["findings_blocking"].as_u64().unwrap_or(0);
    let warnings = findings.saturating_sub(blocking);
    let files = stats["files_analyzed"].as_u64().unwrap_or(0);
    if findings == 0 {
        format!(
            "sad check — pass: 0 findings in {files} {}",
            plural(files, "file"),
        )
    } else if ok {
        format!(
            "sad check — pass: 0 blocking, {warnings} {} in {files} {}",
            plural(warnings, "warning"),
            plural(files, "file"),
        )
    } else {
        format!(
            "sad check — fail: {blocking} blocking, {warnings} {} in {files} {}",
            plural(warnings, "warning"),
            plural(files, "file"),
        )
    }
}

/// The first-finding clause, separator included; only a failing run names one.
fn first_finding_clause(findings: &Value, ok: bool) -> Option<String> {
    if ok {
        return None;
    }
    first_finding_brief(findings).map(|brief| format!("; {brief}"))
}

/// The suffix naming every warning with `rule_name`, or `None` when there is none.
///
/// Text mode reads the machine contract rather than recomputing the skip set,
/// so it cannot drift from the JSON; `noun` labels the entities, and
/// `with_reason` appends the warning message in parentheses, which is how a
/// skipped source states why it was not judged. A skipped directory carries no
/// reason beyond its own path.
fn skipped_suffix(
    warnings: &Value,
    rule_name: &str,
    noun: &str,
    with_reason: bool,
) -> Option<String> {
    let mut entities: Vec<String> = Vec::new();
    for warning in warnings.as_array()? {
        if warning["rule_name"] != rule_name {
            continue;
        }
        let id = warning["entity_id"].as_str()?;
        entities.push(if with_reason {
            format!("{id} ({})", warning["message"].as_str()?)
        } else {
            id.to_string()
        });
    }
    if entities.is_empty() {
        return None;
    }
    let count = entities.len() as u64;
    Some(format!(
        "; skipped {count} {}: {}",
        plural(count, noun),
        entities.join(", "),
    ))
}

/// `count` copies of `noun`, pluralised by appending `s` or turning a trailing
/// `y` into `ies`.
fn plural(count: u64, noun: &str) -> String {
    if count == 1 {
        noun.to_string()
    } else if let Some(stem) = noun.strip_suffix('y') {
        format!("{stem}ies")
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
