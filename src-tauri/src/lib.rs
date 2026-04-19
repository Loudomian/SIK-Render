mod blender;
mod commands;
mod db;
mod node;
mod queue;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = db::init(&app_handle).await {
                    log::error!("DB init failed: {e}");
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::jobs::list_jobs,
            commands::jobs::add_job,
            commands::jobs::remove_job,
            commands::jobs::cancel_job,
            commands::jobs::reorder_job,
            commands::blender::get_blender_versions,
            commands::blender::validate_blend_file,
            commands::settings::get_settings,
            commands::settings::save_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running sik-render");
}
