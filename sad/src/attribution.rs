//! Purpose: attribute one extracted raw comment to exactly one channel —
//! header, doc, marker, suppression, exemption, or finding-eligible prose.
//! Responsibilities: apply the channel precedence (doc kinds never move to
//! marker/rationale/suppression; header-position nodes stay header), recognise
//! the closed rationale tag set, decide the exemption class for Python file
//! metadata and bounded licence-header lines, and match marker-prefixed IDs
//! with a regex compiled once per file while resolving each cite to its full
//! id token. Suppression grammar lives in [`directives`] and is re-exported
//! here so attribution stays the one entry point.
//! Rationale: attribution is structural and side-effect free, so it lives
//! apart from tree walking; keeping one precedence function means every
//! frontend classifies identically.

mod directives;

use regex::Regex;

use crate::model::{AttributionClass, CommentKind, SuppressionDirective};

pub use self::directives::parse_directive;

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
/// Precedence: header-position → doc kind → marker → rationale →
/// suppression → exemption → prose. Header wins over the doc kind because
/// module docs are the header channel; doc kinds win over everything else so
/// markers and suppression text inside documentation cannot route it
/// elsewhere.
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
    if markers.is_marker(body, marker_prefix) {
        return (AttributionClass::Marker, None);
    }
    if is_rationale(body) {
        return (AttributionClass::Rationale, None);
    }
    if let Some(directive) = parse_directive(body) {
        return (AttributionClass::Suppression, Some(directive));
    }
    if is_exemption(body) {
        return (AttributionClass::Exemption, None);
    }
    (AttributionClass::Prose, None)
}

fn is_rationale(body: &str) -> bool {
    let text = body.trim();
    ["SAFETY:", "INVARIANT:", "SECURITY:", "WHY:"]
        .into_iter()
        .any(|tag| {
            text.strip_prefix(tag)
                .is_some_and(|rest| !rest.trim().is_empty())
        })
}

/// Licence and generated-code markers exempt their file from findings.
///
/// Every SPDX file tag (`SPDX-License-Identifier`, `SPDX-FileCopyrightText`,
/// `SPDX-FileContributor`, …) is licence metadata rather than prose, so a
/// body that opens with a tag is an exemption; the same holds for a bounded
/// licence line beside the tag ([`starts_with_licence_tag`]), and `@generated`
/// counts only when it is marker-shaped — the start of one of the body's
/// lines.
pub fn is_exemption(body: &str) -> bool {
    starts_with_spdx(body) || starts_with_licence_tag(body) || has_generated_marker(body)
}

/// True when a `#`-stripped Python body is a shebang line.
///
/// The frontend strips the `#` marker, so a shebang body opens with `!`.
/// Position is the caller's concern: only source row 0 is file metadata, and
/// the same text later in the file stays finding-eligible prose.
pub fn is_shebang(body: &str) -> bool {
    body.starts_with('!')
}

/// True when a `#`-stripped body is a PEP 263 coding cookie.
///
/// PEP 263 admits `coding: <encoding>` and `coding=<encoding>` in a comment on
/// source lines 1-2, optionally wrapped in the Emacs `-*-` marker; the
/// encoding is a bounded bare-word token, so `coding standards` is prose while
/// `coding: utf-8` is a cookie. The row window is the caller's concern.
pub fn is_coding_cookie(body: &str) -> bool {
    let text = body.trim();
    let text = text.strip_prefix("-*-").unwrap_or(text).trim_start();
    let Some(rest) = text.strip_prefix("coding") else {
        return false;
    };
    let Some(encoding) = rest.strip_prefix([':', '=']) else {
        return false;
    };
    let token = encoding.trim_start();
    token.starts_with(is_encoding_char)
}

/// True for the characters PEP 263 admits in an encoding name.
fn is_encoding_char(next: char) -> bool {
    next.is_ascii_alphanumeric() || matches!(next, '-' | '_' | '.')
}

/// True when the body opens with a bounded licence-header marker.
///
/// The shapes are deliberately narrow: a bare `Copyright` word is prose
/// (`Copyright is a legal concept`), while a copyright line names a year or
/// the `(c)`/`©` sign. Accepted false-negative boundary: prose that literally
/// opens with one of these marker shapes is exempt, because the marker is
/// structural and exempting every line that merely mentions licensing would
/// swallow narration.
pub fn starts_with_licence_tag(body: &str) -> bool {
    let text = body.trim_start();
    if let Some(rest) = strip_ascii_prefix_ci(text, "copyright") {
        let rest = rest.trim_start();
        return starts_with_year(rest)
            || rest.starts_with('©')
            || rest
                .get(..3)
                .is_some_and(|head| head.eq_ignore_ascii_case("(c)"));
    }
    text.starts_with('©')
        || text
            .get(..3)
            .is_some_and(|head| head.eq_ignore_ascii_case("(c)"))
        || starts_with_phrase(text, "licensed under")
        || strip_ascii_prefix_ci(text, "license:").is_some()
        || starts_with_phrase(text, "all rights reserved")
}

/// Case-insensitive prefix strip.
fn strip_ascii_prefix_ci<'a>(text: &'a str, needle: &str) -> Option<&'a str> {
    let head = text.get(..needle.len())?;
    head.eq_ignore_ascii_case(needle)
        .then_some(&text[needle.len()..])
}

/// True when the body opens with `phrase` and the phrase ends on a word
/// boundary, so `licensed undergraduate` is not a licence header.
fn starts_with_phrase(text: &str, phrase: &str) -> bool {
    strip_ascii_prefix_ci(text, phrase).is_some_and(|rest| {
        rest.chars()
            .next()
            .is_none_or(|next| !next.is_alphanumeric())
    })
}

/// True when the text opens with a four-digit year, so `Copyright 20261` is
/// not a copyright notice.
fn starts_with_year(text: &str) -> bool {
    let Some(head) = text.get(..4) else {
        return false;
    };
    head.bytes().all(|byte| byte.is_ascii_digit())
        && text
            .get(4..)
            .and_then(|rest| rest.chars().next())
            .is_none_or(|next| !next.is_ascii_alphanumeric())
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
    fn generated_marker_must_be_marker_shaped() {
        assert!(is_exemption("@generated by the schema compiler"));
        assert!(is_exemption("SPDX-License-Identifier: MIT"));
        assert!(
            !is_exemption("this module header documents @generated"),
            "a prose mention of the marker is not generated code"
        );
        assert!(is_exemption("Licence note.\n@generated"));
    }

    #[test]
    fn licence_tag_predicate_is_bounded() {
        for body in [
            "Copyright 2026 Charly Vibes",
            "Copyright (c) 2026 Charly Vibes",
            "Copyright © 2026 Charly Vibes",
            "© 2026 Charly Vibes",
            "(c) 2026 Charly Vibes",
            "Licensed under the Apache License, Version 2.0",
            "License: MIT",
            "All rights reserved.",
        ] {
            assert!(is_exemption(body), "{body:?} is a bounded licence line");
        }
        assert!(
            !is_exemption("Copyright is a legal concept"),
            "a bare `Copyright` word is prose, not a licence header"
        );
        assert!(!is_exemption("Licensed undergraduate work"));
        assert!(!is_exemption("All rights reservedly claimed"));
    }

    #[test]
    fn python_file_metadata_predicates_match_their_spellings() {
        assert!(is_shebang("!/usr/bin/env python3"));
        assert!(!is_shebang(" /usr/bin/env python3"));

        for body in ["-*- coding: utf-8 -*-", "coding: utf-8", "coding=utf-8"] {
            assert!(is_coding_cookie(body), "{body:?} is a PEP 263 cookie");
        }
        assert!(!is_coding_cookie(
            "coding standards are documented elsewhere"
        ));
        assert!(!is_coding_cookie(
            "this mentions coding: utf-8 mid-sentence"
        ));
    }
}
