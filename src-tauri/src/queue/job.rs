use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Pending,
    Running,
    Done,
    Failed,
    Cancelled,
    Interrupted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderJob {
    pub id: String,
    pub job_number: i32,
    pub name: String,
    pub note: Option<String>,
    pub crash_count: i32,
    pub auto_transcode_mp4: bool,
    pub transcode_name_override: Option<String>,
    pub transcode_fps_override: Option<f32>,
    pub transcode_output_path_override: Option<PathBuf>,
    pub transcode_crf_override: Option<i32>,
    pub transcode_preset_override: Option<String>,
    pub fps: Option<f32>,
    pub blend_file: PathBuf,
    pub blender_executable: PathBuf,
    pub output_path: PathBuf,
    pub output_format: String,
    pub frame_start: i32,
    pub frame_end: i32,
    pub preview_width: Option<i32>,
    pub preview_height: Option<i32>,
    pub resume_from_existing: bool,
    pub status: JobStatus,
    pub priority: i32,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
    pub current_frame: Option<i32>,
    pub total_frames: Option<i32>,
    pub last_rendered_frame: Option<i32>,
    pub time_elapsed: Option<f32>,
    pub remaining_secs: Option<f32>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DbRenderJob {
    pub id: String,
    pub job_number: i32,
    pub name: String,
    pub note: Option<String>,
    pub crash_count: i32,
    pub auto_transcode_mp4: bool,
    pub transcode_name_override: Option<String>,
    pub transcode_fps_override: Option<f32>,
    pub transcode_output_path_override: Option<String>,
    pub transcode_crf_override: Option<i32>,
    pub transcode_preset_override: Option<String>,
    pub fps: Option<f32>,
    pub blend_file: String,
    pub blender_exec: String,
    pub output_path: String,
    pub output_format: String,
    pub frame_start: i32,
    pub frame_end: i32,
    pub preview_width: Option<i32>,
    pub preview_height: Option<i32>,
    pub resume_from_existing: bool,
    pub status: JobStatus,
    pub priority: i32,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
    pub current_frame: Option<i32>,
    pub total_frames: Option<i32>,
    pub last_rendered_frame: Option<i32>,
    pub time_elapsed: Option<f32>,
    pub remaining_secs: Option<f32>,
}

impl From<DbRenderJob> for RenderJob {
    fn from(value: DbRenderJob) -> Self {
        Self {
            id: value.id,
            job_number: value.job_number,
            name: value.name,
            note: value.note,
            crash_count: value.crash_count,
            auto_transcode_mp4: value.auto_transcode_mp4,
            transcode_name_override: value.transcode_name_override,
            transcode_fps_override: value.transcode_fps_override,
            transcode_output_path_override: value
                .transcode_output_path_override
                .map(PathBuf::from),
            transcode_crf_override: value.transcode_crf_override,
            transcode_preset_override: value.transcode_preset_override,
            fps: value.fps,
            blend_file: PathBuf::from(value.blend_file),
            blender_executable: PathBuf::from(value.blender_exec),
            output_path: PathBuf::from(value.output_path),
            output_format: value.output_format,
            frame_start: value.frame_start,
            frame_end: value.frame_end,
            preview_width: value.preview_width,
            preview_height: value.preview_height,
            resume_from_existing: value.resume_from_existing,
            status: value.status,
            priority: value.priority,
            created_at: value.created_at,
            started_at: value.started_at,
            finished_at: value.finished_at,
            current_frame: value.current_frame,
            total_frames: value.total_frames,
            last_rendered_frame: value.last_rendered_frame,
            time_elapsed: value.time_elapsed,
            remaining_secs: value.remaining_secs,
        }
    }
}

impl RenderJob {
    pub fn new(
        name: String,
        blend_file: PathBuf,
        blender_executable: PathBuf,
        output_path: PathBuf,
        output_format: String,
        frame_start: i32,
        frame_end: i32,
        resume_from_existing: bool,
        priority: i32,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            job_number: 0, // assigned by add_job before DB insert
            name,
            note: None,
            crash_count: 0,
            auto_transcode_mp4: true,
            transcode_name_override: None,
            transcode_fps_override: None,
            transcode_output_path_override: None,
            transcode_crf_override: None,
            transcode_preset_override: None,
            fps: None,
            blend_file,
            blender_executable,
            output_path,
            output_format,
            frame_start,
            frame_end,
            preview_width: None,
            preview_height: None,
            resume_from_existing,
            status: JobStatus::Pending,
            priority,
            created_at: Utc::now().timestamp_millis(),
            started_at: None,
            finished_at: None,
            current_frame: None,
            total_frames: None,
            last_rendered_frame: None,
            time_elapsed: None,
            remaining_secs: None,
        }
    }

    pub fn total_frames(&self) -> i32 {
        self.frame_end - self.frame_start + 1
    }
}
