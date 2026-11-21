> *"It's sad (So sad), so sad*
> *It's a sad, sad situation*
> *And it's gettin' more and more absurd*
> *It's sad (So sad), so sad*
> *Why can't we talk it over?"*
> — Elton John

# Hook Wiring

How to attach the gate to lefthook's `pre-commit` and `pre-push` stages with
`sad hooks install`, and remove it again with `sad hooks uninstall`. Both
commands report what each stage became and never leave the config half-edited
by a refused run.

## Prerequisites

- A git repository with an existing `lefthook.yml` (or `lefthook.yaml`) —
  sad wires into a config, it never creates one.
- `sad` on `PATH` ([build from source](getting-started.md#build-from-source)).

## Install

```bash
sad hooks install
```

A stage that carries no gate gains a marker-owned managed block:

```yaml
pre-commit:
  parallel: true
  commands:
# sad:managed:start
    sad-check:
      run: sad check
# sad:managed:end
```

The same block lands under `pre-push`. The wired command is the plain
[`sad check`](gate-model.md) — baseline-suppressed, failing on
error-severity findings, exactly as the CLI evaluates it.

Per-stage outcomes:

| Outcome            | Meaning                                                            |
|--------------------|--------------------------------------------------------------------|
| `injected`         | The managed block was inserted into the stage.                     |
| `already_wired`    | The managed block was already present; nothing changed.            |
| `skipped_user_gate`| The stage already runs the gate outside the block; wiring again would run it twice. |

The command refuses (non-zero exit, remediation in the envelope) when the
starting directory is not inside a git repository, when no lefthook config
exists, or when the config cannot be anchored. A stage that fails leaves
stages wired before it in place; fix the config and re-run — the wired
stages report `already_wired`.

## Marker ownership

The `# sad:managed:start` / `# sad:managed:end` pair belongs to sad. The
wiring mechanics (column-0 anchor injection, file-level idempotence) come
from genesis's `ensure_wired`; the marker and the gate command are
sad-owned. Inside the markers the block is sad's to manage; every byte
outside them is yours.

## Idempotency and coexistence

- Re-running `sad hooks install` is safe: present blocks report
  `already_wired`.
- A hand-written `run: sad check` anywhere in a stage is detected and
  reported as `skipped_user_gate` — the doubled gate never ships.
- Other managed blocks coexist untouched. This repository runs the ddl
  blocks (`# ddl:managed:start …`) beside the sad block; foreign managed
  blocks survive every install and uninstall byte-for-byte, because
  markers are matched as exact trimmed lines. Never edit a ddl-managed
  block by hand — the same ownership rule applies in reverse.

## Uninstall

```bash
sad hooks uninstall
```

| Outcome    | Meaning                                                  |
|------------|----------------------------------------------------------|
| `removed`  | The stage carried the sad managed block; it was removed. |
| `not_wired`| The stage carried no sad managed block; nothing changed. |

Removal is marker-guarded: only complete `sad:managed` blocks disappear.
The command refuses instead of guessing when the config has unbalanced
markers (a nested start marker, a stray end marker, an unterminated block),
or when a balanced sad block sits outside the two pinned stages — a stray
block would otherwise be stripped while every stage reports `not_wired`.
A stage without the block is a no-op success.

## Envelope

Like every sad command, the output is the genesis suite envelope; `--json`
prints it raw and the exit status mirrors `ok`. The `data` field carries
the two per-stage outcomes; failures carry a remediation naming the
concrete blocker.
