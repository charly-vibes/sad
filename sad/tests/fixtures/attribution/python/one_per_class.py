"""Purpose: pin one node per attribution class in Python.
Responsibilities: hold module header, docstrings, marker, suppression, prose.
Rationale: attribution is structural; exported and private docstrings classify equally.
"""


def public_sum(values):
    """Return the sum of values."""
    total = 0
    # see sad-lcy: cap rule from review
    for value in values:
        total += value
    return total


def _private_helper():
    """see sad-lcy: this remains documentation, not a marker."""
    # noqa
    return None


class Exporter:
    """Summarise rows; the text '# noqa' here is not a directive."""

    def render(self):
        # this method renders the export
        return "export"