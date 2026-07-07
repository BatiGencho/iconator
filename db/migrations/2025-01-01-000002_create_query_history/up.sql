-- Audit log of icon lookups served by the API. One row per query.
-- Lives in the `icons` schema (created in the previous migration).
CREATE TABLE icons.query_history (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    query_kind  TEXT        NOT NULL CHECK (query_kind IN ('file', 'folder')),
    query_path  TEXT        NOT NULL,   -- the raw path/name the caller asked about
    icon_id     BIGINT,                 -- resolved icon id, NULL when no icon matched
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_query_history_created_at
    ON icons.query_history (created_at DESC);

-- Housekeeping: use pg_cron to delete rows older than N days, or partition by
-- created_at and drop old partitions if this log grows large.
