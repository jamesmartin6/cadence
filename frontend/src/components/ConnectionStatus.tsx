import type { ConnectionStatus as Status } from "../hooks/useDocSocket";

const LABELS: Record<Status, string> = {
  connecting: "Connecting…",
  connected: "Connected",
  disconnected: "Offline",
  reconnecting: "Reconnecting…",
};

export function ConnectionStatus({ status }: { status: Status }) {
  return (
    <span className={`connection-status connection-status--${status}`}>
      <span className="connection-status__dot" aria-hidden="true" />
      {LABELS[status]}
    </span>
  );
}
