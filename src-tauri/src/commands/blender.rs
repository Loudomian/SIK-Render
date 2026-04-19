use crate::blender::discovery::{discover, BlenderInstall};
use std::path::PathBuf;

#[tauri::command]
pub fn get_blender_versions() -> Vec<BlenderInstall> {
    discover()
}

#[tauri::command]
pub fn validate_blend_file(path: String) -> Result<bool, String> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err(format!("File not found: {path}"));
    }
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
    if ext != "blend" {
        return Err(format!("Not a .blend file: {path}"));
    }
    Ok(true)
}
