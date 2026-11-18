/* Purpose: exercise the TypeScript `@ts-expect-error` directive.
   Responsibilities: hold one expect-error line inside a function body.
   Rationale: the directive is suppression debt, never prose. */

/** Adds two numbers. */
export function add(a: number, b: number): number {
    // @ts-expect-error the fixture deliberately passes strings
    return (a as unknown as string) + (b as unknown as string);
}