use crate::commands::settings::AppSettings;
use crate::path_template::{
    blend_file_name_from_path, default_context, folder_name_from_source_path, resolve_output_path,
    PathKind,
};
use crate::queue::ffmpeg_job::{DbFfmpegJob, FfmpegJob, FfmpegJobSourceType, FfmpegJobStatus};
use crate::queue::job::{JobStatus, RenderJob};
use crate::state::AppState;
use chrono::{SecondsFormat, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::Mutex;

const IMAGE_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "exr", "tif", "tiff", "tga", "bmp", "hdr", "webp",
];

#[derive(Debug, Deserialize)]
pub struct AddFfmpegJobPayload {
    pub name: String,
    pub source_type: String,
    pub source_blender_job_id: Option<String>,
    pub input_path: String,
    pub frame_start: i32,
    pub frame_end: i32,
    pub fps: f32,
    pub output_path: String,
    pub crf: u32,
    pub preset: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscodeProgressEvent {
    pub job_id: String,
    pub frame: u32,
    pub total_frames: u32,
    pub encode_speed: Option<f32>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscodeLogEvent {
    pub job_id: String,
    pub line: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FfmpegJobUpdatedEvent {
    pub job: FfmpegJob,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderFrameGroup {
    pub name: String,
    pub input_path: String,
    pub frame_start: i32,
    pub frame_end: i32,
    pub frame_count: i32,
    pub detected_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderFrameGroups {
    pub groups: Vec<FolderFrameGroup>,
}

#[derive(Debug, Clone)]
struct TempDirGuard(PathBuf);

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[derive(Debug, Clone)]
struct FrameFile {
    frame: i32,
    path: PathBuf,
}

#[derive(Debug, Clone, Default)]
struct FfmpegProgress {
    frame: Option<u32>,
    speed: Option<f32>,
}

#[derive(Serialize)]
struct BlenderJobToml {
    job: BlenderTomlJobSection,
    source: BlenderTomlSourceSection,
    output: BlenderTomlOutputSection,
    result: BlenderTomlResultSection,
    timing: TimingSection,
}

#[derive(Serialize)]
struct BlenderTomlJobSection {
    id: String,
    number: i32,
    name: String,
    note: String,
}

#[derive(Serialize)]
struct BlenderTomlSourceSection {
    blend_file: String,
    blender: String,
}

#[derive(Serialize)]
struct BlenderTomlOutputSection {
    path: String,
    format: String,
    render_mode: String,
    frame_start: i32,
    frame_end: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    fps: Option<f32>,
}

#[derive(Serialize)]
struct BlenderTomlResultSection {
    status: JobStatus,
    frames_rendered: i32,
    total_frames: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_rendered_frame: Option<i32>,
    crash_count: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    preview_width: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    preview_height: Option<i32>,
}

#[derive(Serialize)]
struct FfmpegJobToml {
    job: FfmpegTomlJobSection,
    input: FfmpegTomlInputSection,
    output: FfmpegTomlOutputSection,
    encoding: FfmpegTomlEncodingSection,
    result: FfmpegTomlResultSection,
    timing: TimingSection,
}

#[derive(Serialize)]
struct FfmpegTomlJobSection {
    id: String,
    number: i32,
    name: String,
    source_type: FfmpegJobSourceType,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_blender_job_id: Option<String>,
}

#[derive(Serialize)]
struct FfmpegTomlInputSection {
    path: String,
    frame_start: i32,
    frame_end: i32,
    fps: f32,
}

#[derive(Serialize)]
struct FfmpegTomlOutputSection {
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    size_bytes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    duration_secs: Option<f32>,
}

#[derive(Serialize)]
struct FfmpegTomlEncodingSection {
    crf: u32,
    preset: String,
}

#[derive(Serialize)]
struct FfmpegTomlResultSection {
    status: FfmpegJobStatus,
}

#[derive(Serialize)]
struct TimingSection {
    created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    finished_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    duration_secs: Option<i64>,
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

fn effective_render_job_transcode_range(job: &RenderJob) -> (i32, i32) {
    match (
        job.transcode_frame_start_override,
        job.transcode_frame_end_override,
    ) {
        (Some(start), Some(end)) if start <= end => (start, end),
        _ => (job.frame_start, job.frame_end),
    }
}

pub fn default_output_path_for_render_job(job: &RenderJob, settings: &AppSettings) -> PathBuf {
    let (frame_start, frame_end) = effective_render_job_transcode_range(job);
    if let Ok(path) = resolve_output_path(
        &settings.blender_transcode_output_path_template,
        &default_context(
            PathKind::BlenderFfmpeg,
            job.blend_file.parent().map(|value| value.to_path_buf()),
            blend_file_name_from_path(&job.blend_file),
            None,
            frame_start,
            frame_end,
        ),
    ) {
        return path;
    }

    let dir = if job.output_path.is_dir() {
        job.output_path.clone()
    } else {
        job.output_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    };

    dir.join(format!("{}.mp4", sanitize_file_name(&job.name)))
}

pub fn build_ffmpeg_payload_for_render_job(
    job: &RenderJob,
    settings: Option<&AppSettings>,
) -> AddFfmpegJobPayload {
    let settings = settings.cloned().unwrap_or_default();
    let (frame_start, frame_end) = effective_render_job_transcode_range(job);
    let output_path = job
        .transcode_output_path_override
        .clone()
        .unwrap_or_else(|| default_output_path_for_render_job(job, &settings))
        .to_string_lossy()
        .to_string();

    AddFfmpegJobPayload {
        name: job
            .transcode_name_override
            .clone()
            .unwrap_or_else(|| job.name.clone()),
        source_type: String::from("blender_job"),
        source_blender_job_id: Some(job.id.clone()),
        input_path: job.output_path.to_string_lossy().to_string(),
        frame_start,
        frame_end,
        fps: job
            .transcode_fps_override
            .or(job.fps)
            .unwrap_or(30.0)
            .max(0.001),
        output_path,
        crf: job
            .transcode_crf_override
            .map(|value| value.clamp(0, 51) as u32)
            .unwrap_or(settings.transcode_crf.min(51)),
        preset: normalize_preset(
            job.transcode_preset_override
                .as_deref()
                .unwrap_or(&settings.transcode_preset),
        ),
    }
}

pub fn normalize_preset(preset: &str) -> String {
    match preset {
        "ultrafast" | "superfast" | "veryfast" | "faster" | "fast" | "medium" | "slow"
        | "slower" | "veryslow" => preset.to_string(),
        _ => String::from("medium"),
    }
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

#[tauri::command]
pub fn scan_folder_frame_groups(folder_path: String) -> Result<FolderFrameGroups, String> {
    let dir = PathBuf::from(&folder_path);
    if !dir.is_dir() {
        return Err(format!("{} is not a directory", dir.display()));
    }

    let mut groups: HashMap<(String, String, usize), Vec<i32>> = HashMap::new();
    let entries = std::fs::read_dir(&dir).map_err(|error| error.to_string())?;

    for entry in entries.flatten() {
        let path = entry.path();
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

        let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };

        let digits_len = stem
            .chars()
            .rev()
            .take_while(|ch| ch.is_ascii_digit())
            .count();
        if digits_len == 0 {
            continue;
        }

        let frame_start_index = stem.len() - digits_len;
        let frame = match stem[frame_start_index..].parse::<i32>() {
            Ok(value) => value,
            Err(_) => continue,
        };

        let raw_prefix = stem[..frame_start_index].to_string();
        let name = raw_prefix
            .trim_end_matches(|ch: char| ch == '_' || ch == '-')
            .to_string();
        if name.is_empty() {
            continue;
        }

        groups
            .entry((raw_prefix, ext, digits_len))
            .or_default()
            .push(frame);
    }

    let mut result = groups
        .into_iter()
        .filter_map(|((raw_prefix, ext, digits_len), mut frames)| {
            if frames.is_empty() {
                return None;
            }
            frames.sort_unstable();
            frames.dedup();
            let frame_start = *frames.first()?;
            let frame_end = *frames.last()?;
            let name = raw_prefix
                .trim_end_matches(|ch: char| ch == '_' || ch == '-')
                .to_string();
            let input_path = dir.join(format!(
                "{}{}.{}",
                raw_prefix,
                "#".repeat(digits_len),
                ext
            ));

            Some(FolderFrameGroup {
                name,
                input_path: input_path.to_string_lossy().to_string(),
                frame_start,
                frame_end,
                frame_count: frames.len() as i32,
                detected_format: ext_to_format(&ext),
            })
        })
        .collect::<Vec<_>>();

    result.sort_by(|a, b| a.frame_start.cmp(&b.frame_start).then(a.name.cmp(&b.name)));

    Ok(FolderFrameGroups { groups: result })
}

fn output_template_hints(output_path: &Path) -> (String, String) {
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

    (prefix_hint, suffix_hint)
}

fn inspect_folder_frame_files(
    dir: &Path,
    format_hint: Option<&str>,
) -> Result<(Vec<FrameFile>, Option<String>), String> {
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

        frames.push(FrameFile { frame, path });
    }

    frames.sort_by(|a, b| a.frame.cmp(&b.frame).then_with(|| a.path.cmp(&b.path)));
    frames.dedup_by(|a, b| a.frame == b.frame);

    Ok((frames, detected_format))
}

fn collect_template_frame_files(
    output_path: &Path,
    frame_start: i32,
    frame_end: i32,
) -> Result<(Vec<FrameFile>, Option<String>), String> {
    let dir = output_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(output_path);
    let ext_hint = output_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|value| value.to_ascii_lowercase());
    let (prefix_hint, suffix_hint) = output_template_hints(output_path);

    let mut frames = std::fs::read_dir(dir)
        .map_err(|error| error.to_string())?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            let Some(ext) = path
                .extension()
                .and_then(|value| value.to_str())
                .map(|value| value.to_ascii_lowercase())
            else {
                return false;
            };
            if !IMAGE_EXTS.contains(&ext.as_str()) {
                return false;
            }
            match ext_hint.as_deref() {
                Some(expected) => expected == ext,
                None => true,
            }
        })
        .filter(|path| {
            let stem = path.file_stem().and_then(|name| name.to_str()).unwrap_or_default();
            (prefix_hint.is_empty() || stem.starts_with(&prefix_hint))
                && (suffix_hint.is_empty() || stem.ends_with(&suffix_hint))
        })
        .filter_map(|path| {
            let frame = crate::blender::frame_path::trailing_frame_number(&path)?;
            if frame < frame_start || frame > frame_end {
                return None;
            }
            Some(FrameFile { frame, path })
        })
        .collect::<Vec<_>>();

    frames.sort_by(|a, b| a.frame.cmp(&b.frame).then_with(|| a.path.cmp(&b.path)));
    frames.dedup_by(|a, b| a.frame == b.frame);

    let detected_format = frames.first().and_then(|frame| {
        frame
            .path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| ext_to_format(&value.to_ascii_lowercase()))
    });

    Ok((frames, detected_format))
}

fn collect_input_frame_files(
    job: &FfmpegJob,
) -> Result<(Vec<FrameFile>, Option<String>), String> {
    if job.input_path.is_dir() {
        let (frames, detected_format) = inspect_folder_frame_files(&job.input_path, None)?;
        let selected = frames
            .into_iter()
            .filter(|frame| frame.frame >= job.frame_start && frame.frame <= job.frame_end)
            .collect::<Vec<_>>();
        return Ok((selected, detected_format));
    }

    if job
        .input_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.contains('#'))
        .unwrap_or(false)
    {
        return collect_template_frame_files(&job.input_path, job.frame_start, job.frame_end);
    }

    let dir = job
        .input_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .ok_or_else(|| format!("{} is not a valid frame source", job.input_path.display()))?;
    let format_hint = job
        .input_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext_to_format(&ext.to_ascii_lowercase()));
    let (frames, detected_format) = inspect_folder_frame_files(dir, format_hint.as_deref())?;
    let selected = frames
        .into_iter()
        .filter(|frame| frame.frame >= job.frame_start && frame.frame <= job.frame_end)
        .collect::<Vec<_>>();
    Ok((selected, detected_format.or(format_hint)))
}

fn write_ffmpeg_concat_index(index_path: &Path, files: &[PathBuf], fps: f32) -> Result<(), String> {
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

fn to_rfc3339(timestamp_ms: i64) -> String {
    chrono::DateTime::<Utc>::from_timestamp_millis(timestamp_ms)
        .unwrap_or_else(Utc::now)
        .to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn duration_secs(started_at: Option<i64>, finished_at: Option<i64>) -> Option<i64> {
    match (started_at, finished_at) {
        (Some(started_at), Some(finished_at)) if finished_at >= started_at => {
            Some((finished_at - started_at) / 1000)
        }
        _ => None,
    }
}

pub fn write_blender_job_toml(job: &RenderJob) -> anyhow::Result<()> {
    let dir = crate::app_paths::job_logs_dir(job.job_number, &job.id)?;
    let snapshot = BlenderJobToml {
        job: BlenderTomlJobSection {
            id: job.id.clone(),
            number: job.job_number,
            name: job.name.clone(),
            note: job.note.clone().unwrap_or_default(),
        },
        source: BlenderTomlSourceSection {
            blend_file: job.blend_file.to_string_lossy().to_string(),
            blender: job.blender_executable.to_string_lossy().to_string(),
        },
        output: BlenderTomlOutputSection {
            path: job.output_path.to_string_lossy().to_string(),
            format: job.output_format.clone(),
            render_mode: match job.render_mode {
                crate::queue::job::RenderMode::ImageSequence => String::from("image_sequence"),
                crate::queue::job::RenderMode::QuickMp4 => String::from("quick_mp4"),
            },
            frame_start: job.frame_start,
            frame_end: job.frame_end,
            fps: job.fps,
        },
        result: BlenderTomlResultSection {
            status: job.status.clone(),
            frames_rendered: job.current_frame.unwrap_or_default(),
            total_frames: job.total_frames.unwrap_or_else(|| job.total_frames()),
            last_rendered_frame: job.last_rendered_frame,
            crash_count: job.crash_count,
            preview_width: job.preview_width,
            preview_height: job.preview_height,
        },
        timing: TimingSection {
            created_at: to_rfc3339(job.created_at),
            started_at: job.started_at.map(to_rfc3339),
            finished_at: job.finished_at.map(to_rfc3339),
            duration_secs: duration_secs(job.started_at, job.finished_at),
        },
    };

    let contents = toml::to_string_pretty(&snapshot)?;
    crate::app_paths::write_job_toml(&dir, &contents)?;
    Ok(())
}

pub fn write_ffmpeg_job_toml(job: &FfmpegJob) -> anyhow::Result<()> {
    let dir = crate::app_paths::ffmpeg_job_logs_dir(job.job_number, &job.id)?;
    let snapshot = FfmpegJobToml {
        job: FfmpegTomlJobSection {
            id: job.id.clone(),
            number: job.job_number,
            name: job.name.clone(),
            source_type: job.source_type.clone(),
            source_blender_job_id: job.source_blender_job_id.clone(),
        },
        input: FfmpegTomlInputSection {
            path: job.input_path.to_string_lossy().to_string(),
            frame_start: job.frame_start,
            frame_end: job.frame_end,
            fps: job.fps,
        },
        output: FfmpegTomlOutputSection {
            path: job.output_path.to_string_lossy().to_string(),
            size_bytes: job.output_size_bytes,
            duration_secs: job.output_duration_secs,
        },
        encoding: FfmpegTomlEncodingSection {
            crf: job.crf,
            preset: job.preset.clone(),
        },
        result: FfmpegTomlResultSection {
            status: job.status.clone(),
        },
        timing: TimingSection {
            created_at: to_rfc3339(job.created_at),
            started_at: job.started_at.map(to_rfc3339),
            finished_at: job.finished_at.map(to_rfc3339),
            duration_secs: duration_secs(job.started_at, job.finished_at),
        },
    };

    let contents = toml::to_string_pretty(&snapshot)?;
    crate::app_paths::write_job_toml(&dir, &contents)?;
    Ok(())
}

fn parse_source_type(value: &str) -> Result<FfmpegJobSourceType, String> {
    match value {
        "blender_job" => Ok(FfmpegJobSourceType::BlenderJob),
        "folder" => Ok(FfmpegJobSourceType::Folder),
        _ => Err(format!("unsupported source_type: {value}")),
    }
}

async fn next_ffmpeg_job_priority<'e, E>(executor: E) -> Result<i32, sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let next =
        sqlx::query_scalar::<_, i32>("SELECT COALESCE(MAX(priority), 0) + 1 FROM ffmpeg_jobs")
            .fetch_one(executor)
            .await?;

    Ok(next.max(1))
}

pub async fn load_ffmpeg_job(pool: &sqlx::SqlitePool, id: &str) -> anyhow::Result<FfmpegJob> {
    let job = sqlx::query_as::<_, DbFfmpegJob>(
        r#"
        SELECT
            id,
            job_number,
            name,
            source_type,
            source_blender_job_id,
            input_path,
            frame_start,
            frame_end,
            fps,
            output_path,
            crf,
            preset,
            status,
            priority,
            created_at,
            started_at,
            finished_at,
            progress_frame,
            total_frames,
            output_size_bytes,
            output_duration_secs
        FROM ffmpeg_jobs
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(job.into())
}

async fn fetch_ffmpeg_jobs(pool: &sqlx::SqlitePool) -> Result<Vec<FfmpegJob>, sqlx::Error> {
    sqlx::query_as::<_, DbFfmpegJob>(
        r#"
        SELECT
            id,
            job_number,
            name,
            source_type,
            source_blender_job_id,
            input_path,
            frame_start,
            frame_end,
            fps,
            output_path,
            crf,
            preset,
            status,
            priority,
            created_at,
            started_at,
            finished_at,
            progress_frame,
            total_frames,
            output_size_bytes,
            output_duration_secs
        FROM ffmpeg_jobs
        ORDER BY priority ASC, created_at ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .map(|rows| rows.into_iter().map(FfmpegJob::from).collect())
}

pub fn emit_ffmpeg_job_update(app: &AppHandle, job: &FfmpegJob) {
    let _ = app.emit(
        "ffmpeg-job-updated",
        FfmpegJobUpdatedEvent { job: job.clone() },
    );
}

async fn append_ffmpeg_log_line(
    app: &AppHandle,
    job_id: &str,
    log_file_path: &Path,
    log_write_lock: &Arc<Mutex<()>>,
    line: impl Into<String>,
) {
    let line = line.into();
    let rendered_line = crate::app_paths::timestamped_log_line(&line);
    let _ = app.emit(
        "transcode-log",
        TranscodeLogEvent {
            job_id: job_id.to_string(),
            line: rendered_line.clone(),
        },
    );
    let _guard = log_write_lock.lock().await;
    let _ = crate::app_paths::append_log_line(log_file_path, &rendered_line);
}

fn parse_ffmpeg_progress(line: &str) -> Option<FfmpegProgress> {
    let frame_re = Regex::new(r"frame=\s*(\d+)").ok()?;
    let speed_re = Regex::new(r"speed=\s*([0-9]*\.?[0-9]+)x").ok()?;

    let frame = frame_re
        .captures(line)
        .and_then(|captures| captures.get(1))
        .and_then(|value| value.as_str().parse::<u32>().ok());
    let speed = speed_re
        .captures(line)
        .and_then(|captures| captures.get(1))
        .and_then(|value| value.as_str().parse::<f32>().ok());

    if frame.is_none() && speed.is_none() {
        None
    } else {
        Some(FfmpegProgress { frame, speed })
    }
}

async fn persist_ffmpeg_progress(
    state: &AppState,
    job_id: &str,
    frame: Option<u32>,
    total_frames: u32,
) {
    let _ = sqlx::query(
        "UPDATE ffmpeg_jobs SET progress_frame = COALESCE(?, progress_frame), total_frames = ? WHERE id = ?",
    )
    .bind(frame.map(|value| value as i32))
    .bind(total_frames as i32)
    .bind(job_id)
    .execute(&state.pool)
    .await;
}

async fn resolve_blender_probe(state: &AppState, job: &FfmpegJob) -> PathBuf {
    if let Some(blender_job_id) = &job.source_blender_job_id {
        if let Ok(Some(path)) = sqlx::query_scalar::<_, String>(
            "SELECT blender_exec FROM jobs WHERE id = ?",
        )
        .bind(blender_job_id)
        .fetch_optional(&state.pool)
        .await
        {
            return PathBuf::from(path);
        }
    }

    state
        .cached_settings()
        .map(|settings| {
            if settings.default_blender.trim().is_empty() {
                std::env::current_exe().unwrap_or_else(|_| PathBuf::from("ffmpeg"))
            } else {
                PathBuf::from(settings.default_blender.trim())
            }
        })
        .unwrap_or_else(|| std::env::current_exe().unwrap_or_else(|_| PathBuf::from("ffmpeg")))
}

async fn resolve_ffmpeg_executable(
    app: &AppHandle,
    state: &AppState,
    job: &FfmpegJob,
) -> Result<(PathBuf, Option<&'static str>, Option<PathBuf>), String> {
    let settings = state.cached_settings().unwrap_or_default();
    let configured_ffmpeg = if settings.ffmpeg_executable.trim().is_empty() {
        None
    } else {
        Some(PathBuf::from(settings.ffmpeg_executable.trim()))
    };
    let blender_probe = resolve_blender_probe(state, job).await;
    let lookup = crate::blender::ffmpeg::find_ffmpeg_executable(
        Some(app),
        configured_ffmpeg.as_deref(),
        &blender_probe,
    );

    match lookup.executable {
        Some(path) => Ok((path, lookup.source, configured_ffmpeg)),
        None => {
            let message = if let Some(configured) = configured_ffmpeg {
                format!(
                    "未找到可用的 FFmpeg。当前设置路径不存在或不可用：{}。请前往设置页重新指定 FFmpeg 可执行文件。",
                    configured.display()
                )
            } else {
                String::from("未找到可用的 FFmpeg。请前往设置页指定 FFmpeg 可执行文件。")
            };
            Err(message)
        }
    }
}

pub async fn insert_ffmpeg_job(
    state: &AppState,
    payload: AddFfmpegJobPayload,
) -> Result<FfmpegJob, String> {
    if payload.frame_start > payload.frame_end {
        return Err("frame_start must be <= frame_end".into());
    }
    if payload.fps <= 0.0 {
        return Err("fps must be greater than 0".into());
    }

    let source_type = parse_source_type(&payload.source_type)?;
    let input_path = PathBuf::from(payload.input_path.trim());
    let resolved_output_path = match source_type {
        FfmpegJobSourceType::BlenderJob => {
            let source_blender_job_id = payload
                .source_blender_job_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| String::from("source_blender_job_id is required for blender_job output templates"))?;
            let (blend_file, render_mode) = sqlx::query_as::<_, (String, crate::queue::job::RenderMode)>(
                "SELECT blend_file, render_mode FROM jobs WHERE id = ?",
            )
                .bind(source_blender_job_id)
                .fetch_optional(&state.pool)
                .await
                .map_err(|error| error.to_string())?
                .ok_or_else(|| format!("blender job {source_blender_job_id} was not found"))?;
            if render_mode.is_quick_mp4() {
                return Err("quick_mp4 render jobs cannot create FFmpeg jobs".into());
            }
            let blend_file = PathBuf::from(blend_file);
            resolve_output_path(
                payload.output_path.trim(),
                &default_context(
                    PathKind::BlenderFfmpeg,
                    blend_file.parent().map(|value| value.to_path_buf()),
                    blend_file_name_from_path(&blend_file),
                    None,
                    payload.frame_start,
                    payload.frame_end,
                ),
            )
            .map_err(|error| error.to_string())?
        }
        FfmpegJobSourceType::Folder => resolve_output_path(
            payload.output_path.trim(),
            &default_context(
                PathKind::StandaloneFfmpeg,
                Some(
                    if input_path.is_dir() {
                        input_path.clone()
                    } else {
                        input_path.parent().unwrap_or(&input_path).to_path_buf()
                    },
                ),
                None,
                folder_name_from_source_path(&input_path),
                payload.frame_start,
                payload.frame_end,
            ),
        )
        .map_err(|error| error.to_string())?,
    };

    let mut job = FfmpegJob::new(
        payload.name.trim().to_string(),
        source_type,
        payload.source_blender_job_id.and_then(|value| {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        }),
        input_path,
        payload.frame_start,
        payload.frame_end,
        payload.fps,
        resolved_output_path,
        payload.crf.min(51),
        normalize_preset(&payload.preset),
        0,
    );

    if job.name.is_empty() {
        return Err("name cannot be empty".into());
    }

    let mut tx = state.pool.begin().await.map_err(|error| error.to_string())?;
    let job_number: i64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(job_number), 0) + 1 FROM ffmpeg_jobs")
            .fetch_one(&mut *tx)
            .await
            .map_err(|error| error.to_string())?;
    job.job_number = job_number as i32;
    job.priority = next_ffmpeg_job_priority(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;

    sqlx::query(
        r#"
        INSERT INTO ffmpeg_jobs (
            id,
            job_number,
            name,
            source_type,
            source_blender_job_id,
            input_path,
            frame_start,
            frame_end,
            fps,
            output_path,
            crf,
            preset,
            status,
            priority,
            created_at,
            started_at,
            finished_at,
            progress_frame,
            total_frames,
            output_size_bytes,
            output_duration_secs
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&job.id)
    .bind(job.job_number)
    .bind(&job.name)
    .bind(job.source_type.clone())
    .bind(&job.source_blender_job_id)
    .bind(job.input_path.to_string_lossy().to_string())
    .bind(job.frame_start)
    .bind(job.frame_end)
    .bind(job.fps)
    .bind(job.output_path.to_string_lossy().to_string())
    .bind(job.crf as i32)
    .bind(&job.preset)
    .bind(FfmpegJobStatus::Pending)
    .bind(job.priority)
    .bind(job.created_at)
    .bind(job.started_at)
    .bind(job.finished_at)
    .bind(job.progress_frame)
    .bind(Some(job.total_frames()))
    .bind(job.output_size_bytes)
    .bind(job.output_duration_secs)
    .execute(&mut *tx)
    .await
    .map_err(|error| error.to_string())?;

    tx.commit().await.map_err(|error| error.to_string())?;

    Ok(job)
}

pub async fn enqueue_ffmpeg_job(
    app: &AppHandle,
    state: &AppState,
    payload: AddFfmpegJobPayload,
) -> Result<FfmpegJob, String> {
    let job = insert_ffmpeg_job(state, payload).await?;
    emit_ffmpeg_job_update(app, &job);
    state.ffmpeg_notify.notify_one();
    Ok(job)
}

pub async fn recover_running_ffmpeg_jobs(pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
    let jobs = sqlx::query_as::<_, (String, i32)>(
        "SELECT id, job_number FROM ffmpeg_jobs WHERE status = 'running'",
    )
    .fetch_all(pool)
    .await?;

    if jobs.is_empty() {
        return Ok(());
    }

    let line = "[cancelled] Reason: startup recovery. This ffmpeg job was still marked running when the app started, so the previous app session ended while transcoding was in progress.";
    for (job_id, job_number) in &jobs {
        let _ = crate::app_paths::append_ffmpeg_job_log_event(*job_number, job_id, line);
    }

    sqlx::query(
        "UPDATE ffmpeg_jobs SET status = 'cancelled', finished_at = ? WHERE status = 'running'",
    )
    .bind(Utc::now().timestamp_millis())
    .execute(pool)
    .await?;

    for (job_id, _) in jobs {
        if let Ok(job) = load_ffmpeg_job(pool, &job_id).await {
            let _ = write_ffmpeg_job_toml(&job);
        }
    }

    Ok(())
}

pub async fn finalize_ffmpeg_shutdown_jobs(state: &AppState) {
    let running_jobs = {
        let active = state.active_ffmpeg_jobs.lock().await;
        active
            .iter()
            .map(|(job_id, pid)| (job_id.clone(), *pid))
            .collect::<Vec<_>>()
    };

    let line = "[cancelled] Reason: app shutdown. The app closed while FFmpeg was transcoding, so this FFmpeg Job was cancelled.";
    for (job_id, pid) in &running_jobs {
        if let Ok(Some(job_number)) = sqlx::query_scalar::<_, i32>(
            "SELECT job_number FROM ffmpeg_jobs WHERE id = ?",
        )
        .bind(job_id)
        .fetch_optional(&state.pool)
        .await
        {
            let _ = crate::app_paths::append_ffmpeg_job_log_event(job_number, job_id, line);
        }
        let _ = AppState::kill_process_tree(*pid);
    }

    if !running_jobs.is_empty() {
        let ids = running_jobs.iter().map(|(job_id, _)| job_id.clone()).collect::<Vec<_>>();
        let placeholders = vec!["?"; ids.len()].join(", ");
        let query = format!(
            "UPDATE ffmpeg_jobs SET status = 'cancelled', finished_at = ? WHERE status = 'running' AND id IN ({placeholders})"
        );
        let mut sql = sqlx::query(&query).bind(Utc::now().timestamp_millis());
        for id in &ids {
            sql = sql.bind(id);
        }
        let _ = sql.execute(&state.pool).await;

        for id in ids {
            if let Ok(job) = load_ffmpeg_job(&state.pool, &id).await {
                let _ = write_ffmpeg_job_toml(&job);
            }
        }
    }
}

pub async fn run_ffmpeg_job(
    app: AppHandle,
    state: AppState,
    job: FfmpegJob,
) -> Result<FfmpegJobStatus, String> {
    let total_frames = job.total_frames().max(0) as u32;
    let (frames, detected_format) = collect_input_frame_files(&job)?;
    if frames.is_empty() {
        return Err(String::from("No matching frames found for this FFmpeg Job."));
    }

    if let Some(parent) = job.output_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let (ffmpeg_executable, ffmpeg_source, configured_ffmpeg) =
        resolve_ffmpeg_executable(&app, &state, &job).await?;
    let log_file_path = crate::app_paths::create_ffmpeg_job_log_file(job.job_number, &job.id)
        .map_err(|error| error.to_string())?;
    let log_write_lock = Arc::new(Mutex::new(()));

    let temp_root = std::env::temp_dir().join(format!("sik-ffmpeg-job-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_root).map_err(|error| error.to_string())?;
    let _temp_root_guard = TempDirGuard(temp_root.clone());
    let concat_index_path = temp_root.join("ffmpeg-input.txt");
    let files = frames.iter().map(|frame| frame.path.clone()).collect::<Vec<_>>();
    write_ffmpeg_concat_index(&concat_index_path, &files, job.fps)?;

    append_ffmpeg_log_line(
        &app,
        &job.id,
        &log_file_path,
        &log_write_lock,
        format!("[ffmpeg] executable: {}", ffmpeg_executable.display()),
    )
    .await;
    if let Some(source) = ffmpeg_source {
        append_ffmpeg_log_line(
            &app,
            &job.id,
            &log_file_path,
            &log_write_lock,
            format!("[ffmpeg] source: {source}"),
        )
        .await;
    }
    if let Some(configured) = configured_ffmpeg {
        append_ffmpeg_log_line(
            &app,
            &job.id,
            &log_file_path,
            &log_write_lock,
            format!("[ffmpeg] configured path: {}", configured.display()),
        )
        .await;
    }
    append_ffmpeg_log_line(
        &app,
        &job.id,
        &log_file_path,
        &log_write_lock,
        format!("[ffmpeg] output: {}", job.output_path.display()),
    )
    .await;
    append_ffmpeg_log_line(
        &app,
        &job.id,
        &log_file_path,
        &log_write_lock,
        format!("[ffmpeg] input: {}", job.input_path.display()),
    )
    .await;
    if let Some(format) = detected_format {
        append_ffmpeg_log_line(
            &app,
            &job.id,
            &log_file_path,
            &log_write_lock,
            format!("[ffmpeg] detected format: {format}"),
        )
        .await;
    }
    append_ffmpeg_log_line(
        &app,
        &job.id,
        &log_file_path,
        &log_write_lock,
        format!("[ffmpeg] preset: {}, crf: {}", job.preset, job.crf),
    )
    .await;

    let mut child = crate::blender::ffmpeg::concat_to_mp4_command(
        &ffmpeg_executable,
        &concat_index_path,
        job.fps,
        &job.output_path,
        job.crf,
        &job.preset,
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

    state.cancelled_ffmpeg_jobs.lock().await.remove(&job.id);
    if let Some(pid) = child.id() {
        state
            .active_ffmpeg_jobs
            .lock()
            .await
            .insert(job.id.clone(), pid);
    }

    persist_ffmpeg_progress(&state, &job.id, Some(0), total_frames).await;

    let progress_job_id = job.id.clone();
    let progress_state = state.clone();
    let progress_app = app.clone();
    let progress_log_file_path = log_file_path.clone();
    let progress_log_write_lock = log_write_lock.clone();
    let stdout_task = tokio::spawn(async move {
        let mut collected = Vec::new();
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            append_ffmpeg_log_line(
                &progress_app,
                &progress_job_id,
                &progress_log_file_path,
                &progress_log_write_lock,
                line.clone(),
            )
            .await;
            if let Some(progress) = parse_ffmpeg_progress(&line) {
                if let Some(frame) = progress.frame {
                    persist_ffmpeg_progress(
                        &progress_state,
                        &progress_job_id,
                        Some(frame.min(total_frames)),
                        total_frames,
                    )
                    .await;
                    let _ = progress_app.emit(
                        "transcode-progress",
                        TranscodeProgressEvent {
                            job_id: progress_job_id.clone(),
                            frame: frame.min(total_frames),
                            total_frames,
                            encode_speed: progress.speed,
                        },
                    );
                }
            }
            collected.push(line);
        }
        collected
    });

    let stderr_job_id = job.id.clone();
    let stderr_state = state.clone();
    let stderr_app = app.clone();
    let stderr_log_file_path = log_file_path.clone();
    let stderr_log_write_lock = log_write_lock.clone();
    let stderr_task = tokio::spawn(async move {
        let mut collected = Vec::new();
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            append_ffmpeg_log_line(
                &stderr_app,
                &stderr_job_id,
                &stderr_log_file_path,
                &stderr_log_write_lock,
                line.clone(),
            )
            .await;
            if let Some(progress) = parse_ffmpeg_progress(&line) {
                if let Some(frame) = progress.frame {
                    persist_ffmpeg_progress(
                        &stderr_state,
                        &stderr_job_id,
                        Some(frame.min(total_frames)),
                        total_frames,
                    )
                    .await;
                    let _ = stderr_app.emit(
                        "transcode-progress",
                        TranscodeProgressEvent {
                            job_id: stderr_job_id.clone(),
                            frame: frame.min(total_frames),
                            total_frames,
                            encode_speed: progress.speed,
                        },
                    );
                }
            }
            collected.push(line);
        }
        collected
    });

    let status = child
        .wait()
        .await
        .map_err(|error| format!("Failed to wait for ffmpeg: {error}"))?;
    state.active_ffmpeg_jobs.lock().await.remove(&job.id);
    let was_cancelled = state.cancelled_ffmpeg_jobs.lock().await.remove(&job.id);

    let mut output_lines = stdout_task.await.unwrap_or_default();
    output_lines.extend(stderr_task.await.unwrap_or_default());

    if was_cancelled {
        append_ffmpeg_log_line(
            &app,
            &job.id,
            &log_file_path,
            &log_write_lock,
            "[ffmpeg] FFmpeg Job cancelled",
        )
        .await;
        return Ok(FfmpegJobStatus::Cancelled);
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
        append_ffmpeg_log_line(
            &app,
            &job.id,
            &log_file_path,
            &log_write_lock,
            format!("[ffmpeg] FFmpeg Job failed: {details}"),
        )
        .await;
        return Ok(FfmpegJobStatus::Failed);
    }

    append_ffmpeg_log_line(
        &app,
        &job.id,
        &log_file_path,
        &log_write_lock,
        format!(
            "[ffmpeg] export completed: {} frames -> {} ({:.3} fps)",
            files.len(),
            job.output_path.display(),
            job.fps
        ),
    )
    .await;

    Ok(FfmpegJobStatus::Done)
}

#[tauri::command]
pub async fn list_ffmpeg_jobs(state: State<'_, AppState>) -> Result<Vec<FfmpegJob>, String> {
    fetch_ffmpeg_jobs(&state.pool)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_ffmpeg_job(
    id: String,
    state: State<'_, AppState>,
) -> Result<FfmpegJob, String> {
    load_ffmpeg_job(&state.pool, &id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn add_ffmpeg_job(
    payload: AddFfmpegJobPayload,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<FfmpegJob, String> {
    enqueue_ffmpeg_job(&app, state.inner(), payload).await
}

#[tauri::command]
pub async fn cancel_ffmpeg_job(
    id: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let status = sqlx::query_scalar::<_, FfmpegJobStatus>(
        "SELECT status FROM ffmpeg_jobs WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|error| error.to_string())?;

    match status {
        None => Ok(()),
        Some(FfmpegJobStatus::Pending) => {
            sqlx::query("UPDATE ffmpeg_jobs SET status = 'cancelled', finished_at = ? WHERE id = ?")
                .bind(Utc::now().timestamp_millis())
                .bind(&id)
                .execute(&state.pool)
                .await
                .map_err(|error| error.to_string())?;

            if let Ok(Some((job_number, resolved_job_id))) =
                sqlx::query_as::<_, (i32, String)>("SELECT job_number, id FROM ffmpeg_jobs WHERE id = ?")
                    .bind(&id)
                    .fetch_optional(&state.pool)
                    .await
            {
                let _ = crate::app_paths::append_ffmpeg_job_log_event(
                    job_number,
                    &resolved_job_id,
                    "[cancelled] Reason: manual stop. This FFmpeg Job was cancelled before execution started.",
                );
            }

            if let Ok(job) = load_ffmpeg_job(&state.pool, &id).await {
                let _ = write_ffmpeg_job_toml(&job);
                emit_ffmpeg_job_update(&app, &job);
            }
            Ok(())
        }
        Some(FfmpegJobStatus::Running) => {
            state.cancelled_ffmpeg_jobs.lock().await.insert(id.clone());
            if let Ok(Some((job_number, resolved_job_id))) =
                sqlx::query_as::<_, (i32, String)>("SELECT job_number, id FROM ffmpeg_jobs WHERE id = ?")
                    .bind(&id)
                    .fetch_optional(&state.pool)
                    .await
            {
                let _ = crate::app_paths::append_ffmpeg_job_log_event(
                    job_number,
                    &resolved_job_id,
                    "[cancelled] Reason: manual stop. Cancellation requested.",
                );
            }

            let pid = {
                let active = state.active_ffmpeg_jobs.lock().await;
                active.get(&id).copied()
            };
            if let Some(pid) = pid {
                let _ = AppState::kill_process_tree(pid);
            }
            Ok(())
        }
        Some(_) => Ok(()),
    }
}

#[tauri::command]
pub async fn delete_ffmpeg_job(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let row = sqlx::query_as::<_, (i32, String, FfmpegJobStatus)>(
        "SELECT job_number, id, status FROM ffmpeg_jobs WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|error| error.to_string())?;

    let Some((job_number, resolved_job_id, status)) = row else {
        return Ok(());
    };

    if status == FfmpegJobStatus::Running {
        return Err("cannot delete a running FFmpeg Job".into());
    }

    sqlx::query("DELETE FROM ffmpeg_jobs WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(|error| error.to_string())?;

    let target = crate::app_paths::ffmpeg_job_logs_dir(job_number, &resolved_job_id)
        .map_err(|error| error.to_string())?;
    if target.exists() {
        std::fs::remove_dir_all(&target).map_err(|error| error.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub async fn reorder_ffmpeg_jobs(
    ordered_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<FfmpegJob>, String> {
    let all_rows = sqlx::query_as::<_, (String, FfmpegJobStatus)>(
        "SELECT id, status FROM ffmpeg_jobs ORDER BY priority ASC, created_at ASC",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|error| error.to_string())?;

    let existing_ids = all_rows
        .iter()
        .filter(|(_, status)| *status != FfmpegJobStatus::Running)
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    let existing_set = existing_ids.iter().cloned().collect::<HashSet<_>>();
    let provided_set = ordered_ids.iter().cloned().collect::<HashSet<_>>();

    if existing_ids.len() != ordered_ids.len() || existing_set != provided_set {
        return Err("ffmpeg job order is out of date, please refresh and try again".into());
    }

    let mut ordered_iter = ordered_ids.into_iter();
    let final_order = all_rows
        .into_iter()
        .map(|(id, status)| {
            if status == FfmpegJobStatus::Running {
                Ok(id)
            } else {
                ordered_iter.next().ok_or_else(|| {
                    "ffmpeg job order is out of date, please refresh and try again".to_string()
                })
            }
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut tx = state.pool.begin().await.map_err(|error| error.to_string())?;
    for (index, id) in final_order.iter().enumerate() {
        sqlx::query("UPDATE ffmpeg_jobs SET priority = ? WHERE id = ?")
            .bind((index as i32) + 1)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(|error| error.to_string())?;
    }
    tx.commit().await.map_err(|error| error.to_string())?;

    state.ffmpeg_notify.notify_one();
    fetch_ffmpeg_jobs(&state.pool)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_ffmpeg_job_logs(
    job_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let (job_number, resolved_job_id) =
        sqlx::query_as::<_, (i32, String)>("SELECT job_number, id FROM ffmpeg_jobs WHERE id = ?")
            .bind(&job_id)
            .fetch_one(&state.pool)
            .await
            .map_err(|error| error.to_string())?;

    crate::app_paths::read_ffmpeg_job_log_lines(job_number, &resolved_job_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_ffmpeg_job_latest_logs(
    job_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let (job_number, resolved_job_id) =
        sqlx::query_as::<_, (i32, String)>("SELECT job_number, id FROM ffmpeg_jobs WHERE id = ?")
            .bind(&job_id)
            .fetch_one(&state.pool)
            .await
            .map_err(|error| error.to_string())?;

    crate::app_paths::read_latest_ffmpeg_job_log_lines(job_number, &resolved_job_id)
        .map_err(|error| error.to_string())
}
