//! Purpose: the directory-name skip lists that bound the source walk.
//! Responsibilities: name every directory that is never traversed, and the
//! ambiguous subset whose nested appearances are reported rather than dropped
//! silently.
//! Rationale: the lists are data, not logic; keeping them out of
//! `discovery.rs` keeps that file about the walk itself.

/// Directory names never traversed: VCS metadata, dependency installs, caches, and build output.
///
/// Two rules govern the list. First, every listed name is reserved for a
/// machine tree — `.git`, `.beads`, dependency installs (`node_modules`,
/// `vendor`, `third_party`, `.venv`, `venv`, `.tox`), caches (`.mypy_cache`,
/// `.ruff_cache`, `.pytest_cache`, `__pycache__`, `.turbo`), build and bundler
/// output (`target`, `dist`, `build`, `out`, `.next`, `.svelte-kit`,
/// `coverage`), or the rendered `book` — so a hand-written source directory
/// must not use one. Second, the names match at any depth, so a matching
/// directory nested below the repository root is ambiguous: a legitimate
/// source directory can be named `target`, `build`, `dist`, `book`, or `out`.
/// Nested matches from that ambiguous subset are recorded in
/// [`super::Discovery::skipped`] so the run names the directory instead of
/// dropping its sources silently, while a root-level `target/` or `build/`
/// stays the conventional build output and is silent. Skipping the list keeps
/// the walk finite, independent of what tooling last ran in the tree, and —
/// because untracked files are in default scope — keeps a local virtualenv
/// from flooding the gate with third-party prose.
pub(super) const SKIPPED_DIRECTORIES: [&str; 21] = [
    ".git",
    "target",
    "book",
    ".beads",
    "node_modules",
    "vendor",
    "third_party",
    ".venv",
    "venv",
    ".tox",
    ".mypy_cache",
    ".ruff_cache",
    "dist",
    "build",
    "out",
    ".next",
    ".turbo",
    ".svelte-kit",
    "coverage",
    ".pytest_cache",
    "__pycache__",
];

/// Skip-list names that are also plausible source-directory names.
///
/// Only the ambiguous subset is reported when it appears below the root: a
/// nested `.venv` or `node_modules` is unambiguously machine-generated, while
/// a nested `target` may be hand-written code the gate would otherwise hide.
pub(super) const AMBIGUOUS_SKIPPED_DIRECTORIES: [&str; 5] =
    ["target", "build", "dist", "book", "out"];
