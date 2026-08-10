import { useCallback, useEffect, useRef, useState } from "react";
import { useCrdtDoc } from "../hooks/useCrdtDoc";
import { useDocSocket, type ServerMessage } from "../hooks/useDocSocket";
import type { TextDiff } from "../lib/diff";
import { ConnectionStatus } from "./ConnectionStatus";
import { Editor } from "./Editor";
import { PresenceCursors, type RemoteCursor } from "./PresenceCursors";

const PRESENCE_STALE_MS = 8_000;
const PRESENCE_SWEEP_INTERVAL_MS = 2_000;

interface Props {
  docId: string;
  onBack: () => void;
}

/**
 * Wires the CRDT doc, the WebSocket, and the UI pieces together for one open document.
 * Parented with `key={docId}` by App.tsx so switching documents remounts this cleanly
 * (fresh WASM doc, fresh socket) instead of trying to reuse state across documents.
 */
export function DocumentPage({ docId, onBack }: Props) {
  const crdt = useCrdtDoc();
  const myUserId = String(crdt.siteId);

  const [remoteRevision, setRemoteRevision] = useState(0);
  const [presence, setPresence] = useState<Record<string, { index: number; lastSeen: number }>>({});
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);

  const handleServerMessage = useCallback(
    (msg: ServerMessage) => {
      switch (msg.kind) {
        case "history":
          for (const op of msg.ops) crdt.applyRemote(op);
          setRemoteRevision((r) => r + 1);
          break;
        case "op":
          crdt.applyRemote(msg.payload);
          setRemoteRevision((r) => r + 1);
          break;
        case "cursor":
          if (msg.user_id === myUserId) break;
          setPresence((prev) => ({
            ...prev,
            [msg.user_id]: { index: msg.index, lastSeen: Date.now() },
          }));
          break;
      }
    },
    [crdt, myUserId],
  );

  const { status, send } = useDocSocket(docId, handleServerMessage);

  // A tab that drops off the network without a clean close never tells anyone it left,
  // so sweep cursors that haven't been refreshed in a while rather than showing them
  // parked forever.
  useEffect(() => {
    const timer = setInterval(() => {
      const cutoff = Date.now() - PRESENCE_STALE_MS;
      setPresence((prev) => {
        let changed = false;
        const next: typeof prev = {};
        for (const [id, entry] of Object.entries(prev)) {
          if (entry.lastSeen >= cutoff) {
            next[id] = entry;
          } else {
            changed = true;
          }
        }
        return changed ? next : prev;
      });
    }, PRESENCE_SWEEP_INTERVAL_MS);
    return () => clearInterval(timer);
  }, []);

  const handleLocalEdit = useCallback(
    (diff: TextDiff) => {
      for (let i = 0; i < diff.removed.length; i++) {
        const op = crdt.deleteAt(diff.index);
        send({ kind: "op", payload: op, site_id: crdt.siteId });
      }
      for (let i = 0; i < diff.inserted.length; i++) {
        const op = crdt.insertAt(diff.index + i, diff.inserted[i]);
        send({ kind: "op", payload: op, site_id: crdt.siteId });
      }
    },
    [crdt, send],
  );

  const lastSentSelectionRef = useRef<number | null>(null);
  const handleSelectionChange = useCallback(
    (index: number) => {
      if (lastSentSelectionRef.current === index) return;
      lastSentSelectionRef.current = index;
      send({ kind: "cursor", user_id: myUserId, index });
    },
    [send, myUserId],
  );

  const cursors: RemoteCursor[] = Object.entries(presence).map(([userId, entry]) => ({
    userId,
    index: entry.index,
  }));

  return (
    <div className="document-page">
      <header className="document-header">
        <button className="link-button" onClick={onBack}>
          &larr; All documents
        </button>
        <ConnectionStatus status={status} />
      </header>
      <div className="editor-surface">
        <Editor
          text={crdt.text}
          remoteRevision={remoteRevision}
          onLocalEdit={handleLocalEdit}
          onSelectionChange={handleSelectionChange}
          textareaRef={textareaRef}
        />
        <PresenceCursors text={crdt.text} cursors={cursors} textareaRef={textareaRef} />
      </div>
      <p className="share-hint">
        Share this URL to collaborate live: <code>{window.location.href}</code>
      </p>
    </div>
  );
}
