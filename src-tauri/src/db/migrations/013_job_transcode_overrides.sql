ALTER TABLE jobs ADD COLUMN transcode_name_override TEXT;
ALTER TABLE jobs ADD COLUMN transcode_fps_override REAL;
ALTER TABLE jobs ADD COLUMN transcode_output_path_override TEXT;
ALTER TABLE jobs ADD COLUMN transcode_crf_override INTEGER;
ALTER TABLE jobs ADD COLUMN transcode_preset_override TEXT;
