use crate::blender::parser;
use crate::queue::job::{JobStatus, RenderJob};
use anyhow::Result;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use tauri::{AppHandle, Emitter};

/// Payload emitted to the frontend on each rendered frame.
#[derive(Clone, serde::Serialize)]
pub struct RenderProgressEvent {
    pub job_id: String,
    pub frame: u32,
    pub total_frames: u32,
    pub time_elapsed: f32,
    pub memory_mb: f32,
}

/// Spawn Blender as a child process and stream progress back to the frontend.
/// Blocks until the render completes or is cancelled.
pub async fn run_job(app: AppHandle, job: RenderJob) -> Result<JobStatus> {
    let total_frames = (job.frame_end - job.frame_start + 1) as u32;
    let mut child = spawn_blender(&job)?;

    let stdout = child.stdout.take().expect("stdout not captured");
    let reader = BufReader::new(stdout);

    for line in reader.lines() {
        let line = line?;
        if let Some(progress) = parser::parse_line(&line) {
            let _ = app.emit(
                "render-progress",
                RenderProgressEvent {
                    job_id: job.id.clone(),
                    frame: progress.frame,
                    total_frames,
                    time_elapsed: progress.time_elapsed,
                    memory_mb: progress.memory_mb,
                },
            );
        }
    }

    let status = child.wait()?;
    if status.success() {
        Ok(JobStatus::Done)
    } else {
        Ok(JobStatus::Failed)
    }
}

fn spawn_blender(job: &RenderJob) -> Result<Child> {
    let child = Command::new(&job.blender_executable)
        .args([
            "--background",
            job.blend_file.to_str().unwrap(),
            "--render-output",
            job.output_path.to_str().unwrap(),
            "--render-frame",
            &format!("{}..{}", job.frame_start, job.frame_end),
            "--render-format",
            &job.output_format,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(child)
}
