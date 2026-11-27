#!/usr/bin/env bash
# tests-pass.sh — oracle for the tdd-ro5 pipeline: run the narrow test suite.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
python3 scripts/test-record-lag.py
