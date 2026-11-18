# Design: comment-smell-triage

## Enforcement model

A comment is a diagnostic trigger, not a verdict. The gate answers two
questions in order:

1. Is the comment inside a legitimate channel or tagged rationale? Then it
   is not debt (except suppression, which is counted).
2. If it is prose, what kind of smell is it, and how confident is that
   classification?

Only a confident smell blocks. Everything else is visible but non-fatal.
This matches the investigation: the population is dominated by
documentation and test intent, and a blanket error converts the gate into
a style enforcer.

Enforcement uses the baseline as the frontier:

- `sad check --all --adopt` records the current inventory once.
- A normal run blocks only on ids absent from `sad-baseline.json`; ids are
  content-derived and independent of severity, so classification changes
  never churn the baseline.
- A run without a baseline reports the whole tree and blocks on every
  error — the inventory mode used by the field investigation.
- Warnings are adopted too, so a clean run does not repeat them; fixing a
  warning shrinks the baseline on the next adoption.

## Smell classification

Deterministic, structural first, textual only where confidence is high.

| smell | evidence | severity |
|---|---|---|
| `todo` | word-boundary TODO/FIXME/XXX with no resolvable marker | error |
| `apology` | "for now", "temporary", "workaround", "hack", "legacy shim" with no resolvable marker | error |
| `narration` | inside a function body, and either the body restates identifiers/literals of the next non-empty statement (distinctive-token overlap) or starts with a sequencer (`now`, `then`, `first`, `next`, `finally`) and names the next call | error |
| `banner` | body has no alphanumerics, or is a separator run; includes separator-only `commented_out_code` bodies | warning |
| `doc_cap` | `doc_summary_cap_exceeded` | warning |
| `commented_out_code` | block predicate matched a non-banner body | error |
| `unclassified` | finding-eligible prose that matches none of the above | warning |

`unclassified` is deliberate: a `// Purpose:` header block in TypeScript,
a test scenario comment, or an inline domain explanation is prose outside
the channels but not evidence of inexpressiveness. It is a warning to
move or tag, not a failure.

The narration detector is the only semantic-looking heuristic and is
intentionally biased to false negatives: any ambiguity yields
`unclassified`. False-negative debt is visible in the warning count;
false-positive errors are not acceptable.

## Rationale channel

A leading `SAFETY:`, `INVARIANT:`, or `SECURITY:` token attributes the
node to the `rationale` class. It must have a non-empty body. Rationale
is counted in `stats.comments_rationale` so its growth is observable, but
it is not debt. This keeps required explanations (Rust unsafe contracts)
out of the finding stream without forcing every rationale into the beads
ledger; traceable decisions still use the marker channel.

Attribution stays purely structural (leading token), as the deployed
channel requirement demands.

## Doc cap

`doc_summary_cap` becomes a sentence count over the summary lead (text up
to the first blank line). One sentence passes regardless of how many
source lines it wraps. Content after the blank line is uncapped. A
multi-sentence lead is `doc_cap` at warning severity, because a
structured summary is documentation quality, not code debt.

Rejected alternative: keep lines but exempt wrapped sentences. That
requires guessing line-width intent and still flags idiomatic Rust API
docs (the 40% single-sentence measurement).

## Verdict and envelope

- `stats.findings` counts every new finding after baseline suppression.
- `stats.findings_blocking` counts the new error-severity ones.
- `ok` is true iff `findings_blocking == 0`; the exit status mirrors `ok`.
- Text mode: `pass: N findings (0 blocking, N warnings) in M files` when
  warnings exist; the bare form is unchanged when nothing was found.
- Findings gain `smell` and `severity`. Additive for JSON consumers; the
  verdict change is breaking and bumps the crate to 0.2.0.

Warnings are not silently ignorable: they stay in the envelope, the text
render states the blocking count, and `findings_blocking` is the value CI
should chart.

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

## Migration

- Baseline ids are unchanged; existing repositories keep their adoption.
- Existing error-free runs may now pass with warnings (intended).
- `doc_summary_cap` keeps its key and default (1), now read as sentences.
- Suppression directives keep the `comment_outside_channel` check id, so
  existing directives and debt entries remain valid.
- Consumers that treated any finding as failure must switch to
  `severity`/`findings_blocking`.

## Risks

- Narration heuristics miss paraphrased narration: accepted; it falls to
  `unclassified` warning.
- Tag prefixes can be abused to silence prose: mitigated by the closed
  tag set, the counted `comments_rationale` stat, and review.
- Warnings may be ignored: mitigated by the blocking count in text mode
  and the outcome metric follow-up.
- `commented_out_code` still has low precision on formula-like comments:
  its own ticket narrows the predicate; this change at least removes
  separator bodies from the error class.
- Cross-tool escalation is deferred, so quality coupling is not enforced
  by sad alone; the ddl follow-up closes that gap.