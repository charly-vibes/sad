"""Purpose: hold a private helper whose docstring summary spans two lines."""


def _private_helper(values):
    """Return the sum of the values.
    Zero for an empty sequence."""
    total = 0
    for value in values:
        total += value
    return total