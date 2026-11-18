//! Purpose: enumerate the source files a gate run may analyse.
//! Responsibilities: map supported extensions, collect the adjacent
//! extensions the gate cannot parse instead of dropping them silently, skip
//! VCS metadata, dependency installs, caches, and build output, name the
//! skipped directories whose name a source directory may legitimately use,
//! honor root and nested `.gitignore` files inside a git work tree, and render
//! repo-relative forward-slashed paths in sorted order.
//! Rationale: discovery must be a pure tree walk — no clock, network, or
//! parallelism — so the same tree always yields the same file list, and
//! unreadable directories are skipped deterministically. Untracked files are
//! in default scope, so skipping tool trees keeps a local virtualenv or bundler
//! cache from changing the verdict. Adjacent extensions the gate cannot parse
//! (TSX/JSX and the TypeScript module extensions) are collected rather than
//! dropped, so the language boundary is reported instead of silent. A nested
//! skip-list name is ambiguous — `src/target/` can be hand-written source — so
//! those directories are recorded by path, which reports the gap without
//! enumerating the tree the walk just refused. Ignore files are tree content
//! too, so honoring them only inside a `.git`-bearing work tree keeps the walk
//! pure while letting authors declare generated and vendored sources out of
//! scope; ignored entries are invisible exactly like skip-list directories, and
//! a tree without `.git` walks as before. Results stay sorted and repo-relative.

use std::fs;
use std::path::Path;

use self::gitignore::IgnoreStack;

mod gitignore;

/// Extensions the gate understands (matching [`crate::extract::language_for_path`]).
const SOURCE_EXTENSIONS: [&str; 6] = ["rs", "py", "pyi", "ts", "js", "mjs"];

const UNSUPPORTED_EXTENSIONS: [&str; 4] = ["tsx", "mts", "cts", "jsx"];

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
/// [`Discovery::skipped`] so the run names the directory instead of dropping
/// its sources silently, while a root-level `target/` or `build/` stays the
/// conventional build output and is silent. Skipping the list keeps the walk
/// finite, independent of what tooling last ran in the tree, and — because
/// untracked files are in default scope — keeps a local virtualenv from
/// flooding the gate with third-party prose.
const SKIPPED_DIRECTORIES: [&str; 21] = [
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
const AMBIGUOUS_SKIPPED_DIRECTORIES: [&str; 5] = ["target", "build", "dist", "book", "out"];

#[derive(Debug, Default)]
pub(super) struct Discovery {
    pub files: Vec<String>,
    pub unsupported: Vec<String>,
    /// Ambiguous skip-list directories the walk refused, repo-relative with a
    /// trailing `/`, sorted.
    pub skipped: Vec<String>,
}

/// Every supported source file under `repo`, repo-relative, forward-slashed,
/// and sorted. Unreadable directories and symlinks are skipped.
pub(super) fn discover_files(repo: &Path) -> Discovery {
    let mut discovery = Discovery::default();
    let mut ignores = IgnoreStack::detect(repo);
    walk(repo, repo, &mut discovery, &mut ignores);
    discovery.files.sort();
    discovery.unsupported.sort();
    discovery.skipped.sort();
    discovery
}

fn walk(root: &Path, dir: &Path, discovery: &mut Discovery, ignores: &mut Option<IgnoreStack>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        if file_type.is_dir() {
            walk_directory(root, &path, &name, discovery, ignores);
        } else if file_type.is_file() {
            walk_file(root, &path, discovery, ignores);
        }
        // Symlinks are neither files nor directories here: following them
        // could loop, so the walk stays inside the real tree.
    }
}

/// One directory entry: skip-list, ignore prune, then descend with its frame.
fn walk_directory(
    root: &Path,
    path: &Path,
    name: &str,
    discovery: &mut Discovery,
    ignores: &mut Option<IgnoreStack>,
) {
    if SKIPPED_DIRECTORIES.contains(&name) {
        record_skipped_directory(root, path, name, discovery);
        return;
    }
    if is_ignored(ignores, root, path, true) {
        // WHY: pruned directories cannot be re-included from below, as in git.
        return;
    }
    if let Some(stack) = ignores.as_mut() {
        stack.enter(path);
    }
    walk(root, path, discovery, ignores);
    if let Some(stack) = ignores.as_mut() {
        stack.leave();
    }
}

/// One file entry: dropped when ignored, then sorted by supported extension.
fn walk_file(
    root: &Path,
    path: &Path,
    discovery: &mut Discovery,
    ignores: &mut Option<IgnoreStack>,
) {
    if is_ignored(ignores, root, path, false) {
        return;
    }
    let Some((supported, rendered)) = entry_kind(root, path) else {
        return;
    };
    if supported {
        discovery.files.push(rendered);
    } else {
        discovery.unsupported.push(rendered);
    }
}

/// Whether the ignore stack excludes this entry; no stack means no filtering.
fn is_ignored(ignores: &Option<IgnoreStack>, root: &Path, path: &Path, is_dir: bool) -> bool {
    ignores
        .as_ref()
        .is_some_and(|stack| stack.is_ignored(root, path, is_dir))
}

fn entry_kind(root: &Path, path: &Path) -> Option<(bool, String)> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("");
    let supported = SOURCE_EXTENSIONS.contains(&extension);
    if !supported && !UNSUPPORTED_EXTENSIONS.contains(&extension) {
        return None;
    }
    Some((supported, relative_path(root, path)?))
}

/// Record a skip-list directory the walk refused when its name is ambiguous.
///
/// A root-level `target/` or `build/` is conventional build output, so it stays
/// silent. The same name at depth two or deeper can be hand-written source, so
/// the directory path is recorded — as a path only, without enumerating its
/// files, so the warning costs no extra traversal and stays deterministic.
fn record_skipped_directory(root: &Path, dir: &Path, name: &str, discovery: &mut Discovery) {
    if !AMBIGUOUS_SKIPPED_DIRECTORIES.contains(&name) {
        return;
    }
    let Some(relative) = relative_path(root, dir) else {
        return;
    };
    if relative.contains('/') {
        discovery.skipped.push(format!("{relative}/"));
    }
}

/// Repo-relative forward-slashed rendering of `path`, or `None` above `root`.
fn relative_path(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let parts: Vec<String> = relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect();
    Some(parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_uses_repo_relative_forward_slashes_and_sorts() {
        let root = std::env::temp_dir().join(format!("sad-check-unit-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("nested")).expect("fixture tree is created");
        fs::create_dir_all(root.join("target")).expect("skipped tree is created");
        fs::write(root.join("nested/b.rs"), "fn b() {}").expect("fixture is written");
        fs::write(root.join("a.py"), "").expect("fixture is written");
        fs::write(root.join("target/skip.rs"), "fn skip() {}").expect("fixture is written");

        let discovery = discover_files(&root);
        assert_eq!(
            discovery.files,
            vec!["a.py".to_string(), "nested/b.rs".to_string()]
        );
        assert!(discovery.unsupported.is_empty());
        assert!(
            discovery.skipped.is_empty(),
            "a root-level build tree stays silent: {:?}",
            discovery.skipped
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn added_skip_names_are_skipped() {
        let root = std::env::temp_dir().join(format!("sad-check-skips-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for name in [
            "vendor",
            "third_party",
            ".mypy_cache",
            ".ruff_cache",
            "out",
            ".turbo",
            ".svelte-kit",
        ] {
            let skipped = root.join(name).join("nested");
            fs::create_dir_all(&skipped).expect("skipped tree is created");
            fs::write(skipped.join("hidden.rs"), "fn hidden() {}").expect("fixture is written");
        }

        let discovery = discover_files(&root);
        assert!(
            discovery.files.is_empty(),
            "no added skip name may reach discovery: {:?}",
            discovery.files
        );
        assert!(discovery.unsupported.is_empty());
        assert!(
            discovery.skipped.is_empty(),
            "these names are unambiguous at the root: {:?}",
            discovery.skipped
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn pyi_stubs_are_discovered() {
        let root = std::env::temp_dir().join(format!("sad-check-pyi-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src")).expect("fixture tree is created");
        fs::write(root.join("src/api.pyi"), "def f() -> None: ...\n").expect("fixture is written");

        let discovery = discover_files(&root);
        assert_eq!(discovery.files, vec!["src/api.pyi".to_string()]);
        assert!(
            discovery.unsupported.is_empty(),
            "a stub is supported, not unsupported: {:?}",
            discovery.unsupported
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn nested_ambiguous_directories_are_recorded_once() {
        let root = std::env::temp_dir().join(format!("sad-check-nested-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src/target")).expect("skipped tree is created");
        fs::create_dir_all(root.join("src/build")).expect("skipped tree is created");
        fs::create_dir_all(root.join("src/node_modules")).expect("skipped tree is created");
        fs::create_dir_all(root.join("target")).expect("root build tree is created");
        fs::write(root.join("src/target/foo.rs"), "fn foo() {}").expect("fixture is written");
        fs::write(root.join("src/build/bar.rs"), "fn bar() {}").expect("fixture is written");
        fs::write(root.join("target/skip.rs"), "fn skip() {}").expect("fixture is written");

        let discovery = discover_files(&root);
        assert_eq!(
            discovery.skipped,
            vec!["src/build/".to_string(), "src/target/".to_string()]
        );
        assert!(discovery.files.is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn unsupported_adjacent_extensions_are_collected_not_dropped() {
        let root =
            std::env::temp_dir().join(format!("sad-check-unsupported-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src")).expect("fixture tree is created");
        fs::write(root.join("src/app.js"), "f()").expect("fixture is written");
        fs::write(root.join("src/app.mjs"), "f()").expect("fixture is written");
        fs::write(root.join("src/view.tsx"), "f()").expect("fixture is written");
        fs::write(root.join("src/legacy.jsx"), "f()").expect("fixture is written");

        let discovery = discover_files(&root);
        assert_eq!(
            discovery.files,
            vec!["src/app.js".to_string(), "src/app.mjs".to_string()]
        );
        assert_eq!(
            discovery.unsupported,
            vec!["src/legacy.jsx".to_string(), "src/view.tsx".to_string()]
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn ignored_directory_cannot_be_reincluded_from_below() {
        let root = temp_root("sad-check-prune");
        fs::create_dir_all(root.join(".git")).expect("work-tree marker is created");
        generated_tree(&root, "generated/\n");
        fs::write(root.join("generated/.gitignore"), "!*\n").expect("nested rules are written");
        assert!(
            discover_files(&root).files.is_empty(),
            "a pruned directory cannot be re-included from below"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn linked_worktree_marker_file_enables_filtering() {
        let root = temp_root("sad-check-worktree");
        generated_tree(&root, "generated/\n");
        fs::write(root.join(".git"), "gitdir: /elsewhere\n").expect("marker is written");
        assert!(
            discover_files(&root).files.is_empty(),
            "a `.git` file (linked worktree) enables filtering"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn broken_root_ignore_file_degrades_to_the_plain_walk() {
        let root = temp_root("sad-check-broken");
        fs::create_dir_all(root.join(".git")).expect("work-tree marker is created");
        generated_tree(&root, "generated/\n");
        fs::write(root.join(".gitignore"), [0xff, 0xfe, b'\n']).expect("rules are written");
        assert_eq!(
            discover_files(&root).files,
            vec!["generated/api.rs".to_string()],
            "an unreadable root ignore file disables filtering entirely"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn ignore_files_outside_a_work_tree_are_not_read() {
        let root = temp_root("sad-check-nogit");
        generated_tree(&root, "generated/\n");
        assert_eq!(
            discover_files(&root).files,
            ["generated/api.rs".to_string()]
        );
        let _ = fs::remove_dir_all(&root);
    }

    fn temp_root(tag: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        root
    }

    fn generated_tree(root: &Path, ignore: &str) {
        fs::create_dir_all(root.join("generated")).expect("fixture tree is created");
        fs::write(root.join(".gitignore"), ignore).expect("root rules are written");
        fs::write(root.join("generated/api.rs"), "fn api() {}").expect("fixture is written");
    }
}
