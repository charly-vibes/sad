//! Purpose: enumerate the source files a gate run may analyse.
//! Responsibilities: map supported extensions, skip VCS metadata, build
//! output, and the tool trees the supported languages create, and render
//! repo-relative forward-slashed paths in sorted order.
//! Rationale: discovery must be a pure tree walk — no clock, network, or
//! parallelism — so the same tree always yields the same file list, and
//! unreadable directories are skipped deterministically. Untracked files are
//! in default scope, so skipping tool trees keeps a local virtualenv or bundler
//! cache from changing the verdict.

use std::fs;
use std::path::Path;

/// Extensions the gate understands (matching [`crate::extract::language_for_path`]).
const SOURCE_EXTENSIONS: [&str; 3] = ["rs", "py", "ts"];

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

/// Every supported source file under `repo`, repo-relative, forward-slashed,
/// and sorted. Unreadable directories and symlinks are skipped.
pub(super) fn discover_files(repo: &Path) -> Vec<String> {
    let mut files = Vec::new();
    walk(repo, repo, &mut files);
    files.sort();
    files
}

fn walk(root: &Path, dir: &Path, files: &mut Vec<String>) {
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
            walk(root, &entry.path(), files);
            continue;
        }
        if !file_type.is_file() {
            // Symlinks are neither files nor directories here: following them
            // could loop, so the walk stays inside the real tree.
            continue;
        }
        let is_source = entry
            .path()
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| SOURCE_EXTENSIONS.contains(&extension));
        if !is_source {
            continue;
        }
        let path = entry.path();
        let Ok(relative) = path.strip_prefix(root) else {
            continue;
        };
        let parts: Vec<String> = relative
            .components()
            .map(|component| component.as_os_str().to_string_lossy().into_owned())
            .collect();
        files.push(parts.join("/"));
    }
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

        let files = discover_files(&root);
        assert_eq!(files, vec!["a.py".to_string(), "nested/b.rs".to_string()]);
        let _ = fs::remove_dir_all(&root);
    }
}
