//! Purpose: build the envelope warnings a gate run publishes for sources it
//! could not judge.
//! Responsibilities: turn the unsupported-extension paths discovery kept and
//! the ambiguous skip-list directories it refused into one `Warning` per path,
//! carrying the stable `rule_name`, entity id, message, and remediation the
//! JSON envelope and the text render share.
//! Rationale: a warning is the run's only record of a source the gate never
//! judged, so each constructor is a pure map over a deterministic path list
//! and the wording lives here rather than in the renderer. No message depends
//! on the clock or the machine, which keeps the envelope reproducible and the
//! CLI output in step with the JSON consumers.

use genesis::envelope::Warning;

/// One envelope warning per file whose extension the gate cannot parse.
///
/// Discovery keeps these paths instead of dropping them so the language
/// boundary is visible; the warning names the file and the supported set.
pub(super) fn unsupported_warnings(paths: &[String]) -> Vec<Warning> {
    paths
        .iter()
        .map(|path| Warning {
            rule_name: "unsupported_extension".to_string(),
            entity_id: Some(path.clone()),
            message: format!(
                "`{path}` has an extension the gate cannot parse; it was skipped, so its \
                 comments were not judged (supported extensions: .rs, .py, .pyi, .ts, .js, .mjs)"
            ),
            suggested_remediation: Some(
                "Rename to a supported extension (.rs, .py, .pyi, .ts, .js, .mjs) when the \
                 file's comments must be gated; JSX/TSX and .mts/.cts support is out of scope \
                 in v1."
                    .to_string(),
            ),
        })
        .collect()
}

/// One envelope warning per skipped directory whose name is ambiguous.
///
/// A nested `target/`, `build/`, `dist/`, `book/`, or `out/` can be a
/// hand-written source directory, so the run names every directory discovery
/// refused instead of hiding its sources. Discovery records the directory
/// itself, so the warning needs no file enumeration, and a root-level build
/// tree keeps the conventional meaning and stays silent.
pub(super) fn skipped_directory_warnings(paths: &[String]) -> Vec<Warning> {
    paths
        .iter()
        .map(|path| {
            let name = path
                .trim_end_matches('/')
                .rsplit('/')
                .next()
                .unwrap_or(path.as_str());
            Warning {
                rule_name: "skipped_source_directory".to_string(),
                entity_id: Some(path.clone()),
                message: format!(
                    "`{path}` was skipped because `{name}` is a skip-list directory name; \
                     if it holds hand-written sources, their comments were not judged"
                ),
                suggested_remediation: Some(
                    "Rename the hand-written source directory so it does not use a \
                     build/vendor/cache name (target, build, dist, book, out), or keep those \
                     names exclusively for build output."
                        .to_string(),
                ),
            }
        })
        .collect()
}
