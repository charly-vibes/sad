//! Purpose: contract tests for `sad.toml` configuration (`sad::config`).
//! Responsibilities: pin defaults, full-config parsing, strict unknown-key and
//! type rejection, semantic validation, I/O error mapping, and the
//! dogfood-only `require_header` default.
//! Rationale: the config is the single source of every tunable in the gate;
//! callers depend on defaults being stable and on every mistake surfacing as a
//! typed `ConfigError` variant rather than a silently ignored key.

use std::path::{Path, PathBuf};

use sad::config::{Attribution, CheckToggles, Config, ConfigError, Resolvers};

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/config")
}

fn fixture(name: &str) -> PathBuf {
    fixtures_dir().join(name)
}

fn fixture_text(name: &str) -> String {
    std::fs::read_to_string(fixture(name))
        .unwrap_or_else(|err| panic!("fixture {name} must be readable: {err}"))
}

#[track_caller]
fn parse_fixture(name: &str) -> Config {
    let text = fixture_text(name);
    match Config::from_toml_str(&text) {
        Ok(config) => config,
        Err(err) => panic!("fixture {name} must parse, got {err:?}"),
    }
}

#[track_caller]
fn parse_error(text: &str) -> ConfigError {
    match Config::from_toml_str(text) {
        Ok(_) => panic!("expected the config text to be rejected, but it parsed"),
        Err(err) => err,
    }
}

#[track_caller]
fn assert_parse_error(text: &str) {
    match parse_error(text) {
        ConfigError::Parse { message } => {
            // Parse errors must explain themselves; an empty message is a bug.
            assert!(!message.is_empty(), "ConfigError::Parse carries a message");
        }
        other => panic!("expected ConfigError::Parse, got {other:?}"),
    }
}

#[track_caller]
fn assert_parse_error_fixture(name: &str) {
    assert_parse_error(&fixture_text(name));
}

#[track_caller]
fn assert_invalid_value(text: &str, expected_key: &str) {
    match parse_error(text) {
        ConfigError::InvalidValue { key, message } => {
            // Semantic validation names the offending key and says why.
            assert_eq!(key, expected_key, "InvalidValue names the offending key");
            assert!(
                !message.is_empty(),
                "ConfigError::InvalidValue carries a message"
            );
        }
        other => panic!("expected ConfigError::InvalidValue, got {other:?}"),
    }
}

#[track_caller]
fn assert_io_error(result: Result<Config, ConfigError>) {
    match result {
        Ok(_) => panic!("expected Config::load to fail with ConfigError::Io"),
        Err(ConfigError::Io { message }) => {
            assert!(!message.is_empty(), "ConfigError::Io carries a message");
        }
        Err(other) => panic!("expected ConfigError::Io, got {other:?}"),
    }
}

/// Every documented default, asserted field by field. `require_header` is
/// separate because the dogfood repo overrides exactly that one knob.
#[track_caller]
fn assert_defaults(config: &Config, require_header: bool) {
    let expected_checks = CheckToggles {
        comment_outside_channel: true,
        doc_summary_cap_exceeded: true,
        unresolvable_marker: true,
        missing_header: true,
        suppression_cap_exceeded: true,
    };
    assert_eq!(
        config.checks.comment_outside_channel,
        expected_checks.comment_outside_channel
    );
    assert_eq!(
        config.checks.doc_summary_cap_exceeded,
        expected_checks.doc_summary_cap_exceeded
    );
    assert_eq!(
        config.checks.unresolvable_marker,
        expected_checks.unresolvable_marker
    );
    assert_eq!(config.checks.missing_header, expected_checks.missing_header);
    assert_eq!(
        config.checks.suppression_cap_exceeded,
        expected_checks.suppression_cap_exceeded
    );

    assert_eq!(config.doc_summary_cap, 1);
    assert_eq!(config.marker_prefix, "see");
    assert_eq!(config.marker_pattern, r"\b[a-z][a-z0-9]*-[a-z0-9]+\b");

    let expected_resolvers = Resolvers {
        beads: true,
        openspec: true,
    };
    assert_eq!(config.resolvers.beads, expected_resolvers.beads);
    assert_eq!(config.resolvers.openspec, expected_resolvers.openspec);

    assert_eq!(config.require_header, require_header);
    assert_eq!(config.suppression_cap, None);
    assert!(config.exclude.is_empty(), "exclude defaults to empty");

    let expected_attribution = Attribution {
        trailer: "Assisted-by".to_string(),
    };
    assert_eq!(config.attribution.trailer, expected_attribution.trailer);
}

/// Every knob of `full.toml`, asserted so a dropped or misread key fails.
#[track_caller]
fn assert_full_config(config: &Config) {
    let expected_checks = CheckToggles {
        comment_outside_channel: false,
        doc_summary_cap_exceeded: false,
        unresolvable_marker: false,
        missing_header: false,
        suppression_cap_exceeded: false,
    };
    assert_eq!(
        config.checks.comment_outside_channel,
        expected_checks.comment_outside_channel
    );
    assert_eq!(
        config.checks.doc_summary_cap_exceeded,
        expected_checks.doc_summary_cap_exceeded
    );
    assert_eq!(
        config.checks.unresolvable_marker,
        expected_checks.unresolvable_marker
    );
    assert_eq!(config.checks.missing_header, expected_checks.missing_header);
    assert_eq!(
        config.checks.suppression_cap_exceeded,
        expected_checks.suppression_cap_exceeded
    );

    assert_eq!(config.doc_summary_cap, 3);
    assert_eq!(config.marker_prefix, "ticket");
    assert_eq!(config.marker_pattern, r"\bTICKET-[0-9]+\b");

    let expected_resolvers = Resolvers {
        beads: false,
        openspec: true,
    };
    assert_eq!(config.resolvers.beads, expected_resolvers.beads);
    assert_eq!(config.resolvers.openspec, expected_resolvers.openspec);

    assert!(config.require_header);
    assert_eq!(config.suppression_cap, Some(5));
    assert_eq!(
        config.exclude,
        vec!["vendor/**".to_string(), "**/generated/**".to_string()]
    );

    let expected_attribution = Attribution {
        trailer: "Session".to_string(),
    };
    assert_eq!(config.attribution.trailer, expected_attribution.trailer);
}

#[test]
fn default_config_pins_every_documented_default() {
    // `Config::default()` is the contract every absent key falls back to —
    // including `require_header = false`, which the sad repo itself overrides.
    let config = Config::default();
    assert_defaults(&config, false);
}

#[test]
fn empty_config_falls_back_to_all_defaults() {
    // An empty document (and a comment-only file) sets nothing, so every
    // option must come back at its documented default.
    assert_defaults(&parse_fixture("empty.toml"), false);

    let config = match Config::from_toml_str("") {
        Ok(config) => config,
        Err(err) => panic!("the empty config string must parse, got {err:?}"),
    };
    assert_defaults(&config, false);
}

#[test]
fn full_config_parses_every_knob() {
    // Every supported key is spelled out in full.toml; each must land in its
    // field with exactly the configured value.
    assert_full_config(&parse_fixture("full.toml"));
}

#[test]
fn load_reads_and_validates_a_file_from_disk() {
    // `Config::load` is the path-taking entry point and must produce the same
    // typed config as `from_toml_str` on the same bytes.
    let config = match Config::load(&fixture("full.toml")) {
        Ok(config) => config,
        Err(err) => panic!("full.toml must load, got {err:?}"),
    };
    assert_full_config(&config);
}

#[test]
fn unknown_keys_are_rejected_as_parse_errors() {
    // Unknown keys (including near-miss typos of real keys) must be rejected,
    // never silently ignored — a typo'd knob would otherwise run at defaults.
    assert_parse_error_fixture("unknown-key.toml");
    assert_parse_error_fixture("typo-key.toml");
}

#[test]
fn bad_value_types_are_parse_errors() {
    // Type mismatches fail during deserialization: negative usize, integer
    // where an option belongs, and a scalar where the checks table belongs.
    for name in [
        "doc-cap-negative.toml",
        "suppression-cap-string.toml",
        "checks-not-a-table.toml",
    ] {
        assert_parse_error_fixture(name);
    }
}

#[test]
fn malformed_toml_is_a_parse_error() {
    // Broken TOML syntax is reported as a parse error, not an I/O error and
    // not a panic.
    assert_parse_error_fixture("malformed.toml");
}

#[test]
fn zero_doc_summary_cap_is_an_invalid_value() {
    // A zero cap would disable doc summaries entirely; semantic validation
    // rejects it and names the key.
    assert_invalid_value(&fixture_text("doc-cap-zero.toml"), "doc_summary_cap");
}

#[test]
fn empty_marker_prefix_is_an_invalid_value() {
    // An empty marker prefix cannot build meaningful marker diagnostics.
    assert_invalid_value(&fixture_text("marker-prefix-empty.toml"), "marker_prefix");
}

#[test]
fn invalid_marker_pattern_is_an_invalid_value() {
    // `marker_pattern` is a regex; an unparsable pattern is a semantic error
    // naming that key.
    assert_invalid_value(
        &fixture_text("marker-pattern-invalid.toml"),
        "marker_pattern",
    );
}

#[test]
fn load_missing_file_is_an_io_error() {
    // A path that does not exist is an I/O failure, distinct from a parse
    // failure — callers use the variant to choose the remediation.
    let missing = fixtures_dir().join("this-config-does-not-exist.toml");
    assert_io_error(Config::load(&missing));
}

#[test]
fn load_directory_is_an_io_error() {
    // Reading a config path that is a directory fails at I/O, not with a
    // panic or a bogus default config.
    assert_io_error(Config::load(&fixtures_dir()));
}

#[test]
fn sad_repo_config_enables_header_requirement_only() {
    // The sad repo dogfoods `require_header = true`; every other knob stays
    // at its default, and the default (no file) is `require_header = false`.
    let config = parse_fixture("sad-repo.toml");
    assert_defaults(&config, true);
    assert!(
        !Config::default().require_header,
        "header enforcement is dogfood-only, never the global default"
    );
}
