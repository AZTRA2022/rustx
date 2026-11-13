// The types of `tags.mjs`.
//
// The module itself is plain JavaScript so that `node --test` can run
// its rules without a build or a running editor; this is what the
// TypeScript beside it needs to know about it.

/**
 * What the `>` just typed finishes, or `undefined` when it finishes
 * nothing.

 */
export function closingFor(before: string): string | undefined;
