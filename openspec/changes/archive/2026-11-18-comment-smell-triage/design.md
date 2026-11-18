# Design: comment-smell-triage

## Enforcement model

A comment is a diagnostic trigger, not a verdict. The gate answers two
questions in order:

1. Which structural class owns the node (header, doc, marker, rationale,
   suppression, exemption, prose)?
2. For prose, what kind of smell is it, and how confident is that
   classification?

Only a confident smell blocks. Everything else is visible but non-fatal.

Enforcement uses the baseline as the frontier:

- `sad check --all --adopt` records the current inventory once.
- A normal run blocks only on ids absent from `sad-baseline.json`.
- A run without a baseline reports the whole tree and blocks on every
  error.
- Warnings are adopted too, so a clean run does not repeat them; fixing a
  warning shrinks the baseline on the next adoption.

Rollout for a repository:

1. Land the gate with `sad check --all --json` in inventory mode; review
   the warning volume.
2. Run `sad check --all --adopt`, commit `sad-baseline.json`, and run
   `sad check --all` in hooks/CI.
3. New errors block; new warnings report. Add `--strict` only after the
   warning volume is under control.

Severity drift is explicit: ids are content-derived and never encode
severity, so a classifier change that promotes an adopted warning to
error stays grandfathered until re-adoption. This is documented in the
migration notes; `--strict` and adoption are the levers, not id churn.

## Finding model

`severity` is required on every finding. `smell` is nullable and only
present for findings derived from comment prose.

| finding | smell | severity |
|---|---|---|
| `comment_outside_channel` (narration/apology/todo) | that smell | error |
| `comment_outside_channel` (banner/unclassified) | that smell | warning |
| `commented_out_code` | `commented_out_code` | warning until `sad-jdw`, then error |
| `doc_summary_cap_exceeded` | `doc_cap` | warning |
| `missing_header` | null | error |
| `unresolvable_marker` | null | error |
| `suppression_cap_exceeded` | null | error |

Serialization: snake_case strings, both fields always present, `smell:
null` for structural findings.

## Smell classification

Classification runs once, after every check, as a shared post-pass over
all findings in `check::collect_findings`. It runs on each maximal
adjacent prose run so a multi-line comment is judged as one unit; every
member node keeps its own id and receives the run's smell/severity.
Narration scope includes function bodies, module-level statements, and
class bodies.

| smell | deterministic rule | severity |
|---|---|---|
| `todo` | body matches `\b(TODO\|FIXME\|XXX)\b` and no resolvable marker is in the node | error |
| `apology` | body matches `\bfor now\b`, `\btemporary (fix\|hack\|workaround\|shim\|patch)\b`, `\bworkaround\b`, `\bhack\s*:`, `\blegacy shim\b`, or `\bto be removed\b`; bare "temporary"/"hack" is not enough; no resolvable marker | error |
| `narration` | see algorithm below | error |
| `banner` | body has no alphanumerics, or is a run of ≥3 repeated separator characters; applies to `commented_out_code` bodies too | warning |
| `commented_out_code` | block predicate matched a non-banner body | warning |
| `doc_cap` | `doc_summary_cap_exceeded` | warning |
| `unclassified` | prose that matches none of the above | warning |

### Narration algorithm

1. Take the maximal adjacent prose run and the first non-empty,
   non-comment source line after it in the same scope.
2. Tokenize both, lowercased: identifiers/literals of length ≥4.
3. Drop a fixed English/stopword/keyword list (`this`, `that`, `with`,
   `from`, `then`, `when`, `where`, `which`, `will`, `have`, `into`,
   `over`, `under`, `about`, `after`, `before`, `these`, `those`,
   `their`, `there`, `function`, `value`, `return`, `create`, `make`,
   `call`, `line`, plus language keywords).
4. `narration` when at least two distinct comment tokens appear in the
   next statement, or the run starts with `now`/`then`/`first`/`next`/
   `finally` and the next statement contains a call or identifier named
   in the run.
5. If there is no next statement, or no scope, the run is
   `unclassified`.

The classifier is biased to false negatives: ambiguity yields
`unclassified`, which is a warning. False-positive errors are not
acceptable.

## Rationale channel

A leading `SAFETY:`, `INVARIANT:`, `SECURITY:`, or `WHY:` token with a
non-empty body attributes the node to the rationale class, counted in
`stats.comments_rationale`. `NOTE:` stays prose because it usually
labels narration. Rationale is not debt.

Precedence is structural and fixed:
file-header > doc > marker > rationale > suppression > exemption > prose.
So `//! SAFETY:` is a header, `/// SAFETY:` is doc, and
`// see sad-1 SAFETY: ...` is a marker.

## Doc cap

`doc_summary_cap` becomes a sentence count over the summary lead (text
up to the first blank line). Strip inline code spans and URLs. A
boundary is `.`, `!`, or `?` followed by whitespace and an uppercase
character, digit, quote, or end of lead. Do not split on a fixed
abbreviation/version set (`e.g.`, `i.e.`, `etc.`, `vs.`, `cf.`,
`approx.`, `v0.1`, `Node.js`). One sentence passes regardless of source
line count; more than `doc_summary_cap` sentences is a `doc_cap`
warning.

## Verdict and envelope

- `stats.findings` counts every new finding after baseline suppression.
- `stats.findings_blocking` counts the new error-severity findings.
- Default: `ok` is true iff `findings_blocking == 0`.
- `--strict`: `ok` is true iff `findings == 0`; `findings_blocking`
  still counts errors only.
- Exit status mirrors `ok`.
- Text mode:
  - nothing found: `sad check — pass: 0 findings in M files`
  - warnings only: `sad check — pass: 0 blocking, W warnings in M files`
  - errors: `sad check — fail: B blocking, W warnings in M files; <first>`
- Suppression directives are counted debt and do not withhold findings;
  an error smell cannot be silenced by `sad:ignore`.

Findings gain `smell` and `severity` (additive); the verdict change is
breaking and bumps the crate to 0.2.0. Consumers switch from "any
finding" to `findings_blocking` or `--strict`.

## Remediation dispositions

Per smell kind the hardcoded remediation describes the acceptable
outcomes:

- narration: rename/extract so the code says it (preferred), or delete.
- apology/todo: cite a resolvable ID (earns the marker channel) or fix.
- commented_out_code: delete and recover from VCS.
- banner: delete.
- unclassified/doc_cap: move to header/doc with a one-sentence summary,
  tag as rationale, or cite a marker. Deletion is only offered for a
  redundant comment.

The contract is "do not delete knowledge to satisfy the gate".

## Migration and compatibility

- Baseline ids are unchanged; existing repositories keep their adoption.
- `doc_summary_cap` keeps its key and default (1), now read as sentences.
- Suppression directives keep the `comment_outside_channel` check id.
- Existing any-finding-fails hooks can adopt `--strict` before upgrading;
  the CHANGELOG entry names this as the breaking change, and the fleet
  rollout updates hooks and CI configs in the same release.
- Consumers that treated any finding as failure must switch to
  `severity`/`findings_blocking` or `--strict`.

## How we will know

The change is an enforcement model, not proof of quality. The follow-up
telemetry change measures:

- remediation disposition ratio: comment-only vs code-changing fixes
  (target: code-changing > 50% within two releases);
- complexity of units touched by narration/todo fixes (via the ddl
  orchestrator, not sad);
- defect and rework rate per agent-attributed change before/after;
- warning volume per 1k diff lines (should fall, not flatline).

If the ratio stays comment-only, the gate is still a style tool and the
error set must be narrowed again.

## Risks

- Narration/apology heuristics still produce false positives: mitigated
  by the conservative rules, warning fallback, and `--strict` being
  opt-in.
- Tag prefixes can silence prose: mitigated by the closed tag set, the
  counted rationale stat, and review.
- Warnings may be ignored: mitigated by the text render, the blocking
  count, and the outcome metrics.
- Block findings stay noisy until `sad-jdw`; warning severity means no
  false blocking.
- Cross-tool escalation is deferred; the ddl follow-up closes that gap.