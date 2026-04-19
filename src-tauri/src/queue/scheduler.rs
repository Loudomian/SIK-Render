use crate::blender::process;
use crate::queue::job::{JobStatus, RenderJob};
use std::collections::VecDeque;
use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::Mutex;

pub type JobQueue = Arc<Mutex<VecDeque<RenderJob>>>;

pub fn new_queue() -> JobQueue {
    Arc::new(Mutex::new(VecDeque::new()))
}

/// Consume jobs from the queue one by one until empty.
pub async fn run_queue(app: AppHandle, queue: JobQueue) {
    loop {
        let job = {
            let mut q = queue.lock().await;
            q.pop_front()
        };

        match job {
            None => break,
            Some(mut job) => {
                log::info!("Starting job: {} ({})", job.name, job.id);
                job.status = JobStatus::Running;

                let result = process::run_job(app.clone(), job.clone()).await;

                match result {
                    Ok(status) => log::info!("Job {} finished: {:?}", job.id, status),
                    Err(e) => log::error!("Job {} errored: {e}", job.id),
                }
            }
        }
    }
}
