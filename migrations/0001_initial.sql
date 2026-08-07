CREATE TABLE sessions (
  id          INTEGER PRIMARY KEY,
  created_at  INTEGER NOT NULL CHECK (created_at >= 0),
  updated_at  INTEGER NOT NULL CHECK (updated_at >= 0),
  cwd         TEXT NOT NULL CHECK (cwd <> ''),
  model       TEXT NOT NULL CHECK (model <> ''),
  title       TEXT,
  entries     TEXT NOT NULL CHECK (JSON_VALID(entries))
) STRICT;

CREATE INDEX sessions_cwd_updated_at
ON sessions (cwd, updated_at DESC, id DESC);
