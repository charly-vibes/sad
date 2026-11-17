# sad — comment-gate premise: multi-provider evidence review (2026-11-16/17)

Research question: does the empirical record support sad's premise —
"comments are an apology for code that failed to express itself; the fix
is a refactor, not more prose" — and its designed gate (three legitimate
channels + exempt-but-counted suppressions — channels excluded from findings
but reported in the envelope), or does Ousterhout's
comments-are-design position win?

Method: catitas multi-chat research pipeline, topic `sad-comment-gates`,
9 conversations, 8 providers with real answers (chatgpt, claude, deepseek,
gemini, grok, kimi, perplexity, zai; qwencloud = prompt echo, no answer
at collection time).
Evidence-quality ceiling: all inputs are unreviewed LLM-generated
research; provider citations (Agrawal 2026, Topalidi 2026, Hu FSE 2025,
etc.) were NOT independently verified — treat them as leads to verify,
not settled literature. Verify before any normative spec claim.
Note: 3 of the 8 (kimi, perplexity, zai (12-page memo session)) were found via corpus ledger
scan after browser-dispatch status falsely reported failure — dispatch
records are not ground truth for whether a conversation exists.

## Verdict matrix (6 questions × provider consensus)

1. **Do comment bans improve code?** INCONCLUSIVE (leaning contradicts:
   no supporting causal study; direct studies point the other way).
   No controlled experiment of a pure ban exists anywhere. Only
   correlational support (Aman 2015: excess comments 1.6–2.8×
   fault-prone; grok cites up to 3.5×). KAKEN 2013 position effect:
   pre-method comments correlate with FEWER bugs, in-method with more →
   position, not volume, is the variable that tracks quality
   (correlational; KAKEN was a non-peer-reviewed status report).
2. **Lineage.** "Apology" metaphor has a primary source: Tim Ottinger
   "Apologize in Code" (2006, butunclebob.com) — found by kimi.
   Cunningham attribution remains folklore. Clean Code quote is stripped
   of context ("at best, a necessary evil"; warns against dogmatic
   no-comment reading). Ousterhout = strongest counter; his framing
   ("comments describe what is not obvious") is largely HARMONIOUS with
   sad's three channels.
3. **Comment mass = noise?** PARTIALLY SUPPORTS. 43–51% of member docs
   boilerplate (Maalej & Robillard TSE'13); ~59% non-readability, ~1.7%
   rationale (Pascarella & Bacchelli MSR'17); 46.7% of TODOs low-quality
   (Wang et al. 2024).
   Supports sad's discrimination, not a blanket ban.
4. **≤3-line pub-API doc cap.** CONTRADICTS (6/8 providers — explicit
   CONTRADICTS verdicts from claude, gemini, grok, kimi, perplexity, zai). Rust API
   Guidelines (Errors/Panics/Safety/Examples), PEP 257, TSDoc, numpydoc
   all structurally exceed it. zai: "a design bug, not a research gap" —
   fixable before v1.
5. **Suppression debt as signal.** SUPPORTS (zai/kimi/grok) vs
   INCONCLUSIVE (perplexity: no peer-reviewed density↔defect study).
   Topalidi 2026: rubocop:disable = 41.8% of SATD at 0.5% FP vs 93.1%
   bare REVIEW — validates sad's exempt-but-counted envelope "almost
   verbatim". Hu FSE'25: ~50% of suppressions affect no warning. Keep
   the tension flagged: counting is defensible; defect prediction is not
   (yet) evidenced.
6. **Prior art.** INCONCLUSIVE; split is clean: presence-only gates
   (rustdoc missing_docs) and observability surfaces stick;
   intent-classifying gates die or get absorbed (pydocstyle→Ruff,
   JSDoc EOL 2018); TODO-blocking stays niche. No purist ban ever stuck.
   Adoption kill-switch: >10% effective false positives (Sadowski
   CACM'18).

LLM-specific evidence (undercuts the "agents spray narration" premise):
AIDev: no significant agent comment increase (+4.3%, n.s.); MANGO: more
comments RAISED pass rates; Agrawal 2026: LLM comments grammatically
worse but LLM code not disproportionately buggy (kills "bad prose ⟺ bad
code" co-symptom); 2026 controlled study: correct comments from stronger
models +17.2% pass rate for weaker models (unrelated comments −20.8%).
Prompt-side style constraints curb verbosity without hurting correctness.

## Overall confidence

LOW–MEDIUM (claude/deepseek/perplexity/chatgpt low-to-medium-low; kimi
low-to-medium; gemini medium-high outlier on one 2025 study; zai: symptom
side medium-high, treatment side low).

## Design implications for bd sad-lcy spec

Keep: three-channel architecture; header channel (ban+require);
`unresolvable_marker` (stale-ref detection is measurable — 869 stale
Linux refs study); `suppression_debt` exempt-but-counted.

Redesign before spec:
- `doc_cap_exceeded` → "summary ≤1 line + optional structured sections
  (Errors/Panics/Safety/Examples)" — cap the summary, not the doc (zai
  suggestion, adopted by consensus).
- Scope `comment_outside_channel` to LLM-touched files at v1 (zai) to
  keep effective FP < 10% on human-written legacy code.
- Position-aware extraction: flag in-body narration, exempt pre-method
  rationale (KAKEN effect).
- Channels default `warn`, escalate via `--max-warnings` (zai) — avoids
  the presence-mandate stub-noise failure mode (Go lints precedent).
- Do NOT claim suppression density predicts defects; report-only v1
  stands.

## Provenance

- Distillations: `~/para/areas/dev/gh/ak/catitas/catitas/data/distilled/
  sad-comment-gates/` (9 files)
- Canonical transcripts: `.../data/raw/sad-comment-gates/<provider>/`;
  zai corpus session + memo artifacts: `.../data/raw/_conversations/zai/
  5619db84-b139-4608-bad4-7e3df46fe994/` (incl. sad-evidence-research-
  memo.pdf, 12 pages)
- Dossier: `~/para/areas/dev/gh/ak/pinky/ledger/topics/sad-comment-gates.org`
- Prompt: `catitas/catitas/topics/sad-comment-gates/prompt.md`
- Feeds: bd sad-lcy (P1, "specify the gate objective") → EARS/openspec
  proposal next

## Consideration set v2 (post Rule-of-5 review, 2026-11-17)

Review verdict PASS after revision; 4 findings REVIEW_REQUIRED signed off
by user. Deltas from the first proposal set, driven by review findings:

### Evidence-strength relabeling (fixes design-freeze bias)
- Evidence-backed: suppression envelope (Topalidi validates almost
  verbatim), stale-marker detection (CoCC 93.6%; Linux 869-stale-refs).
- Design-inherited (no architecture-level evidence; prompt froze design):
  three-channel structure, pretender-mirror output contract, remediation
  templates, family boundary.
- Bet: header requirement (zero corpus support, prior art against
  presence mandates), premise causality (correlational only).

### Fixed knobs
- C5 marker syntax: `\b[a-z][a-z0-9]*-[a-z0-9]+\b` (matches beads IDs
  like sad-lcy/sad-8b7 — digit-required patterns were wrong), repo-
  configurable, beads-seeded default. openspec resolver no-ops until
  specs exist. Explicit-prefix channel marker remains the low-FP option.
- Header channel demoted: dogfood-only default (sad's own repo);
  explicit opt-in elsewhere; justify in-spec or cut at review.
- B8 attribution: explicit agent marker (commit trailer / session tag),
  never git-diff inference (squash/rebase/multi-author breakage) — and
  must not mask new agent edits behind grandfathered baselines.

### New sections
- F. Validation (new, mandatory): sad works must be falsifiable —
  dogfood on sad repo; finding-rate telemetry on known agent-authored
  PRs; A/B path when volume permits (corpus: kimi flagged ban
  interventions need sad's own A/B). Compliance (no comments) ≠ outcome
  (better code); the gate never verifies the refactor happened.
- C10 suppression_debt cap: default off, block-on-exceed knob — else
  rational agents route through ignore-spam (debt moves, gate doesn't).
- Reader-side cost consideration: +17.2% correct-comment transfer study
  means stripping repos can degrade downstream agent performance;
  mitigation = keep structured doc sections + markers (the high-transfer
  signal), optional docs-density report per module.

### Extended non-goals
+ non-v1 languages; non-code comment surfaces (SQL/shell/config/YAML);
+ reader-side doc quality (sad measures absence of prose, not docs that
  help agents read).

### Evidence hygiene for spec citations
Cite primary-checkable sources only (Hu FSE'25, Topalidi 2026, Rust API
Guidelines, PEP 257, CoCC, Aman 2015). Provider consensus is not a
citation. Canonical-sync chatgpt/gemini/grok before relying on them.

