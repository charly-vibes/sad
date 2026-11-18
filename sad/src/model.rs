//! Purpose: the shared data contract of the documentation gate — languages,
//! comment nodes, attribution channels, stable finding IDs, and the findings
//! and suppression-debt records every later module produces.
//! Responsibilities: keep the serialised shape of those records stable,
//! normalise comment bodies before identity hashing, and pin the check-id
//! spelling that findings serialise into the CLI envelope.
//! Rationale: extraction, attribution, checks, and the envelope all speak in
//! these types; pinning names and the FNV-1a content hash here makes finding
//! IDs reproducible across runs and machines.

use genesis::envelope::RemediationEntry;
use serde::{Deserialize, Serialize};

/// Source language frontends supported in v1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    Rust,
    Python,
    #[serde(rename = "typescript")]
    TypeScript,
}

/// Syntactic form of an extracted comment node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommentKind {
    Line,
    Block,
    Doc,
}

/// The single channel a comment node is attributed to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttributionClass {
    Header,
    Doc,
    Marker,
    Suppression,
    Exemption,
    Prose,
}

/// Every suppression directive form recognised in v1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuppressionForm {
    Noqa,
    TypeIgnore,
    TsIgnore,
    EslintDisable,
    SadIgnore,
}

/// Parsed suppression directive carried by a [`CommentNode`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuppressionDirective {
    pub form: SuppressionForm,
    pub check: Option<String>,
    pub reason: Option<String>,
}

/// One comment or doc-string extracted from a source file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommentNode {
    /// 1-based source line where the node starts.
    pub line: usize,
    pub kind: CommentKind,
    pub class: AttributionClass,
    /// Nearest named-ancestor chain joined by `::`; `-` when file-scoped.
    pub symbol_path: String,
    /// Marker-stripped, whitespace-normalised comment content.
    pub body: String,
    /// Raw source text of the node.
    pub text: String,
    pub directive: Option<SuppressionDirective>,
}

/// Everything the gate learned about one analysed file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalyzedFile {
    pub path: String,
    pub language: Language,
    pub nodes: Vec<CommentNode>,
    /// True when the whole file is generated or carries only licence text.
    pub exempt: bool,
    /// True when the file opens with its header-channel block.
    pub header: bool,
}

/// FNV-1a 64-bit content hash, rendered as 16 lowercase hex characters.
///
/// Bodies are normalised while hashing: line endings collapse to `\n` and
/// trailing whitespace is stripped per line, so identity survives editor
/// churn without allocating a second copy of the body.
pub fn content_hash(body: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut first_line = true;
    for line in body.split('\n') {
        if !first_line {
            hash ^= u64::from(b'\n');
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        first_line = false;
        for byte in line.trim_end().as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    format!("{hash:016x}")
}

/// Stable finding ID: `<path>::<symbol-path>::<content-hash>`.
pub fn finding_id(path: &str, symbol_path: &str, body: &str) -> String {
    format!("{path}::{symbol_path}::{}", content_hash(body))
}

/// Check identifier used to explain a finding and to scope suppressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingKind {
    CommentOutsideChannel,
    CommentedOutCode,
    DocSummaryCapExceeded,
    UnresolvableMarker,
    MissingHeader,
    SuppressionCapExceeded,
}

impl FindingKind {
    /// The snake_case check id, identical to the serialised form.
    pub fn id(self) -> &'static str {
        match self {
            Self::CommentOutsideChannel => "comment_outside_channel",
            Self::CommentedOutCode => "commented_out_code",
            Self::DocSummaryCapExceeded => "doc_summary_cap_exceeded",
            Self::UnresolvableMarker => "unresolvable_marker",
            Self::MissingHeader => "missing_header",
            Self::SuppressionCapExceeded => "suppression_cap_exceeded",
        }
    }
}

/// One check finding, anchored to the stable ID of the offending node.
#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub check: FindingKind,
    pub id: String,
    pub path: String,
    pub line: usize,
    pub symbol_path: String,
    pub message: String,
    pub text: String,
    /// Hardcoded fix guidance for this finding kind, filled by the check that
    /// emitted the finding so the envelope is self-contained.
    pub remediation: RemediationEntry,
}

/// One counted suppression directive in the envelope's suppression debt.
#[derive(Debug, Clone, Serialize)]
pub struct SuppressionEntry {
    pub path: String,
    pub line: usize,
    pub form: SuppressionForm,
    pub check: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finding_kind_serialises_as_its_check_id() {
        for kind in [
            FindingKind::CommentOutsideChannel,
            FindingKind::CommentedOutCode,
            FindingKind::DocSummaryCapExceeded,
            FindingKind::UnresolvableMarker,
            FindingKind::MissingHeader,
            FindingKind::SuppressionCapExceeded,
        ] {
            let serialised = serde_json::to_string(&kind).expect("FindingKind must serialise");
            assert_eq!(serialised, format!("\"{}\"", kind.id()));
        }

        let finding = Finding {
            check: FindingKind::MissingHeader,
            id: "src/lib.rs::-::0123456789abcdef".to_string(),
            path: "src/lib.rs".to_string(),
            line: 1,
            symbol_path: "-".to_string(),
            message: "missing header".to_string(),
            text: "//! no header".to_string(),
            remediation: crate::remediation::for_kind(FindingKind::MissingHeader, "src/lib.rs"),
        };
        let value = serde_json::to_value(&finding).expect("Finding must serialise");
        assert_eq!(value["check"], FindingKind::MissingHeader.id());
        assert!(
            value["remediation"]["description"]
                .as_str()
                .expect("remediation description is a string")
                .contains("header")
        );
    }

    #[test]
    fn content_hash_matches_the_fnv1a_reference_vector() {
        assert_eq!(content_hash("hello"), "a430d84680aabd0b");
    }
}
