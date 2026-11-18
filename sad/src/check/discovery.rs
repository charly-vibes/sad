//! Purpose: enumerate the source files a gate run may analyse.
//! Responsibilities: map supported extensions, collect the adjacent
//! extensions the gate cannot parse instead of dropping them silently, skip
//! VCS metadata, build output, and the tool trees the supported languages
//! create, and render repo-relative forward-slashed paths in sorted order.
//! Rationale: discovery must be a pure tree walk — no clock, network, or
//! parallelism — so the same tree always yields the same file list, and
//! unreadable directories are skipped deterministically. Untracked files are
//! in default scope, so skipping tool trees keeps a local virtualenv or bundler
//! cache from changing the verdict. Adjacent extensions the gate cannot parse
//! (TSX/JSX and the TypeScript module extensions) are collected rather than
//! dropped, so the language boundary is reported instead of silent, and both
//! result lists are sorted and repo-relative so warnings are reproducible.

use std::fs;
use std::path::Path;

/// Extensions the gate understands (matching [`crate::extract::language_for_path`]).
const SOURCE_EXTENSIONS: [&str; 5] = ["rs", "py", "ts", "js", "mjs"];

const UNSUPPORTED_EXTENSIONS: [&str; 4] = ["tsx", "mts", "cts", "jsx"];

/// Directory names never traversed: VCS metadata, build output, the rendered
/// book, the issue ledger, and the tool trees the supported languages create
/// (dependency installs, caches, bundler output). Skipping them keeps the walk
/// finite, independent of what tooling last ran in the tree, and — because
/// untracked files are in default scope — keeps a local virtualenv from
/// flooding the gate with third-party prose.
const SKIPPED_DIRECTORIES: [&str; 14] = [
    ".git",
    "target",
    "book",
    ".beads",
    "node_modules",
    ".venv",
    "venv",
    ".tox",
    "dist",
    "build",
    ".next",
    "coverage",
    ".pytest_cache",
    "__pycache__",
];

#[derive(Debug, Default)]
pub(super) struct Discovery {
    pub files: Vec<String>,
    pub unsupported: Vec<String>,
}

/// Every supported source file under `repo`, repo-relative, forward-slashed,
/// and sorted. Unreadable directories and symlinks are skipped.
pub(super) fn discover_files(repo: &Path) -> Discovery {
    let mut discovery = Discovery::default();
    walk(repo, repo, &mut discovery);
    discovery.files.sort();
    discovery.unsupported.sort();
    discovery
}

fn walk(root: &Path, dir: &Path, discovery: &mut Discovery) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        if file_type.is_dir() {
            if SKIPPED_DIRECTORIES.contains(&name.as_str()) {
                continue;
            }
            walk(root, &entry.path(), discovery);
            continue;
        }
        if !file_type.is_file() {
            // Symlinks are neither files nor directories here: following them
            // could loop, so the walk stays inside the real tree.
            continue;
        }
        let Some((supported, rendered)) = entry_kind(root, &entry.path()) else {
            continue;
        };
        if supported {
            discovery.files.push(rendered);
        } else {
            discovery.unsupported.push(rendered);
        }
    }
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
    let relative = path.strip_prefix(root).ok()?;
    let parts: Vec<String> = relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect();
    Some((supported, parts.join("/")))
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
}
