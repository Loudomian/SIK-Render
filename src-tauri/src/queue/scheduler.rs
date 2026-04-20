use crate::blender::process;
use crate::queue::job::{DbRenderJob, JobStatus, RenderJob};
use crate::state::AppState;
use chrono::Utc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobUpdatedEvent {
    pub job: RenderJob,
}

pub fn start(app: AppHandle, state: AppState) {
    tauri::async_runtime::spawn(async move {
        loop {
            match run_next_job(&app, &state).await {
                Ok(true) => continue,
                Ok(false) => state.scheduler_notify.notified().await,
                Err(error) => {
                    log::error!("Scheduler loop failed: {error}");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    });
}

async fn run_next_job(app: &AppHandle, state: &AppState) -> anyhow::Result<bool> {
    let next_job = sqlx::query_as::<_, DbRenderJob>(
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
        WHERE status = 'pending'
        ORDER BY priority ASC, created_at ASC
        LIMIT 1
        "#,
    )
    .fetch_optional(&state.pool)
    .await?;

    let Some(job_row) = next_job else {
        return Ok(false);
    };

    let job: RenderJob = job_row.into();
    let started_at = Utc::now().timestamp_millis();
    let rows_affected = sqlx::query(
        "UPDATE jobs SET status = 'running', started_at = ?, finished_at = NULL WHERE id = ? AND status = 'pending'",
    )
    .bind(started_at)
    .bind(&job.id)
    .execute(&state.pool)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        return Ok(true);
    }

    let running_job = load_job(&state.pool, &job.id).await?;
    emit_job_update(app, &running_job);

    log::info!("Starting job: {} ({})", running_job.name, running_job.id);
    let result = process::run_job(app.clone(), state.clone(), running_job.clone()).await;

    let final_status = {
        let mut interrupted_jobs = state.interrupted_jobs.lock().await;
        if interrupted_jobs.remove(&running_job.id) {
            JobStatus::Interrupted
        } else {
            let mut cancelled_jobs = state.cancelled_jobs.lock().await;
            if cancelled_jobs.remove(&running_job.id) {
                JobStatus::Cancelled
            } else {
                match result {
                    Ok(status) => status,
                    Err(error) => {
                        log::error!("Job {} errored: {error}", running_job.id);
                        JobStatus::Failed
                    }
                }
            }
        }
    };

    sqlx::query("UPDATE jobs SET status = ?, finished_at = ? WHERE id = ?")
        .bind(final_status)
        .bind(Utc::now().timestamp_millis())
        .bind(&running_job.id)
        .execute(&state.pool)
        .await?;

    let updated_job = load_job(&state.pool, &running_job.id).await?;
    emit_job_update(app, &updated_job);

    Ok(true)
}

pub async fn load_job(pool: &sqlx::SqlitePool, id: &str) -> anyhow::Result<RenderJob> {
    let job = sqlx::query_as::<_, DbRenderJob>(
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
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(job.into())
}

pub fn emit_job_update(app: &AppHandle, job: &RenderJob) {
    let _ = app.emit("job-updated", JobUpdatedEvent { job: job.clone() });
}
