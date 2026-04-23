use serde::{Deserialize, Serialize};
use std::fs;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub default_blender: String,
    #[serde(default)]
    pub ffmpeg_executable: String,
    pub blend_inspect_timeout_seconds: u64,
    #[serde(default = "default_transcode_crf")]
    pub transcode_crf: u32,
    #[serde(default = "default_transcode_preset")]
    pub transcode_preset: String,
    #[serde(default = "default_ffmpeg_max_concurrent")]
    pub ffmpeg_max_concurrent: u32,
    pub theme: String,
    #[serde(default)]
    pub extra_blender_paths: Vec<String>,
    #[serde(default)]
    pub excluded_blender_paths: Vec<String>,
    #[serde(default = "default_max_crash_retries")]
    pub max_crash_retries: u32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            default_blender: String::new(),
            ffmpeg_executable: String::new(),
            blend_inspect_timeout_seconds: default_blend_inspect_timeout_seconds(),
            transcode_crf: default_transcode_crf(),
            transcode_preset: default_transcode_preset(),
            ffmpeg_max_concurrent: default_ffmpeg_max_concurrent(),
            theme: "dark".into(),
            extra_blender_paths: Vec::new(),
            excluded_blender_paths: Vec::new(),
            max_crash_retries: default_max_crash_retries(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SettingsFile {
    #[serde(default)]
    tools: ToolsSettings,
    #[serde(default)]
    ui: UiSettings,
    #[serde(default)]
    blender: BlenderSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ToolsSettings {
    #[serde(default)]
    default_blender: String,
    #[serde(default)]
    ffmpeg_executable: String,
    #[serde(default = "default_blend_inspect_timeout_seconds")]
    blend_inspect_timeout_seconds: u64,
    #[serde(default = "default_transcode_crf")]
    transcode_crf: u32,
    #[serde(default = "default_transcode_preset")]
    transcode_preset: String,
    #[serde(default = "default_ffmpeg_max_concurrent")]
    ffmpeg_max_concurrent: u32,
    #[serde(default = "default_max_crash_retries")]
    max_crash_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UiSettings {
    #[serde(default = "default_theme")]
    theme: String,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            theme: default_theme(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct BlenderSettings {
    #[serde(default)]
    extra_blender_paths: Vec<String>,
    #[serde(default)]
    excluded_blender_paths: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum SettingsFileCompat {
    Grouped(SettingsFile),
    Flat(AppSettings),
}

fn default_blend_inspect_timeout_seconds() -> u64 {
    30
}

fn default_max_crash_retries() -> u32 {
    3
}

fn default_transcode_crf() -> u32 {
    18
}

fn default_transcode_preset() -> String {
    String::from("medium")
}

fn default_ffmpeg_max_concurrent() -> u32 {
    2
}

fn default_theme() -> String {
    String::from("dark")
}

fn normalize_theme(theme: String) -> String {
    match theme.as_str() {
        "light" => String::from("light"),
        _ => default_theme(),
    }
}

fn normalize_blend_inspect_timeout_seconds(seconds: u64) -> u64 {
    seconds.clamp(5, 600)
}

fn normalize_max_crash_retries(retries: u32) -> u32 {
    retries.min(10)
}

fn normalize_ffmpeg_max_concurrent(value: u32) -> u32 {
    value.clamp(1, 8)
}

fn normalize_preset(preset: &str) -> String {
    match preset {
        "ultrafast" | "superfast" | "veryfast" | "faster" | "fast" | "medium" | "slow"
        | "slower" | "veryslow" => preset.to_string(),
        _ => default_transcode_preset(),
    }
}

impl From<SettingsFile> for AppSettings {
    fn from(value: SettingsFile) -> Self {
        Self {
            default_blender: value.tools.default_blender,
            ffmpeg_executable: value.tools.ffmpeg_executable,
            blend_inspect_timeout_seconds: normalize_blend_inspect_timeout_seconds(
                value.tools.blend_inspect_timeout_seconds,
            ),
            transcode_crf: value.tools.transcode_crf.min(51),
            transcode_preset: normalize_preset(&value.tools.transcode_preset),
            ffmpeg_max_concurrent: normalize_ffmpeg_max_concurrent(
                value.tools.ffmpeg_max_concurrent,
            ),
            theme: normalize_theme(value.ui.theme),
            extra_blender_paths: value.blender.extra_blender_paths,
            excluded_blender_paths: value.blender.excluded_blender_paths,
            max_crash_retries: normalize_max_crash_retries(value.tools.max_crash_retries),
        }
    }
}

impl From<AppSettings> for SettingsFile {
    fn from(value: AppSettings) -> Self {
        Self {
            tools: ToolsSettings {
                default_blender: value.default_blender,
                ffmpeg_executable: value.ffmpeg_executable,
                blend_inspect_timeout_seconds: normalize_blend_inspect_timeout_seconds(
                    value.blend_inspect_timeout_seconds,
                ),
                transcode_crf: value.transcode_crf.min(51),
                transcode_preset: value.transcode_preset.clone(),
                ffmpeg_max_concurrent: normalize_ffmpeg_max_concurrent(
                    value.ffmpeg_max_concurrent,
                ),
                max_crash_retries: normalize_max_crash_retries(value.max_crash_retries),
            },
            ui: UiSettings { theme: value.theme },
            blender: BlenderSettings {
                extra_blender_paths: value.extra_blender_paths,
                excluded_blender_paths: value.excluded_blender_paths,
            },
        }
    }
}

fn settings_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let _ = app;
    crate::app_paths::config_path().map_err(|error| error.to_string())
}

fn legacy_settings_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    crate::db::app_data_dir(app)
        .map(|dir| dir.join("settings.json"))
        .map_err(|error| error.to_string())
}

fn read_settings_from_disk(app: &AppHandle) -> Result<AppSettings, String> {
    let path = settings_path(app)?;
    if !path.exists() {
        let legacy_path = legacy_settings_path(app)?;
        if legacy_path.exists() {
            let content = fs::read_to_string(&legacy_path).map_err(|error| error.to_string())?;
            let settings: AppSettings =
                serde_json::from_str(&content).map_err(|error| error.to_string())?;
            save_settings(app.clone(), settings.clone())?;
            let _ = fs::remove_file(&legacy_path);
            return Ok(settings);
        }

        let settings = AppSettings::default();
        save_settings(app.clone(), settings.clone())?;
        return Ok(settings);
    }

    let content = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    match toml::from_str::<SettingsFileCompat>(&content) {
        Ok(SettingsFileCompat::Grouped(settings)) => Ok(settings.into()),
        Ok(SettingsFileCompat::Flat(settings)) => {
            save_settings(app.clone(), settings.clone())?;
            Ok(settings)
        }
        Err(error) => {
            log::error!("failed to parse settings file {}: {}", path.display(), error);
            let fallback = AppSettings::default();
            save_settings(app.clone(), fallback.clone())?;
            Ok(fallback)
        }
    }
}

#[cfg(target_os = "windows")]
fn replace_file_atomically(
    source: &std::path::Path,
    destination: &std::path::Path,
) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let source_wide = source
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let destination_wide = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();

    unsafe {
        MoveFileExW(
            PCWSTR(source_wide.as_ptr()),
            PCWSTR(destination_wide.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
        .map_err(|error| error.to_string())
    }
}

#[cfg(not(target_os = "windows"))]
fn replace_file_atomically(
    source: &std::path::Path,
    destination: &std::path::Path,
) -> Result<(), String> {
    fs::rename(source, destination).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Result<AppSettings, String> {
    if let Some(state) = app.try_state::<crate::state::AppState>() {
        if let Some(settings) = state.cached_settings() {
            return Ok(settings);
        }
    }

    let settings = read_settings_from_disk(&app)?;
    if let Some(state) = app.try_state::<crate::state::AppState>() {
        state.set_cached_settings(settings.clone());
    }
    Ok(settings)
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: AppSettings) -> Result<(), String> {
    let path = settings_path(&app)?;
    let mut settings = settings;
    settings.theme = normalize_theme(settings.theme);
    settings.blend_inspect_timeout_seconds =
        normalize_blend_inspect_timeout_seconds(settings.blend_inspect_timeout_seconds);
    settings.transcode_crf = settings.transcode_crf.min(51);
    settings.transcode_preset = normalize_preset(&settings.transcode_preset);
    settings.ffmpeg_max_concurrent =
        normalize_ffmpeg_max_concurrent(settings.ffmpeg_max_concurrent);
    settings.max_crash_retries = normalize_max_crash_retries(settings.max_crash_retries);
    let content = toml::to_string_pretty(&SettingsFile::from(settings.clone()))
        .map_err(|error| error.to_string())?;
    let tmp_path = path.with_extension("toml.tmp");
    fs::write(&tmp_path, content).map_err(|error| error.to_string())?;
    replace_file_atomically(&tmp_path, &path)?;
    if let Some(state) = app.try_state::<crate::state::AppState>() {
        state.set_cached_settings(settings);
    }
    Ok(())
}
