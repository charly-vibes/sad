/* Purpose: hold comment-looking text inside string literals.
   Responsibilities: prove literals never yield comment nodes.
   Rationale: nodes come from the grammar, not from scanning bytes. */

export function describe(value: string): string {
    const plain = "// not a comment";
    const template = `// also not a comment ${value}`;
    // real narration
    return plain + template;
}