import { useLayoutEffect, useRef, useState } from "react";
import type { RefObject } from "react";
import { colorForUser } from "../lib/color";

export interface RemoteCursor {
  userId: string;
  index: number;
}

interface Props {
  text: string;
  cursors: RemoteCursor[];
  textareaRef: RefObject<HTMLTextAreaElement | null>;
}

interface Point {
  top: number;
  left: number;
}

const MIRRORED_STYLE_PROPS = [
  "fontFamily",
  "fontSize",
  "fontWeight",
  "fontStyle",
  "lineHeight",
  "letterSpacing",
  "textTransform",
  "wordSpacing",
  "paddingTop",
  "paddingRight",
  "paddingBottom",
  "paddingLeft",
  "borderTopWidth",
  "borderRightWidth",
  "borderBottomWidth",
  "borderLeftWidth",
  "boxSizing",
  "width",
  "whiteSpace",
  "wordWrap",
  "wordBreak",
] as const;

/**
 * Renders a colored flag + caret for every other connected user's last-known cursor
 * position, overlaid on top of the (plain) `<textarea>` editor surface.
 *
 * Textareas don't expose per-character pixel coordinates, so this uses the standard
 * "mirror div" trick: an offscreen div styled identically to the textarea, holding the
 * same text up to each cursor's index, with a marker span whose `offsetTop`/`offsetLeft`
 * tells us where that character actually landed after wrapping. Approximate rather than
 * pixel-perfect (documented as a known limitation), but tracks real position/wrapping
 * far better than a naive character-count-based estimate would.
 */
export function PresenceCursors({ text, cursors, textareaRef }: Props) {
  const mirrorRef = useRef<HTMLDivElement | null>(null);
  const [positions, setPositions] = useState<Record<string, Point>>({});

  useLayoutEffect(() => {
    const textarea = textareaRef.current;
    const mirror = mirrorRef.current;
    if (!textarea || !mirror) return;

    const computed = window.getComputedStyle(textarea);
    for (const prop of MIRRORED_STYLE_PROPS) {
      mirror.style[prop] = computed[prop];
    }

    const next: Record<string, Point> = {};
    for (const cursor of cursors) {
      const index = Math.max(0, Math.min(cursor.index, text.length));
      mirror.textContent = text.slice(0, index);
      const marker = document.createElement("span");
      marker.textContent = "​"; // zero-width space, just needs to exist for offsets
      mirror.appendChild(marker);
      next[cursor.userId] = {
        top: marker.offsetTop - textarea.scrollTop,
        left: marker.offsetLeft - textarea.scrollLeft,
      };
      mirror.removeChild(marker);
    }
    setPositions(next);
  }, [text, cursors, textareaRef]);

  return (
    <div className="presence-layer" aria-hidden="true">
      <div ref={mirrorRef} className="presence-mirror" />
      {cursors.map((cursor) => {
        const pos = positions[cursor.userId];
        if (!pos) return null;
        const color = colorForUser(cursor.userId);
        return (
          <div
            key={cursor.userId}
            className="presence-cursor"
            style={{ top: pos.top, left: pos.left, borderColor: color }}
          >
            <span className="presence-cursor__flag" style={{ background: color }}>
              {cursor.userId.slice(0, 4)}
            </span>
          </div>
        );
      })}
    </div>
  );
}
