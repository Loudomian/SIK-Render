CREATE TABLE IF NOT EXISTS node_job_events (
    id        TEXT    PRIMARY KEY,
    node_id   TEXT    NOT NULL,
    job_id    TEXT    NOT NULL,
    timestamp INTEGER NOT NULL,
    kind      TEXT    NOT NULL,
    level     TEXT    NOT NULL,
    title     TEXT    NOT NULL,
    message   TEXT    NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_node_job_events_lookup
    ON node_job_events (node_id, job_id, timestamp);
