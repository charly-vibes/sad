"""Purpose: exercise the Python suppression directive forms.
Responsibilities: hold one noqa line and one type-ignore line.
Rationale: every directive form must be counted exactly once.
"""


def compute(values):
    total = 0  # noqa
    # type: ignore
    for value in values:
        total += value
    return total