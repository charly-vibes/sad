## ADDED Requirements

### Requirement: Discovery honors gitignore inside a git work tree

In a git work tree (`<repo>/.git` exists, internals unread, no upward search), discovery SHALL apply `.gitignore` rules: reached files shallow-to-deep, last match wins, ignored directories pruned. Patterns are basename-at-any-depth, `/`-anchored, `!`-negatable, trailing-`/` directory-only; `*`/`?` stay in a segment, `**` spans segments, classes and braces are literal. Missing, unreadable, or non-UTF-8 files add no rules. Tracked status is not consulted; ignored paths add no counter or warning.

#### Scenario: Gitignored sources are not gated

- **WHEN** a tree with a `.git` directory carries a root `.gitignore` covering
  `src/generated/`, and `src/generated/api.ts` holds finding-eligible prose
- **THEN** discovery omits `src/generated/api.ts`, a `sad check --all` run
  emits no finding for that path, and no counter, warning, or exemption count
  changes; the omission follows only from tree bytes plus repo-relative path,
  so an edited `.gitignore` can change which sources are gated

#### Scenario: A non-ignored source stays gated

- **WHEN** a source sits in a path that merely looks generated or vendored and
  no ignore rule matches it
- **THEN** discovery reports the file and its prose yields
  `comment_outside_channel`, proving that matching keys on ignore content
  rather than path names

#### Scenario: Negation re-includes a specific entry

- **WHEN** the root `.gitignore` ignores `src/generated/*.ts` and a later
  `!src/generated/keep.ts` re-includes one entry
- **THEN** `src/generated/keep.ts` is discovered and its prose yields
  `comment_outside_channel`, while a sibling `api.ts` is omitted

#### Scenario: A directory-only rule leaves a same-named file gated

- **WHEN** the root `.gitignore` carries a directory-only rule such as
  `gen.ts/` and the tree holds a `gen.ts` directory plus a regular `gen.ts`
  file at another path
- **THEN** the directory's contents are omitted while the regular file is
  discovered and its prose yields `comment_outside_channel`

#### Scenario: A nested ignore file overrides the root

- **WHEN** the root `.gitignore` ignores `*.ts` and `src/.gitignore` carries
  `!keep.ts`
- **THEN** `src/keep.ts` is discovered and gated while a `.ts` file outside
  `src/` is omitted

#### Scenario: Without a git work tree the walk is unchanged

- **WHEN** a tree contains a `.gitignore` but no `.git` entry
- **THEN** discovery reads no ignore file and a source matching a written
  pattern still yields its finding