ALTER TABLE jobs ADD COLUMN current_frame INTEGER;
ALTER TABLE jobs ADD COLUMN total_frames INTEGER;
ALTER TABLE jobs ADD COLUMN last_rendered_frame INTEGER;
ALTER TABLE jobs ADD COLUMN time_elapsed REAL;
ALTER TABLE jobs ADD COLUMN remaining_secs REAL;
