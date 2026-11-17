//! Purpose: attribute one extracted raw comment to exactly one channel —
//! header, doc, marker, suppression, exemption, or finding-eligible prose —
//! and parse suppression directives.
//! Responsibilities: apply the channel precedence (doc kinds never move to
//! marker/suppression; header-position nodes stay header), recognise every
//! v1 suppression form, and match marker-prefixed IDs with a regex compiled
//! once per file while resolving each cite to its full id token.
//! Rationale: attribution is structural and side-effect free, so it lives
//! apart from tree walking; keeping one precedence function means every
//! frontend classifies identically.

use regex::Regex;

use crate::model::{AttributionClass, CommentKind, SuppressionDirective, SuppressionForm};

/// Marker-prefix matcher compiled once per analysed file.
///
/// The pattern comes from validated `sad.toml`; an invalid pattern degrades
/// to "no marker recognition" rather than panicking mid-walk.
pub struct MarkerMatcher {
    regex: Option<Regex>,
}

impl MarkerMatcher {
    pub fn new(pattern: &str) -> Self {
        Self {
            regex: Regex::new(pattern).ok(),
        }
    }

    /// True when `body` starts with `prefix` followed by an id the configured
    /// marker pattern matches at its start.
    pub fn is_marker(&self, body: &str, prefix: &str) -> bool {
        self.regex
            .as_ref()
            .is_some_and(|regex| leading_marker_id(body, prefix, regex).is_some())
    }
}

/// The leading marker id after `prefix`.
///
/// The configured pattern only has to match at the start of the cited id; the
/// id itself always ends at a delimiter. A match that stops before another
/// hyphenated segment is widened to the whole id, so `sad-lcy-extra` is never
/// treated as the truncation `sad-lcy`. A match glued to further alphanumerics
/// (`issue-12` in `issue-12ab` under a narrowed pattern) is not a marker id at
/// all, because the configured boundary was never a boundary.
pub fn leading_marker_id<'a>(body: &'a str, prefix: &str, regex: &Regex) -> Option<&'a str> {
    let rest = body.trim_start().strip_prefix(prefix)?;
    let rest = rest.trim_start();
    let found = regex.find(rest)?;
    if found.start() != 0 {
        return None;
    }
    let end = match rest[found.end()..].chars().next() {
        Some('-') | Some('_') => rest
            .find(|next: char| !is_id_continue(next))
            .unwrap_or(rest.len()),
        Some(next) if next.is_alphanumeric() => return None,
        _ => found.end(),
    };
    let id = rest[..end].trim_end_matches(['-', '_']);
    (!id.is_empty()).then_some(id)
}

/// True for characters that continue a bead-shaped id.
fn is_id_continue(next: char) -> bool {
    next.is_alphanumeric() || next == '-' || next == '_'
}

/// Classify one raw comment body.
///
/// Precedence: header-position → doc kind → exemption → suppression →
/// marker → prose. Header wins over the doc kind because module docs are the
/// header channel; doc kinds win over everything else so markers and
/// suppression text inside documentation cannot route it elsewhere.
pub fn classify(
    body: &str,
    kind: CommentKind,
    is_header_position: bool,
    marker_prefix: &str,
    markers: &MarkerMatcher,
) -> (AttributionClass, Option<SuppressionDirective>) {
    if is_header_position {
        return (AttributionClass::Header, None);
    }
    if kind == CommentKind::Doc {
        return (AttributionClass::Doc, None);
    }
    if is_exemption(body) {
        return (AttributionClass::Exemption, None);
    }
    if let Some(directive) = parse_directive(body) {
        return (AttributionClass::Suppression, Some(directive));
    }
    if markers.is_marker(body, marker_prefix) {
        return (AttributionClass::Marker, None);
    }
    (AttributionClass::Prose, None)
}

/// Licence and generated-code markers exempt their file from findings.
///
/// Every SPDX file tag (`SPDX-License-Identifier`, `SPDX-FileCopyrightText`,
/// `SPDX-FileContributor`, …) is licence metadata rather than prose, so a
/// body that opens with a tag is an exemption; `@generated` counts only when
/// it is marker-shaped — the start of one of the body's lines.
pub fn is_exemption(body: &str) -> bool {
    starts_with_spdx(body) || has_generated_marker(body)
}

/// True when the body opens with an SPDX file tag.
fn starts_with_spdx(body: &str) -> bool {
    body.trim_start()
        .get(..5)
        .is_some_and(|head| head.eq_ignore_ascii_case("spdx-"))
}

/// True when one line of a normalised body starts with `@generated`.
///
/// Marker-shaped rather than a substring search: prose that merely mentions
/// the marker is not generated code.
pub fn has_generated_marker(body: &str) -> bool {
    const MARKER: &str = "@generated";
    body.lines().any(|line| {
        line.trim_start()
            .get(..MARKER.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(MARKER))
    })
}

/// Parse one of the five v1 suppression forms from a marker-stripped body.
pub fn parse_directive(body: &str) -> Option<SuppressionDirective> {
    let text = body.trim();
    let lower = text.to_ascii_lowercase();

    if let Some(rest) = strip_directive(&lower, text, "sad:ignore") {
        let mut words = rest.split_whitespace();
        // A directive scopes to exactly one check, so a `sad:ignore` form with
        // no check id is prose rather than suppression debt.
        let check = clean_check(words.next()?);
        if check.is_empty() {
            return None;
        }
        let reason = words.collect::<Vec<_>>().join(" ");
        return Some(SuppressionDirective {
            form: SuppressionForm::SadIgnore,
            check: Some(check),
            reason: (!reason.is_empty()).then_some(reason),
        });
    }

    if let Some(rest) = strip_directive(&lower, text, "noqa") {
        let check = clean_check(rest);
        return Some(SuppressionDirective {
            form: SuppressionForm::Noqa,
            check: (!check.is_empty()).then_some(check),
            reason: None,
        });
    }

    if let Some(rest) = strip_directive(&lower, text, "type: ignore") {
        let check = rest
            .find('[')
            .and_then(|start| {
                rest[start + 1..]
                    .find(']')
                    .map(|end| &rest[start + 1..start + 1 + end])
            })
            .map(clean_check)
            .filter(|code| !code.is_empty());
        return Some(SuppressionDirective {
            form: SuppressionForm::TypeIgnore,
            check,
            reason: None,
        });
    }

    if strip_directive(&lower, text, "@ts-ignore").is_some() {
        return Some(SuppressionDirective {
            form: SuppressionForm::TsIgnore,
            check: None,
            reason: None,
        });
    }

    if let Some(rest) = strip_directive(&lower, text, "eslint-disable") {
        let rest = rest.trim_start();
        let args = if let Some(args) = rest.strip_prefix("-next-line") {
            args
        } else if let Some(args) = rest.strip_prefix("-line") {
            args
        } else {
            rest
        };
        let check = args.split("--").next().unwrap_or("").trim();
        return Some(SuppressionDirective {
            form: SuppressionForm::EslintDisable,
            check: (!check.is_empty()).then(|| check.to_string()),
            reason: None,
        });
    }

    None
}

/// Return the text after `needle` when the lower-cased body starts with it
/// and the needle ends on a word boundary (so `noqa` cannot match `noqax`).
fn strip_directive<'a>(lower: &str, text: &'a str, needle: &str) -> Option<&'a str> {
    if !lower.starts_with(needle) {
        return None;
    }
    let rest = &text[needle.len()..];
    let boundary = rest
        .chars()
        .next()
        .is_none_or(|next| !next.is_alphanumeric());
    boundary.then_some(rest)
}

/// Trim punctuation that separates a directive from its first argument.
fn clean_check(raw: &str) -> String {
    raw.trim()
        .trim_start_matches([':', ','])
        .trim_end_matches([',', ';'])
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classify_plain(body: &str, kind: CommentKind) -> AttributionClass {
        let markers = MarkerMatcher::new(r"\b[a-z][a-z0-9]*-[a-z0-9]+\b");
        classify(body, kind, false, "see", &markers).0
    }

    #[test]
    fn doc_kind_always_wins_over_markers_and_suppressions() {
        assert_eq!(
            classify_plain("see sad-lcy: note", CommentKind::Doc),
            AttributionClass::Doc
        );
        assert_eq!(
            classify_plain("noqa", CommentKind::Doc),
            AttributionClass::Doc
        );
    }

    #[test]
    fn sad_ignore_captures_check_and_reason() {
        let directive = parse_directive("sad:ignore comment_outside_channel reason here")
            .expect("sad:ignore must parse");
        assert_eq!(directive.form, SuppressionForm::SadIgnore);
        assert_eq!(directive.check.as_deref(), Some("comment_outside_channel"));
        assert_eq!(directive.reason.as_deref(), Some("reason here"));
    }

    #[test]
    fn marker_prefix_requires_a_matching_leading_id() {
        let markers = MarkerMatcher::new(r"\b[a-z][a-z0-9]*-[a-z0-9]+\b");
        assert!(markers.is_marker("see sad-lcy: note", "see"));
        assert!(!markers.is_marker("requires serde-1 compat", "see"));
        assert!(!markers.is_marker("see the sad-lcy note", "see"));
    }

    #[test]
    fn marker_ids_end_at_a_delimiter() {
        let regex = Regex::new(r"\b[a-z][a-z0-9]*-[a-z0-9]+\b").expect("pattern compiles");
        assert_eq!(
            leading_marker_id("see sad-lcy: note", "see", &regex),
            Some("sad-lcy")
        );
        assert_eq!(
            leading_marker_id("see sad-lcy. done", "see", &regex),
            Some("sad-lcy")
        );
        assert_eq!(
            leading_marker_id("see sad-lcy-extra: note", "see", &regex),
            Some("sad-lcy-extra"),
            "a match that stops inside the id is widened to the cited id"
        );
        assert_eq!(
            leading_marker_id("see sad-lcy-", "see", &regex),
            Some("sad-lcy"),
            "trailing id punctuation is not part of the id"
        );

        let narrowed = Regex::new(r"\bissue-[0-9]+\b").expect("pattern compiles");
        assert_eq!(
            leading_marker_id("see issue-12ab", "see", &narrowed),
            None,
            "a match glued to alphanumerics is not a marker id"
        );
    }

    #[test]
    fn sad_ignore_requires_a_check_id_and_a_word_boundary() {
        assert!(parse_directive("sad:ignorex comment_outside_channel").is_none());
        assert!(parse_directive("sad:ignored").is_none());
        assert!(
            parse_directive("sad:ignore").is_none(),
            "a directive without a check id is prose, not suppression debt"
        );
    }

    #[test]
    fn generated_marker_must_be_marker_shaped() {
        assert!(is_exemption("@generated by the schema compiler"));
        assert!(is_exemption("SPDX-License-Identifier: MIT"));
        assert!(
            !is_exemption("this module header documents @generated"),
            "a prose mention of the marker is not generated code"
        );
        assert!(is_exemption("Licence note.\n@generated"));
    }
}
