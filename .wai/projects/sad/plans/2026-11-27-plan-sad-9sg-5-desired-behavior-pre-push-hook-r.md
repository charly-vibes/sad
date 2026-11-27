---
tags: [pipeline-run:tdd-ro5-2026-11-27-sad-9sg-5-kaya-record-gates, pipeline-step:plan]
---

PLAN sad-9sg.5: Desired behavior - pre-push hook runs kaya check; CI installs kaya@0.4.1, verifies committed record via kaya check --index, runs scripts/record-lag.py bound 60; nightly record.yml runs just record-capture then git pull --rebase + push retry loop; just record-lag recipe. Out of scope: ledger or docs/research changes, capture hooks. Test cases: python3 scripts/record-lag.py --help works, default bound 60; actionlint clean; just --dry-run record-lag. Narrow: python3 scripts/record-lag.py --help. Full: actionlint .github/workflows/*.yml && just --dry-run record-lag && kaya check. Files: lefthook.yml, .github/workflows/ci.yml, .github/workflows/record.yml, scripts/record-lag.py, justfile.
