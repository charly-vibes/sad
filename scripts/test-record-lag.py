#!/usr/bin/env python3
"""Tests for scripts/record-lag.py (sad-9sg.5).

Purpose: pin the vendored record-lag gate's contract.
Responsibilities: assert the CLI help works, the default bound is 60, and the
record-relevant path list targets sad's tree.
Rationale: the vendored script deviates from its kaya upstream by design;
these tests keep the deviations intentional and visible.
"""

import subprocess
import sys
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parent / "record-lag.py"


class RecordLagTest(unittest.TestCase):
    def test_help(self):
        result = subprocess.run(
            [sys.executable, str(SCRIPT), "--help"],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0)
        self.assertIn("--bound", result.stdout)
        self.assertIn("--mode", result.stdout)

    def test_default_bound_is_60(self):
        source = SCRIPT.read_text()
        self.assertIn('--bound", type=int, default=60', source)

    def test_sad_record_paths(self):
        source = SCRIPT.read_text()
        self.assertIn('"sad/src"', source)
        self.assertIn('"docs"', source)
        self.assertIn('"openspec"', source)
        self.assertIn('".wai"', source)


if __name__ == "__main__":
    unittest.main()
