/** Deterministic per-user color, derived from their id so it's stable across renders and
 * consistent for everyone looking at the same document. */
export function colorForUser(userId: string): string {
  let hash = 0;
  for (let i = 0; i < userId.length; i++) {
    hash = (hash * 31 + userId.charCodeAt(i)) >>> 0;
  }
  const hue = hash % 360;
  return `hsl(${hue}, 70%, 45%)`;
}
