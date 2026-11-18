# comment-gate specification delta

## MODIFIED Requirements

### Requirement: Exemptions are observable

The system SHALL exempt licence/SPDX header blocks, `@generated` marker
regions, and paths matching `sad.toml` exclude globs from findings, and
SHALL report exemption counts in the envelope. Licence text beside an SPDX
tag SHALL count as part of the licence header block when a comment body
opens with a bounded licence marker: `Copyright` followed by a four-digit
year, `(c)`, or `©`; a leading `©` or `(c)`; `Licensed under`; `License:`;
or `All rights reserved`. Licence words without such a marker SHALL remain
finding-eligible prose.

#### Scenario: Generated file excluded

- **WHEN** a file starts with `// @generated` and matches no include
  override
- **THEN** its comments produce no findings; the envelope reports the
  file as exempt

#### Scenario: Exclusion glob

- **WHEN** `sad.toml` excludes `vendor/**` and a vendored file contains
  narration comments
- **THEN** no findings are emitted for that path

#### Scenario: Copyright line beside SPDX is licence text

- **WHEN** a file's leading comments are
  `// SPDX-License-Identifier: MIT` followed by
  `// Copyright 2026 Charly Vibes`
- **THEN** both nodes are attribution-class exemption, the exemption
  summary counts them, no `comment_outside_channel` finding is emitted,
  and the file reports exempt

#### Scenario: Licence words without a marker stay prose

- **WHEN** a comment reads `// Copyright is a legal concept, not a licence
  header` or `// License negotiation happens elsewhere`
- **THEN** it remains finding-eligible prose and emits a
  `comment_outside_channel` finding