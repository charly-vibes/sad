/* Purpose: pin one node per attribution class in TypeScript.
   Responsibilities: hold module header, JSDoc, marker, suppression, prose.
   Rationale: attribution is structural; comments only. */

/** Adds two numbers. */
export function add(a: number, b: number): number {
    // see sad-lcy: cap rule from review
    // @ts-ignore
    const result = a + b;
    return result;
}

// this function calculates the total
export function total(items: number[]): number {
    return items.reduce((sum, item) => sum + item, 0);
}