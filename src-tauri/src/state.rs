use chrono::Utc;
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub scheduler_notify: Arc<Notify>,
    pub active_jobs: Arc<Mutex<HashMap<String, u32>>>,
    pub active_mp4_exports: Arc<Mutex<HashMap<String, u32>>>,
    pub cancelled_jobs: Arc<Mutex<HashSet<String>>>,
    pub interrupted_jobs: Arc<Mutex<HashSet<String>>>,
    pub cancelled_mp4_exports: Arc<Mutex<HashSet<String>>>,
}

impl AppState {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            scheduler_notify: Arc::new(Notify::new()),
            active_jobs: Arc::new(Mutex::new(HashMap::new())),
            active_mp4_exports: Arc::new(Mutex::new(HashMap::new())),
            cancelled_jobs: Arc::new(Mutex::new(HashSet::new())),
            interrupted_jobs: Arc::new(Mutex::new(HashSet::new())),
            cancelled_mp4_exports: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn kill_process_tree(pid: u32) -> std::io::Result<()> {
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .status()?;
            return Ok(());
        }

        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            // Child render processes are started in their own process group.
            // Send the signal to the whole group so helper subprocesses die too.
            std::process::Command::new("kill")
                .args(["-9", &format!("-{pid}")])
                .status()?;
            return Ok(());
        }

        #[allow(unreachable_code)]
        Ok(())
    }

    pub async fn terminate_all_processes(&self) {
        let interrupted_line = "[interrupted] Reason: app shutdown. The app closed while Blender was still rendering, so the Blender child process was terminated. This job was marked interrupted and will not auto-resume; continue it manually from the last recorded frame.";
        let active_jobs = {
            let active = self.active_jobs.lock().await;
            active
                .iter()
                .map(|(job_id, pid)| (job_id.clone(), *pid))
                .collect::<Vec<_>>()
        };
        for (job_id, pid) in active_jobs {
            self.interrupted_jobs.lock().await.insert(job_id.clone());
            if let Ok(Some(job_number)) =
                sqlx::query_scalar::<_, i32>("SELECT job_number FROM jobs WHERE id = ?")
                    .bind(&job_id)
                    .fetch_optional(&self.pool)
                    .await
            {
                let _ = crate::app_paths::append_job_log_event(
                    job_number,
                    crate::app_paths::BLENDER_LOG_KIND,
                    interrupted_line,
                );
            }
            let _ = sqlx::query(
                "UPDATE jobs
                 SET status = 'interrupted',
                     finished_at = ?,
                     resume_from_existing = 1
                 WHERE id = ? AND status = 'running'",
            )
            .bind(Utc::now().timestamp_millis())
            .bind(&job_id)
            .execute(&self.pool)
            .await;
            let _ = Self::kill_process_tree(pid);
        }
        self.active_jobs.lock().await.clear();

        let active_mp4_exports = {
            let active = self.active_mp4_exports.lock().await;
            active.values().copied().collect::<Vec<_>>()
        };
        for pid in active_mp4_exports {
            let _ = Self::kill_process_tree(pid);
        }
        self.active_mp4_exports.lock().await.clear();
    }
}
