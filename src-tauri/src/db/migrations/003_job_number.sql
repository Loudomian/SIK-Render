ALTER TABLE jobs ADD COLUMN job_number INTEGER NOT NULL DEFAULT 0;
-- Back-fill existing rows using SQLite's built-in rowid as a stable ordering
UPDATE jobs SET job_number = rowid WHERE job_number = 0;
