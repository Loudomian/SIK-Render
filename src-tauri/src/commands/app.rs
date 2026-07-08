use serde::Serialize;
use tauri::{AppHandle, Manager};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppVersionInfo {
    pub version: String,
    pub commit: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeResetResult {
    pub root_path: String,
    pub removed_paths: Vec<String>,
    pub failed_paths: Vec<RuntimeResetFailure>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeResetFailure {
    pub path: String,
    pub error: String,
}

#[tauri::command]
pub fn get_app_version_info() -> AppVersionInfo {
    AppVersionInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        commit: env!("GIT_COMMIT_HASH").to_string(),
    }
}

#[tauri::command]
pub fn get_app_runtime_dir() -> Result<String, String> {
    crate::app_paths::tool_root_dir()
        .map(|path| path.display().to_string())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn app_ready(app: AppHandle) -> Result<(), String> {
    let main_window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;

    main_window.show().map_err(|error| error.to_string())?;
    main_window.set_focus().map_err(|error| error.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn path_exists(path: String) -> bool {
    std::path::Path::new(path.trim()).exists()
}

#[tauri::command]
pub async fn reset_app_runtime_data(app: AppHandle) -> Result<RuntimeResetResult, String> {
    if let Some(state) = app.try_state::<crate::state::AppState>() {
        let state = state.inner().clone();
        state.set_queue_paused(true);
        state.terminate_all_processes().await;
        state.pool.close().await;
    }

    let (root_path, removed_paths, failed_paths) =
        crate::app_paths::reset_runtime_data().map_err(|error| error.to_string())?;

    Ok(RuntimeResetResult {
        root_path: root_path.display().to_string(),
        removed_paths: removed_paths
            .into_iter()
            .map(|path| path.display().to_string())
            .collect(),
        failed_paths: failed_paths
            .into_iter()
            .map(|(path, error)| RuntimeResetFailure {
                path: path.display().to_string(),
                error,
            })
            .collect(),
    })
}
