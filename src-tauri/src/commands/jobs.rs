use crate::queue::job::{DbRenderJob, JobStatus, RenderJob};
use crate::queue::scheduler;
use crate::state::AppState;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, State};

#[derive(Deserialize)]
pub struct AddJobPayload {
    pub name: String,
    pub blend_file: String,
    pub blender_executable: String,
    pub output_path: String,
    pub output_format: String,
    pub frame_start: i32,
    pub frame_end: i32,
    pub preview_width: Option<i32>,
    pub preview_height: Option<i32>,
    pub resume_from_existing: bool,
    pub priority: i32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobLogSummary {
    pub directory: String,
    pub blender_count: usize,
    pub ffmpeg_count: usize,
    pub total_count: usize,
}

async fn append_manual_cancel_log(
    app: &AppHandle,
    state: &AppState,
    job_id: &str,
    line: &str,
) -> Result<(), String> {
    let (job_number, job_id) = sqlx::query_as::<_, (i32, String)>("SELECT job_number, id FROM jobs WHERE id = ?")
        .bind(job_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|error| error.to_string())?;

    crate::app_paths::append_job_log_event(
        job_number,
        &job_id,
        crate::app_paths::BLENDER_LOG_KIND,
        line,
    )
    .map_err(|error| error.to_string())?;

    let _ = app.emit(
        "render-log",
        crate::blender::process::RenderLogEvent {
            job_id: job_id.to_string(),
            line: line.to_string(),
        },
    );

    Ok(())
}

#[tauri::command]
pub async fn list_jobs(state: State<'_, AppState>) -> Result<Vec<RenderJob>, String> {
    sqlx::query_as::<_, DbRenderJob>(
        r#"
        SELECT
            id,
            job_number,
            name,
            blend_file,
            blender_exec,
            output_path,
            output_format,
            frame_start,
            frame_end,
            preview_width,
            preview_height,
            resume_from_existing,
            status,
            priority,
            created_at,
            started_at,
            finished_at,
            current_frame,
            total_frames,
            last_rendered_frame,
            time_elapsed,
            remaining_secs
        FROM jobs
        ORDER BY
            CASE status
                WHEN 'running' THEN 0
                WHEN 'pending' THEN 1
                ELSE 2
            END,
            CASE
                WHEN status = 'running' THEN COALESCE(started_at, created_at)
                WHEN status = 'pending' THEN created_at
                ELSE COALESCE(finished_at, started_at, created_at)
            END DESC,
            priority ASC,
            created_at DESC
        "#,
    )
    .fetch_all(&state.pool)
    .await
    .map(|rows| rows.into_iter().map(RenderJob::from).collect())
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn add_job(
    payload: AddJobPayload,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<RenderJob, String> {
    if payload.frame_start > payload.frame_end {
        return Err("frame_start must be <= frame_end".into());
    }

    let mut job = RenderJob::new(
        payload.name,
        PathBuf::from(payload.blend_file),
        PathBuf::from(payload.blender_executable),
        PathBuf::from(payload.output_path),
        payload.output_format,
        payload.frame_start,
        payload.frame_end,
        payload.resume_from_existing,
        payload.priority,
    );
    job.preview_width = payload.preview_width;
    job.preview_height = payload.preview_height;

    if job.preview_width.is_none() || job.preview_height.is_none() {
        if job.blender_executable.exists() && job.blend_file.exists() {
            if let Ok(settings) = crate::blender::project::inspect_project(
                &job.blender_executable,
                &job.blend_file,
            )
            .await
            {
                if settings.resolution_x > 0 && settings.resolution_y > 0 {
                    job.preview_width = Some(settings.resolution_x);
                    job.preview_height = Some(settings.resolution_y);
                }
            }
        }
    }

    let mut tx = state.pool.begin().await.map_err(|error| error.to_string())?;
    let job_number: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(job_number), 0) + 1 FROM jobs")
        .fetch_one(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;
    job.job_number = job_number as i32;

    sqlx::query(
        r#"
        INSERT INTO jobs (
            id,
            job_number,
            name,
            blend_file,
            blender_exec,
            output_path,
            output_format,
            frame_start,
            frame_end,
            preview_width,
            preview_height,
            resume_from_existing,
            status,
            priority,
            created_at,
            started_at,
            finished_at,
            current_frame,
            total_frames,
            last_rendered_frame,
            time_elapsed,
            remaining_secs
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&job.id)
    .bind(job.job_number)
    .bind(&job.name)
    .bind(job.blend_file.to_string_lossy().to_string())
    .bind(job.blender_executable.to_string_lossy().to_string())
    .bind(job.output_path.to_string_lossy().to_string())
    .bind(&job.output_format)
    .bind(job.frame_start)
    .bind(job.frame_end)
    .bind(job.preview_width)
    .bind(job.preview_height)
    .bind(job.resume_from_existing)
    .bind(JobStatus::Pending)
    .bind(job.priority)
    .bind(job.created_at)
    .bind(job.started_at)
    .bind(job.finished_at)
    .bind(job.current_frame)
    .bind(job.total_frames)
    .bind(job.last_rendered_frame)
    .bind(job.time_elapsed)
    .bind(job.remaining_secs)
    .execute(&mut *tx)
    .await
    .map_err(|error| error.to_string())?;

    tx.commit().await.map_err(|error| error.to_string())?;

    scheduler::emit_job_update(&app, &job);
    state.scheduler_notify.notify_one();

    Ok(job)
}

#[tauri::command]
pub async fn update_job_preview_dimensions(
    id: String,
    width: i32,
    height: i32,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<RenderJob, String> {
    if width <= 0 || height <= 0 {
        return Err("preview dimensions must be positive".into());
    }

    sqlx::query("UPDATE jobs SET preview_width = ?, preview_height = ? WHERE id = ?")
        .bind(width)
        .bind(height)
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(|error| error.to_string())?;

    let job = scheduler::load_job(&state.pool, &id)
        .await
        .map_err(|error| error.to_string())?;
    scheduler::emit_job_update(&app, &job);
    Ok(job)
}

#[tauri::command]
pub async fn remove_job(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let row = sqlx::query_as::<_, (i32, String, JobStatus)>("SELECT job_number, id, status FROM jobs WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|error| error.to_string())?;

    let Some((job_number, job_id, status)) = row else {
        log::warn!("remove_job called for missing id: {id}");
        return Ok(());
    };

    match status {
        JobStatus::Running => return Err("cannot remove a running job".into()),
        _ => {}
    }

    sqlx::query("DELETE FROM jobs WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(|error| error.to_string())?;

    if let Ok(path) = crate::app_paths::job_logs_dir(job_number, &job_id) {
        let _ = std::fs::remove_dir_all(path);
    }
    if let Ok(path) = crate::app_paths::legacy_job_logs_dir(job_number) {
        let _ = std::fs::remove_dir_all(path);
    }

    Ok(())
}

#[tauri::command]
pub async fn cancel_job(
    id: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let cancel_line = "[cancelled] Reason: manual stop. The render was stopped by the user. Progress is preserved; continue manually to resume from the last frame recorded by this job.";
    let status = sqlx::query_scalar::<_, JobStatus>("SELECT status FROM jobs WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|error| error.to_string())?;

    match status {
        None => Ok(()),
        Some(JobStatus::Pending) => {
            sqlx::query("UPDATE jobs SET status = 'cancelled', finished_at = ? WHERE id = ?")
                .bind(Utc::now().timestamp_millis())
                .bind(&id)
                .execute(&state.pool)
                .await
                .map_err(|error| error.to_string())?;
            append_manual_cancel_log(&app, &state, &id, cancel_line).await?;
            Ok(())
        }
        Some(JobStatus::Running) => {
            // Always register the cancellation first so the render loop stops even
            // when the process was externally killed (OOM / Task Manager) and is
            // currently between crash-recovery retries with no active child.
            let should_log = state.cancelled_jobs.lock().await.insert(id.clone());
            if should_log {
                append_manual_cancel_log(&app, &state, &id, cancel_line).await?;
            }

            let pid = {
                let active_jobs = state.active_jobs.lock().await;
                active_jobs.get(&id).copied()
            };

            if let Some(pid) = pid {
                // Ignore kill errors — process may have already exited.
                let _ = AppState::kill_process_tree(pid);
            } else {
                // No active child: process is between retries or already dead.
                // Update the DB immediately so the UI reflects the cancellation
                // without waiting for the scheduler loop to wake up.
                sqlx::query(
                    "UPDATE jobs SET status = 'cancelled', finished_at = ? WHERE id = ?",
                )
                .bind(Utc::now().timestamp_millis())
                .bind(&id)
                .execute(&state.pool)
                .await
                .map_err(|error| error.to_string())?;

                if let Ok(job) = scheduler::load_job(&state.pool, &id).await {
                    scheduler::emit_job_update(&app, &job);
                }
            }

            Ok(())
        }
        Some(_) => Ok(()),
    }
}

#[tauri::command]
pub async fn get_job_logs(
    job_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let (job_number, job_id) = sqlx::query_as::<_, (i32, String)>("SELECT job_number, id FROM jobs WHERE id = ?")
    .bind(&job_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| e.to_string())?;

    crate::app_paths::read_job_log_lines(job_number, &job_id, crate::app_paths::BLENDER_LOG_KIND)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_job_latest_logs(
    job_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let (job_number, job_id) = sqlx::query_as::<_, (i32, String)>("SELECT job_number, id FROM jobs WHERE id = ?")
        .bind(&job_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| e.to_string())?;

    crate::app_paths::read_latest_job_log_lines(job_number, &job_id, crate::app_paths::BLENDER_LOG_KIND)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_job_mp4_logs(
    job_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let (job_number, job_id) = sqlx::query_as::<_, (i32, String)>("SELECT job_number, id FROM jobs WHERE id = ?")
        .bind(&job_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| e.to_string())?;

    crate::app_paths::read_job_log_lines(job_number, &job_id, crate::app_paths::FFMPEG_LOG_KIND)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_job_latest_mp4_logs(
    job_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let (job_number, job_id) = sqlx::query_as::<_, (i32, String)>("SELECT job_number, id FROM jobs WHERE id = ?")
        .bind(&job_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| e.to_string())?;

    crate::app_paths::read_latest_job_log_lines(job_number, &job_id, crate::app_paths::FFMPEG_LOG_KIND)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_job_log_summary(
    job_id: String,
    state: State<'_, AppState>,
) -> Result<JobLogSummary, String> {
    let (job_number, job_id) = sqlx::query_as::<_, (i32, String)>("SELECT job_number, id FROM jobs WHERE id = ?")
        .bind(&job_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| e.to_string())?;

    let directory = crate::app_paths::job_logs_dir(job_number, &job_id)
        .map_err(|e| e.to_string())?;
    let blender_count = crate::app_paths::count_job_log_files(job_number, &job_id, crate::app_paths::BLENDER_LOG_KIND)
        .map_err(|e| e.to_string())?;
    let ffmpeg_count = crate::app_paths::count_job_log_files(job_number, &job_id, crate::app_paths::FFMPEG_LOG_KIND)
        .map_err(|e| e.to_string())?;

    Ok(JobLogSummary {
        directory: directory.to_string_lossy().to_string(),
        blender_count,
        ffmpeg_count,
        total_count: blender_count + ffmpeg_count,
    })
}

/// Reset a finished/failed/cancelled/interrupted job back to pending so the scheduler re-runs it.
#[tauri::command]
pub async fn reset_job(
    id: String,
    resume_from_existing: bool,
    frame_start: Option<i32>,
    frame_end: Option<i32>,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<RenderJob, String> {
    let Some((current_frame_start, current_frame_end, status)) = sqlx::query_as::<_, (i32, i32, JobStatus)>(
        "SELECT frame_start, frame_end, status FROM jobs WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| e.to_string())? else {
        return Err(format!("job {id} was not found"));
    };

    if !matches!(
        status,
        JobStatus::Failed | JobStatus::Cancelled | JobStatus::Interrupted | JobStatus::Done
    ) {
        return Err(format!("job {id} is not in a retriable state"));
    }

    let target_frame_start = frame_start.unwrap_or(current_frame_start);
    let target_frame_end = frame_end.unwrap_or(current_frame_end);

    if target_frame_start > target_frame_end {
        return Err("frameStart must be <= frameEnd".into());
    }

    let range_changed =
        target_frame_start != current_frame_start || target_frame_end != current_frame_end;
    let preserve_progress = resume_from_existing && !range_changed;

    sqlx::query(
        "UPDATE jobs SET status = 'pending', started_at = NULL, finished_at = NULL, \
         frame_start = ?, frame_end = ?, resume_from_existing = ?, \
         current_frame = CASE WHEN ? THEN current_frame ELSE NULL END, \
         total_frames = CASE WHEN ? THEN total_frames ELSE NULL END, \
         last_rendered_frame = CASE WHEN ? THEN last_rendered_frame ELSE NULL END, \
         time_elapsed = NULL, remaining_secs = NULL \
         WHERE id = ?",
    )
    .bind(target_frame_start)
    .bind(target_frame_end)
    .bind(resume_from_existing)
    .bind(preserve_progress)
    .bind(preserve_progress)
    .bind(preserve_progress)
    .bind(&id)
    .execute(&state.pool)
    .await
    .map_err(|e| e.to_string())?;

    let job = scheduler::load_job(&state.pool, &id)
        .await
        .map_err(|e| e.to_string())?;

    scheduler::emit_job_update(&app, &job);
    state.scheduler_notify.notify_one();

    Ok(job)
}

#[tauri::command]
pub async fn reorder_job(
    id: String,
    priority: i32,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let rows = sqlx::query("UPDATE jobs SET priority = ? WHERE id = ?")
        .bind(priority)
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(|error| error.to_string())?
        .rows_affected();

    if rows == 0 {
        return Err(format!("job {id} was not found"));
    }

    state.scheduler_notify.notify_one();

    Ok(())
}
