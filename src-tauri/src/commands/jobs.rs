use crate::queue::job::RenderJob;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
pub struct AddJobPayload {
    pub name: String,
    pub blend_file: String,
    pub blender_executable: String,
    pub output_path: String,
    pub output_format: String,
    pub frame_start: i32,
    pub frame_end: i32,
    pub priority: i32,
}

#[tauri::command]
pub async fn list_jobs() -> Result<Vec<RenderJob>, String> {
    // TODO: query SQLite
    Ok(vec![])
}

#[tauri::command]
pub async fn add_job(payload: AddJobPayload) -> Result<RenderJob, String> {
    let job = RenderJob::new(
        payload.name,
        PathBuf::from(payload.blend_file),
        PathBuf::from(payload.blender_executable),
        PathBuf::from(payload.output_path),
        payload.output_format,
        payload.frame_start,
        payload.frame_end,
        payload.priority,
    );
    // TODO: persist to SQLite, push to scheduler queue
    Ok(job)
}

#[tauri::command]
pub async fn remove_job(id: String) -> Result<(), String> {
    log::info!("remove_job: {id}");
    // TODO: remove from DB and queue
    Ok(())
}

#[tauri::command]
pub async fn cancel_job(id: String) -> Result<(), String> {
    log::info!("cancel_job: {id}");
    // TODO: send cancel signal to running process
    Ok(())
}

#[tauri::command]
pub async fn reorder_job(id: String, priority: i32) -> Result<(), String> {
    log::info!("reorder_job: {id} -> priority {priority}");
    // TODO: update priority in DB and re-sort queue
    Ok(())
}
