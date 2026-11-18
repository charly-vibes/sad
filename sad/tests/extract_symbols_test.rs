//! Purpose: contract tests for the symbol-path and stable-ID surface of
//! `sad::extract` + `sad::model`.
//! Responsibilities: pin the named-ancestor chain of symbol paths, the 64-bit
//! content hash shape, the `path::symbol::hash` finding id, and the set of
//! extensions v1 claims.
//! Rationale: finding identity and anchoring are what baseline ids and
//! remediation paths depend on, so they are pinned directly against the
//! extraction helpers.

mod support;

use std::path::Path;

use sad::extract::language_for_path;
use sad::model::{Language, content_hash, finding_id};

use support::extract::*;

// spec: Prose outside channels is a finding / findings anchor to the nearest named ancestor chain.
#[test]
fn rust_symbol_paths_chain_named_ancestors() {
    let file = analyze("rust", "nested_symbols.rs");
    assert_eq!(
        node_at(&file, 5).symbol_path,
        "-",
        "a comment outside every item is file-scoped"
    );
    assert_eq!(node_at(&file, 10).symbol_path, "Foo");
    assert_eq!(node_at(&file, 12).symbol_path, "Foo::bar");
    assert_eq!(node_at(&file, 17).symbol_path, "x");
    assert_eq!(node_at(&file, 20).symbol_path, "x::nested");
}

// spec: Prose outside channels is a finding / stable ID carries a 64-bit content hash.
#[test]
fn content_hash_is_sixteen_lowercase_hex_and_normalises_whitespace() {
    let hash = content_hash("this function calculates the total");
    assert_eq!(hash.len(), 16, "hash is 64-bit FNV-1a rendered as hex");
    assert!(
        hash.chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
        "hash is lowercase hex, got {hash:?}"
    );
    assert_eq!(
        hash,
        content_hash("this function calculates the total"),
        "the same body always hashes the same"
    );
    assert_eq!(
        content_hash("this function calculates the total"),
        content_hash("this function calculates the total   "),
        "trailing whitespace does not change identity"
    );
    assert_eq!(
        content_hash("a\nb"),
        content_hash("a\r\nb"),
        "CRLF and LF bodies share an identity"
    );
    assert_ne!(
        content_hash("this function calculates the total"),
        content_hash("this function calculates the sum"),
        "changed body text changes identity"
    );
}

// spec: Prose outside channels is a finding / ID is `path::symbol::hash`.
#[test]
fn finding_id_composes_path_symbol_and_hash() {
    let hash = content_hash("this function calculates the total");
    assert_eq!(
        finding_id(
            "src/lib.rs",
            "Foo::bar",
            "this function calculates the total"
        ),
        format!("src/lib.rs::Foo::bar::{hash}")
    );
    assert_eq!(
        finding_id("src/lib.rs", "-", "this function calculates the total"),
        format!("src/lib.rs::-::{hash}"),
        "file-scoped findings use `-` as the symbol path"
    );
}

// spec: Language frontends / v1 recognizes .rs, .py, and .ts only.
#[test]
fn language_for_path_recognises_v1_extensions() {
    assert!(matches!(
        language_for_path(Path::new("src/a.rs")),
        Some(Language::Rust)
    ));
    assert!(matches!(
        language_for_path(Path::new("pkg/a.py")),
        Some(Language::Python)
    ));
    assert!(matches!(
        language_for_path(Path::new("web/a.ts")),
        Some(Language::TypeScript)
    ));
    assert!(
        language_for_path(Path::new("web/a.tsx")).is_none(),
        "v1 does not claim TSX"
    );
    assert!(language_for_path(Path::new("docs/a.md")).is_none());
    assert!(language_for_path(Path::new("Cargo.toml")).is_none());
}
