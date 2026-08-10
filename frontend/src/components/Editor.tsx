import { useLayoutEffect, useRef } from "react";
import type { RefObject } from "react";
import { diffText, type TextDiff } from "../lib/diff";

interface EditorProps {
  text: string;
  /** Bumped by the parent each time `text` changed because of a *remote* op (not local
   * typing). The only signal Editor needs to know it should nudge the caret instead of
   * leaving it exactly where the browser already put it. */
  remoteRevision: number;
  onLocalEdit: (diff: TextDiff) => void;
  onSelectionChange: (index: number) => void;
  textareaRef: RefObject<HTMLTextAreaElement | null>;
}

/**
 * The editor surface. A plain `<textarea>` rather than contenteditable -- much simpler
 * and more reliable cursor/selection handling via `selectionStart`/`selectionEnd`, at
 * the cost of plain-text-only editing (matches the build plan's v1 scope: no rich text).
 *
 * The fiddly part: translating between "the browser just changed this DOM value" and
 * "here are the index-based insert/delete calls the CRDT needs", in both directions --
 * local keystrokes going out, and remote ops re-rendering text out from under the local
 * cursor coming in.
 */
export function Editor({ text, remoteRevision, onLocalEdit, onSelectionChange, textareaRef }: EditorProps) {
  const prevTextRef = useRef(text);

  const handleChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const newValue = e.target.value;
    const diff = diffText(prevTextRef.current, newValue);
    prevTextRef.current = newValue;
    if (diff.removed.length > 0 || diff.inserted.length > 0) {
      onLocalEdit(diff);
    }
  };

  // Remote-triggered text changes: figure out where the change landed relative to what
  // we last rendered, and nudge the local caret by the net length delta if it was sitting
  // after that point -- otherwise a remote insert earlier in the doc silently yanks the
  // user's cursor to the wrong character. Only runs on `remoteRevision` changes, never on
  // local edits (where the browser has already positioned the caret correctly).
  useLayoutEffect(() => {
    const textarea = textareaRef.current;
    const before = prevTextRef.current;
    prevTextRef.current = text;
    if (!textarea) return;

    const diff = diffText(before, text);
    const netDelta = diff.inserted.length - diff.removed.length;
    const adjust = (pos: number) => (pos > diff.index ? Math.max(diff.index, pos + netDelta) : pos);

    const nextStart = adjust(textarea.selectionStart);
    const nextEnd = adjust(textarea.selectionEnd);
    if (nextStart !== textarea.selectionStart || nextEnd !== textarea.selectionEnd) {
      textarea.setSelectionRange(nextStart, nextEnd);
    }
    // remoteRevision is the deliberate trigger; `text`/`textareaRef` are read, not watched.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [remoteRevision]);

  return (
    <textarea
      ref={textareaRef}
      className="editor"
      value={text}
      onChange={handleChange}
      onSelect={(e) => onSelectionChange(e.currentTarget.selectionStart)}
      onClick={(e) => onSelectionChange(e.currentTarget.selectionStart)}
      onKeyUp={(e) => onSelectionChange(e.currentTarget.selectionStart)}
      spellCheck={false}
      aria-label="Document editor"
      placeholder="Start typing… open this URL in another tab to see it sync live."
    />
  );
}
