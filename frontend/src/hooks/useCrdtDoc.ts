import { useCallback, useRef, useState } from "react";
import { CrdtDoc } from "../wasm/crdt_engine";

/** An operation as produced by the WASM engine -- opaque JSON as far as the frontend's
 * networking code is concerned, just handed straight to the socket / straight back into
 * `applyRemote`. */
export type CrdtOp = Record<string, unknown>;

function randomSiteId(): number {
  // Fits comfortably in the Rust `u32` param and in a JS safe-integer/number.
  return Math.floor(Math.random() * 0x7fffffff);
}

export interface UseCrdtDoc {
  text: string;
  siteId: number;
  insertAt: (index: number, ch: string) => CrdtOp;
  deleteAt: (index: number) => CrdtOp;
  applyRemote: (op: CrdtOp) => void;
}

/**
 * Owns one WASM `CrdtDoc` instance for the lifetime of the component tree that calls
 * this hook (one per open document per browser tab). Every local edit goes through
 * `insertAt`/`deleteAt`, which mutate the WASM doc, re-sync React state from its
 * `to_string()`, and return the operation to broadcast over the socket.
 */
export function useCrdtDoc(): UseCrdtDoc {
  const docRef = useRef<CrdtDoc | null>(null);
  docRef.current ??= new CrdtDoc(randomSiteId());
  const doc = docRef.current;

  const [text, setText] = useState<string>(() => doc.toString());

  const insertAt = useCallback(
    (index: number, ch: string): CrdtOp => {
      const op = doc.insert(index, ch) as CrdtOp;
      setText(doc.toString());
      return op;
    },
    [doc],
  );

  const deleteAt = useCallback(
    (index: number): CrdtOp => {
      const op = doc.delete(index) as CrdtOp;
      setText(doc.toString());
      return op;
    },
    [doc],
  );

  const applyRemote = useCallback(
    (op: CrdtOp) => {
      doc.applyRemote(op);
      setText(doc.toString());
    },
    [doc],
  );

  return { text, siteId: doc.siteId(), insertAt, deleteAt, applyRemote };
}
