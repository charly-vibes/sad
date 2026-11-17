"""Purpose: prove doc-channel text never becomes suppression debt.
Responsibilities: keep a directive-looking token inside documentation.
Rationale: doc precedence wins over directive parsing.
"""


def documented():
    """Returns one; the text # noqa here is documentation."""
    return 1