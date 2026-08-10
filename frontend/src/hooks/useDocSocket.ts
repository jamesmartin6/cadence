import { useCallback, useEffect, useRef, useState } from "react";
import { wsUrlForDoc } from "../lib/api";
import type { CrdtOp } from "./useCrdtDoc";

export type ConnectionStatus = "connecting" | "connected" | "disconnected" | "reconnecting";

export type ClientMessage =
  | { kind: "op"; payload: CrdtOp; site_id: number }
  | { kind: "cursor"; user_id: string; index: number };

export type ServerMessage =
  | { kind: "op"; payload: CrdtOp; site_id?: number | null }
  | { kind: "cursor"; user_id: string; index: number }
  | { kind: "history"; ops: CrdtOp[] };

const BASE_BACKOFF_MS = 500;
const MAX_BACKOFF_MS = 10_000;

/**
 * Owns the WebSocket connection for one document: connects, tracks connection state,
 * reconnects with exponential backoff, and queues outgoing messages while offline so
 * they flush the moment the connection comes back (local edits keep applying to the
 * local CRDT doc the whole time regardless -- that happens up in `useCrdtDoc`, this hook
 * only cares about getting bytes on and off the wire).
 *
 * Also listens for the browser's own `online`/`offline` events, not just the socket's
 * `close`/`error` handlers. This matters because "offline" doesn't always mean "the
 * socket cleanly closed" -- e.g. WiFi dropping, or a network condition change -- can
 * leave a WebSocket object sitting open from the browser's point of view for a long
 * time (well past what a TCP-level timeout would take) with no data actually able to
 * flow. Reacting to `navigator.onLine` changes gets a much snappier, more realistic
 * disconnect/reconnect experience than waiting on the socket alone.
 */
export function useDocSocket(docId: string, onMessage: (msg: ServerMessage) => void) {
  const [status, setStatus] = useState<ConnectionStatus>("connecting");
  const wsRef = useRef<WebSocket | null>(null);
  const queueRef = useRef<ClientMessage[]>([]);
  const attemptRef = useRef(0);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Keep the latest callback without re-running the connection effect on every render.
  const onMessageRef = useRef(onMessage);
  onMessageRef.current = onMessage;

  const send = useCallback((msg: ClientMessage) => {
    const ws = wsRef.current;
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify(msg));
    } else {
      queueRef.current.push(msg);
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    attemptRef.current = 0;

    function scheduleReconnect() {
      if (reconnectTimerRef.current !== null) return; // already scheduled
      const attempt = attemptRef.current;
      attemptRef.current += 1;
      const delay = Math.min(BASE_BACKOFF_MS * 2 ** attempt, MAX_BACKOFF_MS);
      reconnectTimerRef.current = setTimeout(() => {
        reconnectTimerRef.current = null;
        connect();
      }, delay);
    }

    function connect() {
      if (cancelled || !navigator.onLine) return;
      setStatus(attemptRef.current === 0 ? "connecting" : "reconnecting");
      const ws = new WebSocket(wsUrlForDoc(docId));
      wsRef.current = ws;

      ws.onopen = () => {
        if (cancelled) return;
        attemptRef.current = 0;
        setStatus("connected");
        const queued = queueRef.current;
        queueRef.current = [];
        for (const msg of queued) {
          ws.send(JSON.stringify(msg));
        }
      };

      ws.onmessage = (event: MessageEvent<string>) => {
        try {
          const msg = JSON.parse(event.data) as ServerMessage;
          onMessageRef.current(msg);
        } catch {
          // Ignore malformed frames rather than tearing down the connection over them.
        }
      };

      ws.onclose = () => {
        if (cancelled) return;
        wsRef.current = null;
        setStatus("disconnected");
        scheduleReconnect();
      };

      ws.onerror = () => {
        ws.close();
      };
    }

    function handleOffline() {
      setStatus("disconnected");
      wsRef.current?.close();
    }

    function handleOnline() {
      if (reconnectTimerRef.current !== null) {
        clearTimeout(reconnectTimerRef.current);
        reconnectTimerRef.current = null;
      }
      attemptRef.current = 0;
      if (!wsRef.current || wsRef.current.readyState === WebSocket.CLOSED) {
        connect();
      }
    }

    window.addEventListener("offline", handleOffline);
    window.addEventListener("online", handleOnline);
    connect();

    return () => {
      cancelled = true;
      window.removeEventListener("offline", handleOffline);
      window.removeEventListener("online", handleOnline);
      if (reconnectTimerRef.current !== null) {
        clearTimeout(reconnectTimerRef.current);
      }
      wsRef.current?.close();
      wsRef.current = null;
    };
  }, [docId]);

  return { status, send };
}
