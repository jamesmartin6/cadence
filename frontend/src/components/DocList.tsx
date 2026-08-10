import { useEffect, useState } from "react";
import { createDoc, listDocs, type DocSummary } from "../lib/api";

interface Props {
  onOpenDoc: (docId: string) => void;
}

export function DocList({ onOpenDoc }: Props) {
  const [docs, setDocs] = useState<DocSummary[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);

  const refresh = () => {
    listDocs()
      .then(setDocs)
      .catch((err: unknown) => setError(err instanceof Error ? err.message : String(err)));
  };

  useEffect(refresh, []);

  const handleCreate = async () => {
    setCreating(true);
    setError(null);
    try {
      const doc = await createDoc();
      onOpenDoc(doc.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setCreating(false);
    }
  };

  return (
    <div className="doc-list-page">
      <header className="doc-list-header">
        <h1>Cadence</h1>
        <p>
          A real-time collaborative text editor with a CRDT engine written from scratch in
          Rust, compiled to WebAssembly. Open a document, then open it again in a second
          tab &mdash; edits sync live, and survive going offline.
        </p>
      </header>

      <button className="primary-button" onClick={handleCreate} disabled={creating}>
        {creating ? "Creating…" : "+ New document"}
      </button>

      {error && <p className="error-banner">Couldn't reach the relay server: {error}</p>}

      <ul className="doc-list">
        {docs === null && !error && <li className="doc-list__empty">Loading documents…</li>}
        {docs !== null && docs.length === 0 && (
          <li className="doc-list__empty">No documents yet &mdash; create one above.</li>
        )}
        {docs?.map((doc) => (
          <li key={doc.id}>
            <button className="doc-list__item" onClick={() => onOpenDoc(doc.id)}>
              <span className="doc-list__title">{doc.title ?? "Untitled document"}</span>
              <span className="doc-list__meta">
                {new Date(doc.created_at).toLocaleString()} &middot; {doc.id.slice(0, 8)}
              </span>
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
