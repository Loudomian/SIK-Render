use crate::commands::settings::AppSettings;
use chrono::Utc;
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::sync::RwLock;
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
    pub settings: Arc<RwLock<Option<AppSettings>>>,
    pub queue_paused: Arc<RwLock<bool>>,
}

impl AppState {
    pub fn new(pool: SqlitePool, settings: Option<AppSettings>) -> Self {
        Self {
            pool,
            scheduler_notify: Arc::new(Notify::new()),
            active_jobs: Arc::new(Mutex::new(HashMap::new())),
            active_mp4_exports: Arc::new(Mutex::new(HashMap::new())),
            cancelled_jobs: Arc::new(Mutex::new(HashSet::new())),
            interrupted_jobs: Arc::new(Mutex::new(HashSet::new())),
            cancelled_mp4_exports: Arc::new(Mutex::new(HashSet::new())),
            settings: Arc::new(RwLock::new(settings)),
            queue_paused: Arc::new(RwLock::new(true)),
        }
    }

    pub fn cached_settings(&self) -> Option<AppSettings> {
        self.settings.read().ok().and_then(|settings| settings.clone())
    }

    pub fn set_cached_settings(&self, settings: AppSettings) {
        if let Ok(mut cached) = self.settings.write() {
            *cached = Some(settings);
        }
    }

    pub fn is_queue_paused(&self) -> bool {
        self.queue_paused.read().map(|paused| *paused).unwrap_or(true)
    }

    pub fn set_queue_paused(&self, paused: bool) {
        match self.queue_paused.write() {
            Ok(mut queue_paused) => {
                *queue_paused = paused;
            }
            Err(error) => {
                log::error!("Failed to update queue paused state: {error}");
            }
        }
    }

    pub fn kill_process_tree(pid: u32) -> std::io::Result<()> {
        #[cfg(target_os = "windows")]
        {
            let mut command = std::process::Command::new("taskkill");
            command.args(["/PID", &pid.to_string(), "/T", "/F"]);
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000); // CREATE_NO_WINDOW
            command.status()?;
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
            if let Ok(Some((job_number, resolved_job_id))) =
                sqlx::query_as::<_, (i32, String)>("SELECT job_number, id FROM jobs WHERE id = ?")
                    .bind(&job_id)
                    .fetch_optional(&self.pool)
                    .await
            {
                let _ = crate::app_paths::append_job_log_event(
                    job_number,
                    &resolved_job_id,
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
