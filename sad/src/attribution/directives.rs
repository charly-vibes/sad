//! Purpose: parse the six v1 suppression-directive forms from a
//! marker-stripped comment body.
//! Responsibilities: recognise `sad:ignore`, `noqa`, `type: ignore`,
//! `@ts-ignore`, `@ts-expect-error`, and `eslint-disable`, bind each to its
//! check scope and optional reason, and reject spellings that only share a
//! prefix with a directive (`noqax`, `sad:ignored`).
//! Rationale: every form has its own grammar, so each gets its own small
//! parser and the first match owns the body; keeping this apart from channel
//! attribution lets the precedence function stay readable and keeps one form
//! from swallowing another.

use crate::model::{SuppressionDirective, SuppressionForm};

/// Parse one of the six v1 suppression forms from a marker-stripped body.
///
/// Each form has its own parser so one form's grammar cannot swallow another;
/// the first form that matches owns the body.
pub fn parse_directive(body: &str) -> Option<SuppressionDirective> {
    let text = body.trim();
    let lower = text.to_ascii_lowercase();

    parse_sad_ignore(&lower, text)
        .or_else(|| parse_noqa(&lower, text))
        .or_else(|| parse_type_ignore(&lower, text))
        .or_else(|| parse_ts_ignore(&lower, text))
        .or_else(|| parse_ts_expect_error(&lower, text))
        .or_else(|| parse_eslint_disable(&lower, text))
}

/// `sad:ignore <check> [reason…]`; a form without a check id is prose.
fn parse_sad_ignore(lower: &str, text: &str) -> Option<SuppressionDirective> {
    let rest = strip_directive(lower, text, "sad:ignore")?;
    let mut words = rest.split_whitespace();
    let check = clean_check(words.next()?);
    if check.is_empty() {
        return None;
    }
    let reason = words.collect::<Vec<_>>().join(" ");
    Some(SuppressionDirective {
        form: SuppressionForm::SadIgnore,
        check: Some(check),
        reason: (!reason.is_empty()).then_some(reason),
    })
}

/// `noqa[: check[, check…]]`.
fn parse_noqa(lower: &str, text: &str) -> Option<SuppressionDirective> {
    let check = clean_check(strip_directive(lower, text, "noqa")?);
    Some(SuppressionDirective {
        form: SuppressionForm::Noqa,
        check: (!check.is_empty()).then_some(check),
        reason: None,
    })
}

/// `type: ignore[code]`, where only the bracketed code scopes the directive.
fn parse_type_ignore(lower: &str, text: &str) -> Option<SuppressionDirective> {
    let rest = strip_directive(lower, text, "type: ignore")?;
    let check = rest
        .find('[')
        .and_then(|start| {
            rest[start + 1..]
                .find(']')
                .map(|end| &rest[start + 1..start + 1 + end])
        })
        .map(clean_check)
        .filter(|code| !code.is_empty());
    Some(SuppressionDirective {
        form: SuppressionForm::TypeIgnore,
        check,
        reason: None,
    })
}

/// `@ts-ignore` is unscoped: it carries no check id.
fn parse_ts_ignore(lower: &str, text: &str) -> Option<SuppressionDirective> {
    strip_directive(lower, text, "@ts-ignore")?;
    Some(SuppressionDirective {
        form: SuppressionForm::TsIgnore,
        check: None,
        reason: None,
    })
}

/// `@ts-expect-error` is unscoped, like `@ts-ignore`.
fn parse_ts_expect_error(lower: &str, text: &str) -> Option<SuppressionDirective> {
    strip_directive(lower, text, "@ts-expect-error")?;
    Some(SuppressionDirective {
        form: SuppressionForm::TsExpectError,
        check: None,
        reason: None,
    })
}

/// `eslint-disable[-line|-next-line] rules`, rules ending at a `--` comment.
fn parse_eslint_disable(lower: &str, text: &str) -> Option<SuppressionDirective> {
    let rest = strip_directive(lower, text, "eslint-disable")?.trim_start();
    let args = rest
        .strip_prefix("-next-line")
        .or_else(|| rest.strip_prefix("-line"))
        .unwrap_or(rest);
    let check = args.split("--").next().unwrap_or("").trim();
    Some(SuppressionDirective {
        form: SuppressionForm::EslintDisable,
        check: (!check.is_empty()).then(|| check.to_string()),
        reason: None,
    })
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

    #[test]
    fn sad_ignore_captures_check_and_reason() {
        let directive = parse_directive("sad:ignore comment_outside_channel reason here")
            .expect("sad:ignore must parse");
        assert_eq!(directive.form, SuppressionForm::SadIgnore);
        assert_eq!(directive.check.as_deref(), Some("comment_outside_channel"));
        assert_eq!(directive.reason.as_deref(), Some("reason here"));
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
    fn ts_expect_error_is_an_unscoped_directive() {
        let directive = parse_directive("@ts-expect-error: node builtin has no types")
            .expect("@ts-expect-error must parse as a directive");
        assert_eq!(directive.form, SuppressionForm::TsExpectError);
        assert_eq!(directive.check, None, "ts-expect-error is unscoped");
    }
}
