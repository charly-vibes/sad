> *"It's sad (So sad), so sad*
> *It's a sad, sad situation*
> *And it's gettin' more and more absurd*
> *It's sad (So sad), so sad*
> *Why can't we talk it over?"*
> — Elton John

# Validation

sad does not claim "code carries no comments"; it claims gating prose produces
better code, and the [Gate Model](gate-model.md#evidence-status) records that
as a bet. A bet that cannot lose is not a design: fix the panel, publish the
numbers, state what would retire the premise.

## Panel

- **Unit:** one analyzed source file (`stats.files_analyzed`). Per-file rates,
  not raw counts, are what compare across snapshots of different size.
- **Attribution:** the file's governing revision — the last commit touching
  it — decides the side of the split. An `Assisted-by:` trailer means agent;
  no trailer means human. Files with uncommitted edits or no commit are
  counted in the totals but excluded from the split, since they have no
  governing revision ([Scope](gate-model.md#scope)).
- **Fixtures:** `sad.toml` excludes them, so `files_excluded` is input
  hygiene, not panel attrition.
- **Command:** `sad check --all --json`. `--all` lifts attribution scoping so
  human and agent files are measured in one run.
- **Envelope fields:** `stats.files_*`, `stats.comments_*`,
  `stats.suppression_debt`, `stats.findings`, `stats.findings_suppressed`,
  plus `data.findings[].check` for the per-check split.

## First published rates

Snapshot: working tree at commit `2abebd0`, 2026-11-18, pre-gate corpus. The
gate is specified but not enforced, so these findings are grandfathered by
the baseline, not fixed.

| Measure | Value |
|---|---|
| `stats.files_discovered` | 106 |
| `stats.files_excluded` (test fixtures) | 61 |
| `stats.files_selected` | 45 |
| `stats.files_analyzed` | 45 |
| `stats.files_skipped` | 0 |
| `stats.files_exempt` | 0 |
| `stats.comments_header` | 458 |
| `stats.comments_doc` | 748 |
| `stats.comments_prose` | 216 |
| `stats.comments_marker` | 0 |
| `stats.comments_suppression` | 0 |
| `stats.suppression_debt` | 0 |
| `stats.findings` | 0 |
| `stats.findings_suppressed` | 297 |
| baseline grandfathered IDs | 296 |
| `comment_outside_channel` mass, pre-adoption | 207 |
| `doc_summary_cap_exceeded` mass, pre-adoption | 82 |
| total finding mass, pre-adoption | 289 |
| agent-attributed files in history | 0 |

Derived rates (same snapshot, same denominator):

| Rate | Value |
|---|---|
| suppressed findings per analyzed file | 6.6 |
| prose comment nodes per analyzed file | 4.8 |
| header comment nodes per analyzed file | 10.2 |
| doc comment nodes per analyzed file | 16.6 |
| `comment_outside_channel` findings per analyzed file | 4.6 |
| `doc_summary_cap_exceeded` findings per analyzed file | 1.8 |
| reported findings per analyzed file | 0 |

The by-check split predates the last corrective round while the baseline
postdates it, so per-check rates do not sum to the suppressed rate. The
suppressed total exceeding the grandfathered set is unexplained; take the
larger figure as the honest pre-gate mass.

### Attribution split

Not yet measurable: sad's history carries no attribution trailers, so every
file counts as human. The split becomes meaningful once commits carry
trailers; until then these numbers support no agent-versus-human claim and
the primary falsifier is dormant.

## Decision rule

The premise is allowed to fail: with the gate adopted, measure for six months
from the commit that ships it. After that window the premise loses when both
hold:

1. Agent-attributed files show no reduction in `comment_outside_channel`
   findings per file relative to this baseline.
2. Pretender metrics (complexity, duplication, defect proxies) show no
   improvement on gated files compared with ungated history.

A single signal keeps the premise alive and extends the window; with neither,
the spec is revisited rather than defended. The header requirement is the
first candidate to cut — the bet with no corpus support.

**A/B path.** When volume permits, compare pretender metrics on matched file
sets (same language, same size band) and publish the delta beside the table.
Falsifier 2 is only decidable this way; falsifier 1 needs trailers first.

## Reporting cadence

- Rerun on every adoption and baseline change, with the same command and
  fields.
- Add a dated snapshot section; never edit an earlier snapshot.
- Label the snapshot commit; scope differs between the working tree and
  committed revisions
  ([Output contract](gate-model.md#output-contract-pretender-mirror)).
- Baselines are shrink-only, so `stats.findings` on a plain run is a compliance
  number; `stats.findings_suppressed` is the pre-gate mass and the measure of
  progress.

## Limits

- Gate compliance is not code quality. A clean run proves the absence of prose,
  not the presence of a better decomposition.
- Findings are a proxy. Counts move with grammar pins, channel caps, and knob
  values, so only same-config snapshots are comparable.
- These are cross-sectional numbers from sad's own corpus. No causal claim
  about the gate follows from them, and they say nothing about another
  repository until the same measurement is published there; only the
  longitudinal reduction and the A/B comparison bear on the premise.
