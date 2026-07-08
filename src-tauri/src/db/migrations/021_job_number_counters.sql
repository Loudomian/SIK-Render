CREATE TABLE IF NOT EXISTS job_number_counters (
    kind        TEXT PRIMARY KEY NOT NULL,
    next_number INTEGER NOT NULL
);

INSERT INTO job_number_counters (kind, next_number)
VALUES ('render', (SELECT COALESCE(MAX(job_number), 0) + 1 FROM jobs))
ON CONFLICT(kind) DO NOTHING;

INSERT INTO job_number_counters (kind, next_number)
VALUES ('ffmpeg', (SELECT COALESCE(MAX(job_number), 0) + 1 FROM ffmpeg_jobs))
ON CONFLICT(kind) DO NOTHING;
