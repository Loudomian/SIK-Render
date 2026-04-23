use crate::commands::transcode;
use crate::queue::ffmpeg_job::{DbFfmpegJob, FfmpegJob, FfmpegJobStatus};
use crate::state::AppState;
use chrono::Utc;
use std::time::Duration;
use tauri::AppHandle;

pub fn start(app: AppHandle, state: AppState) {
    tauri::async_runtime::spawn(async move {
        loop {
            let app_handle = app.clone();
            let app_state = state.clone();
            let scheduler = tokio::spawn(async move {
                scheduler_loop(app_handle, app_state).await;
            });

            match scheduler.await {
                Ok(()) => {
                    log::warn!("Transcode scheduler loop exited unexpectedly; restarting");
                }
                Err(error) => {
                    log::error!("Transcode scheduler panicked: {error}");
                }
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
}

async fn scheduler_loop(app: AppHandle, state: AppState) {
    loop {
        match schedule_jobs(&app, &state).await {
            Ok(true) => continue,
            Ok(false) => {
                tokio::select! {
                    _ = state.ffmpeg_notify.notified() => {}
                    _ = tokio::time::sleep(Duration::from_secs(2)) => {}
                }
            }
            Err(error) => {
                log::error!("Transcode scheduler loop failed: {error}");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

async fn running_job_count(state: &AppState) -> anyhow::Result<u32> {
    let count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM ffmpeg_jobs WHERE status = 'running'")
            .fetch_one(&state.pool)
            .await?;
    Ok(count.max(0) as u32)
}

fn ffmpeg_max_concurrent(state: &AppState) -> u32 {
    state
        .cached_settings()
        .map(|settings| settings.ffmpeg_max_concurrent.clamp(1, 8))
        .unwrap_or(2)
}

async fn schedule_jobs(app: &AppHandle, state: &AppState) -> anyhow::Result<bool> {
    let running_jobs = running_job_count(state).await?;
    let max_concurrent = ffmpeg_max_concurrent(state);
    if running_jobs >= max_concurrent {
        return Ok(false);
    }

    let slots = (max_concurrent - running_jobs).max(1) as i64;
    let jobs = try_start_next_jobs(app, state, slots).await?;
    if jobs.is_empty() {
        return Ok(false);
    }

    for job in jobs {
        spawn_job_runner(app.clone(), state.clone(), job);
    }

    Ok(true)
}

async fn try_start_next_jobs(
    app: &AppHandle,
    state: &AppState,
    limit: i64,
) -> anyhow::Result<Vec<FfmpegJob>> {
    let next_jobs = sqlx::query_as::<_, DbFfmpegJob>(
        r#"
        SELECT
            id,
            job_number,
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
            status,
            priority,
            created_at,
            started_at,
            finished_at,
            progress_frame,
            total_frames,
            output_size_bytes,
            output_duration_secs
        FROM ffmpeg_jobs
        WHERE status = 'pending'
        ORDER BY priority ASC, created_at ASC
        LIMIT ?
        "#,
    )
    .bind(limit)
    .fetch_all(&state.pool)
    .await?;

    let mut started_jobs = Vec::new();
    for job_row in next_jobs {
        let job: FfmpegJob = job_row.into();
        let started_at = Utc::now().timestamp_millis();
        let rows_affected = sqlx::query(
            "UPDATE ffmpeg_jobs
             SET status = 'running',
                 started_at = ?,
                 finished_at = NULL,
                 progress_frame = 0,
                 total_frames = ?,
                 output_size_bytes = NULL,
                 output_duration_secs = NULL
             WHERE id = ? AND status = 'pending'",
        )
        .bind(started_at)
        .bind(job.total_frames())
        .bind(&job.id)
        .execute(&state.pool)
        .await?
        .rows_affected();

        if rows_affected == 0 {
            continue;
        }

        let running_job = transcode::load_ffmpeg_job(&state.pool, &job.id).await?;
        transcode::emit_ffmpeg_job_update(app, &running_job);
        started_jobs.push(running_job);
    }

    Ok(started_jobs)
}

fn spawn_job_runner(app: AppHandle, state: AppState, running_job: FfmpegJob) {
    tauri::async_runtime::spawn(async move {
        log::info!(
            "Starting ffmpeg job: {} ({})",
            running_job.name,
            running_job.id
        );

        let result = transcode::run_ffmpeg_job(app.clone(), state.clone(), running_job.clone()).await;
        let final_status = match result {
            Ok(status) => status,
            Err(error) => {
                log::error!("FFmpeg Job {} errored: {error}", running_job.id);
                FfmpegJobStatus::Failed
            }
        };

        let finished_at = Utc::now().timestamp_millis();
        let output_metadata = if final_status == FfmpegJobStatus::Done {
            std::fs::metadata(&running_job.output_path).ok()
        } else {
            None
        };
        let output_size_bytes = output_metadata.as_ref().map(|metadata| metadata.len() as i64);
        let output_duration_secs = if final_status == FfmpegJobStatus::Done {
            Some((running_job.total_frames().max(0) as f32) / running_job.fps.max(0.001))
        } else {
            None
        };
        let progress_frame = if final_status == FfmpegJobStatus::Done {
            Some(running_job.total_frames())
        } else {
            None
        };

        if let Err(error) = sqlx::query(
            "UPDATE ffmpeg_jobs
             SET status = ?,
                 finished_at = ?,
                 progress_frame = COALESCE(?, progress_frame),
                 total_frames = COALESCE(total_frames, ?),
                 output_size_bytes = ?,
                 output_duration_secs = ?
             WHERE id = ?",
        )
        .bind(final_status.clone())
        .bind(finished_at)
        .bind(progress_frame)
        .bind(running_job.total_frames())
        .bind(output_size_bytes)
        .bind(output_duration_secs)
        .bind(&running_job.id)
        .execute(&state.pool)
        .await
        {
            log::error!("Failed to finalize ffmpeg job {}: {error}", running_job.id);
            state.ffmpeg_notify.notify_one();
            return;
        }

        match transcode::load_ffmpeg_job(&state.pool, &running_job.id).await {
            Ok(updated_job) => {
                transcode::emit_ffmpeg_job_update(&app, &updated_job);
                let _ = transcode::write_ffmpeg_job_toml(&updated_job);
            }
            Err(error) => {
                log::error!("Failed to reload ffmpeg job {}: {error}", running_job.id);
            }
        }

        state.ffmpeg_notify.notify_one();
    });
}
