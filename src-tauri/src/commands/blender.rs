use crate::blender::discovery::BlenderInstall;
use crate::blender::project::{
    inspect_project_with_timeout, normalize_versions, BlendProjectSettings,
};
use crate::state::AppState;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, State};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedFramesStatus {
    pub frame_count: u32,
    pub last_frame: Option<i32>,
    pub next_frame: i32,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderFramesInspection {
    pub detected_format: Option<String>,
    pub frame_start: Option<i32>,
    pub frame_end: Option<i32>,
    pub frame_count: i32,
    pub missing_count: i32,
    pub has_gaps: bool,
    pub folder_name: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolchainStatus {
    pub blender_installs: Vec<BlenderInstall>,
    pub ffmpeg_found: bool,
    pub ffmpeg_executable: Option<String>,
    pub ffmpeg_source: Option<String>,
}

#[derive(Debug, Clone)]
struct FolderFrameFile {
    frame: i32,
    path: PathBuf,
}

fn display_path(path: &std::path::Path) -> String {
    let rendered = path.display().to_string();
    #[cfg(target_os = "windows")]
    {
        rendered
            .strip_prefix(r"\\?\")
            .unwrap_or(&rendered)
            .to_string()
    }
    #[cfg(not(target_os = "windows"))]
    {
        rendered
    }
}

fn blend_inspect_timeout_seconds(state: &AppState) -> u64 {
    state
        .cached_settings()
        .map(|settings| settings.blend_inspect_timeout_seconds.max(1))
        .unwrap_or(30)
}

fn format_exts_for(format: &str) -> &'static [&'static str] {
    match format.to_ascii_uppercase().as_str() {
        "PNG" => &["png"],
        "JPEG" | "JPG" => &["jpg", "jpeg"],
        "OPEN_EXR" | "EXR" => &["exr"],
        "TARGA" | "TGA" => &["tga"],
        "TIFF" => &["tif", "tiff"],
        "BMP" => &["bmp"],
        "HDR" => &["hdr"],
        "WEBP" => &["webp"],
        _ => &[],
    }
}

fn ext_to_format(ext: &str) -> String {
    match ext {
        "png" => "PNG".to_string(),
        "jpg" | "jpeg" => "JPEG".to_string(),
        "exr" => "OPEN_EXR".to_string(),
        "tga" => "TARGA".to_string(),
        "tif" | "tiff" => "TIFF".to_string(),
        "bmp" => "BMP".to_string(),
        "hdr" => "HDR".to_string(),
        "webp" => "WEBP".to_string(),
        _ => ext.to_ascii_uppercase(),
    }
}

fn inspect_folder_frame_files(
    dir: &Path,
    format_hint: Option<&str>,
) -> Result<(Vec<FolderFrameFile>, Option<String>), String> {
    if !dir.is_dir() {
        return Err(format!("{} is not a directory", dir.display()));
    }

    let allowed_exts = format_hint.map(format_exts_for);
    let mut frames = Vec::new();
    let mut detected_format = None;

    let entries = std::fs::read_dir(dir).map_err(|error| error.to_string())?;
    for entry in entries {
        let path = match entry {
            Ok(entry) => entry.path(),
            Err(_) => continue,
        };
        if !path.is_file() {
            continue;
        }

        let Some(ext) = path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase())
        else {
            continue;
        };

        if !IMAGE_EXTS.contains(&ext.as_str()) {
            continue;
        }

        if let Some(allowed) = allowed_exts {
            if !allowed.iter().any(|candidate| *candidate == ext) {
                continue;
            }
        }

        let Some(frame) = crate::blender::frame_path::trailing_frame_number(&path) else {
            continue;
        };

        if detected_format.is_none() {
            detected_format = Some(ext_to_format(&ext));
        }

        frames.push(FolderFrameFile { frame, path });
    }

    frames.sort_by(|a, b| a.frame.cmp(&b.frame).then_with(|| a.path.cmp(&b.path)));
    frames.dedup_by(|a, b| a.frame == b.frame);

    Ok((frames, detected_format))
}

#[tauri::command]
pub fn open_path(path: String) {
    let p = PathBuf::from(&path);
    // If path doesn't exist (e.g. contains ###### pattern), open parent directory
    let target = if p.exists() {
        p
    } else {
        let cleaned = crate::blender::frame_path::strip_frame_placeholders(&path);
        PathBuf::from(cleaned.trim_end_matches(['\\', '/']).to_string())
    };
    let target = if target.exists() {
        target.clone()
    } else {
        target.parent().map(|d| d.to_path_buf()).unwrap_or(target)
    };

    #[cfg(target_os = "windows")]
    {
        let explorer = "explorer.exe";
        let normalized = target.canonicalize().unwrap_or_else(|_| target.clone());

        let mut command = std::process::Command::new(explorer);
        if normalized.is_file() {
            command.arg(format!("/select,{}", normalized.display()));
        } else {
            command.arg(&normalized);
        }
        let _ = command.spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(&target).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(&target).spawn();
    }
}

async fn resolved_blender_versions(app: &AppHandle) -> Vec<BlenderInstall> {
    let settings = crate::commands::settings::get_settings(app.clone()).unwrap_or_default();
    let excluded = settings
        .excluded_blender_paths
        .into_iter()
        .map(PathBuf::from)
        .collect::<BTreeSet<_>>();
    let extra_paths = settings.extra_blender_paths;

    tokio::task::spawn_blocking(move || {
        let mut installs = crate::blender::discovery::discover();
        installs.extend(extra_paths.into_iter().filter_map(|path| {
            let candidate = PathBuf::from(path);
            crate::blender::discovery::blender_install_at(&candidate).ok()
        }));
        normalize_versions(installs)
            .into_iter()
            .filter(|install| !excluded.contains(&install.executable))
            .collect()
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
pub async fn get_blender_versions(app: AppHandle) -> Vec<BlenderInstall> {
    resolved_blender_versions(&app).await
}

#[tauri::command]
pub async fn inspect_toolchain(app: AppHandle) -> ToolchainStatus {
    let blender_installs = resolved_blender_versions(&app).await;
    let settings = crate::commands::settings::get_settings(app.clone()).unwrap_or_default();
    let configured_ffmpeg = if settings.ffmpeg_executable.trim().is_empty() {
        None
    } else {
        Some(PathBuf::from(settings.ffmpeg_executable.trim()))
    };

    let blender_probe = if !settings.default_blender.trim().is_empty() {
        PathBuf::from(settings.default_blender.trim())
    } else if let Some(first_install) = blender_installs.first() {
        first_install.executable.clone()
    } else {
        std::env::current_exe().unwrap_or_else(|_| PathBuf::from("ffmpeg"))
    };

    let ffmpeg_lookup = crate::blender::ffmpeg::find_ffmpeg_executable(
        Some(&app),
        configured_ffmpeg.as_deref(),
        &blender_probe,
    );

    ToolchainStatus {
        blender_installs,
        ffmpeg_found: ffmpeg_lookup.executable.is_some(),
        ffmpeg_executable: ffmpeg_lookup.executable.map(|path| display_path(&path)),
        ffmpeg_source: ffmpeg_lookup.source.map(str::to_string),
    }
}

#[tauri::command]
pub fn add_blender_by_path(path: String) -> Result<BlenderInstall, String> {
    let exe = std::path::PathBuf::from(&path);
    if !exe.exists() {
        return Err(format!("File not found: {path}"));
    }
    crate::blender::discovery::blender_install_at(&exe).map_err(|e| e.to_string())
}

const IMAGE_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "exr", "tif", "tiff", "tga", "bmp", "hdr", "webp",
];
fn collect_rendered_frames(
    output_path: &std::path::Path,
    format: &str,
    frame_start: Option<i32>,
    frame_end: Option<i32>,
) -> Vec<(i32, PathBuf)> {
    use crate::blender::process::{format_to_ext, frame_filename};

    let mut exact_matches = Vec::new();
    if frame_start.is_some()
        && frame_end.is_some()
        && output_path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.contains('#'))
            .unwrap_or(false)
    {
        for frame in frame_start.unwrap_or_default()..=frame_end.unwrap_or_default() {
            if let Some(file) = frame_filename(output_path, frame, format) {
                if file.exists() {
                    exact_matches.push((frame, file));
                }
            }
        }
    }

    if !exact_matches.is_empty() {
        return exact_matches;
    }

    let dir = if output_path.is_dir() {
        output_path.to_path_buf()
    } else {
        match output_path.parent() {
            Some(parent) => parent.to_path_buf(),
            None => return Vec::new(),
        }
    };

    let expected_ext = format_to_ext(format).to_ascii_lowercase();
    let template = output_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let prefix_hint = template
        .find('#')
        .map(|idx| template[..idx].to_string())
        .unwrap_or_default();
    let suffix_hint = template
        .find('#')
        .map(|idx| {
            let hash_count = template[idx..].chars().take_while(|&ch| ch == '#').count();
            let suffix_raw = &template[idx + hash_count..];
            if let Some(dot) = suffix_raw.rfind('.') {
                suffix_raw[..dot].to_string()
            } else {
                suffix_raw.to_string()
            }
        })
        .unwrap_or_default();

    let mut scanned: Vec<(i32, PathBuf)> = match std::fs::read_dir(&dir) {
        Ok(entries) => entries
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .filter(|path| {
                path.extension()
                    .map(|ext| ext.to_string_lossy().to_ascii_lowercase() == expected_ext)
                    .unwrap_or(false)
            })
            .filter(|path| {
                let stem = path
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default();
                (prefix_hint.is_empty() || stem.starts_with(&prefix_hint))
                    && (suffix_hint.is_empty() || stem.ends_with(&suffix_hint))
            })
            .filter_map(|path| {
                let frame = crate::blender::frame_path::trailing_frame_number(&path)?;
                if let Some(start) = frame_start {
                    if frame < start {
                        return None;
                    }
                }
                if let Some(end) = frame_end {
                    if frame > end {
                        return None;
                    }
                }
                if frame < 0 {
                    return None;
                }
                Some((frame, path))
            })
            .collect(),
        Err(_) => Vec::new(),
    };
    scanned.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    scanned
}

/// Returns the number of rendered image files in the output directory.
/// Accepts either a plain directory path or a Blender output pattern (containing ######).
#[tauri::command]
pub fn has_output_files(path: String) -> u32 {
    let p = std::path::PathBuf::from(&path);
    let dir = if p.is_dir() {
        p
    } else {
        // Strip filename / ###### pattern to get the parent directory
        let cleaned = crate::blender::frame_path::strip_frame_placeholders(&path);
        let cleaned_path = std::path::PathBuf::from(cleaned.trim_end_matches(['/', '\\']));
        if cleaned_path.is_dir() {
            cleaned_path
        } else {
            match cleaned_path.parent() {
                Some(parent) if parent.is_dir() => parent.to_path_buf(),
                _ => return 0,
            }
        }
    };

    std::fs::read_dir(&dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|x| IMAGE_EXTS.contains(&x.to_string_lossy().to_lowercase().as_str()))
                        .unwrap_or(false)
                })
                .count() as u32
        })
        .unwrap_or(0)
}

#[tauri::command]
pub fn count_rendered_frames(
    output_path: String,
    format: String,
    frame_start: i32,
    frame_end: i32,
) -> u32 {
    collect_rendered_frames(
        &std::path::PathBuf::from(output_path),
        &format,
        Some(frame_start),
        Some(frame_end),
    )
    .len() as u32
}

#[tauri::command]
pub fn inspect_rendered_frames(
    output_path: String,
    format: String,
    frame_start: i32,
    frame_end: i32,
) -> RenderedFramesStatus {
    let frames = collect_rendered_frames(
        &std::path::PathBuf::from(output_path),
        &format,
        Some(frame_start),
        Some(frame_end),
    );
    let rendered: BTreeSet<i32> = frames.iter().map(|(frame, _)| *frame).collect();
    let mut next_frame = frame_start;
    while next_frame <= frame_end && rendered.contains(&next_frame) {
        next_frame += 1;
    }

    RenderedFramesStatus {
        frame_count: frames.len() as u32,
        last_frame: frames.last().map(|(frame, _)| *frame),
        next_frame,
    }
}

/// Delete rendered frame files matching the output pattern for [frame_start, frame_end].
/// Returns the number of files deleted.
#[tauri::command]
pub fn clear_rendered_frames(
    output_path: String,
    format: String,
    frame_start: i32,
    frame_end: i32,
) -> u32 {
    let path = std::path::PathBuf::from(&output_path);
    let mut deleted = 0u32;
    for (_, file) in collect_rendered_frames(&path, &format, Some(frame_start), Some(frame_end)) {
        if file.exists() && std::fs::remove_file(&file).is_ok() {
            deleted += 1;
        }
    }
    deleted
}

/// Return the path of the last rendered frame.
/// First tries template-based matching (######), then falls back to scanning
/// the output directory for the most recently modified image with the right extension.
#[tauri::command]
pub fn get_last_rendered_frame(
    output_path: String,
    format: String,
    frame_start: i32,
    frame_end: i32,
) -> Option<String> {
    let path = std::path::PathBuf::from(&output_path);
    collect_rendered_frames(&path, &format, Some(frame_start), Some(frame_end))
        .into_iter()
        .last()
        .map(|(_, path)| path.to_string_lossy().to_string())
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

    // Some valid Blender projects may be stored with additional compression layers.
    // Let Blender itself perform the final validation instead of rejecting on a raw header check.
    Ok(true)
}

#[tauri::command]
pub async fn inspect_blend_file(
    blender_executable: String,
    path: String,
    state: State<'_, AppState>,
) -> Result<BlendProjectSettings, String> {
    let blender_path = PathBuf::from(&blender_executable);
    if !blender_path.exists() {
        return Err(format!(
            "Blender executable not found: {blender_executable}"
        ));
    }

    let blend_path = PathBuf::from(&path);
    if !blend_path.exists() {
        return Err(format!("Blend file not found: {path}"));
    }
    validate_blend_file(path.clone())?;

    inspect_project_with_timeout(
        &blender_path,
        &blend_path,
        blend_inspect_timeout_seconds(state.inner()),
    )
    .await
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn inspect_folder_frames(
    folder_path: String,
    format_hint: Option<String>,
) -> Result<FolderFramesInspection, String> {
    let dir = PathBuf::from(&folder_path);
    let folder_name = dir
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();

    let (frames, detected_format) = inspect_folder_frame_files(&dir, format_hint.as_deref())?;

    if frames.is_empty() {
        return Ok(FolderFramesInspection {
            detected_format: None,
            frame_start: None,
            frame_end: None,
            frame_count: 0,
            missing_count: 0,
            has_gaps: false,
            folder_name,
        });
    }

    let frame_start = frames.first().map(|frame| frame.frame).unwrap_or_default();
    let frame_end = frames.last().map(|frame| frame.frame).unwrap_or_default();
    let expected = (frame_end - frame_start + 1).max(0) as usize;
    let actual = frames.len();
    let missing_count = expected.saturating_sub(actual) as i32;

    Ok(FolderFramesInspection {
        detected_format,
        frame_start: Some(frame_start),
        frame_end: Some(frame_end),
        frame_count: actual as i32,
        missing_count,
        has_gaps: missing_count > 0,
        folder_name,
    })
}
