<!--
Purpose: record the disposition of every hand-maintained documentation item
when sad cut over to the kaya event-sourced record (ticket sad-9sg.7), so
future cleanup knows what was retired and why what remains still exists.
Responsibilities: enumerate the three bootstrap inventory surfaces
(openspec/changes, .wai/projects/sad, docs/src) and give each item an
explicit kept/retired disposition with its reason.
Rationale: the kaya ledger (.kaya/ledger.jsonl) and its generated
projections (docs/research/, docs/public/) replace hand-maintained
*research-record* prose, but not spec history, working notes, or book pages
that the record does not compile — deleting those silently would destroy
ledger sources and dangling links alike.
-->

# Record inventory — kaya SSOT cutover

At the bootstrap capture (`kaya init` + first `just record-capture`), the
record imported three hand-maintained surfaces:

| Surface | Items at bootstrap | Today | Disposition |
|---|---|---|---|
| `openspec/changes/` | 55 Markdown files | 50 (all under `archive/`) | **kept** — record sources |
| `.wai/projects/sad/` | 5 Markdown files | 14 | **kept** — record sources |
| `docs/src/` | 8 pages | 8 (6 hand-authored, 2 CI-generated) | **kept** — no duplicates |

Retirement bar applied (from sad-9sg.7): a `docs/src/` page is retired only
if it **truly duplicates a generated projection** in `docs/research/` or
`docs/public/` **and** nothing references it. References were grepped across
`docs/src/SUMMARY.md`, the other pages, `README.md`, `CHANGELOG.md`,
`openspec/`, CI workflows and scripts before deciding. **No page met both
conditions; nothing was `git rm`-ed.**

## 1. openspec/changes — kept (record sources)

Every archived change document is a ledger source (visible in
`docs/research/index.md` under **Sources**) and the spec history the
`openspec` workflow requires. The generated publications compile the
research *graph* (hypotheses, experiments, artifacts), not these documents,
so nothing here duplicates a projection. All files live under
`openspec/changes/archive/`:

| Change | Files | Disposition |
|---|---|---|
| `2026-11-18-add-commented-out-code` | `design.md`, `proposal.md`, `specs/comment-gate/spec.md`, `tasks.md` | kept — record source |
| `2026-11-18-comment-smell-triage` | `design.md`, `evidence.md`, `proposal.md`, `specs/comment-gate/spec.md`, `tasks.md` | kept — record source |
| `2026-11-18-commented-out-code-precision` | `design.md`, `proposal.md`, `specs/comment-gate/spec.md`, `tasks.md` | kept — record source |
| `2026-11-18-gate-field-hardening` | `design.md`, `proposal.md`, `specs/comment-gate/spec.md`, `tasks.md` | kept — record source |
| `2026-11-18-gate-skipped-sources` | `design.md`, `proposal.md`, `specs/comment-gate/spec.md`, `tasks.md` | kept — record source |
| `2026-11-18-gitignore-aware-discovery` | `design.md`, `proposal.md`, `specs/comment-gate/spec.md`, `tasks.md` | kept — record source |
| `2026-11-18-licence-header-lines` | `design.md`, `proposal.md`, `specs/comment-gate/spec.md`, `tasks.md` | kept — record source |
| `2026-11-18-pyi-stub-coverage` | `design.md`, `proposal.md`, `specs/comment-gate/spec.md`, `tasks.md` | kept — record source |
| `2026-11-18-python-file-metadata` | `proposal.md`, `specs/comment-gate/spec.md`, `tasks.md` | kept — record source |
| `2026-11-18-skipped-directory-visibility` | `design.md`, `proposal.md`, `specs/comment-gate/spec.md`, `tasks.md` | kept — record source |
| `2026-11-18-specify-gate-objective` | `design.md`, `proposal.md`, `specs/comment-gate/spec.md`, `tasks.md` | kept — record source |
| `2026-11-18-trim-comment-gate-requirements` | `proposal.md`, `specs/comment-gate/spec.md`, `tasks.md` | kept — record source |
| `2026-11-18-ts-expect-error-suppression` | `proposal.md`, `specs/comment-gate/spec.md`, `tasks.md` | kept — record source |

Note: the bootstrap capture counted 55 files; 50 remain. Git history shows no
deletions under `openspec/changes/` — the delta is capture-time counting of
working-tree state, not retired documents.

## 2. .wai/projects/sad — kept (record sources)

Imported by `kaya import wai --project sad` on every `just record-capture`;
each file is captured by digest (see the **Sources**/**Artifacts** sections
of `docs/research/index.md`). The 5 bootstrap files are the pre-cutover ones;
the 2026-11-27 files postdate the sad-9sg-5/9sg-7 record work itself.

| File | Disposition |
|---|---|
| `designs/2026-11-16-sad-gate-design-grill-me-interview-2026-11-1.md` | kept — record source |
| `designs/2026-11-17-commented-out-code-finding-class.md` | kept — record source |
| `designs/2026-11-18-gitignore-and-generated-path-off-ramp.md` | kept — record source |
| `designs/2026-11-27-green-python3-scripts-test-record-lag-py-ok-3.md` | kept — record source |
| `designs/2026-11-27-refactor-none-needed-script-vendored-nearly-ver.md` | kept — record source |
| `plans/2026-11-27-plan-sad-9sg-5-desired-behavior-pre-push-hook-r.md` | kept — record source |
| `research/2026-11-17-sad-comment-gate-premise-multi-provider-evide.md` | kept — record source |
| `research/2026-11-27-orient-sad-9sg-5-kaya-record-gates-vendor-recor.md` | kept — record source |
| `research/2026-11-27-quality-ledger-sad-9sg-5-tests-python3-scripts-te.md` | kept — record source |
| `research/2026-11-27-quality-ledger-sad-9sg-5-tests-python3-scripts-te-2.md` | kept — record source |
| `research/2026-11-27-red-command-python3-scripts-test-record-lag-py.md` | kept — record source |
| `research/2026-11-27-ro5u-fixes-none-no-review-findings-requiring-fi.md` | kept — record source |
| `research/2026-11-27-ship-sad-9sg-5-edits-complete-in-worktree-no-com.md` | kept — record source |
| `reviews/2026-11-17-rule-of-5-review-sad-comment-gate-premise-rese.md` | kept — record source |

## 3. docs/src — kept (book pages; none duplicate a projection)

The generated trees serve a different artifact class: `docs/research/` is the
compiled dossier and `docs/public/` publishes reader-question pages from the
ledger graph (overview, reference, how-to, status-compatibility, …). No
`docs/src/` page carries the same content as a generated page — where names
appear similar (`getting-started`, `status`), the generated page is a
projection of the record, not a rewrite of the hand page.

| Page | References found | Disposition |
|---|---|---|
| `SUMMARY.md` | book spine; `record-links` gate resolves it | kept — book table of contents |
| `gate-model.md` | README (×2), CHANGELOG (×2), `openspec/specs/comment-gate/spec.md` (3 requirements cite it) | kept — hand-authored gate narrative the spec requires; `docs/public/reference.md` is a research-map projection, not a duplicate |
| `getting-started.md` | README install section; CHANGELOG | kept — unique install/dev-setup content; `docs/public/getting-started.md` is a publication projection with different content |
| `hooks.md` | CHANGELOG ("How-to: Hook Wiring") | kept — Diataxis how-to with per-stage outcome tables; no generated counterpart |
| `status.md` | README status link | kept — hand status page; `docs/public/status-compatibility.md` answers a different reader question from the ledger |
| `validation.md` | CHANGELOG (measurement duty) | kept — measurement record; no generated counterpart |
| `release.md` | `docs/src/SUMMARY.md`; CI `docs.yml` | kept — **CI-generated** (written by `docs.yml` from `Cargo.toml`, gitignored); not hand-maintained, nothing to retire |
| `specs/comment-gate.md` | `docs/src/SUMMARY.md`; CI `docs.yml` | kept — **CI-generated** copy of `openspec/specs/comment-gate/spec.md` (gitignored); removing it would only be recreated by CI |

## Retired

None. At cutover no inventory item was both a true duplicate of a generated
projection and free of references, so no `git rm` was performed and
`docs/src/SUMMARY.md` is unchanged with no dangling entries.
