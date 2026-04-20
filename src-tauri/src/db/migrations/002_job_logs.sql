CREATE TABLE IF NOT EXISTS job_logs (
    job_id TEXT    NOT NULL,
    seq    INTEGER NOT NULL,
    line   TEXT    NOT NULL,
    PRIMARY KEY (job_id, seq)
);
CREATE INDEX IF NOT EXISTS idx_job_logs_job_id ON job_logs (job_id);
