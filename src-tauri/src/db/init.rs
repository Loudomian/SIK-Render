use crate::state::AppState;
use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub fn app_data_dir(app: &AppHandle) -> Result<std::path::PathBuf> {
    let dir = app
        .path()
        .app_data_dir()
        .context("failed to resolve app data directory")?;
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn tool_root_db_path() -> Result<PathBuf> {
    Ok(crate::app_paths::tool_root_dir()?.join("sik-render.sqlite3"))
}

pub async fn init(_app: &AppHandle) -> Result<AppState> {
    let db_path = tool_root_db_path()?;

    let connect_options = SqliteConnectOptions::new()
        .filename(&db_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options)
        .await?;

    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await?;

    sqlx::migrate!("./src/db/migrations").run(&pool).await?;

    let interrupted_jobs =
        sqlx::query_as::<_, (String, i32)>("SELECT id, job_number FROM jobs WHERE status = 'running'")
            .fetch_all(&pool)
            .await?;
    if !interrupted_jobs.is_empty() {
        let line = "[interrupted] Reason: startup recovery. This job was still marked running when the app started, which means the previous app session ended while rendering was in progress. It was marked interrupted and will not auto-resume; continue it manually from the last recorded frame.";
        for (job_id, job_number) in &interrupted_jobs {
            let _ = crate::app_paths::append_job_log_event(
                *job_number,
                job_id,
                crate::app_paths::BLENDER_LOG_KIND,
                line,
            );
        }
        sqlx::query(
            "UPDATE jobs
             SET status = 'interrupted',
                 finished_at = ?,
                 started_at = COALESCE(started_at, ?),
                 resume_from_existing = 1
             WHERE status = 'running'",
        )
        .bind(Utc::now().timestamp_millis())
        .bind(Utc::now().timestamp_millis())
        .execute(&pool)
        .await?;
    }

    crate::commands::transcode::recover_running_ffmpeg_jobs(&pool).await?;

    log::info!("DB initialized at {}", db_path.display());

    Ok(AppState::new(pool, None))
}
