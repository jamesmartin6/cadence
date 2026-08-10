CREATE TABLE docs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- The full, append-only operation log for every document. The relay server never
-- interprets these payloads (it doesn't implement RGA itself) -- it just stores them in
-- arrival order and replays the whole log to newly-joining clients, who reconstruct the
-- document themselves with their own CRDT engine instance. `seq` is assigned per-doc,
-- starting at 1, gapless and strictly increasing (enforced by relay-server's insert
-- logic, which locks the parent `docs` row for the duration of the insert).
CREATE TABLE ops (
    id BIGSERIAL PRIMARY KEY,
    doc_id UUID NOT NULL REFERENCES docs(id) ON DELETE CASCADE,
    seq BIGINT NOT NULL,
    site_id BIGINT,
    payload JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (doc_id, seq)
);

CREATE INDEX ops_doc_id_seq_idx ON ops (doc_id, seq);
