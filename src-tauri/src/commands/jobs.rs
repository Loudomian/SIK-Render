use crate::queue::job::{DbRenderJob, JobStatus, RenderJob};
use crate::queue::scheduler;
use crate::state::AppState;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashSet;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, State};

#[derive(Deserialize)]
pub struct AddJobPayload {
    pub name: String,
    pub note: Option<String>,
    pub auto_transcode_mp4: bool,
    pub fps: Option<f32>,
    pub blend_file: String,
    pub blender_executable: String,
    pub output_path: String,
    pub output_format: String,
    pub frame_start: i32,
    pub frame_end: i32,
    pub preview_width: Option<i32>,
    pub preview_height: Option<i32>,
    pub resume_from_existing: bool,
    pub initial_current_frame: Option<i32>,
    pub initial_total_frames: Option<i32>,
    pub initial_last_rendered_frame: Option<i32>,
    pub priority: i32,
}

#[derive(Deserialize)]
pub struct UpdateJobMetadataPayload {
    pub id: String,
    pub name: String,
    pub note: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateJobTranscodeSettingsPayload {
    pub id: String,
    pub auto_transcode_mp4: bool,
    pub transcode_name_override: Option<String>,
    pub transcode_fps_override: Option<f32>,
    pub transcode_output_path_override: Option<String>,
    pub transcode_crf_override: Option<u32>,
    pub transcode_preset_override: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateJobFpsPayload {
    pub id: String,
    pub fps: f32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobLogSummary {
    pub directory: String,
    pub blender_count: usize,
    pub ffmpeg_count: usize,
    pub total_count: usize,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueState {
    pub paused: bool,
    pub paused_job: Option<String>,
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

async fn fetch_jobs(pool: &sqlx::SqlitePool) -> Result<Vec<RenderJob>, sqlx::Error> {
    sqlx::query_as::<_, DbRenderJob>(
        r#"
        SELECT
            id,
            job_number,
            name,
            note,
            crash_count,
            auto_transcode_mp4,
            transcode_name_override,
            transcode_fps_override,
            transcode_output_path_override,
            transcode_crf_override,
            transcode_preset_override,
            fps,
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
        ORDER BY priority ASC, created_at ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .map(|rows| rows.into_iter().map(RenderJob::from).collect())
}

async fn next_queue_priority<'e, E>(executor: E) -> Result<i32, sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let next = sqlx::query_scalar::<_, i32>("SELECT COALESCE(MAX(priority), 0) + 1 FROM jobs")
        .fetch_one(executor)
        .await?;

    Ok(next.max(1))
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|text| {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn normalize_optional_positive_f32(value: Option<f32>, field_name: &str) -> Result<Option<f32>, String> {
    match value {
        Some(current) if current > 0.0 => Ok(Some(current)),
        Some(_) => Err(format!("{field_name} must be greater than 0")),
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn list_jobs(state: State<'_, AppState>) -> Result<Vec<RenderJob>, String> {
    fetch_jobs(&state.pool).await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_queue_state(state: State<'_, AppState>) -> Result<QueueState, String> {
    let paused_job = state.paused_job_id.lock().await.clone();
    Ok(QueueState {
        paused: state.is_queue_paused(),
        paused_job,
    })
}

#[tauri::command]
pub async fn start_queue(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<QueueState, String> {
    state.set_queue_paused(false);

    let paused_id = state.paused_job_id.lock().await.take();

    if let Some(job_id) = &paused_id {
        let status = sqlx::query_scalar::<_, crate::queue::job::JobStatus>(
            "SELECT status FROM jobs WHERE id = ?",
        )
        .bind(job_id)
        .fetch_optional(&state.pool)
        .await
        .unwrap_or(None);

        if matches!(status, Some(crate::queue::job::JobStatus::Interrupted)) {
            let _ = sqlx::query(
                "UPDATE jobs \
                 SET status = 'pending', \
                     started_at = NULL, \
                     finished_at = NULL, \
                     resume_from_existing = 1, \
                     crash_count = 0, \
                     time_elapsed = NULL, \
                     remaining_secs = NULL \
                 WHERE id = ?",
            )
            .bind(job_id)
            .execute(&state.pool)
            .await;

            if let Ok(job) = crate::queue::scheduler::load_job(&state.pool, job_id).await {
                crate::queue::scheduler::emit_job_update(&app, &job);
            }
        }
    }

    state.scheduler_notify.notify_one();
    scheduler::emit_queue_state_full(&app, false, None);
    Ok(QueueState {
        paused: false,
        paused_job: None,
    })
}

#[tauri::command]
pub async fn pause_queue(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<QueueState, String> {
    let pause_line = "[paused] Reason: user paused the queue. The render was stopped mid-job. \
        Progress is preserved; the job will auto-resume from the last recorded frame when the \
        queue is started again.";

    state.set_queue_paused(true);

    let running = {
        let active = state.active_jobs.lock().await;
        active.iter().map(|(id, pid)| (id.clone(), *pid)).next()
    };

    let paused_id = if let Some((job_id, pid)) = running {
        state.interrupted_jobs.lock().await.insert(job_id.clone());

        let _ = sqlx::query(
            "UPDATE jobs \
             SET status = 'interrupted', \
                 finished_at = ?, \
                 resume_from_existing = 1 \
             WHERE id = ? AND status = 'running'",
        )
        .bind(chrono::Utc::now().timestamp_millis())
        .bind(&job_id)
        .execute(&state.pool)
        .await;

        let _ = append_manual_cancel_log(&app, &state, &job_id, pause_line).await;

        if let Ok(job) = crate::queue::scheduler::load_job(&state.pool, &job_id).await {
            crate::queue::scheduler::emit_job_update(&app, &job);
        }

        let _ = AppState::kill_process_tree(pid);

        Some(job_id)
    } else {
        None
    };

    *state.paused_job_id.lock().await = paused_id.clone();

    scheduler::emit_queue_state_full(&app, true, paused_id.clone());
    Ok(QueueState {
        paused: true,
        paused_job: paused_id,
    })
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
    job.note = normalize_optional_text(payload.note);
    job.auto_transcode_mp4 = payload.auto_transcode_mp4;
    job.fps = payload.fps.filter(|value| *value > 0.0);
    job.preview_width = payload.preview_width;
    job.preview_height = payload.preview_height;
    if payload.resume_from_existing {
        let total_frames = job.total_frames();
        let resume_floor = 0.max(job.frame_start - 1);
        job.current_frame = payload
            .initial_current_frame
            .map(|value| value.clamp(0, total_frames));
        job.total_frames = payload
            .initial_total_frames
            .map(|value| value.clamp(0, total_frames))
            .or(Some(total_frames));
        job.last_rendered_frame = payload.initial_last_rendered_frame.map(|value| {
            value.clamp(resume_floor, job.frame_end)
        });
    }

    let mut tx = state.pool.begin().await.map_err(|error| error.to_string())?;
    let job_number: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(job_number), 0) + 1 FROM jobs")
        .fetch_one(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;
    job.job_number = job_number as i32;
    job.priority = next_queue_priority(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;

    sqlx::query(
        r#"
        INSERT INTO jobs (
            id,
            job_number,
            name,
            note,
            crash_count,
            auto_transcode_mp4,
            transcode_name_override,
            transcode_fps_override,
            transcode_output_path_override,
            transcode_crf_override,
            transcode_preset_override,
            fps,
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
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&job.id)
    .bind(job.job_number)
    .bind(&job.name)
    .bind(&job.note)
    .bind(job.crash_count)
    .bind(job.auto_transcode_mp4)
    .bind(&job.transcode_name_override)
    .bind(job.transcode_fps_override)
    .bind(
        job.transcode_output_path_override
            .as_ref()
            .map(|value| value.to_string_lossy().to_string()),
    )
    .bind(job.transcode_crf_override)
    .bind(&job.transcode_preset_override)
    .bind(job.fps)
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

    Ok(job)
}

#[tauri::command]
pub async fn update_job_metadata(
    payload: UpdateJobMetadataPayload,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<RenderJob, String> {
    let name = payload.name.trim();
    if name.is_empty() {
        return Err("name cannot be empty".into());
    }

    let rows_affected = sqlx::query("UPDATE jobs SET name = ?, note = ? WHERE id = ?")
        .bind(name)
        .bind(normalize_optional_text(payload.note))
        .bind(&payload.id)
        .execute(&state.pool)
        .await
        .map_err(|error| error.to_string())?
        .rows_affected();

    if rows_affected == 0 {
        return Err(format!("job {} was not found", payload.id));
    }

    let job = scheduler::load_job(&state.pool, &payload.id)
        .await
        .map_err(|error| error.to_string())?;
    scheduler::emit_job_update(&app, &job);
    Ok(job)
}

#[tauri::command]
pub async fn update_job_transcode_settings(
    payload: UpdateJobTranscodeSettingsPayload,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<RenderJob, String> {
    let transcode_name_override = normalize_optional_text(payload.transcode_name_override);
    let transcode_output_path_override =
        normalize_optional_text(payload.transcode_output_path_override);
    let transcode_preset_override = payload
        .transcode_preset_override
        .map(|value| crate::commands::transcode::normalize_preset(value.trim()))
        .filter(|value| !value.is_empty());
    let transcode_fps_override =
        normalize_optional_positive_f32(payload.transcode_fps_override, "transcode_fps_override")?;
    let transcode_crf_override = payload
        .transcode_crf_override
        .map(|value| value.min(51) as i32);

    let rows_affected = sqlx::query(
        "UPDATE jobs
         SET auto_transcode_mp4 = ?,
             transcode_name_override = ?,
             transcode_fps_override = ?,
             transcode_output_path_override = ?,
             transcode_crf_override = ?,
             transcode_preset_override = ?
         WHERE id = ?",
    )
        .bind(payload.auto_transcode_mp4)
        .bind(transcode_name_override)
        .bind(transcode_fps_override)
        .bind(transcode_output_path_override)
        .bind(transcode_crf_override)
        .bind(transcode_preset_override)
        .bind(&payload.id)
        .execute(&state.pool)
        .await
        .map_err(|error| error.to_string())?
        .rows_affected();

    if rows_affected == 0 {
        return Err(format!("job {} was not found", payload.id));
    }

    let job = scheduler::load_job(&state.pool, &payload.id)
        .await
        .map_err(|error| error.to_string())?;
    scheduler::emit_job_update(&app, &job);
    Ok(job)
}

#[tauri::command]
pub async fn update_job_fps(
    payload: UpdateJobFpsPayload,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<RenderJob, String> {
    if payload.fps <= 0.0 {
        return Err("fps must be greater than 0".into());
    }

    let rows_affected = sqlx::query("UPDATE jobs SET fps = ? WHERE id = ?")
        .bind(payload.fps)
        .bind(&payload.id)
        .execute(&state.pool)
        .await
        .map_err(|error| error.to_string())?
        .rows_affected();

    if rows_affected == 0 {
        return Err(format!("job {} was not found", payload.id));
    }

    let job = scheduler::load_job(&state.pool, &payload.id)
        .await
        .map_err(|error| error.to_string())?;
    scheduler::emit_job_update(&app, &job);
    Ok(job)
}

#[tauri::command]
pub async fn update_job_preview_dimensions(
    id: String,
    width: i32,
    height: i32,
    state: State<'_, AppState>,
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

    scheduler::load_job(&state.pool, &id)
        .await
        .map_err(|error| error.to_string())
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
pub async fn get_job_log_summary(
    job_id: String,
    state: State<'_, AppState>,
) -> Result<JobLogSummary, String> {
    let (job_number, job_id) = sqlx::query_as::<_, (i32, String)>("SELECT job_number, id FROM jobs WHERE id = ?")
        .bind(&job_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| e.to_string())?;

    let directory = crate::app_paths::job_log_dir(job_number, &job_id)
        .map_err(|e| e.to_string())?;
    let blender_count = crate::app_paths::count_job_log_files(job_number, &job_id, crate::app_paths::BLENDER_LOG_KIND)
        .map_err(|e| e.to_string())?;
    let ffmpeg_count = 0;

    Ok(JobLogSummary {
        directory: directory.to_string_lossy().to_string(),
        blender_count,
        ffmpeg_count,
        total_count: blender_count,
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
    let Some((current_frame_start, current_frame_end, current_priority, status)) = sqlx::query_as::<_, (i32, i32, i32, JobStatus)>(
        "SELECT frame_start, frame_end, priority, status FROM jobs WHERE id = ?",
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
    let preserve_progress = resume_from_existing && !range_changed && status != JobStatus::Done;
    let mut tx = state.pool.begin().await.map_err(|e| e.to_string())?;

    sqlx::query(
        "UPDATE jobs SET status = 'pending', started_at = NULL, finished_at = NULL, \
           frame_start = ?, frame_end = ?, resume_from_existing = ?, priority = ?, \
           current_frame = CASE WHEN ? THEN current_frame ELSE NULL END, \
           total_frames = CASE WHEN ? THEN total_frames ELSE NULL END, \
           last_rendered_frame = CASE WHEN ? THEN last_rendered_frame ELSE NULL END, \
           crash_count = 0, time_elapsed = NULL, remaining_secs = NULL \
           WHERE id = ?",
    )
    .bind(target_frame_start)
    .bind(target_frame_end)
    .bind(resume_from_existing)
    .bind(current_priority)
    .bind(preserve_progress)
    .bind(preserve_progress)
    .bind(preserve_progress)
    .bind(&id)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;

    let job = scheduler::load_job(&state.pool, &id)
        .await
        .map_err(|e| e.to_string())?;

    scheduler::emit_job_update(&app, &job);
    state.scheduler_notify.notify_one();

    Ok(job)
}

#[tauri::command]
pub async fn reorder_job(
    ordered_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<RenderJob>, String> {
    let all_rows = sqlx::query_as::<_, (String, JobStatus)>(
        "SELECT id, status FROM jobs ORDER BY priority ASC, created_at ASC",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|error| error.to_string())?;

    // The reorder algorithm intentionally preserves running jobs in-place and
    // only remaps non-running jobs into the remaining slots. This currently
    // relies on the scheduler invariant that at most one job can be running.
    debug_assert!(
        all_rows
            .iter()
            .filter(|(_, status)| *status == JobStatus::Running)
            .count()
            <= 1,
        "reorder_job assumes at most one running job"
    );

    let existing_ids = all_rows
        .iter()
        .filter(|(_, status)| *status != JobStatus::Running)
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    let existing_set = existing_ids.iter().cloned().collect::<HashSet<_>>();
    let provided_set = ordered_ids.iter().cloned().collect::<HashSet<_>>();

    if existing_ids.len() != ordered_ids.len() || existing_set != provided_set {
        return Err("job order is out of date, please refresh and try again".into());
    }

    let mut ordered_iter = ordered_ids.into_iter();
    let final_order = all_rows
        .into_iter()
        .map(|(id, status)| {
            if status == JobStatus::Running {
                Ok(id)
            } else {
                ordered_iter.next().ok_or_else(|| "job order is out of date, please refresh and try again".to_string())
            }
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut tx = state.pool.begin().await.map_err(|error| error.to_string())?;
    for (index, id) in final_order.iter().enumerate() {
        sqlx::query("UPDATE jobs SET priority = ? WHERE id = ?")
            .bind((index as i32) + 1)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(|error| error.to_string())?;
    }
    tx.commit().await.map_err(|error| error.to_string())?;

    state.scheduler_notify.notify_one();
    fetch_jobs(&state.pool).await.map_err(|error| error.to_string())
}
