//! Purpose: decide which repo-relative paths the gate excludes, and count the
//! whole-file and node-level exemptions the envelope reports.
//! Responsibilities: match exclude globs against normalised forward-slash
//! repo-relative paths, and summarise `exempt` files plus exemption-class
//! nodes across an analyzed tree.
//! Rationale: exclusion must be a pure, allocation-light predicate because it
//! runs once per discovered file; the glob rule is implemented here (the
//! manifest has no glob crate) so `**` spans path separators, `*` never does,
//! and the semantics are identical on every platform regardless of the
//! separator the caller used.

use std::borrow::Cow;

use crate::config::Config;
use crate::model::{AnalyzedFile, AttributionClass};

/// Whole-file and node-level exemption counts for one gate run.
#[derive(Debug, Default, Clone, Copy)]
pub struct ExemptionSummary {
    /// Files whose generated/licence-only detection set `exempt`.
    pub files_exempt: usize,
    /// Exemption-class comment nodes across every analyzed file.
    pub nodes_exempt: usize,
}

/// Whether `rel_path` matches any exclude glob in `cfg`.
///
/// Paths and patterns are normalised first: backslashes become `/`, empty and
/// `.` segments are dropped, and a trailing `/` is stripped, so `vendor/`,
/// `./vendor`, and `vendor//` are the same pattern and either separator style
/// is accepted. Matching is segment based and case-sensitive: a `**` segment
/// matches zero or more whole path segments (crossing `/`), while `*` and `?`
/// never cross a separator, and a `**` inside a longer segment behaves like
/// `*`. Character classes and brace expansion are not supported — everything
/// else is literal.
///
/// A pattern with no `**` names a directory, so it matches that directory and
/// everything below it: `vendor` and `vendor/` exclude the same tree as
/// `vendor/**`. Absolute paths never match, since patterns are repo-relative.
pub fn is_excluded(rel_path: &str, cfg: &Config) -> bool {
    let path = normalize(rel_path);
    cfg.exclude
        .iter()
        .any(|pattern| glob_match(&normalize(pattern), &path))
}

/// Count whole-file exemptions and exemption-class nodes across `files`.
pub fn summary(files: &[AnalyzedFile]) -> ExemptionSummary {
    ExemptionSummary {
        files_exempt: files.iter().filter(|file| file.exempt).count(),
        nodes_exempt: files
            .iter()
            .flat_map(|file| file.nodes.iter())
            .filter(|node| node.class == AttributionClass::Exemption)
            .count(),
    }
}

/// Normalise a path or pattern for matching.
///
/// Backslashes become `/`, empty and `.` segments are dropped, and the result
/// carries no trailing separator. That makes `vendor/`, `./vendor`, and
/// `vendor//` one pattern, and lets a trailing-slash pattern line up with the
/// path it names. Absolute paths keep their leading `/` and therefore never
/// match the repo-relative patterns the gate accepts.
fn normalize(value: &str) -> Cow<'_, str> {
    let already_normal = !value.contains('\\')
        && !value.starts_with('/')
        && value
            .split('/')
            .all(|segment| !segment.is_empty() && segment != ".");
    if already_normal {
        return Cow::Borrowed(value);
    }

    let forward = if value.contains('\\') {
        Cow::Owned(value.replace('\\', "/"))
    } else {
        Cow::Borrowed(value)
    };

    let absolute = forward.starts_with('/');
    let mut normalized = String::with_capacity(forward.len());
    if absolute {
        normalized.push('/');
    }
    for segment in forward.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if !normalized.is_empty() && !normalized.ends_with('/') {
            normalized.push('/');
        }
        normalized.push_str(segment);
    }
    Cow::Owned(normalized)
}

/// Match a normalised glob pattern against a normalised path.
///
/// A pattern with no `**` segment names a directory (or a file): once its
/// segments are consumed, any path segments still below it are part of the
/// match, so `vendor` and `vendor/` exclude the same tree as `vendor/**`.
/// Patterns containing `**` keep full-expression semantics instead, because
/// they say exactly how deep they reach.
fn glob_match(pattern: &str, path: &str) -> bool {
    if pattern.is_empty() {
        return path.is_empty();
    }
    match_segments(
        Some(pattern),
        (!path.is_empty()).then_some(path),
        !pattern.split('/').any(|segment| segment == "**"),
    )
}

/// Match remaining pattern segments against remaining path segments.
///
/// `None` means "no segments remain", which stays distinct from a present but
/// empty remainder; after [`normalize`] the latter cannot occur, so the split
/// is unambiguous. `directory` is true when the whole pattern names a
/// directory rather than a full glob expression.
fn match_segments(pattern: Option<&str>, path: Option<&str>, directory: bool) -> bool {
    let Some(pattern) = pattern else {
        // An exhausted directory pattern still covers every segment below the
        // matched prefix; a full glob expression must consume the whole path.
        return directory || path.is_none();
    };
    let (segment, rest_pattern) = split_segment(pattern);

    if segment == "**" {
        // Try zero segments first (`a/**` matches `a`), then consume one path
        // segment and retry at the same `**`.
        if match_segments(rest_pattern, path, directory) {
            return true;
        }
        let Some(path_text) = path else {
            return false;
        };
        let (_, rest_path) = split_segment(path_text);
        return match_segments(Some(pattern), rest_path, directory);
    }

    let Some(path_text) = path else {
        return false;
    };
    let (text, rest_path) = split_segment(path_text);
    match_segment(segment, text) && match_segments(rest_pattern, rest_path, directory)
}

/// Split off the first `/`-delimited segment, returning the remainder if any.
fn split_segment(value: &str) -> (&str, Option<&str>) {
    match value.find('/') {
        Some(index) => (&value[..index], Some(&value[index + 1..])),
        None => (value, None),
    }
}

/// Match one path segment against a pattern segment.
///
/// `*` matches any run of characters (including none), `?` matches exactly one
/// character, and everything else is literal; separators never reach here.
fn match_segment(pattern: &str, text: &str) -> bool {
    let mut chars = pattern.chars();
    match chars.next() {
        None => text.is_empty(),
        Some('*') => {
            let rest = chars.as_str();
            let mut remaining = text;
            loop {
                if match_segment(rest, remaining) {
                    return true;
                }
                match remaining.chars().next() {
                    Some(character) => remaining = &remaining[character.len_utf8()..],
                    None => return false,
                }
            }
        }
        Some('?') => match text.chars().next() {
            Some(character) => match_segment(chars.as_str(), &text[character.len_utf8()..]),
            None => false,
        },
        Some(literal) => match text.chars().next() {
            Some(character) if character == literal => {
                match_segment(chars.as_str(), &text[character.len_utf8()..])
            }
            _ => false,
        },
    }
}

#[cfg(test)]
mod tests {
    use crate::config::Config;

    use super::*;

    fn excluding(patterns: &[&str]) -> Config {
        Config {
            exclude: patterns
                .iter()
                .map(|pattern| (*pattern).to_string())
                .collect(),
            ..Config::default()
        }
    }

    #[test]
    fn directory_patterns_match_the_directory_and_everything_below_it() {
        for pattern in ["vendor", "vendor/", "./vendor/", "vendor//", "vendor/**"] {
            let cfg = excluding(&[pattern]);
            assert!(
                is_excluded("vendor/x.rs", &cfg),
                "`{pattern}` must exclude a direct child"
            );
            assert!(
                is_excluded("vendor/nested/deep/y.py", &cfg),
                "`{pattern}` must exclude nested contents"
            );
            assert!(
                !is_excluded("src/vendorish/x.rs", &cfg),
                "`{pattern}` must not match a sibling prefix"
            );
            assert!(
                !is_excluded("src/x.rs", &cfg),
                "`{pattern}` must not match elsewhere"
            );
        }
    }

    #[test]
    fn recursive_patterns_keep_full_expression_semantics() {
        let cfg = excluding(&["**/generated/**"]);
        assert!(is_excluded("a/b/generated/c.rs", &cfg));
        assert!(
            !is_excluded("a/b/generatedish/c.rs", &cfg),
            "a `**` pattern must not become a directory prefix"
        );
        assert!(is_excluded("generated/c.rs", &cfg));
    }

    #[test]
    fn separators_and_dot_segments_normalise_in_patterns_and_paths() {
        assert!(is_excluded("vendor//x.rs", &excluding(&["vendor/x.rs"])));
        assert!(is_excluded(
            "./vendor/./x.rs",
            &excluding(&["vendor//x.rs"])
        ));
        assert!(is_excluded("vendor\\x.rs", &excluding(&["./vendor/"])));
        assert!(
            !is_excluded("/vendor/x.rs", &excluding(&["vendor/x.rs"])),
            "absolute paths never match repo-relative patterns"
        );
    }

    #[test]
    fn empty_pattern_excludes_nothing() {
        assert!(!is_excluded("src/x.rs", &excluding(&[""])));
        assert!(!is_excluded("src/x.rs", &excluding(&["."])));
    }
}
