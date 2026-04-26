mod app_paths;
mod blender;
mod commands;
mod db;
mod path_template;
mod queue;
mod state;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            crate::app_paths::ensure_runtime_layout()
                .expect("failed to initialize runtime layout");
            let app_handle = app.handle().clone();
            let state = tauri::async_runtime::block_on(db::init(&app_handle))
                .expect("failed to initialize database");
            app.manage(state);
            crate::commands::settings::get_settings(app_handle.clone())
                .expect("failed to initialize settings file");
            let state = app_handle
                .state::<state::AppState>()
                .inner()
                .clone();
            queue::scheduler::start(app_handle, state);
            let app_handle = app.handle().clone();
            let state = app_handle
                .state::<state::AppState>()
                .inner()
                .clone();
            queue::transcode_scheduler::start(app_handle, state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::jobs::list_jobs,
            commands::jobs::get_queue_state,
            commands::jobs::start_queue,
            commands::jobs::pause_queue,
            commands::jobs::add_job,
            commands::jobs::update_job_metadata,
            commands::jobs::update_job_transcode_settings,
            commands::jobs::remove_job,
            commands::jobs::cancel_job,
            commands::jobs::reorder_job,
            commands::jobs::update_job_fps,
            commands::jobs::get_job_logs,
            commands::jobs::get_job_latest_logs,
            commands::jobs::get_job_log_summary,
            commands::jobs::reset_job,
            commands::jobs::update_job_preview_dimensions,
            commands::path_template::preview_output_path_template,
            commands::blender::get_blender_versions,
            commands::blender::inspect_toolchain,
            commands::blender::add_blender_by_path,
            commands::blender::has_output_files,
            commands::blender::count_rendered_frames,
            commands::blender::inspect_rendered_frames,
            commands::blender::open_path,
            commands::blender::validate_blend_file,
            commands::blender::inspect_blend_file,
            commands::blender::clear_rendered_frames,
            commands::blender::get_last_rendered_frame,
            commands::blender::inspect_folder_frames,
            commands::transcode::list_ffmpeg_jobs,
            commands::transcode::get_ffmpeg_job,
            commands::transcode::add_ffmpeg_job,
            commands::transcode::cancel_ffmpeg_job,
            commands::transcode::delete_ffmpeg_job,
            commands::transcode::reorder_ffmpeg_jobs,
            commands::transcode::get_ffmpeg_job_logs,
            commands::transcode::get_ffmpeg_job_latest_logs,
            commands::transcode::scan_folder_frame_groups,
            commands::settings::get_settings,
            commands::settings::save_settings,
        ])
        .build(tauri::generate_context!())
        .expect("error while building sik-render");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::ExitRequested { .. } = event {
            if let Some(state) = app_handle.try_state::<state::AppState>() {
                tauri::async_runtime::block_on(state.inner().terminate_all_processes());
            }
        }
    });
}
