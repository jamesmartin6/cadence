export interface TextDiff {
  /** Index (into the *old* string) where the change starts. */
  index: number;
  removed: string;
  inserted: string;
}

/**
 * Minimal common-prefix / common-suffix diff between two strings that differ in one
 * contiguous region. Covers everything a `<textarea>` onChange can produce: a single
 * keystroke, holding backspace/delete across a selection, paste, cut, IME composition,
 * select-all-and-replace. Used in both directions: turning local keystrokes into CRDT
 * ops, and (in reverse, comparing before/after `to_string()`) figuring out where a
 * remote op landed so the local cursor can be nudged to stay in the right place.
 */
export function diffText(oldText: string, newText: string): TextDiff {
  const maxCommon = Math.min(oldText.length, newText.length);

  let start = 0;
  while (start < maxCommon && oldText[start] === newText[start]) {
    start++;
  }

  let oldEnd = oldText.length;
  let newEnd = newText.length;
  while (oldEnd > start && newEnd > start && oldText[oldEnd - 1] === newText[newEnd - 1]) {
    oldEnd--;
    newEnd--;
  }

  return {
    index: start,
    removed: oldText.slice(start, oldEnd),
    inserted: newText.slice(start, newEnd),
  };
}
