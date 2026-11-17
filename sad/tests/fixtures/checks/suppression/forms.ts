/* Purpose: exercise the TypeScript suppression directive.
   Responsibilities: hold one ts-ignore directive.
   Rationale: every directive form must be counted exactly once. */

/** Adds two numbers. */
export function add(a: number, b: number): number {
    // @ts-ignore
    return a + b;
}