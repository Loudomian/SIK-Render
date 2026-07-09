mod app_paths;
mod blender;
mod commands;
mod db;
mod network;
mod path_template;
mod queue;
mod state;

use chrono::Local;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().skip_logger().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            crate::app_paths::ensure_runtime_layout().expect("failed to initialize runtime layout");
            let log_path = init_app_logger().expect("failed to initialize app logger");
            log::info!(
                "\n{}",
                r#" ____    ______   __  __      ____    ____    __  __  ____    ____    ____
/\  _`\ /\__  _\ /\ \/\ \    /\  _`\ /\  _`\ /\ \/\ \/\  _`\ /\  _`\ /\  _`\
\ \,\L\_\/_/\ \/ \ \ \/'/'   \ \ \L\ \ \ \L\_\ \ `\\ \ \ \/\ \ \ \L\_\ \ \L\ \
 \/_\__ \  \ \ \  \ \ , <     \ \ ,  /\ \  _\L\ \ , ` \ \ \ \ \ \  _\L\ \ ,  /
   /\ \L\ \ \_\ \__\ \ \\`\    \ \ \\ \\ \ \L\ \ \ \`\ \ \ \_\ \ \ \L\ \ \ \\ \
   \ `\____\/\_____\\ \_\ \_\   \ \_\ \_\ \____/\ \_\ \_\ \____/\ \____/\ \_\ \_\
    \/_____/\/_____/ \/_/\/_/    \/_/\/ /\/___/  \/_/\/_/\/___/  \/___/  \/_/\/ /"#
            );
            log::info!("SIK Render v{} starting", env!("CARGO_PKG_VERSION"));
            log::info!("app log file initialized at {}", log_path.display());
            let app_handle = app.handle().clone();
            let state = tauri::async_runtime::block_on(db::init(&app_handle))
                .expect("failed to initialize database");
            app.manage(state);
            crate::commands::settings::get_settings(app_handle.clone())
                .expect("failed to initialize settings file");
            let state = app_handle.state::<state::AppState>().inner().clone();
            queue::scheduler::start(app_handle, state);
            let app_handle = app.handle().clone();
            let state = app_handle.state::<state::AppState>().inner().clone();
            queue::transcode_scheduler::start(app_handle, state);
            let app_handle = app.handle().clone();
            let state = app_handle.state::<state::AppState>().inner().clone();
            network::server::start(app_handle.clone(), state.clone());
            network::discovery::start(app_handle, state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::jobs::list_jobs,
            commands::jobs::get_queue_state,
            commands::jobs::apply_shadow_resolution_recovery,
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
            commands::nodes::get_node_info,
            commands::nodes::get_peers,
            commands::nodes::get_node_job_events,
            commands::nodes::forget_peer,
            commands::nodes::list_node_interfaces,
            commands::path_template::preview_output_path_template,
            commands::app::get_app_version_info,
            commands::app::get_app_runtime_dir,
            commands::app::app_ready,
            commands::app::path_exists,
            commands::app::reset_app_runtime_data,
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
            commands::transcode::detect_video_encoders,
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

fn sikfilm_log_file_name() -> String {
    let timestamp = Local::now().format("%Y%m%d_%H%M%S_%3f");
    format!("sikrender_{timestamp}")
}

fn app_log_file_path() -> std::path::PathBuf {
    let log_dir = crate::app_paths::app_logs_dir().unwrap_or_else(|error| {
        eprintln!("failed to resolve app log directory: {error}");
        std::env::temp_dir()
            .join("SIKFilm")
            .join("Render")
            .join("logs")
            .join("app")
            .join(env!("CARGO_PKG_VERSION"))
    });
    log_dir.join(format!("{}.log", sikfilm_log_file_name()))
}

fn init_app_logger() -> anyhow::Result<std::path::PathBuf> {
    let log_path = app_log_file_path();
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    #[allow(unused_mut)]
    let mut dispatch = tauri_plugin_log::fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}][{}][{}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(tauri_plugin_log::fern::log_file(&log_path)?);

    // Windows release 版无控制台窗口，写 Stdout 会导致进程无法启动。
    #[cfg(any(debug_assertions, not(target_os = "windows")))]
    {
        dispatch = dispatch.chain(std::io::stdout());
    }

    let (max_level, logger) = dispatch.into_log();
    tauri_plugin_log::attach_logger(max_level, logger)?;
    Ok(log_path)
}
