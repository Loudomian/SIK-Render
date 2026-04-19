CREATE TABLE IF NOT EXISTS jobs (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    blend_file    TEXT NOT NULL,
    blender_exec  TEXT NOT NULL,
    output_path   TEXT NOT NULL,
    output_format TEXT NOT NULL DEFAULT 'PNG',
    frame_start   INTEGER NOT NULL DEFAULT 1,
    frame_end     INTEGER NOT NULL DEFAULT 1,
    status        TEXT NOT NULL DEFAULT 'pending',
    priority      INTEGER NOT NULL DEFAULT 5,
    created_at    INTEGER NOT NULL,
    started_at    INTEGER,
    finished_at   INTEGER
);

CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
