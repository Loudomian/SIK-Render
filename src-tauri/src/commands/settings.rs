use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub default_blender: String,
    pub default_output_dir: String,
    pub max_concurrent_jobs: u32,
    pub theme: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            default_blender: String::new(),
            default_output_dir: String::new(),
            max_concurrent_jobs: 1,
            theme: "dark".into(),
        }
    }
}

#[tauri::command]
pub fn get_settings() -> AppSettings {
    // TODO: load from tauri-plugin-store
    AppSettings::default()
}

#[tauri::command]
pub fn save_settings(settings: AppSettings) -> Result<(), String> {
    log::info!("save_settings: {:?}", settings);
    // TODO: persist via tauri-plugin-store
    Ok(())
}
