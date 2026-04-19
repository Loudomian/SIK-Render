use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum JobStatus {
    Pending,
    Running,
    Done,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RenderJob {
    pub id: String,
    pub name: String,
    pub blend_file: PathBuf,
    pub blender_executable: PathBuf,
    pub output_path: PathBuf,
    pub output_format: String,
    pub frame_start: i32,
    pub frame_end: i32,
    pub status: JobStatus,
    pub priority: i32,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
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
        priority: i32,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            blend_file,
            blender_executable,
            output_path,
            output_format,
            frame_start,
            frame_end,
            status: JobStatus::Pending,
            priority,
            created_at: Utc::now().timestamp_millis(),
            started_at: None,
            finished_at: None,
        }
    }

    pub fn total_frames(&self) -> i32 {
        self.frame_end - self.frame_start + 1
    }
}
