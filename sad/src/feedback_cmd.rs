//! Purpose: expose `sad feedback` as a genesis feedback envelope —
//! reporting-only, never part of the check gate.
//! Responsibilities: mirror the genesis feedback arguments with
//! [`FeedbackArgs`], route the request through
//! `genesis::feedback::handle_feedback` against `charly-vibes/sad`, and map
//! its [`genesis::feedback::gh::GhResult`] outcome (created issue, fallback
//! URL, or local file) into a success envelope via [`feedback_envelope`];
//! every failure, including an invalid issue kind, becomes an error envelope
//! with non-empty remediation. Failed commands record a local scratch entry
//! via [`record_last_error`] so `--from-last-error` has content to read.
//! Rationale: feedback is advisory — it reports an issue and never blocks or
//! alters `sad check` results — so this module only translates between the
//! genesis feedback API and sad's envelope conventions and takes no part in
//! gate verdicts.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use genesis::envelope::{Envelope, EnvelopeKind, ErrorResult, RemediationEntry};
use genesis::feedback::{self, gh, scratch};

/// Persist the last failed command for `sad feedback --from-last-error`,
/// best-effort: the scratch store is local and advisory, and a write
/// failure never changes the outcome being reported.
pub fn record_last_error(kind: &str, footer: Option<String>, exit: i32) {
    scratch::write_scratch_best_effort(
        "sad",
        &scratch::ErrorRecord {
            ts: rfc3339_now(),
            argv: std::env::args().collect(),
            exit,
            footer,
            kind: kind.to_string(),
        },
    );
}

/// Record a failed command envelope: the kind is the envelope kind and the
/// footer the first remediation command, so the scratch record answers the
/// same questions the printed envelope answers. The CLI builds the envelope
/// as a JSON string, so it is re-parsed here; an unparsable envelope
/// records nothing rather than failing the command being reported.
pub fn record_last_error_from_envelope(json: &str, exit: i32) {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(json) else {
        return;
    };
    let kind = json
        .get("envelope_kind")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("error");
    let footer = json
        .pointer("/data/remediation/0/command")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    record_last_error(kind, footer, exit);
}

/// Current UTC time as RFC 3339 — `ErrorRecord::ts` documents ISO 8601.
fn rfc3339_now() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| unix_secs_fallback())
}

/// Degraded timestamp when RFC 3339 formatting fails: whole seconds since
/// the Unix epoch (0 on clock failure), so the record still lands.
fn unix_secs_fallback() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

/// Sad-side mirror of `genesis::feedback::FeedbackArgs`, one word per field.
///
#[derive(Debug, Clone)]
pub struct FeedbackArgs {
    /// Kind of feedback: "bug", "feature", "question", or "chore".
    pub kind: String,
    /// User-supplied issue title overriding the derived one.
    pub title: Option<String>,
    /// If true, print the issue body and gh command without submitting.
    pub dry_run: bool,
    /// If true, read the last error from scratch to auto-populate the body.
    pub from_last_error: bool,
}

/// Serializable payload of the `sad feedback` success envelope: the
/// actionable fields of the gh fallback ladder outcome.
///
#[derive(Debug, serde::Serialize)]
struct FeedbackData {
    /// True when the report was handled (submitted, prepared, or dry-run).
    ok: bool,
    /// How the report landed: `submitted`, `fallback_url`, `local_file`, or
    /// `dry_run`.
    outcome: String,
    /// Issue URL when created, or the prefilled web URL for manual filing.
    url: Option<String>,
    /// Issue number when created.
    number: Option<u64>,
    /// Local file path when the network was unavailable.
    file: Option<String>,
    /// Human-readable summary of what happened.
    message: String,
}

/// Payload for a dry-run: the report was previewed, not submitted.
fn dry_run_data() -> FeedbackData {
    FeedbackData {
        ok: true,
        outcome: "dry_run".to_string(),
        url: None,
        number: None,
        file: None,
        message: "dry run: the issue was previewed, not submitted".to_string(),
    }
}

/// Payload for a created issue, carrying its URL and number.
fn created_data(url: String, number: u64) -> FeedbackData {
    FeedbackData {
        ok: true,
        outcome: "submitted".to_string(),
        url: Some(url),
        number: Some(number),
        file: None,
        message: format!("issue #{number} created"),
    }
}

/// Payload for the prefilled-URL fallback when gh is unavailable.
fn fallback_url_data(url: String) -> FeedbackData {
    FeedbackData {
        ok: true,
        outcome: "fallback_url".to_string(),
        url: Some(url),
        number: None,
        file: None,
        message: "gh unavailable: file the issue via the prefilled URL".to_string(),
    }
}

/// Payload for the local-file fallback when the network is unavailable.
fn local_file_data(path: PathBuf) -> FeedbackData {
    FeedbackData {
        ok: true,
        outcome: "local_file".to_string(),
        url: None,
        number: None,
        file: Some(path.display().to_string()),
        message: "network unavailable: the issue body was written to a local file".to_string(),
    }
}

/// Map a gh fallback-ladder outcome onto its serialisable payload.
fn feedback_data(result: gh::GhResult, dry_run: bool) -> FeedbackData {
    if dry_run {
        return dry_run_data();
    }
    match result {
        gh::GhResult::Created { url, number } => created_data(url, number),
        gh::GhResult::FallbackUrl(url) => fallback_url_data(url),
        gh::GhResult::LocalFile(path) => local_file_data(path),
    }
}

/// Translate the sad-side arguments into the genesis feedback arguments.
fn genesis_args(args: &FeedbackArgs) -> feedback::FeedbackArgs {
    feedback::FeedbackArgs {
        kind: args.kind.clone(),
        title: args.title.clone(),
        dry_run: args.dry_run,
        from_last_error: args.from_last_error,
    }
}

/// Build the success envelope for a handled report.
fn success_envelope(result: gh::GhResult, dry_run: bool) -> serde_json::Value {
    let data = feedback_data(result, dry_run);
    serde_json::to_value(Envelope::success(
        env!("CARGO_PKG_VERSION"),
        EnvelopeKind::Info,
        data,
        vec![],
        vec![],
    ))
    .expect("envelope serialization cannot fail")
}

/// Build the error envelope for a rejected report, remediation non-empty.
fn error_envelope(message: &str) -> serde_json::Value {
    let error = ErrorResult::new(
        "feedback_error",
        message,
        None,
        None,
        Some("feedback"),
        vec![],
        vec![RemediationEntry {
            command: "sad feedback --help".into(),
            description: "Check the issue kind (bug, feature, question, chore) and gh authentication (`gh auth status`), then re-run `sad feedback`.".into(),
        }],
    )
    .expect("remediation is non-empty by construction (Invariant 3.2.5)");
    serde_json::to_value(Envelope::error(env!("CARGO_PKG_VERSION"), error, vec![]))
        .expect("envelope serialization cannot fail")
}

/// Build the `sad feedback` envelope for a report request.
///
/// Delegates to `genesis::feedback::handle_feedback` (which validates the
/// kind with typo suggestions) and maps its outcome: a [`gh::GhResult`]
/// becomes a success envelope carrying the actionable fields, and any error
/// message becomes an error envelope with non-empty remediation. The call is
/// reporting-only: it never blocks or alters check results.
pub fn feedback_envelope(args: &FeedbackArgs, root: &Path) -> serde_json::Value {
    match feedback::handle_feedback(
        &genesis_args(args),
        "sad",
        env!("CARGO_PKG_VERSION"),
        "charly-vibes/sad",
        root,
    ) {
        Ok(result) => success_envelope(result, args.dry_run),
        Err(message) => error_envelope(&message),
    }
}
