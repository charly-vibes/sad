"""Purpose: hold a docstring whose summary starts with a marker prefix."""


def _render_rows(rows):
    """see sad-lcy: render every row.
    A marker prefix must not turn documentation into a marker.
    The summary still spans three lead lines.

    Returns the rendered rows as text.
    """
    return "\n".join(str(row) for row in rows)