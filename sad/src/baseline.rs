//! Purpose: own the `sad-baseline.json` artifact — the stable set of finding
//! ids a repository has explicitly adopted — including its persisted shape,
//! loading, and refresh.
//! Responsibilities: define the JSON contract (`schema_version`,
//! `adopted_at`, and sorted+deduped `finding_ids`), load an existing baseline
//! (absent → `None`, unreadable or corrupt → a typed error naming the path),
//! and write a refreshed baseline with an RFC3339 UTC timestamp derived from
//! `SystemTime` without pulling in a date-time dependency.
//! Rationale: the baseline is the only sanctioned way to tolerate existing
//! prose, so its file shape is a committed contract and every read failure
//! must be loud: silently treating a corrupt file as empty would fail open and
//! hide every finding the team intended to track. Normalising ids at both
//! boundaries keeps the id set stable across hand edits and repeated adoption,
//! so adoption diffs stay reviewable. Schema version 1 predates the finding
//! occurrence discriminator and is still read; the one adoption that rewrites
//! it may add ids, because a legacy id set cannot be reproduced.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Baseline file name, relative to the repository root.
pub const BASELINE_FILE: &str = "sad-baseline.json";

/// Schema version of the persisted baseline document.
///
/// Version 2 is the finding-identity scheme that appends `#<n>` to repeated
/// base identities. Version 1 is still read so a pre-discriminator baseline
/// keeps suppressing exact matches; the next adoption migrates it.
pub const SCHEMA_VERSION: u32 = 2;

/// Older schema versions a load accepts, for migration on the next adoption.
const LEGACY_SCHEMA_VERSIONS: [u32; 1] = [1];

/// The set of grandfathered finding ids a repository has adopted.
///
/// Ids use the same stable `<path>::<symbol-path>::<content-hash>` scheme as
/// findings (plus the occurrence discriminator when a base identity repeats),
/// so unrelated edits do not churn the baseline. They are persisted sorted
/// and deduped for stable diffs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Baseline {
    pub schema_version: u32,
    /// RFC3339 UTC timestamp of the most recent adoption.
    pub adopted_at: String,
    pub finding_ids: Vec<String>,
}

/// Typed failure from reading, parsing, or writing the baseline.
///
/// Every variant names the path so the CLI envelope can point the caller at
/// the exact artifact that needs fixing.
#[derive(Debug)]
pub enum BaselineError {
    /// The baseline exists but could not be read.
    Read { path: PathBuf, message: String },
    /// The baseline was read but is not the expected JSON document.
    Parse { path: PathBuf, message: String },
    /// The refreshed baseline could not be written.
    Write { path: PathBuf, message: String },
}

impl Baseline {
    /// Load the baseline at `path`.
    ///
    /// Only [`io::ErrorKind::NotFound`] means "not adopted yet" and returns
    /// `Ok(None)`; every other read failure and every parse failure is an
    /// error naming the path, so a corrupt baseline is never mistaken for an
    /// absent one.
    pub fn load(path: &Path) -> Result<Option<Self>, BaselineError> {
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(err) => {
                return Err(BaselineError::Read {
                    path: path.to_path_buf(),
                    message: err.to_string(),
                });
            }
        };
        let mut baseline = Self::parse_checked(path, &bytes)?;
        // Tolerate hand-edited ordering/duplication without letting it change
        // gate semantics or the next persisted document.
        baseline.finding_ids = normalize(baseline.finding_ids);
        Ok(Some(baseline))
    }

    /// Parse the baseline document and refuse unsupported schema versions,
    /// naming `path` in every failure.
    fn parse_checked(path: &Path, bytes: &[u8]) -> Result<Self, BaselineError> {
        let baseline: Self = serde_json::from_slice(bytes).map_err(|err| BaselineError::Parse {
            path: path.to_path_buf(),
            message: err.to_string(),
        })?;
        if baseline.schema_version != SCHEMA_VERSION
            && !LEGACY_SCHEMA_VERSIONS.contains(&baseline.schema_version)
        {
            return Err(BaselineError::Parse {
                path: path.to_path_buf(),
                message: format!(
                    "unsupported schema_version {} (expected {SCHEMA_VERSION})",
                    baseline.schema_version
                ),
            });
        }
        Ok(baseline)
    }

    pub fn needs_migration(&self) -> bool {
        self.schema_version != SCHEMA_VERSION
    }

    /// Write a refreshed baseline at `path` from `finding_ids`.
    ///
    /// The ids are normalised before persistence. `adopted_at` is carried over
    /// from `existing` while the id set is unchanged, so a no-op re-adoption
    /// leaves the document byte-stable; otherwise the timestamp is the only
    /// clock input. The write is atomic: a sibling temp file is renamed over
    /// `path`, so an interrupted write cannot truncate the committed baseline.
    pub fn persist(
        path: &Path,
        finding_ids: &[String],
        existing: Option<&Baseline>,
    ) -> Result<Self, BaselineError> {
        let finding_ids = normalize(finding_ids.to_vec());
        let baseline = Self {
            schema_version: SCHEMA_VERSION,
            adopted_at: adopted_at_for(existing, &finding_ids),
            finding_ids,
        };
        let mut bytes = serde_json::to_vec_pretty(&baseline)
            .map_err(|err| write_error(path, &err.to_string()))?;
        bytes.push(b'\n');
        create_parent(path)?;
        write_atomic(path, &bytes)?;
        Ok(baseline)
    }
}

/// Sort and dedupe ids so hand edits and repeated adoption stay byte-stable.
fn normalize(mut ids: Vec<String>) -> Vec<String> {
    ids.sort();
    ids.dedup();
    ids
}

/// Keep the loaded stamp while the id set is unchanged; otherwise stamp now.
fn adopted_at_for(existing: Option<&Baseline>, ids: &[String]) -> String {
    match existing {
        Some(previous) if previous.finding_ids == ids => previous.adopted_at.clone(),
        _ => now_rfc3339_utc(),
    }
}

/// A typed `Write` failure naming the baseline path.
fn write_error(path: &Path, message: &str) -> BaselineError {
    BaselineError::Write {
        path: path.to_path_buf(),
        message: message.to_string(),
    }
}

/// Create the baseline's parent directory when it has one.
fn create_parent(path: &Path) -> Result<(), BaselineError> {
    let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    else {
        return Ok(());
    };
    fs::create_dir_all(parent).map_err(|err| write_error(path, &err.to_string()))
}

/// Write `bytes` to `path` via a sibling temp file renamed over the target.
///
/// The temp file lives in the target's directory so the rename stays on one
/// filesystem, and it is removed when either step fails.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), BaselineError> {
    let temp = temp_path(path);
    let result = fs::write(&temp, bytes).and_then(|()| fs::rename(&temp, path));
    if let Err(err) = result {
        let _ = fs::remove_file(&temp);
        return Err(write_error(path, &err.to_string()));
    }
    Ok(())
}

/// Sibling temp path on the target's filesystem, unique to this process.
fn temp_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| BASELINE_FILE.to_string());
    path.with_file_name(format!("{name}.tmp-{}", std::process::id()))
}

impl fmt::Display for BaselineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, message } => {
                write!(f, "failed to read baseline `{}`: {message}", path.display())
            }
            Self::Parse { path, message } => {
                write!(
                    f,
                    "failed to parse baseline `{}`: {message}",
                    path.display()
                )
            }
            Self::Write { path, message } => {
                write!(
                    f,
                    "failed to write baseline `{}`: {message}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for BaselineError {}

/// Current UTC time as `YYYY-MM-DDTHH:MM:SSZ`.
fn now_rfc3339_utc() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    format_rfc3339_utc(seconds)
}

/// Render Unix epoch `seconds` as an RFC3339 UTC timestamp.
fn format_rfc3339_utc(seconds: u64) -> String {
    let days = seconds / 86_400;
    let remainder = seconds % 86_400;
    let (year, month, day) = days_to_ymd(days as i64);
    let hour = remainder / 3_600;
    let minute = (remainder % 3_600) / 60;
    let second = remainder % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Civil date from days since 1970-01-01.
///
/// Algorithm: <https://howardhinnant.github.io/date_algorithms.html>.
fn days_to_ymd(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era: i64 = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp_is_rfc3339_utc() {
        assert_eq!(format_rfc3339_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_rfc3339_utc(1_700_000_000), "2023-11-14T22:13:20Z");
        // Century leap day (divisible by 400).
        assert_eq!(format_rfc3339_utc(951_782_400), "2000-02-29T00:00:00Z");
        // Ordinary leap day (divisible by 4, not by 100).
        assert_eq!(format_rfc3339_utc(1_709_164_800), "2024-02-29T00:00:00Z");
        // Year rollover one second early.
        assert_eq!(format_rfc3339_utc(1_704_067_199), "2023-12-31T23:59:59Z");
    }

    #[test]
    fn persist_sorts_and_dedupes_ids() {
        let root = std::env::temp_dir().join(format!("sad-baseline-unit-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("fixture tree is created");
        let path = root.join(BASELINE_FILE);
        let written = Baseline::persist(
            &path,
            &["b".to_string(), "a".to_string(), "b".to_string()],
            None,
        )
        .expect("baseline is written");
        assert_eq!(written.finding_ids, vec!["a", "b"]);
        let loaded = Baseline::load(&path)
            .expect("baseline loads")
            .expect("baseline is present");
        assert_eq!(loaded, written);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_unchanged_id_set_keeps_adopted_at() {
        let root = std::env::temp_dir().join(format!("sad-baseline-stamp-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("fixture tree is created");
        let path = root.join(BASELINE_FILE);
        let existing = Baseline {
            schema_version: SCHEMA_VERSION,
            adopted_at: "2000-02-29T00:00:00Z".to_string(),
            finding_ids: vec!["a".to_string()],
        };
        let written = Baseline::persist(&path, &["a".to_string()], Some(&existing))
            .expect("baseline is written");
        assert_eq!(written.adopted_at, existing.adopted_at);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_changed_id_set_gets_a_fresh_adopted_at() {
        let root =
            std::env::temp_dir().join(format!("sad-baseline-restamp-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("fixture tree is created");
        let path = root.join(BASELINE_FILE);
        let existing = Baseline {
            schema_version: SCHEMA_VERSION,
            adopted_at: "2000-02-29T00:00:00Z".to_string(),
            finding_ids: vec!["a".to_string()],
        };
        let written =
            Baseline::persist(&path, &["a".to_string(), "b".to_string()], Some(&existing))
                .expect("baseline is written");
        assert_ne!(written.adopted_at, existing.adopted_at);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn corrupt_baseline_is_an_error_that_names_the_path() {
        let root =
            std::env::temp_dir().join(format!("sad-baseline-corrupt-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("fixture tree is created");
        let path = root.join(BASELINE_FILE);
        fs::write(&path, "not json").expect("fixture is written");
        let error = Baseline::load(&path).expect_err("corrupt baseline must fail");
        assert!(error.to_string().contains(BASELINE_FILE));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_unsupported_schema_version_is_a_parse_error() {
        let root = std::env::temp_dir().join(format!("sad-baseline-schema-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("fixture tree is created");
        let path = root.join(BASELINE_FILE);
        fs::write(
            &path,
            r#"{"schema_version":3,"adopted_at":"2026-01-01T00:00:00Z","finding_ids":[]}"#,
        )
        .expect("fixture is written");
        let error = Baseline::load(&path).expect_err("a future schema must fail");
        let message = error.to_string();
        assert!(message.contains(BASELINE_FILE), "names the path: {message}");
        assert!(
            message.contains("schema_version 3"),
            "names the version: {message}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_version_one_baseline_loads_and_asks_for_migration() {
        let root = std::env::temp_dir().join(format!("sad-baseline-legacy-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("fixture tree is created");
        let path = root.join(BASELINE_FILE);
        fs::write(
            &path,
            r#"{"schema_version":1,"adopted_at":"2026-01-01T00:00:00Z","finding_ids":["a"]}"#,
        )
        .expect("fixture is written");
        let loaded = Baseline::load(&path)
            .expect("a legacy baseline loads")
            .expect("baseline is present");
        assert_eq!(loaded.schema_version, 1);
        assert!(loaded.needs_migration());
        let _ = fs::remove_dir_all(&root);
    }
}
