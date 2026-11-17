"""Purpose: hold three suppression directives for cap tests.
Responsibilities: breach a cap of two but not a cap of three.
Rationale: the cap counts every directive in the file.
"""


def total(values):
    result = 0
    result += 1  # noqa
    # type: ignore
    for value in values:
        result += value  # noqa
    return result