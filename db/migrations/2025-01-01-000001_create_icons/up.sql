-- One lookup table for all three match kinds, mirroring the fst maps in iconator:
--   kind = 'ext'      -> file extension     (e.g. "rs")
--   kind = 'filename' -> exact file name    (e.g. "CMakeCache.txt")
--   kind = 'folder'   -> exact folder name  (e.g. ".github")
-- icon_id points at libs/iconator/icons/{icon_id}.svg (BIGINT to match iconator's u64).
-- The same name can appear under different kinds (".api" is both an extension and a
-- folder), so it's unique on (kind, name), not name alone.

CREATE SCHEMA IF NOT EXISTS icons;

CREATE TABLE icons.lookups (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    kind        TEXT        NOT NULL CHECK (kind IN ('ext', 'filename', 'folder')),
    name        TEXT        NOT NULL,
    icon_id     BIGINT      NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (kind, name)
);

-- Reverse lookup: "which names resolve to this icon?". The forward lookup
-- (kind, name) -> icon rides the UNIQUE (kind, name) btree index.
CREATE INDEX idx_icons_lookups_icon_id ON icons.lookups (icon_id);
