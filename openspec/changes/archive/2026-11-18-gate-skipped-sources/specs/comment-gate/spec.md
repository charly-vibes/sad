# comment-gate specification delta

## ADDED Requirements

### Requirement: Unreadable sources are named by the envelope

Every selected source file the gate cannot read as UTF-8 SHALL be counted
in `stats.files_skipped` and named by exactly one `unreadable_source`
envelope warning carrying its repo-relative path and a reason: `not valid
UTF-8`, `permission denied`, `file disappeared during the run`, or the
operating-system error text for any other I/O failure. Analysis SHALL NOT
invent findings, guess an encoding, or change the exit status because of a
warning.

#### Scenario: Undecodable sources are named

- **WHEN** a tree contains one valid UTF-8 `.rs`, one latin-1 `.rs`, and
  one invalid-UTF-8 `.rs`, and `sad check --all` runs
- **THEN** `stats.files_skipped` counts both undecodable files, each has
  one `unreadable_source` warning carrying its path and `not valid UTF-8`,
  and no finding is derived from them

#### Scenario: Unreadable permissions are named

- **WHEN** a selected source file cannot be opened for reading
- **THEN** it is counted in `stats.files_skipped` and named by one
  `unreadable_source` warning with reason `permission denied`

#### Scenario: A fully readable run is unchanged

- **WHEN** every selected file is readable UTF-8
- **THEN** no `unreadable_source` warning is emitted and the text render is
  the same one-liner as before this change

### Requirement: Text mode never hides a skipped source

WHEN a check run skipped one or more selected files, the text render SHALL
append the skipped-file count and every skipped path with its reason to
its single line. Text mode SHALL read the same envelope warnings the JSON
consumer reads, so the paths and reasons cannot drift from the machine
contract.

#### Scenario: Text mode names skipped sources instead of a bare pass

- **WHEN** a run skips a latin-1 and an invalid-UTF-8 source
- **THEN** the text render names both paths with `not valid UTF-8` and the
  skipped count, instead of only reporting `pass`