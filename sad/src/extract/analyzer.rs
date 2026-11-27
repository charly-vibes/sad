//! Purpose: turn source text into an [`AnalyzedFile`] — the reusable per-run
//! entry point and the single-file convenience wrapper.
//! Responsibilities: resolve the language from the path, normalise line
//! endings, parse with the pinned grammar, and assemble the analysed file
//! (detected header, walked comment nodes, whole-file exemption).
//! Rationale: compiling the marker pattern and configuring a parser are
//! per-run costs, not per-file ones, so [`Analyzer`] keeps that state for a
//! whole tree walk while [`analyze_file`] preserves the same contract for
//! callers that analyse exactly one file.

use std::path::Path;

use tree_sitter::{Parser, Tree};

use crate::attribution::MarkerMatcher;
use crate::config::Config;
use crate::model::{AnalyzedFile, Language};

use super::{
    WalkContext, collect_nodes, compute_exempt, detect_header, empty_file, language_for_path,
    normalise_source,
};

/// Analyse one source file exactly as the gate would.
///
/// Convenience wrapper for single-file callers (tests, one-off tools): it
/// compiles its own marker matcher and parser. A whole run reuses [`Analyzer`]
/// so that work happens once.
///
/// `path` selects the grammar; `rel_path` is the stable path reported in the
/// returned [`AnalyzedFile`]. Unsupported extensions yield an empty analysis
/// with `Rust` as a neutral placeholder — callers filter by
/// [`language_for_path`] before relying on the result.
pub fn analyze_file(path: &Path, rel_path: &str, source: &str, cfg: &Config) -> AnalyzedFile {
    let Some(language) = language_for_path(path).or_else(|| language_for_path(Path::new(rel_path)))
    else {
        // Documented contract: no grammar is ever guessed for an unsupported
        // extension, so the result is empty rather than a wrong-grammar parse.
        return empty_file(rel_path, Language::Rust);
    };
    let normalised = normalise_source(source);
    let source = normalised.as_ref();
    let markers = MarkerMatcher::new(&cfg.marker_pattern);
    let kit = MarkerKit {
        prefix: &cfg.marker_prefix,
        matcher: &markers,
    };
    let Some(parsed) = parse_source(language, source) else {
        return empty_file(rel_path, language);
    };
    assemble(rel_path, &parsed, kit)
}

/// Reusable extraction state for one gate run: the compiled marker matcher and
/// one configured tree-sitter parser per language.
///
/// Compiling the marker pattern and loading grammars are per-run costs, not
/// per-file ones, so a run creates one of these and analyses every file
/// through it.
pub struct Analyzer {
    markers: MarkerMatcher,
    /// Generated-code marker prefix from the run's config.
    marker_prefix: String,
    parsers: [Option<Parser>; 3],
}

impl Analyzer {
    /// Build the run's matcher, prefix, and parsers from `cfg`.
    pub fn new(cfg: &Config) -> Self {
        Self {
            markers: MarkerMatcher::new(&cfg.marker_pattern),
            marker_prefix: cfg.marker_prefix.clone(),
            parsers: [
                parser_for(Language::Rust),
                parser_for(Language::Python),
                parser_for(Language::TypeScript),
            ],
        }
    }

    /// Analyse `source` with the run's matcher and the language's parser.
    ///
    /// The contract matches [`analyze_file`], including the unsupported
    /// extension placeholder: the language is resolved from `rel_path`, which
    /// is both the reported path and the path the grammar is picked from.
    /// `Parser::parse` resets incremental state, so reuse cannot leak one
    /// file's tree into the next.
    pub fn analyze(&mut self, rel_path: &str, source: &str) -> AnalyzedFile {
        let Some(language) = language_for_path(Path::new(rel_path)) else {
            return empty_file(rel_path, Language::Rust);
        };
        let normalised = normalise_source(source);
        let source = normalised.as_ref();
        let tree = self.parsers[parser_slot(language)]
            .as_mut()
            .and_then(|parser| parser.parse(source, None));
        let Some(tree) = tree else {
            return empty_file(rel_path, language);
        };
        let parsed = Parsed {
            language,
            source,
            tree,
        };
        assemble(rel_path, &parsed, self.marker_kit())
    }

    /// The run's marker kit: prefix and matcher, both configured at [`new`].
    fn marker_kit(&self) -> MarkerKit<'_> {
        MarkerKit {
            prefix: &self.marker_prefix,
            matcher: &self.markers,
        }
    }
}

/// One parsed file: its language, normalised source, and parse tree.
///
/// The tree owns the node storage that walking `root` reads.
struct Parsed<'t> {
    language: Language,
    source: &'t str,
    tree: Tree,
}

/// The per-run marker kit: the generated-code marker prefix and its matcher.
struct MarkerKit<'a> {
    prefix: &'a str,
    matcher: &'a MarkerMatcher,
}

/// Parse `source` with a freshly configured parser for `language`.
fn parse_source(language: Language, source: &str) -> Option<Parsed<'_>> {
    let mut parser = parser_for(language)?;
    let tree = parser.parse(source, None)?;
    Some(Parsed {
        language,
        source,
        tree,
    })
}

/// A parser configured with the pinned grammar for `language`.
pub(crate) fn parser_for(language: Language) -> Option<Parser> {
    let grammar: tree_sitter::Language = match language {
        Language::Rust => tree_sitter_rust::LANGUAGE.into(),
        Language::Python => tree_sitter_python::LANGUAGE.into(),
        Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
    };
    let mut parser = Parser::new();
    parser.set_language(&grammar).ok()?;
    Some(parser)
}

/// Index of `language` in [`Analyzer::parsers`].
fn parser_slot(language: Language) -> usize {
    match language {
        Language::Rust => 0,
        Language::Python => 1,
        Language::TypeScript => 2,
    }
}

/// Build the [`AnalyzedFile`] from a parsed file: header detection, the node
/// walk, and the whole-file exemption decision.
fn assemble(rel_path: &str, parsed: &Parsed<'_>, kit: MarkerKit<'_>) -> AnalyzedFile {
    let root = parsed.tree.root_node();
    let (header, header_span, module_doc_start) =
        detect_header(parsed.language, root, parsed.source);
    let context = WalkContext {
        language: parsed.language,
        source: parsed.source,
        marker_prefix: kit.prefix,
        markers: kit.matcher,
        header_span,
        module_doc_start,
    };
    let nodes = collect_nodes(root, context);
    let exempt = compute_exempt(&nodes);
    AnalyzedFile {
        path: rel_path.to_string(),
        language: parsed.language,
        nodes,
        exempt,
        header,
    }
}
