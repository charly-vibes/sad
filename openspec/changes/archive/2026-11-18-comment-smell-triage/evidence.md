# Evidence: comment finding population

Durable record of the corpus analysis behind the proposal. Raw envelopes
lived in `/tmp/sad-inv` (ephemeral); this file is the audit trail.

## Method

- Binary: `sad` at commit `5d271c0` (release build), before this change.
- Corpus: the 48 repository directories under
  `~/para/areas/dev/gh/charly` that the prior field test used.
- Command, run once per repo from the repo root:
  `sad check --all --json > /tmp/sad-inv/<repo>.json`
- Aggregation: 3,873 files analyzed; 0 skipped or unsupported; 58,781
  findings; 176 suppression-debt entries.
- Classification: deterministic text/feature counts over every finding,
  plus manual reading of 30 prose findings with source context, 40
  non-test inline findings, all 14 `commented_out_code` findings, and 8
  sampled `doc_summary_cap_exceeded` findings.

## Totals

| class | count | share |
|---|---:|---:|
| `comment_outside_channel` | 52,316 | 89.0% |
| `doc_summary_cap_exceeded` | 6,451 | 11.0% |
| `commented_out_code` | 14 | 0.02% |

By language: `.rs` 26,802 · `.ts` 18,687 · `.py` 8,103 · `.js` 4,671 ·
`.mjs` 518.

## Composition

| measure | count | share |
|---|---:|---:|
| in test paths | 24,994 | 42.5% |
| inside the file's leading comment block | 11,169 | 19.0% |
| banner / separator bodies | 1,651 | 2.8% |
| shebang lines | 245 | 0.4% |
| generated docs + agent worktrees | 1,985 | 3.4% |
| known-mechanical union (headers, banners, shebang, generated/agent dirs) | 14,536 | 24.7% |
| TODO/FIXME/XXX | 19 | 0.03% |
| workaround/apology phrases | 26 | 0.04% |

- `doc_summary_cap_exceeded`: 2,556/6,451 (39.6%) are single-sentence
  summaries whose lead wraps across source lines.
- `commented_out_code`: 14/14 inspected bodies were separator banners,
  math formulas, or expected-value annotations; zero were dead code.

## Semantic sampling

The 30-finding prose sample was dominated by documentation headers
written with `//`/`#`, test intent, units/formulas, and design
references; roughly 10-15% of the non-test inline sample restated
adjacent code. No sampled finding matched the design thesis of agent
narration or apology spray.

## Caveats

- Single ecosystem, no control group, no before/after outcome data.
- Semantic shares are sampled estimates; the mechanical counts are
  exhaustive.
- Corpus state is the working trees on 2026-11-18, not pinned commits.