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
//! boundaries keeps the file byte-stable so adoption diffs are reviewable.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Baseline file name, relative to the repository root.
pub const BASELINE_FILE: &str = "sad-baseline.json";

/// Schema version of the persisted baseline document.
pub const SCHEMA_VERSION: u32 = 1;

/// The set of grandfathered finding ids a repository has adopted.
///
/// Ids use the same stable `<path>::<symbol-path>::<content-hash>` scheme as
/// findings, so unrelated edits do not churn the baseline. They are persisted
/// sorted and deduped for stable diffs.
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
        let mut baseline: Self =
            serde_json::from_slice(&bytes).map_err(|err| BaselineError::Parse {
                path: path.to_path_buf(),
                message: err.to_string(),
            })?;
        // Tolerate hand-edited ordering/duplication without letting it change
        // gate semantics or the next persisted document.
        baseline.finding_ids.sort();
        baseline.finding_ids.dedup();
        Ok(Some(baseline))
    }

    /// Write a refreshed baseline at `path` from `finding_ids`.
    ///
    /// The ids are sorted and deduped before persistence. The timestamp is
    /// the only clock input, so the same tree produces the same id set on
    /// every adoption.
    pub fn persist(path: &Path, finding_ids: &[String]) -> Result<Self, BaselineError> {
        let mut finding_ids = finding_ids.to_vec();
        finding_ids.sort();
        finding_ids.dedup();
        let baseline = Self {
            schema_version: SCHEMA_VERSION,
            adopted_at: now_rfc3339_utc(),
            finding_ids,
        };

        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent).map_err(|err| BaselineError::Write {
                path: path.to_path_buf(),
                message: err.to_string(),
            })?;
        }
        let bytes = serde_json::to_vec_pretty(&baseline).map_err(|err| BaselineError::Write {
            path: path.to_path_buf(),
            message: err.to_string(),
        })?;
        fs::write(path, bytes).map_err(|err| BaselineError::Write {
            path: path.to_path_buf(),
            message: err.to_string(),
        })?;
        Ok(baseline)
    }
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
    }

    #[test]
    fn persist_sorts_and_dedupes_ids() {
        let root = std::env::temp_dir().join(format!("sad-baseline-unit-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("fixture tree is created");
        let path = root.join(BASELINE_FILE);
        let written =
            Baseline::persist(&path, &["b".to_string(), "a".to_string(), "b".to_string()])
                .expect("baseline is written");
        assert_eq!(written.finding_ids, vec!["a", "b"]);
        let loaded = Baseline::load(&path)
            .expect("baseline loads")
            .expect("baseline is present");
        assert_eq!(loaded, written);
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
}
