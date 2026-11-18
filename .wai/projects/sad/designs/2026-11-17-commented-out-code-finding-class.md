# Commented-out code: a distinct finding class

Status: design note for the deferred item in `openspec/changes/specify-gate-objective/design.md` and
the proposal non-goal; implementation is a separate ticket and this note changes no spec text.

**Determinism.** Every rule is a deterministic function of source text and the pinned grammar: no
model, no heuristics, no intent inference — only "does this stripped body parse as a syntactic
unit?"
## Context
Prose and dead code share one substrate: tree-sitter comment nodes in the v1 frontends (Rust, Python,
TypeScript), finding-eligible under `comment_outside_channel`, anchored by symbol path plus body
hash. They differ only in remediation: prose moves to a channel, dies, or the code improves; dead
code is deleted, with VCS history as the archive. Applying a prose template to `// fn parse() { ... }`
is useless and teaches agents to hide code in a legitimate channel, so this class changes the
finding's *identity and remediation*, never its *eligibility* — dead code was already a finding and
now gets a truthful one, safe on by default.
## Precedence: classification, not exemption
Attribution runs first and is unchanged: every eligible node stays `Prose`, one of the six
`AttributionClass` variants. `commented_out_code` is a *finding kind* (`FindingKind`, beside
`comment_outside_channel`), not a seventh attribution class: it changes which check reports a node
group, never the node's attribution.

Prose emission stays per node, as landed and specified: one `comment_outside_channel` finding per
prose node (`sad/src/checks/outside_channel.rs`; spec `spec.md:32-34`, "for every comment node
attributed as finding-eligible prose"). Block scoping applies only to the new kind: a maximal run
of eligible nodes whose body parses as a syntactic unit emits one `commented_out_code` finding
(anchor = first node, body = normalized block body) instead of its members' per-node findings. A
12-line dead block is one finding; a predicate miss or disabled check leaves per-node prose, never
silence. (A multi-line statement cannot be recovered line-wise, so the block is the smallest
possibly-complete unit.)

Consequence: prose IDs are unchanged, but a dead-code finding hashes the block body, so it is new.
A knob flip re-labels a single-node block under the same ID; for a multi-node block it replaces one
dead-code finding with N prose findings — pick the knob state before adopting, or re-adopt.

**Spec delta required.** The deployed requirement says every finding-eligible prose node emits a
`comment_outside_channel` finding. This class keeps the node's `Prose` attribution but emits
`commented_out_code` instead, and emits per block rather than per node. Implementing it therefore
needs a spec delta in the comment-gate capability (new check + the emission exception), not just an
implementation ticket; `sad-wfj` carries the design, and the delta must land before code.

Header, doc, marker, suppression, `missing_header`, `unresolvable_marker`, suppression debt/cap, and
`doc_summary_cap_exceeded` are untouched: doc nodes never enter this class, so no double fire. The
new kind needs its own suppression id (targeting is still per-check plumbing); routed dead code
stays observable via debt counting and the cap.
## Block unit and normalization
A **block** is a maximal run of finding-eligible nodes with no row gap:
`next.line <= prev.line + prev.text.lines().count()` — the landed `CommentNode` has a start `line`
and raw `text`, no end row; text row count is the span, and same-row nodes join. A non-eligible
node, code line, blank line, or symbol boundary ends it. Anchor = first node, symbol path = the
anchor's enclosing symbol (`-` file-scoped). Normalization, fixed order:

1. `\r\n` → `\n`, lone `\r` → `\n`, drop a leading BOM.
2. Strip markers per kind: Rust/TS line — `//` plus one following `/` or `!`; Rust/TS block — `/*`
   plus one following `*`/`!`, trailing `*/`, then per line leading whitespace, one `*` only if the
   rest is empty or starts with whitespace, and at most one space; Python — one `#`, one space.
3. Never strip `#` inside a Rust/TS body: `#[test]` is code.
4. Dedent by the common whitespace prefix of non-blank lines; trim trailing whitespace and
   leading/trailing blanks; join `\n`. Empty body → prose, stop.
## Predicate (exact)
`T = parse(body, grammar(lang(file)))` with the same pinned frontend used for attribution. Dead iff:

1. `T` has no `ERROR` node and no node with `is_missing() == true`.
2. The root's named children, ignoring comments, are non-empty.
3. Every root named child kind is in the closed per-language `ROOT_ALLOW` table: items/declarations,
   import/use/export, control flow, let/var/const/assignment, `expression_statement`; unknown kinds
   reject.
4. Some node in `T` has a `SHAPE` kind: call, `new`, assignment (or augmented), declaration with a
   name/pattern, item/function/class/interface/type/enum/struct/trait/impl/mod, import/use/export,
   macro invocation, control flow, or `return`/`throw`/`raise` **carrying an argument**. Keyword-only
   `return`/`break`/`continue`/`pass`/`yield` do not count.
5. `ROOT_DENY` holds. Statement-level expressions are wrapped in `expression_statement` in all
   three grammars, so the deny applies to the *inner* expression kind, not the root child: no
   `expression_statement` payload is a bare identifier/literal, unary/binary/subscript/parenthesized
   expression, or Python annotation-only `assignment` (a `type`, no `right`). No root named child is
   a bodyless signature or `labeled_statement` (TS/JS). `ROOT_DENY` outranks `SHAPE`, so
   `Note: retry();` (TS: label) and `TODO: retry` (Python: annotation-only) are prose.

Rules 3–5 are one table pinned to the grammar version; unknown kinds fail closed to prose, and this
is the only place language knowledge lives.
## Near-misses that must not fire
| Body (stripped) | Language | Why prose |
|---|---|---|
| `This function calculates the total.` | any | parse error |
| `TODO: retry until stable` | Rust; TS | parse error (TS: label needs a statement) |
| `TODO: refactor` | Python | annotation-only `assignment`, denied by rule 5 |
| `https://example.com/spec`; `Returns Option<String>` | Rust/TS; Rust/Python | parse error |
| `list[str] \| None` | Python | binary expression, denied by rule 5 |
| `ls -la` | Rust | parse error (not a parsed binary expression) |
| `Note: retry();` | TS | root `labeled_statement` denied by rule 5 |
| `5`, `foo` | Python/TS; Rust | Rust: parse error; Python/TS: bare expression, denied by rule 5 |
| `fn f();` | Rust | bodyless signature, root deny |
| fenced code in `///` or `/**` | any | doc channel, excluded earlier |

Known false positive: `// e.g. g();` and `// for example step.where(c);` parse as calls and fire — the
doc exclusion covers doc nodes only; accept them or add SonarJS's textual prefix guard.

Deliberate: **disabled tests count** (`// #[test] fn ...` is non-executing code; intent is
undecidable, remediation identical; `#[ignore]`/`test.skip` are live code). **Wrong-language
snippets fall back to prose**, never silence. **No length or ratio thresholds.**
## Config knob
`[checks] commented_out_code = true` (default `true`; `_exceeded` is reserved for cap breaches, and
this check has no threshold). Default on: the toggle re-labels prose as one dead-code finding inside
a block, never adds findings; off downgrades to per-node prose, not silence.

Stable ID: `path::symbol-path::content-hash` — FNV-1a 64-bit over the normalized body, 16 lowercase
hex chars (`sad::model::content_hash`), as every check; the kind is not in the ID. Prose IDs are the
landed per-node IDs (line drift preserves, body edits change); a dead-code ID hashes the block body
and anchors at the first node, a set no prose node had — baselines survive a knob flip only for
single-node blocks.

## Remediation (hardcoded)
One `RemediationEntry` template per check, never configurable, never executed; `<path>` is rendered
from the finding's path at emission, not emitted verbatim:

- `command`: `git log -p --follow <path>`
- `description`: "Delete the commented-out code. If it is still needed, recover it from VCS history;
  never restore it as a comment."

`<path>` is rendered by `remediation::for_kind(kind, path)` at emission, so the persisted command
carries the finding's repo-relative anchor (see `sad/src/remediation.rs`). Deletion is the default
because the code was already decided against; history is the escape hatch for the revived minority
(below), at zero cost in git.
## Evidence status
- **Prior art (checkable):** SonarQube rule S125, "Sections of code should not be commented out";
  SonarJS PR #6696 ("Fix FP in S125: skip documentation example comments flagged as dead code")
  fixes *plain-comment* examples (`// e.g. f();`) with a textual prefix guard, not a structural
  exclusion; it does not establish sad's doc-channel exclusion (`///`, `/**`, docstrings only).
- **Prevalence (evidence-backed):** Pham & Yang, "The Secret Life of Commented-Out Source Code",
  ICPC 2020, DOI 10.1145/3387904.3389259 — six mature Java projects: 1.2–5.3% of comment lines are
  commented-out code (2.3–9.5% excluding license/Javadoc); 4.0–19.3% of commits touching the
  language include such a change; most are never revived (at most 16% uncommented once), so low
  volume supports default-on and the revival minority supports the VCS hatch.
- **Agent harm (preprint, not peer-reviewed):** Huang, Zhou, Chen, Zheng, "Comment Traps" (arXiv
  2512.20334, 2025-12-23) — defective commented-out code in context raises defects in
  Copilot/Cursor output (up to 58.17%); explicit ignore instructions cut them by at most 21.84%.
- **Bet:** deleting dead code (recovering from VCS when needed) improves outcomes; no primary source
  measures that intervention.
- **Design-inherited:** the predicate and tables; prior art used regexes (Pham & Yang: 84–100%
  precision). No primary source located showing an agent-targeted tool gates this class.
## Test plan (red phase)
| Fixture | Expect |
|---|---|
| Rust 4-line `//` fn (plain/`#[test]`); `/* let x = 1; */`; `/* * fn f() {} */` | 1 each |
| Python adjacent `#` loop; single `# x = f()`; TS `// const x = await load();`; `/* fn f(){g()} */` | 1 each |
| Rust prose, `TODO:`, URL, `// Returns Option<String>`, `// let x =` | prose findings, no dead |
| Python `# list[str] \| None`, `# TODO: refactor`; TS `// Note: retry();` | prose (rule 5) |
| Rust `// ls -la`, `// 5`, `// foo` | prose (parse error) |
| `// e.g. g();` (plain comment) | 1 dead finding — known false positive |
| Rust `///` doc with fenced code; `//!` header with code-looking lines | zero findings |
| CRLF copy of each positive, incl. `\r` before `*/` | same kind, identical ID |
| Mixed: 12-line dead block, prose, 3-line dead block, 2 prose | 2 dead (one per block) + 3 prose = 5 |
| Same fixture, knob off | 18 prose, 0 dead (per node), not the knob-on 5 |
| Adopt knob on, rerun knob off | prose IDs survive; block IDs become per-node prose IDs — re-adopt |
| Insert 10 lines above a block; then edit 1 body char | ID stable; then ID changes |

## Decision log and open questions
Settled: block (not line) as the unit *for the dead-code class*; prose emission per node; parse
predicate (not regex); classification within prose eligibility; disabled tests in, doc examples out,
no thresholds; `commented_out_code` default on.

Open: transcribe `ROOT_ALLOW`/`SHAPE`/`ROOT_DENY` per grammar pin (unknown kinds fail closed);
confirm the pretender-parity hash check if a baseline import is ever wanted; memoize parses by
`(language, block-body-hash)`; envelope stats for dead-code line counts are deferred to a spec
change; add `commented_out_code` to the suppression-ID set and `docs/src/gate-model.md`; decide
whether to accept the plain-comment documentation false positive or add the prefix guard (an
explicit exception to determinism); land the spec delta above before implementation.