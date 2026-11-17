"""Purpose: hold comment-looking text inside string literals.
Responsibilities: prove literals never yield comment nodes.
Rationale: nodes come from the grammar, not from scanning bytes.
"""

x = "# not a comment"
y = "'''# also not a comment'''"
# real narration
print(x, y)