use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum FfmpegJobStatus {
    Pending,
    Running,
    Done,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum FfmpegJobSourceType {
    BlenderJob,
    Folder,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FfmpegJob {
    pub id: String,
    pub job_number: i32,
    pub name: String,
    pub source_type: FfmpegJobSourceType,
    pub source_blender_job_id: Option<String>,
    pub input_path: PathBuf,
    pub frame_start: i32,
    pub frame_end: i32,
    pub fps: f32,
    pub output_path: PathBuf,
    pub crf: u32,
    pub preset: String,
    pub status: FfmpegJobStatus,
    pub priority: i32,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
    pub progress_frame: Option<i32>,
    pub total_frames: Option<i32>,
    pub output_size_bytes: Option<i64>,
    pub output_duration_secs: Option<f32>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DbFfmpegJob {
    pub id: String,
    pub job_number: i32,
    pub name: String,
    pub source_type: FfmpegJobSourceType,
    pub source_blender_job_id: Option<String>,
    pub input_path: String,
    pub frame_start: i32,
    pub frame_end: i32,
    pub fps: f32,
    pub output_path: String,
    pub crf: i32,
    pub preset: String,
    pub status: FfmpegJobStatus,
    pub priority: i32,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
    pub progress_frame: Option<i32>,
    pub total_frames: Option<i32>,
    pub output_size_bytes: Option<i64>,
    pub output_duration_secs: Option<f32>,
}

impl From<DbFfmpegJob> for FfmpegJob {
    fn from(value: DbFfmpegJob) -> Self {
        Self {
            id: value.id,
            job_number: value.job_number,
            name: value.name,
            source_type: value.source_type,
            source_blender_job_id: value.source_blender_job_id,
            input_path: PathBuf::from(value.input_path),
            frame_start: value.frame_start,
            frame_end: value.frame_end,
            fps: value.fps,
            output_path: PathBuf::from(value.output_path),
            crf: value.crf.max(0) as u32,
            preset: value.preset,
            status: value.status,
            priority: value.priority,
            created_at: value.created_at,
            started_at: value.started_at,
            finished_at: value.finished_at,
            progress_frame: value.progress_frame,
            total_frames: value.total_frames,
            output_size_bytes: value.output_size_bytes,
            output_duration_secs: value.output_duration_secs,
        }
    }
}

impl FfmpegJob {
    pub fn new(
        name: String,
        source_type: FfmpegJobSourceType,
        source_blender_job_id: Option<String>,
        input_path: PathBuf,
        frame_start: i32,
        frame_end: i32,
        fps: f32,
        output_path: PathBuf,
        crf: u32,
        preset: String,
        priority: i32,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            job_number: 0,
            name,
            source_type,
            source_blender_job_id,
            input_path,
            frame_start,
            frame_end,
            fps,
            output_path,
            crf,
            preset,
            status: FfmpegJobStatus::Pending,
            priority,
            created_at: Utc::now().timestamp_millis(),
            started_at: None,
            finished_at: None,
            progress_frame: None,
            total_frames: None,
            output_size_bytes: None,
            output_duration_secs: None,
        }
    }

    pub fn total_frames(&self) -> i32 {
        self.frame_end - self.frame_start + 1
    }
}
