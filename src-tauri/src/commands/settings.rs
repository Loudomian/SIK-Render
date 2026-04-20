use serde::{Deserialize, Serialize};
use std::fs;
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub default_blender: String,
    #[serde(default)]
    pub ffmpeg_executable: String,
    pub default_output_dir: String,
    pub max_concurrent_jobs: u32,
    pub theme: String,
    #[serde(default)]
    pub extra_blender_paths: Vec<String>,
    #[serde(default)]
    pub excluded_blender_paths: Vec<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            default_blender: String::new(),
            ffmpeg_executable: String::new(),
            default_output_dir: String::new(),
            max_concurrent_jobs: 1,
            theme: "dark".into(),
            extra_blender_paths: Vec::new(),
            excluded_blender_paths: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct SettingsFile {
    #[serde(default)]
    tools: ToolsSettings,
    #[serde(default)]
    paths: PathSettings,
    #[serde(default)]
    queue: QueueSettings,
    #[serde(default)]
    ui: UiSettings,
    #[serde(default)]
    blender: BlenderSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct ToolsSettings {
    #[serde(default)]
    default_blender: String,
    #[serde(default)]
    ffmpeg_executable: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct PathSettings {
    #[serde(default)]
    default_output_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct QueueSettings {
    #[serde(default = "default_max_concurrent_jobs")]
    max_concurrent_jobs: u32,
}

impl Default for QueueSettings {
    fn default() -> Self {
        Self {
            max_concurrent_jobs: default_max_concurrent_jobs(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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

fn default_max_concurrent_jobs() -> u32 {
    1
}

fn default_theme() -> String {
    String::from("dark")
}

impl From<SettingsFile> for AppSettings {
    fn from(value: SettingsFile) -> Self {
        Self {
            default_blender: value.tools.default_blender,
            ffmpeg_executable: value.tools.ffmpeg_executable,
            default_output_dir: value.paths.default_output_dir,
            max_concurrent_jobs: value.queue.max_concurrent_jobs,
            theme: value.ui.theme,
            extra_blender_paths: value.blender.extra_blender_paths,
            excluded_blender_paths: value.blender.excluded_blender_paths,
        }
    }
}

impl From<AppSettings> for SettingsFile {
    fn from(value: AppSettings) -> Self {
        Self {
            tools: ToolsSettings {
                default_blender: value.default_blender,
                ffmpeg_executable: value.ffmpeg_executable,
            },
            paths: PathSettings {
                default_output_dir: value.default_output_dir,
            },
            queue: QueueSettings {
                max_concurrent_jobs: value.max_concurrent_jobs,
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

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Result<AppSettings, String> {
    let path = settings_path(&app)?;
    if !path.exists() {
        let legacy_path = legacy_settings_path(&app)?;
        if legacy_path.exists() {
            let content = fs::read_to_string(&legacy_path).map_err(|error| error.to_string())?;
            let settings: AppSettings = serde_json::from_str(&content).map_err(|error| error.to_string())?;
            save_settings(app.clone(), settings.clone())?;
            return Ok(settings);
        }

        let settings = AppSettings::default();
        save_settings(app.clone(), settings.clone())?;
        return Ok(settings);
    }

    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let settings = match toml::from_str::<SettingsFileCompat>(&content) {
        Ok(SettingsFileCompat::Grouped(settings)) => settings.into(),
        Ok(SettingsFileCompat::Flat(settings)) => {
            save_settings(app.clone(), settings.clone())?;
            settings
        }
        Err(error) => return Err(error.to_string()),
    };

    Ok(settings)
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: AppSettings) -> Result<(), String> {
    let path = settings_path(&app)?;
    let content = toml::to_string_pretty(&SettingsFile::from(settings))
        .map_err(|error| error.to_string())?;
    fs::write(path, content).map_err(|error| error.to_string())
}
