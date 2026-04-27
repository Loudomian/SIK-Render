ALTER TABLE jobs ADD COLUMN original_frame_start INTEGER NOT NULL DEFAULT 1;
ALTER TABLE jobs ADD COLUMN original_frame_end INTEGER NOT NULL DEFAULT 1;

UPDATE jobs
SET
  original_frame_start = frame_start,
  original_frame_end = frame_end;
