//! Purpose: decide which tree entries the `.gitignore` files of a git work tree
//! exclude from source discovery.
//! Responsibilities: parse root and nested ignore files using gitignore's
//! anchoring, directory-only, wildcard, and negation grammar; hold the
//! per-directory rule stack the walk drives; and answer the per-entry ignore
//! question without reading `.git`, the index, global excludes, or invoking git.
//! Rationale: ignore files are tree content, so honoring them keeps discovery a
//! pure function of the tree while letting authors declare generated and vendored
//! sources out of scope where they already record that intent. The grammar is
//! closed — every character outside `*`, `?`, `**`, `!`, and a trailing `/` is
//! literal — so parsing never fails, and a missing, unreadable, or non-UTF-8 file
//! contributes no rules. The rule stack is shallow-first so a deeper file's rules
//! win, one file's rules apply top to bottom so its last match wins, and an
//! ignored directory is pruned, so no nested rule can re-include below it.
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

/// The rules visible at one point of the walk, grouped by owning directory.
///
/// Frame `i` belongs to the directory at repo-relative depth `i`, whose rules
/// match the entry path with its first `i` segments removed. The walk calls
/// [`IgnoreStack::enter`] before descending and [`IgnoreStack::leave`] after.
#[derive(Debug)]
pub(super) struct IgnoreStack {
    frames: Vec<Vec<Rule>>,
    /// Live rule count, so a work tree with no ignore files pays no matching.
    total: usize,
}

impl IgnoreStack {
    /// Build the stack for `repo`, or `None` when filtering does not apply.
    ///
    /// Filtering applies only inside a git work tree: `.git` must exist, as a
    /// directory or as the file a linked worktree or submodule carries, and
    /// its contents are never read. A missing, unreadable, or non-UTF-8 root
    /// `.gitignore` contributes no root rules, so only deeper files apply.
    pub(super) fn detect(repo: &Path) -> Option<Self> {
        if !repo.join(".git").exists() {
            return None;
        }
        let roots = match load(&repo.join(".gitignore")) {
            Loaded::Rules(rules) => rules,
            Loaded::Absent | Loaded::Broken => Vec::new(),
        };
        Some(Self {
            total: roots.len(),
            frames: vec![roots],
        })
    }

    /// Push the rules of `dir`'s `.gitignore`; a broken file contributes none.
    pub(super) fn enter(&mut self, dir: &Path) {
        let rules = match load(&dir.join(".gitignore")) {
            Loaded::Rules(rules) => rules,
            Loaded::Absent | Loaded::Broken => Vec::new(),
        };
        self.total += rules.len();
        self.frames.push(rules);
    }

    /// Pop the rules of the directory the walk just left.
    pub(super) fn leave(&mut self) {
        if self.frames.len() > 1
            && let Some(rules) = self.frames.pop()
        {
            self.total -= rules.len();
        }
    }

    /// Whether `path`, an entry directly inside the current directory, is
    /// ignored after every visible rule is applied shallowest first.
    ///
    /// The last matching rule decides, so a deeper file's later rule overrides
    /// a shallower one and a file's own top-to-bottom order holds. `root`
    /// anchors the repo-relative path; `is_dir` gates directory-only rules.
    pub(super) fn is_ignored(&self, root: &Path, path: &Path, is_dir: bool) -> bool {
        if self.total == 0 {
            return false;
        }
        let Ok(relative) = path.strip_prefix(root) else {
            return false;
        };
        let segments: Vec<String> = relative
            .components()
            .map(|component| component.as_os_str().to_string_lossy().into_owned())
            .collect();
        let mut ignored = false;
        for (depth, rules) in self.frames.iter().enumerate() {
            let Some(tail) = segments.get(depth..) else {
                break;
            };
            for rule in rules {
                if rule.matches(tail, is_dir) {
                    ignored = !rule.negated;
                }
            }
        }
        ignored
    }
}

/// Outcome of reading one ignore file.
enum Loaded {
    Rules(Vec<Rule>),
    Absent,
    Broken,
}

/// Read and parse one ignore file, never failing the walk.
fn load(path: &Path) -> Loaded {
    match fs::read(path) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(text) => Loaded::Rules(parse(&text)),
            Err(_) => Loaded::Broken,
        },
        Err(error) if error.kind() == ErrorKind::NotFound => Loaded::Absent,
        Err(_) => Loaded::Broken,
    }
}

/// Compile every meaningful line of an ignore file, in file order.
fn parse(text: &str) -> Vec<Rule> {
    text.lines().filter_map(compile).collect()
}

/// Compile one line into a rule, or `None` for a blank or comment line.
///
/// Leading/trailing spaces, tabs, and a carriage return are trimmed. A leading
/// `#` is always a comment (there is no escape), and a leading `!` negates. A
/// trailing `/` makes the rule directory-only, and any slash that stays after
/// those are removed anchors the rule at its own directory; without one the
/// rule is compiled as `**/name`, matching a basename at any depth.
fn compile(line: &str) -> Option<Rule> {
    let trimmed = line.trim_matches(|c: char| c == ' ' || c == '\t' || c == '\r');
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let negated = trimmed.starts_with('!');
    let pattern = trimmed.strip_prefix('!').unwrap_or(trimmed);
    let directory_only = pattern.ends_with('/');
    let pattern = pattern.strip_suffix('/').unwrap_or(pattern);
    let anchored = pattern.contains('/');
    let pattern = pattern.trim_start_matches('/');
    if pattern.is_empty() {
        return None;
    }
    let segments = compile_segments(pattern, anchored);
    if segments.is_empty() {
        return None;
    }
    Some(Rule {
        negated,
        directory_only,
        segments,
    })
}

/// Compile `pattern` into its path segments; an unanchored pattern matches at any depth.
fn compile_segments(pattern: &str, anchored: bool) -> Vec<Segment> {
    let mut segments = Vec::new();
    if !anchored {
        segments.push(Segment::AnyDepth);
    }
    for part in pattern.split('/').filter(|part| !part.is_empty()) {
        if part == "**" {
            segments.push(Segment::AnyDepth);
        } else {
            segments.push(Segment::Glob(tokens(part)));
        }
    }
    segments
}

/// Split one pattern segment into wildcard tokens; `**` inside a longer
/// segment behaves like `*`, and every other character is literal.
fn tokens(part: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    for character in part.chars() {
        match character {
            '*' if tokens.last() != Some(&Token::Star) => tokens.push(Token::Star),
            '*' => (),
            '?' => tokens.push(Token::One),
            literal => tokens.push(Token::Literal(literal)),
        }
    }
    tokens
}

/// One compiled ignore rule.
#[derive(Debug)]
struct Rule {
    negated: bool,
    directory_only: bool,
    segments: Vec<Segment>,
}

impl Rule {
    /// Whether this rule matches `relative`, the path below the rule's directory.
    fn matches(&self, relative: &[String], is_dir: bool) -> bool {
        if self.directory_only && !is_dir {
            return false;
        }
        matches_segments(&self.segments, relative)
    }
}

/// One compiled pattern segment.
#[derive(Debug, PartialEq)]
enum Segment {
    /// `**`: zero or more whole path segments; as the final segment it must
    /// consume at least one, so `dir/**` matches descendants but never `dir`.
    AnyDepth,
    Glob(Vec<Token>),
}

/// One token of a within-segment glob; no token crosses a `/`.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Token {
    Star,
    One,
    Literal(char),
}

/// Match a compiled pattern against a whole relative path.
fn matches_segments(pattern: &[Segment], path: &[String]) -> bool {
    let Some((segment, rest)) = pattern.split_first() else {
        return path.is_empty();
    };
    match segment {
        Segment::AnyDepth => {
            if rest.is_empty() {
                !path.is_empty()
            } else {
                (0..=path.len()).any(|used| matches_segments(rest, &path[used..]))
            }
        }
        Segment::Glob(tokens) => match path.split_first() {
            Some((name, tail)) => glob_matches(tokens, name) && matches_segments(rest, tail),
            None => false,
        },
    }
}

/// Wildcard match within one segment: `*` matches any run (including empty), `?` exactly one character.
fn glob_matches(tokens: &[Token], text: &str) -> bool {
    let characters: Vec<char> = text.chars().collect();
    let (mut token, mut index) = (0usize, 0usize);
    let mut star: Option<(usize, usize)> = None;
    while index < characters.len() {
        match tokens.get(token) {
            Some(Token::One) => {
                token += 1;
                index += 1;
            }
            Some(Token::Literal(expected)) if *expected == characters[index] => {
                token += 1;
                index += 1;
            }
            Some(Token::Star) => {
                star = Some((token, index));
                token += 1;
            }
            _ => {
                if !retry_from_star(&mut star, &mut token, &mut index) {
                    return false;
                }
            }
        }
    }
    trailing_stars_consumed(tokens, token) == tokens.len()
}

/// Backtrack to the most recent star, letting it consume one more character; `false` without a star.
fn retry_from_star(
    star: &mut Option<(usize, usize)>,
    token: &mut usize,
    index: &mut usize,
) -> bool {
    let Some((star_token, star_index)) = *star else {
        return false;
    };
    *token = star_token + 1;
    *index = star_index + 1;
    *star = Some((star_token, star_index + 1));
    true
}

/// Advance past trailing stars once the text is consumed; they may match empty.
fn trailing_stars_consumed(tokens: &[Token], mut token: usize) -> usize {
    while tokens.get(token) == Some(&Token::Star) {
        token += 1;
    }
    token
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(line: &str) -> Rule {
        compile(line).unwrap_or_else(|| panic!("{line:?} must compile to a rule"))
    }

    fn matches(line: &str, path: &str, is_dir: bool) -> bool {
        let parts: Vec<String> = path.split('/').map(str::to_owned).collect();
        rule(line).matches(&parts, is_dir)
    }

    #[test]
    fn double_star_spans_zero_or_more_segments() {
        assert!(matches("a/**/b", "a/b", false));
        assert!(matches("a/**/b", "a/x/y/b", false));
        assert!(!matches("a/**/b", "a/xb", false));
        assert!(!matches("dir/**", "dir", true));
        assert!(matches("dir/**", "dir/x", false));
        assert!(matches("**/name", "name", false));
        assert!(matches("**/name", "a/b/name", false));
    }

    #[test]
    fn slash_anchors_while_a_basename_matches_at_any_depth() {
        assert!(matches("src/gen", "src/gen", true));
        assert!(!matches("src/gen", "x/src/gen", true));
        assert!(matches("gen", "gen", false));
        assert!(matches("gen", "x/gen", false));
        assert!(matches("/gen", "gen", false));
        assert!(!matches("/gen", "x/gen", false));
    }

    #[test]
    fn question_mark_is_exactly_one_character() {
        assert!(matches("a?c", "abc", false));
        assert!(!matches("a?c", "ac", false));
        assert!(!matches("a?c", "abcd", false));
        assert!(!matches("a?c", "a/c", false));
    }

    #[test]
    fn trailing_slash_is_directory_only() {
        assert!(matches("name/", "name", true));
        assert!(!matches("name/", "name", false));
        assert!(matches("name/", "x/name", true));
        assert!(matches("src/name/", "src/name", true));
        assert!(!matches("src/name/", "x/src/name", true));
    }

    #[test]
    fn stars_never_cross_a_segment_boundary() {
        assert!(matches("*.ts", "src/a.ts", false));
        assert!(matches("src/*.ts", "src/a.ts", false));
        assert!(!matches("src/*.ts", "src/a/b.ts", false));
        assert!(matches("a**b", "axxb", false));
        assert!(!matches("a**b", "ax/xb", false));
    }

    #[test]
    fn braces_brackets_and_backslashes_are_literal() {
        assert!(matches("{a,b}", "{a,b}", false));
        assert!(!matches("{a,b}", "a", false));
        assert!(matches("[a-z].ts", "[a-z].ts", false));
        assert!(!matches("[a-z].ts", "a.ts", false));
        assert!(matches("a\\b", "a\\b", false));
        assert!(!matches("a\\b", "ab", false));
    }

    #[test]
    fn comments_blanks_and_crlf_are_not_rules() {
        let rules = parse("# comment\r\n\r\n  \r\n*.ts\r\n");
        assert_eq!(rules.len(), 1, "only the pattern line becomes a rule");
        assert!(rules[0].matches(&["a.ts".to_string()], false));
        assert!(parse("!\n").is_empty(), "a bare negation is not a rule");
        assert!(rule("!keep.ts").negated);
        assert!(
            compile("   # comment").is_none(),
            "trimmed comments are inert"
        );
    }

    #[test]
    fn ignored_directory_prune_cannot_be_reversed() {
        let root = std::env::temp_dir().join(format!("sad-gitignore-prune-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".git")).expect("marker is created");
        fs::create_dir_all(root.join("generated")).expect("ignored directory is created");
        fs::write(root.join(".gitignore"), "generated/\n").expect("root rules are written");
        fs::write(root.join("generated/.gitignore"), "!*\n").expect("nested rules are written");

        let mut ignore = IgnoreStack::detect(&root).expect("a work tree filters");
        let directory = root.join("generated");
        assert!(ignore.is_ignored(&root, &directory, true));
        // WHY: the walk prunes this directory, so a nested negation cannot re-include it.
        ignore.enter(&directory);
        assert!(
            ignore.is_ignored(&root, &directory, true),
            "a nested rule cannot rescue its own pruned directory"
        );
        let _ = fs::remove_dir_all(&root);
    }
}
