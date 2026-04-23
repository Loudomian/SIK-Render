CREATE TABLE IF NOT EXISTS ffmpeg_jobs (
    id                    TEXT PRIMARY KEY NOT NULL,
    job_number            INTEGER NOT NULL,
    name                  TEXT NOT NULL,
    source_type           TEXT NOT NULL CHECK (source_type IN ('blender_job', 'folder')),
    source_blender_job_id TEXT REFERENCES jobs(id) ON DELETE SET NULL,
    input_path            TEXT NOT NULL,
    frame_start           INTEGER NOT NULL,
    frame_end             INTEGER NOT NULL,
    fps                   REAL NOT NULL DEFAULT 30.0,
    output_path           TEXT NOT NULL,
    crf                   INTEGER NOT NULL DEFAULT 23,
    preset                TEXT NOT NULL DEFAULT 'medium',
    status                TEXT NOT NULL DEFAULT 'pending'
                              CHECK (status IN ('pending','running','done','failed','cancelled')),
    priority              INTEGER NOT NULL DEFAULT 1,
    created_at            INTEGER NOT NULL,
    started_at            INTEGER,
    finished_at           INTEGER,
    progress_frame        INTEGER,
    total_frames          INTEGER,
    output_size_bytes     INTEGER,
    output_duration_secs  REAL
);
