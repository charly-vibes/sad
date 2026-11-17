//! Purpose: load and validate `sad.toml`, the single source of every tunable
//! in the documentation gate.
//! Responsibilities: the typed configuration schema with documented defaults,
//! strict TOML deserialisation (unknown keys and type mismatches are errors),
//! semantic validation (caps, prefix, marker regex), and path-based loading
//! that maps read failures to [`ConfigError::Io`] — including
//! [`Config::load_if_present`], which treats only `NotFound` as "no config".
//! Rationale: a typo'd or invalid knob must fail loudly as a typed
//! [`ConfigError`] instead of silently running the gate at defaults, and an
//! unreadable but existing config must surface rather than downgrade the gate;
//! keeping the schema in one place means checks consume typed values and never
//! parse TOML themselves.

use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use serde::Deserialize;

/// Lines of documentation summary allowed before the summary cap fires.
const DEFAULT_DOC_SUMMARY_CAP: usize = 1;
/// Prefix used in marker diagnostics (`see <id>`).
const DEFAULT_MARKER_PREFIX: &str = "see";
/// Default marker identifier pattern, matched against source text.
const DEFAULT_MARKER_PATTERN: &str = r"\b[a-z][a-z0-9]*-[a-z0-9]+\b";
/// Trailer key that marks AI-assisted commits.
const DEFAULT_ATTRIBUTION_TRAILER: &str = "Assisted-by";

/// Every tunable of the documentation gate, as read from `sad.toml`.
///
/// Absent keys fall back to [`Config::default`]; unknown keys are rejected so
/// a typo cannot silently disable a check.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Config {
    /// Maximum doc-summary lines before the `doc_summary_cap_exceeded` check.
    pub doc_summary_cap: usize,
    /// Prefix for marker diagnostics.
    pub marker_prefix: String,
    /// Regex an id must match to be a resolvable marker.
    pub marker_pattern: String,
    /// Whether source files must carry a `Purpose/Responsibilities/Rationale`
    /// header. Dogfood-only: the global default is `false`.
    pub require_header: bool,
    /// Maximum tolerated suppression debt per file; `None` means unbounded.
    pub suppression_cap: Option<usize>,
    /// Glob patterns excluded from analysis.
    pub exclude: Vec<String>,
    /// Commit-attribution settings.
    pub attribution: Attribution,
    /// Per-check on/off toggles.
    pub checks: CheckToggles,
    /// Which id resolvers may satisfy a marker.
    pub resolvers: Resolvers,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            doc_summary_cap: DEFAULT_DOC_SUMMARY_CAP,
            marker_prefix: DEFAULT_MARKER_PREFIX.to_string(),
            marker_pattern: DEFAULT_MARKER_PATTERN.to_string(),
            require_header: false,
            suppression_cap: None,
            exclude: Vec::new(),
            attribution: Attribution::default(),
            checks: CheckToggles::default(),
            resolvers: Resolvers::default(),
        }
    }
}

impl Config {
    /// Parse and validate configuration from TOML text.
    pub fn from_toml_str(text: &str) -> Result<Self, ConfigError> {
        let config: Self = toml::from_str(text).map_err(|err| ConfigError::Parse {
            message: err.to_string(),
        })?;
        config.validate()?;
        Ok(config)
    }

    /// Read, parse, and validate the configuration at `path`.
    ///
    /// A missing path, an unreadable file, or a directory is an
    /// [`ConfigError::Io`] naming the path.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        match fs::read_to_string(path) {
            Ok(text) => Self::from_toml_str(&text),
            Err(err) => Err(Self::io_error(path, err)),
        }
    }

    /// Read, parse, and validate the configuration at `path` if it exists.
    ///
    /// Only [`io::ErrorKind::NotFound`] means "absent" and returns `Ok(None)`;
    /// every other read failure is an [`ConfigError::Io`] naming the path, so
    /// an unreadable config never silently degrades discovery to defaults.
    pub fn load_if_present(path: &Path) -> Result<Option<Self>, ConfigError> {
        match fs::read_to_string(path) {
            Ok(text) => Self::from_toml_str(&text).map(Some),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(Self::io_error(path, err)),
        }
    }

    /// Map a read failure to an I/O config error naming the path.
    fn io_error(path: &Path, err: io::Error) -> ConfigError {
        ConfigError::Io {
            message: format!("failed to read config `{}`: {err}", path.display()),
        }
    }

    /// Semantic checks that TOML types cannot express.
    fn validate(&self) -> Result<(), ConfigError> {
        if self.doc_summary_cap == 0 {
            return Err(ConfigError::InvalidValue {
                key: "doc_summary_cap",
                message: "must be greater than zero; zero would suppress every summary".to_string(),
            });
        }
        if self.marker_prefix.trim().is_empty() {
            return Err(ConfigError::InvalidValue {
                key: "marker_prefix",
                message: "must not be empty or whitespace".to_string(),
            });
        }
        if let Err(err) = regex::Regex::new(&self.marker_pattern) {
            return Err(ConfigError::InvalidValue {
                key: "marker_pattern",
                message: format!("must be a valid regular expression: {err}"),
            });
        }
        Ok(())
    }
}

/// On/off switch for each named check.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct CheckToggles {
    /// Flags commentary used as a channel outside the documented path.
    pub comment_outside_channel: bool,
    /// Flags doc summaries longer than `doc_summary_cap`.
    pub doc_summary_cap_exceeded: bool,
    /// Flags markers whose id no resolver can resolve.
    pub unresolvable_marker: bool,
    /// Flags source files without the required header block.
    pub missing_header: bool,
    /// Flags files whose suppression debt exceeds `suppression_cap`.
    pub suppression_cap_exceeded: bool,
}

impl Default for CheckToggles {
    fn default() -> Self {
        Self {
            comment_outside_channel: true,
            doc_summary_cap_exceeded: true,
            unresolvable_marker: true,
            missing_header: true,
            suppression_cap_exceeded: true,
        }
    }
}

/// Which id sources may satisfy a marker.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Resolvers {
    /// Resolve marker ids against the beads tracker.
    pub beads: bool,
    /// Resolve marker ids against OpenSpec changes.
    pub openspec: bool,
}

impl Default for Resolvers {
    fn default() -> Self {
        Self {
            beads: true,
            openspec: true,
        }
    }
}

/// Commit-attribution configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Attribution {
    /// Commit trailer key asserted for AI-assisted work.
    pub trailer: String,
}

impl Default for Attribution {
    fn default() -> Self {
        Self {
            trailer: DEFAULT_ATTRIBUTION_TRAILER.to_string(),
        }
    }
}

/// Typed failure from parsing or loading configuration.
#[derive(Debug, Clone)]
pub enum ConfigError {
    /// The configuration file could not be read.
    Io { message: String },
    /// The TOML text could not be deserialised into the schema.
    Parse { message: String },
    /// A parsed value violated a semantic constraint; `key` names the TOML key.
    InvalidValue { key: &'static str, message: String },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { message } => write!(f, "{message}"),
            Self::Parse { message } => write!(f, "invalid TOML: {message}"),
            Self::InvalidValue { key, message } => {
                write!(f, "invalid value for `{key}`: {message}")
            }
        }
    }
}

impl std::error::Error for ConfigError {}
