// The types of `indent.mjs`.
//
// The module itself is plain JavaScript so that `node --test` can run
// its rules without a build or a running editor; this is what the
// TypeScript beside it needs to know about it.

/** How the editor wants the file indented. */
export interface Indentation {
  /** How many spaces one level is. */
  tabSize: number;
  /** Spaces when true, a tab when false. */
  insertSpaces: boolean;
}

/**
 * The same text, with the spaces at the front of each line rebuilt.
 *
 * The lines are the same lines in the same order: nothing is cut,
 * joined or reordered, and the document's line endings are kept.
 */
export function reindent(text: string, options: Indentation): string;
