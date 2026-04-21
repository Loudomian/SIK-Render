use crate::blender::discovery::BlenderInstall;
use crate::blender::project::{inspect_project_with_timeout, normalize_versions, BlendProjectSettings};
use crate::state::AppState;
use chrono::Local;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, State};
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mp4ExportResult {
    pub output_path: String,
    pub fps: f32,
    pub frame_count: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mp4ExportInspection {
    pub available_start: Option<i32>,
    pub available_end: Option<i32>,
    pub selected_start: Option<i32>,
    pub selected_end: Option<i32>,
    pub frame_count: u32,
    pub missing_count: u32,
    pub has_gaps: bool,
    pub missing_segments: Vec<String>,
    pub missing_segments_truncated: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mp4LogEvent {
    pub job_id: String,
    pub line: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedFramesStatus {
    pub frame_count: u32,
    pub last_frame: Option<i32>,
    pub next_frame: i32,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolchainStatus {
    pub blender_installs: Vec<BlenderInstall>,
    pub ffmpeg_found: bool,
    pub ffmpeg_executable: Option<String>,
    pub ffmpeg_source: Option<String>,
}

struct TempDirGuard(PathBuf);

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
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
        target
            .parent()
            .map(|d| d.to_path_buf())
            .unwrap_or(target)
    };

#[cfg(target_os = "windows")]
    {
        let explorer = "explorer.exe";
        let normalized = target
            .canonicalize()
            .unwrap_or_else(|_| target.clone());

        let mut command = std::process::Command::new(explorer);
        if normalized.is_file() {
            command.arg(format!("/select,{}", normalized.display()));
        } else {
            command.arg(&normalized);
        }
        let _ = command.spawn();
    }
    #[cfg(target_os = "macos")]
    { let _ = std::process::Command::new("open").arg(&target).spawn(); }
    #[cfg(target_os = "linux")]
    { let _ = std::process::Command::new("xdg-open").arg(&target).spawn(); }
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
        ffmpeg_executable: ffmpeg_lookup
            .executable
            .map(|path| display_path(&path)),
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
    "png", "jpg", "jpeg", "exr", "tiff", "tga", "bmp", "hdr", "webp",
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
            let hash_count = template[idx..]
                .chars()
                .take_while(|&ch| ch == '#')
                .count();
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
                let stem = path.file_stem().and_then(|name| name.to_str()).unwrap_or_default();
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

fn format_frame_segment(start: i32, end: i32) -> String {
    if start == end {
        start.to_string()
    } else {
        format!("{start}-{end}")
    }
}

fn summarize_missing_segments(
    start: i32,
    end: i32,
    present: &BTreeSet<i32>,
    max_segments: usize,
) -> (Vec<String>, u32, bool) {
    let mut segments = Vec::new();
    let mut missing_count = 0u32;
    let mut missing_start: Option<i32> = None;
    let mut missing_end = start;

    for frame in start..=end {
        if present.contains(&frame) {
            if let Some(segment_start) = missing_start.take() {
                missing_count += (missing_end - segment_start + 1) as u32;
                if segments.len() < max_segments {
                    segments.push(format_frame_segment(segment_start, missing_end));
                }
            }
            continue;
        }

        if missing_start.is_none() {
            missing_start = Some(frame);
        }
        missing_end = frame;
    }

    if let Some(segment_start) = missing_start {
        missing_count += (missing_end - segment_start + 1) as u32;
        if segments.len() < max_segments {
            segments.push(format_frame_segment(segment_start, missing_end));
        }
    }

    let missing_segments_truncated = missing_count > 0 && segments.len() == max_segments;
    (segments, missing_count, missing_segments_truncated)
}

fn build_mp4_export_inspection(
    output_path: &std::path::Path,
    format: &str,
    selected_start: Option<i32>,
    selected_end: Option<i32>,
) -> Mp4ExportInspection {
    let all_frames = collect_rendered_frames(output_path, format, None, None);
    let available_start = all_frames.first().map(|(frame, _)| *frame);
    let available_end = all_frames.last().map(|(frame, _)| *frame);

    let selected_frames = match (selected_start, selected_end) {
        (Some(start), Some(end)) if start <= end => {
            collect_rendered_frames(output_path, format, Some(start), Some(end))
        }
        _ => Vec::new(),
    };
    let present = selected_frames
        .iter()
        .map(|(frame, _)| *frame)
        .collect::<BTreeSet<_>>();

    let (missing_segments, missing_count, missing_segments_truncated) =
        match (selected_start, selected_end) {
            (Some(start), Some(end)) if start <= end => {
                summarize_missing_segments(start, end, &present, 18)
            }
            _ => (Vec::new(), 0, false),
        };

    Mp4ExportInspection {
        available_start,
        available_end,
        selected_start,
        selected_end,
        frame_count: selected_frames.len() as u32,
        missing_count,
        has_gaps: missing_count > 0,
        missing_segments,
        missing_segments_truncated,
    }
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
pub fn inspect_mp4_export(
    output_path: String,
    format: String,
    job_frame_start: i32,
    job_frame_end: i32,
    range_mode: String,
    custom_start: Option<i32>,
    custom_end: Option<i32>,
) -> Result<Mp4ExportInspection, String> {
    let path = PathBuf::from(output_path);
    let selected_range = match range_mode.as_str() {
        "job" => (Some(job_frame_start), Some(job_frame_end)),
        "all" => {
            let all_frames = collect_rendered_frames(&path, &format, None, None);
            (
                all_frames.first().map(|(frame, _)| *frame),
                all_frames.last().map(|(frame, _)| *frame),
            )
        }
        "custom" => {
            let start = custom_start.ok_or_else(|| String::from("缺少自定义起始帧"))?;
            let end = custom_end.ok_or_else(|| String::from("缺少自定义结束帧"))?;
            if start > end {
                return Err(String::from("自定义起始帧必须小于或等于结束帧"));
            }
            (Some(start), Some(end))
        }
        _ => return Err(String::from("无效的导出范围")),
    };

    Ok(build_mp4_export_inspection(
        &path,
        &format,
        selected_range.0,
        selected_range.1,
    ))
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
        blend_inspect_timeout_seconds(&state),
    )
    .await
    .map_err(|error| error.to_string())
}

fn collect_sequence_frames(
    output_path: &std::path::Path,
    format: &str,
    frame_start: i32,
    frame_end: i32,
) -> Vec<PathBuf> {
    collect_rendered_frames(output_path, format, Some(frame_start), Some(frame_end))
        .into_iter()
        .map(|(_, path)| path)
        .collect()
}

fn sanitize_file_name(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c if c.is_whitespace() => '_',
            c => c,
        })
        .collect::<String>();

    let sanitized = sanitized.trim_matches(|ch: char| ch == '_' || ch == '.' || ch == '-');
    if sanitized.is_empty() {
        String::from("render")
    } else {
        sanitized.to_string()
    }
}

fn derive_mp4_path(
    blend_file: &std::path::Path,
    output_path: &std::path::Path,
    frame_start: i32,
    frame_end: i32,
) -> PathBuf {
    let dir = if output_path.is_dir() {
        output_path.to_path_buf()
    } else {
        output_path
            .parent()
            .map(|parent| parent.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    };

    let base_name = sanitize_file_name(
        blend_file
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("render"),
    );
    let timestamp = Local::now().format("%Y%m%d-%H%M%S");

    dir.join(format!("{base_name}_{timestamp}_{frame_start}-{frame_end}.mp4"))
}

fn write_ffmpeg_concat_index(
    index_path: &std::path::Path,
    files: &[PathBuf],
    fps: f32,
) -> Result<(), String> {
    if files.is_empty() {
        return Err("No input frames found.".into());
    }

    let frame_duration = 1.0f64 / (fps.max(0.001) as f64);
    let mut lines = String::new();

    for file in files {
        let escaped = file
            .to_string_lossy()
            .replace('\'', "\\'")
            .replace('\n', "")
            .replace('\r', "");
        lines.push_str(&format!("file '{}'\n", escaped));
        lines.push_str(&format!("duration {:.12}\n", frame_duration));
    }

    if let Some(last) = files.last() {
        let escaped = last
            .to_string_lossy()
            .replace('\'', "\\'")
            .replace('\n', "")
            .replace('\r', "");
        lines.push_str(&format!("file '{}'\n", escaped));
    }

    std::fs::write(index_path, lines).map_err(|error| error.to_string())
}

async fn emit_mp4_stream<R>(
    reader: R,
    app: tauri::AppHandle,
    job_id: String,
    log_file_path: std::path::PathBuf,
    log_write_lock: std::sync::Arc<tokio::sync::Mutex<()>>,
) -> Vec<String>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut collected = Vec::new();
    let mut lines = BufReader::new(reader).lines();

    while let Ok(Some(line)) = lines.next_line().await {
        let _ = app.emit(
            "mp4-log",
            Mp4LogEvent {
                job_id: job_id.clone(),
                line: line.clone(),
            },
        );
        {
            let _guard = log_write_lock.lock().await;
            let _ = crate::app_paths::append_log_line(&log_file_path, &line);
        }
        collected.push(line);
    }

    collected
}

async fn emit_mp4_log_line(
    app: &AppHandle,
    job_id: &str,
    log_file_path: &Path,
    log_write_lock: &std::sync::Arc<tokio::sync::Mutex<()>>,
    line: impl Into<String>,
) {
    let line = line.into();
    let _ = app.emit(
        "mp4-log",
        Mp4LogEvent {
            job_id: job_id.to_string(),
            line: line.clone(),
        },
    );
    let _guard = log_write_lock.lock().await;
    let _ = crate::app_paths::append_log_line(log_file_path, &line);
}

#[tauri::command]
pub async fn encode_sequence_to_mp4(
    job_id: String,
    blender_executable: String,
    blend_file: String,
    output_path: String,
    format: String,
    frame_start: i32,
    frame_end: i32,
    strict_contiguous: bool,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Mp4ExportResult, String> {
    let blender_path = PathBuf::from(&blender_executable);
    if !blender_path.exists() {
        return Err(format!(
            "Blender executable not found: {blender_executable}"
        ));
    }

    if frame_start > frame_end {
        return Err(String::from("frameStart must be <= frameEnd"));
    }

    let output_template = PathBuf::from(&output_path);
    let inspection = build_mp4_export_inspection(
        &output_template,
        &format,
        Some(frame_start),
        Some(frame_end),
    );
    if strict_contiguous && inspection.has_gaps {
        let details = if inspection.missing_segments.is_empty() {
            String::new()
        } else {
            format!(" 缺失帧: {}", inspection.missing_segments.join(", "))
        };
        return Err(format!(
            "检测到导出范围内存在缺帧（{} 帧缺失）。{}",
            inspection.missing_count,
            details.trim()
        ));
    }

    let frames = collect_sequence_frames(&output_template, &format, frame_start, frame_end);
    if frames.is_empty() {
        return Err("No rendered frames found in the output directory.".into());
    }

    let fps = if PathBuf::from(&blend_file).exists() {
        inspect_project_with_timeout(
            &blender_path,
            &PathBuf::from(&blend_file),
            blend_inspect_timeout_seconds(&state),
        )
            .await
            .map(|settings| if settings.fps > 0.0 { settings.fps } else { 24.0 })
            .unwrap_or(24.0)
    } else {
        24.0
    };

    let app_settings = crate::commands::settings::get_settings(app.clone()).unwrap_or_default();
    let configured_ffmpeg = if app_settings.ffmpeg_executable.trim().is_empty() {
        None
    } else {
        Some(PathBuf::from(app_settings.ffmpeg_executable.trim()))
    };

    let ffmpeg_lookup =
        crate::blender::ffmpeg::find_ffmpeg_executable(
            Some(&app),
            configured_ffmpeg.as_deref(),
            &blender_path,
        );
    let ffmpeg_source = ffmpeg_lookup.source;
    let ffmpeg_executable = match ffmpeg_lookup.executable {
        Some(path) => path,
        None => {
            let message = if let Some(configured) = configured_ffmpeg.as_ref() {
                format!(
                    "未找到可用的 FFmpeg。当前设置路径不存在或不可用：{}。请前往设置页重新指定 FFmpeg 可执行文件。",
                    configured.display()
                )
            } else {
                String::from(
                    "未找到可用的 FFmpeg。请前往设置页指定 FFmpeg 可执行文件。",
                )
            };
            let _ = app.emit(
                "mp4-log",
                Mp4LogEvent {
                    job_id: job_id.clone(),
                    line: format!("[ffmpeg] {message}"),
                },
            );
            return Err(message);
        }
    };

    let output_video = derive_mp4_path(
        &PathBuf::from(&blend_file),
        &output_template,
        frame_start,
        frame_end,
    );
    let (job_number, resolved_job_id) = sqlx::query_as::<_, (i32, String)>("SELECT job_number, id FROM jobs WHERE id = ?")
        .bind(&job_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|error| error.to_string())?;
    let mp4_log_file_path = crate::app_paths::create_job_log_file(
        job_number,
        &resolved_job_id,
        crate::app_paths::FFMPEG_LOG_KIND,
    )
    .map_err(|error| error.to_string())?;
    let mp4_log_write_lock = std::sync::Arc::new(tokio::sync::Mutex::new(()));

    if let Some(parent) = output_video.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let temp_root = std::env::temp_dir().join(format!("sik-render-mp4-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_root).map_err(|error| error.to_string())?;
    let _temp_root_guard = TempDirGuard(temp_root.clone());
    let concat_index_path = temp_root.join("ffmpeg-input.txt");
    write_ffmpeg_concat_index(&concat_index_path, &frames, fps)?;

    emit_mp4_log_line(
        &app,
        &job_id,
        &mp4_log_file_path,
        &mp4_log_write_lock,
        format!("[ffmpeg] executable: {}", ffmpeg_executable.display()),
    )
    .await;
    if let Some(source) = ffmpeg_source {
        emit_mp4_log_line(
            &app,
            &job_id,
            &mp4_log_file_path,
            &mp4_log_write_lock,
            format!("[ffmpeg] source: {source}"),
        )
        .await;
    }
    if let Some(configured) = configured_ffmpeg.as_ref() {
        emit_mp4_log_line(
            &app,
            &job_id,
            &mp4_log_file_path,
            &mp4_log_write_lock,
            format!("[ffmpeg] configured path: {}", configured.display()),
        )
        .await;
    }
    emit_mp4_log_line(
        &app,
        &job_id,
        &mp4_log_file_path,
        &mp4_log_write_lock,
        format!("[ffmpeg] output: {}", output_video.display()),
    )
    .await;
    emit_mp4_log_line(
        &app,
        &job_id,
        &mp4_log_file_path,
        &mp4_log_write_lock,
        format!(
            "[ffmpeg] target fps: {:.3} (FFmpeg input log may still show a default 25 fps stream)",
            fps
        ),
    )
    .await;

    let mut child = crate::blender::ffmpeg::concat_to_mp4_command(
        &ffmpeg_executable,
        &concat_index_path,
        fps,
        &output_video,
    )
    .into_tokio_command();
    child.stdout(std::process::Stdio::piped());
    child.stderr(std::process::Stdio::piped());
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    child.process_group(0);

    let mut child = child
        .spawn()
        .map_err(|error| format!("Failed to launch ffmpeg: {error}"))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| String::from("Failed to capture ffmpeg stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| String::from("Failed to capture ffmpeg stderr"))?;

    if let Some(pid) = child.id() {
        state
            .active_mp4_exports
            .lock()
            .await
            .insert(job_id.clone(), pid);
    }

    let stdout_task = tokio::spawn(emit_mp4_stream(
        stdout,
        app.clone(),
        job_id.clone(),
        mp4_log_file_path.clone(),
        mp4_log_write_lock.clone(),
    ));
    let stderr_task = tokio::spawn(emit_mp4_stream(
        stderr,
        app.clone(),
        job_id.clone(),
        mp4_log_file_path.clone(),
        mp4_log_write_lock.clone(),
    ));
    let status = child
        .wait()
        .await
        .map_err(|error| format!("Failed to wait for ffmpeg: {error}"))?;
    state.active_mp4_exports.lock().await.remove(&job_id);
    let was_cancelled = state.cancelled_mp4_exports.lock().await.remove(&job_id);

    let mut output_lines = stdout_task.await.unwrap_or_default();
    output_lines.extend(stderr_task.await.unwrap_or_default());

    if was_cancelled {
        let message = "MP4 export cancelled";
        emit_mp4_log_line(
            &app,
            &job_id,
            &mp4_log_file_path,
            &mp4_log_write_lock,
            format!("[ffmpeg] {message}"),
        )
        .await;
        return Err(message.into());
    }

    if !status.success() {
        let details = output_lines
            .iter()
            .rev()
            .take(24)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        let message = format!("MP4 export failed: {details}");
        emit_mp4_log_line(
            &app,
            &job_id,
            &mp4_log_file_path,
            &mp4_log_write_lock,
            format!("[ffmpeg] {message}"),
        )
        .await;
        return Err(message);
    }

    emit_mp4_log_line(
        &app,
        &job_id,
        &mp4_log_file_path,
        &mp4_log_write_lock,
        format!(
            "[ffmpeg] export completed: {} frames -> {} ({:.3} fps)",
            frames.len(),
            output_video.display(),
            fps
        ),
    )
    .await;

    Ok(Mp4ExportResult {
        output_path: output_video.to_string_lossy().to_string(),
        fps,
        frame_count: frames.len() as u32,
    })
}

#[tauri::command]
pub async fn cancel_mp4_export(
    job_id: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let pid = {
        let active = state.active_mp4_exports.lock().await;
        active.get(&job_id).copied()
    };

    if let Some(pid) = pid {
        state.cancelled_mp4_exports.lock().await.insert(job_id.clone());
        let _ = app.emit(
            "mp4-log",
            Mp4LogEvent {
                job_id: job_id.clone(),
                line: "[ffmpeg] cancellation requested".into(),
            },
        );
        if let Ok((job_number, resolved_job_id)) = sqlx::query_as::<_, (i32, String)>("SELECT job_number, id FROM jobs WHERE id = ?")
            .bind(&job_id)
            .fetch_one(&state.pool)
            .await
        {
            if let Ok(path) = crate::app_paths::job_logs_dir(job_number, &resolved_job_id) {
                let mut files = std::fs::read_dir(&path)
                    .ok()
                    .into_iter()
                    .flatten()
                    .filter_map(|entry| entry.ok())
                    .map(|entry| entry.path())
                    .filter(|path| {
                        path.file_name()
                            .and_then(|name| name.to_str())
                            .map(|name| name.starts_with("ffmpeg-") && name.ends_with(".log"))
                            .unwrap_or(false)
                    })
                    .collect::<Vec<_>>();
                files.sort();
                if let Some(last) = files.last() {
                    let _ = crate::app_paths::append_log_line(last, "[ffmpeg] cancellation requested");
                }
            }
        }
        let _ = AppState::kill_process_tree(pid);
    }

    Ok(())
}
