use anyhow::{Context, Result};
use chrono::Local;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const BLENDER_LOG_KIND: &str = "blender";
pub const FFMPEG_LOG_KIND: &str = "ffmpeg";

const JOBS_ROOT_DIR_NAME: &str = "jobs";
const LEGACY_LOGS_DIR_NAME: &str = "logs";
const BLENDER_ROOT_DIR_NAME: &str = "blender";
const FFMPEG_ROOT_DIR_NAME: &str = "ffmpeg";
const LOG_DIR_NAME: &str = "log";
const JOB_TOML_FILE_NAME: &str = "job.toml";
const JOB_PREVIEW_FILE_NAME: &str = "preview.jpg";

pub fn tool_root_dir() -> Result<PathBuf> {
    if cfg!(debug_assertions) {
        return PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .map(Path::to_path_buf)
            .context("failed to resolve workspace root");
    }

    let exe = std::env::current_exe().context("failed to resolve current executable")?;
    exe.parent()
        .map(Path::to_path_buf)
        .context("failed to resolve executable directory")
}

pub fn config_path() -> Result<PathBuf> {
    Ok(tool_root_dir()?.join("sik-render.toml"))
}

pub fn logs_dir() -> Result<PathBuf> {
    Ok(tool_root_dir()?.join(LEGACY_LOGS_DIR_NAME))
}

pub fn jobs_root_dir() -> Result<PathBuf> {
    let dir = tool_root_dir()?.join(JOBS_ROOT_DIR_NAME);
    fs::create_dir_all(&dir).context("failed to create jobs directory")?;
    Ok(dir)
}

fn blender_jobs_root_dir() -> Result<PathBuf> {
    let dir = jobs_root_dir()?.join(BLENDER_ROOT_DIR_NAME);
    fs::create_dir_all(&dir).context("failed to create blender jobs directory")?;
    Ok(dir)
}

fn ffmpeg_jobs_root_dir() -> Result<PathBuf> {
    let dir = jobs_root_dir()?.join(FFMPEG_ROOT_DIR_NAME);
    fs::create_dir_all(&dir).context("failed to create ffmpeg jobs directory")?;
    Ok(dir)
}

pub fn ensure_runtime_layout() -> Result<()> {
    let _ = blender_jobs_root_dir()?;
    let _ = ffmpeg_jobs_root_dir()?;
    migrate_known_legacy_logs()?;
    Ok(())
}

pub fn ffmpeg_job_log_suffix(job_id: &str) -> String {
    job_log_suffix(job_id)
}

fn job_log_suffix(job_id: &str) -> String {
    let suffix = job_id
        .chars()
        .filter(|ch| ch.is_ascii_hexdigit())
        .take(4)
        .collect::<String>()
        .to_ascii_lowercase();

    if suffix.len() == 4 {
        suffix
    } else {
        String::from("xxxx")
    }
}

pub fn job_logs_dir_name(job_number: i32, job_id: &str) -> String {
    format!("blender_job_{job_number:04}_{}", job_log_suffix(job_id))
}

pub fn ffmpeg_job_logs_dir_name(job_number: i32, job_id: &str) -> String {
    format!("ffmpeg_job_{job_number:04}_{}", ffmpeg_job_log_suffix(job_id))
}

pub fn legacy_job_logs_dir(job_number: i32) -> Result<PathBuf> {
    Ok(logs_dir()?.join(format!("job-{job_number:04}")))
}

fn legacy_job_logs_dir_with_id(job_number: i32, job_id: &str) -> Result<PathBuf> {
    Ok(logs_dir()?.join(format!("job-{job_number:04}-{}", job_log_suffix(job_id))))
}

fn legacy_ffmpeg_job_logs_dir(job_number: i32, job_id: &str) -> Result<PathBuf> {
    Ok(logs_dir()?.join(format!(
        "ffmpeg-job-{job_number:04}-{}",
        ffmpeg_job_log_suffix(job_id),
    )))
}

fn blender_job_dir_path(job_number: i32, job_id: &str) -> Result<PathBuf> {
    Ok(blender_jobs_root_dir()?.join(job_logs_dir_name(job_number, job_id)))
}

fn ffmpeg_job_dir_path(job_number: i32, job_id: &str) -> Result<PathBuf> {
    Ok(ffmpeg_jobs_root_dir()?.join(ffmpeg_job_logs_dir_name(job_number, job_id)))
}

fn blender_job_dir_path_with_suffix(job_number: i32, suffix: &str) -> Result<PathBuf> {
    Ok(blender_jobs_root_dir()?.join(format!(
        "blender_job_{job_number:04}_{}",
        job_log_suffix(suffix),
    )))
}

fn ffmpeg_job_dir_path_with_suffix(job_number: i32, suffix: &str) -> Result<PathBuf> {
    Ok(ffmpeg_jobs_root_dir()?.join(format!(
        "ffmpeg_job_{job_number:04}_{}",
        job_log_suffix(suffix),
    )))
}

pub fn job_logs_dir(job_number: i32, job_id: &str) -> Result<PathBuf> {
    ensure_blender_job_layout(job_number, job_id)
}

pub fn ffmpeg_job_logs_dir(job_number: i32, job_id: &str) -> Result<PathBuf> {
    ensure_ffmpeg_job_layout(job_number, job_id)
}

pub fn job_log_dir(job_number: i32, job_id: &str) -> Result<PathBuf> {
    let dir = ensure_blender_job_layout(job_number, job_id)?.join(LOG_DIR_NAME);
    fs::create_dir_all(&dir).context("failed to create blender job log directory")?;
    Ok(dir)
}

pub fn job_preview_image_path(job_number: i32, job_id: &str) -> Result<PathBuf> {
    Ok(ensure_blender_job_layout(job_number, job_id)?.join(JOB_PREVIEW_FILE_NAME))
}

pub fn ffmpeg_job_log_dir(job_number: i32, job_id: &str) -> Result<PathBuf> {
    let dir = ensure_ffmpeg_job_layout(job_number, job_id)?.join(LOG_DIR_NAME);
    fs::create_dir_all(&dir).context("failed to create ffmpeg job log directory")?;
    Ok(dir)
}

fn ensure_blender_job_layout(job_number: i32, job_id: &str) -> Result<PathBuf> {
    let target = blender_job_dir_path(job_number, job_id)?;
    fs::create_dir_all(&target).context("failed to create blender job directory")?;
    migrate_directory_into(&legacy_job_logs_dir_with_id(job_number, job_id)?, &target, Some(BLENDER_LOG_KIND))?;
    migrate_directory_into(&legacy_job_logs_dir(job_number)?, &target, Some(BLENDER_LOG_KIND))?;
    normalize_job_layout(&target, BLENDER_LOG_KIND)?;
    Ok(target)
}

fn ensure_ffmpeg_job_layout(job_number: i32, job_id: &str) -> Result<PathBuf> {
    let target = ffmpeg_job_dir_path(job_number, job_id)?;
    fs::create_dir_all(&target).context("failed to create ffmpeg job directory")?;
    migrate_directory_into(&legacy_ffmpeg_job_logs_dir(job_number, job_id)?, &target, Some(FFMPEG_LOG_KIND))?;
    normalize_job_layout(&target, FFMPEG_LOG_KIND)?;
    Ok(target)
}

fn migrate_known_legacy_logs() -> Result<()> {
    let legacy_root = logs_dir()?;
    if !legacy_root.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(&legacy_root)
        .with_context(|| format!("failed to read legacy logs directory {}", legacy_root.display()))?
    {
        let entry = entry?;
        let source = entry.path();
        if !source.is_dir() {
            continue;
        }

        let Some(name) = source.file_name().and_then(|value| value.to_str()) else {
            continue;
        };

        if let Some((job_number, suffix)) = parse_legacy_blender_named_dir(name) {
            let target = blender_job_dir_path_with_suffix(job_number, &suffix)?;
            fs::create_dir_all(&target).context("failed to create blender migration target")?;
            migrate_directory_into(&source, &target, Some(BLENDER_LOG_KIND))?;
            normalize_job_layout(&target, BLENDER_LOG_KIND)?;
            continue;
        }

        if let Some((job_number, suffix)) = parse_legacy_ffmpeg_named_dir(name) {
            let target = ffmpeg_job_dir_path_with_suffix(job_number, &suffix)?;
            fs::create_dir_all(&target).context("failed to create ffmpeg migration target")?;
            migrate_directory_into(&source, &target, Some(FFMPEG_LOG_KIND))?;
            normalize_job_layout(&target, FFMPEG_LOG_KIND)?;
        }
    }

    Ok(())
}

fn parse_legacy_blender_named_dir(name: &str) -> Option<(i32, String)> {
    let rest = name.strip_prefix("job-")?;
    let (job_number, suffix) = rest.split_once('-')?;
    Some((job_number.parse().ok()?, job_log_suffix(suffix)))
}

fn parse_legacy_ffmpeg_named_dir(name: &str) -> Option<(i32, String)> {
    let rest = name.strip_prefix("ffmpeg-job-")?;
    let (job_number, suffix) = rest.split_once('-')?;
    Some((job_number.parse().ok()?, job_log_suffix(suffix)))
}

fn normalize_job_layout(job_dir: &Path, kind: &str) -> Result<()> {
    let log_dir = job_dir.join(LOG_DIR_NAME);
    fs::create_dir_all(&log_dir)
        .with_context(|| format!("failed to create log directory {}", log_dir.display()))?;

    for entry in fs::read_dir(job_dir)
        .with_context(|| format!("failed to read job directory {}", job_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path == log_dir {
            continue;
        }

        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };

        if path.is_file() {
            if let Some(normalized) = normalize_log_file_name(kind, name) {
                move_file_replace(&path, &log_dir.join(normalized))?;
            }
        }
    }

    for entry in fs::read_dir(&log_dir)
        .with_context(|| format!("failed to read log directory {}", log_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };

        if let Some(normalized) = normalize_log_file_name(kind, name) {
            let normalized_path = log_dir.join(&normalized);
            if path != normalized_path {
                move_file_replace(&path, &normalized_path)?;
            }
        }
    }

    Ok(())
}

fn migrate_directory_into(source: &Path, target: &Path, preferred_kind: Option<&str>) -> Result<()> {
    if !source.exists() || source == target {
        return Ok(());
    }

    fs::create_dir_all(target)
        .with_context(|| format!("failed to create migration target {}", target.display()))?;

    for entry in fs::read_dir(source)
        .with_context(|| format!("failed to read source directory {}", source.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if name == LOG_DIR_NAME && path.is_dir() {
            let target_log_dir = target.join(LOG_DIR_NAME);
            fs::create_dir_all(&target_log_dir)
                .with_context(|| format!("failed to create target log directory {}", target_log_dir.display()))?;
            merge_directory_into(&path, &target_log_dir, preferred_kind)?;
            continue;
        }

        if path.is_file() {
            if name == JOB_TOML_FILE_NAME {
                move_file_replace(&path, &target.join(JOB_TOML_FILE_NAME))?;
                continue;
            }

            if let Some(kind) = preferred_kind {
                if let Some(normalized) = normalize_log_file_name(kind, &name) {
                    move_file_replace(&path, &target.join(LOG_DIR_NAME).join(normalized))?;
                    continue;
                }
            }

            move_file_replace(&path, &target.join(&name))?;
            continue;
        }

        if path.is_dir() {
            merge_directory_into(&path, &target.join(&name), preferred_kind)?;
        }
    }

    remove_dir_if_empty(source)?;
    Ok(())
}

fn merge_directory_into(source: &Path, target: &Path, preferred_kind: Option<&str>) -> Result<()> {
    if !source.exists() {
        return Ok(());
    }

    fs::create_dir_all(target)
        .with_context(|| format!("failed to create merge target {}", target.display()))?;

    for entry in fs::read_dir(source)
        .with_context(|| format!("failed to read merge source {}", source.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() {
            merge_directory_into(&path, &target.join(&name), preferred_kind)?;
            continue;
        }

        if let Some(kind) = preferred_kind {
            if let Some(normalized) = normalize_log_file_name(kind, &name) {
                move_file_replace(&path, &target.join(normalized))?;
                continue;
            }
        }

        move_file_replace(&path, &target.join(&name))?;
    }

    remove_dir_if_empty(source)?;
    Ok(())
}

fn move_file_replace(source: &Path, target: &Path) -> Result<()> {
    if !source.exists() || source == target {
        return Ok(());
    }

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create target parent {}", parent.display()))?;
    }

    if target.exists() {
        fs::remove_file(target)
            .with_context(|| format!("failed to replace existing file {}", target.display()))?;
    }

    match fs::rename(source, target) {
        Ok(()) => Ok(()),
        Err(_) => {
            fs::copy(source, target)
                .with_context(|| format!("failed to copy file {} -> {}", source.display(), target.display()))?;
            fs::remove_file(source)
                .with_context(|| format!("failed to remove source file {}", source.display()))?;
            Ok(())
        }
    }
}

fn remove_dir_if_empty(path: &Path) -> Result<()> {
    if !path.exists() || !path.is_dir() {
        return Ok(());
    }

    let is_empty = fs::read_dir(path)
        .with_context(|| format!("failed to inspect directory {}", path.display()))?
        .next()
        .is_none();

    if is_empty {
        fs::remove_dir(path).with_context(|| format!("failed to remove directory {}", path.display()))?;
    }

    Ok(())
}

fn normalize_log_file_name(kind: &str, file_name: &str) -> Option<String> {
    let new_prefix = format!("{kind}_");
    if file_name.starts_with(&new_prefix) && file_name.ends_with(".log") {
        return Some(file_name.to_string());
    }

    let old_prefix = format!("{kind}-");
    let rest = file_name.strip_prefix(&old_prefix)?;
    if !rest.ends_with(".log") {
        return None;
    }

    Some(format!("{kind}_{rest}"))
}

fn collect_log_files(dir: &Path, kind: &str) -> Result<Vec<PathBuf>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut files = fs::read_dir(dir)
        .with_context(|| format!("failed to read log directory {}", dir.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .and_then(|name| normalize_log_file_name(kind, name))
                .is_some()
        })
        .collect::<Vec<_>>();

    files.sort();
    Ok(files)
}

pub fn timestamped_log_line(line: &str) -> String {
    if line.starts_with('[') && line.chars().nth(5) == Some('-') {
        return line.to_string();
    }

    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    format!("[{timestamp}] {line}")
}

pub fn count_job_log_files(job_number: i32, job_id: &str, kind: &str) -> Result<usize> {
    let dir = if kind == FFMPEG_LOG_KIND {
        ffmpeg_job_log_dir(job_number, job_id)?
    } else {
        job_log_dir(job_number, job_id)?
    };
    Ok(collect_log_files(&dir, kind)?.len())
}

pub fn create_job_log_file(job_number: i32, job_id: &str, kind: &str) -> Result<PathBuf> {
    let timestamp = Local::now().format("%Y%m%d-%H%M%S");
    let path = job_log_dir(job_number, job_id)?.join(format!("{kind}_{timestamp}.log"));
    if !path.exists() {
        fs::write(&path, "").with_context(|| format!("failed to create log file {}", path.display()))?;
    }
    Ok(path)
}

pub fn create_ffmpeg_job_log_file(job_number: i32, job_id: &str) -> Result<PathBuf> {
    let timestamp = Local::now().format("%Y%m%d-%H%M%S");
    let path = ffmpeg_job_log_dir(job_number, job_id)?.join(format!("{FFMPEG_LOG_KIND}_{timestamp}.log"));
    if !path.exists() {
        fs::write(&path, "")
            .with_context(|| format!("failed to create ffmpeg log file {}", path.display()))?;
    }
    Ok(path)
}

pub fn append_log_line(path: &Path, line: &str) -> Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open log file {}", path.display()))?;
    let rendered = timestamped_log_line(line);
    writeln!(file, "{rendered}").with_context(|| format!("failed to write log file {}", path.display()))?;
    Ok(())
}

pub fn append_job_log_event(job_number: i32, job_id: &str, kind: &str, line: &str) -> Result<PathBuf> {
    let dir = job_log_dir(job_number, job_id)?;
    let mut files = collect_log_files(&dir, kind)?;

    let path = if let Some(path) = files.pop() {
        path
    } else {
        create_job_log_file(job_number, job_id, kind)?
    };
    append_log_line(&path, line)?;
    Ok(path)
}

pub fn append_ffmpeg_job_log_event(job_number: i32, job_id: &str, line: &str) -> Result<PathBuf> {
    let dir = ffmpeg_job_log_dir(job_number, job_id)?;
    let mut files = collect_log_files(&dir, FFMPEG_LOG_KIND)?;

    let path = if let Some(path) = files.pop() {
        path
    } else {
        create_ffmpeg_job_log_file(job_number, job_id)?
    };
    append_log_line(&path, line)?;
    Ok(path)
}

pub fn read_job_log_lines(job_number: i32, job_id: &str, kind: &str) -> Result<Vec<String>> {
    let dir = job_log_dir(job_number, job_id)?;
    let files = collect_log_files(&dir, kind)?;

    let mut lines = Vec::new();
    for path in files {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read log file {}", path.display()))?;
        lines.extend(content.lines().map(|line| line.to_string()));
    }

    Ok(lines)
}

pub fn read_latest_job_log_lines(job_number: i32, job_id: &str, kind: &str) -> Result<Vec<String>> {
    let dir = job_log_dir(job_number, job_id)?;
    let files = collect_log_files(&dir, kind)?;
    let Some(path) = files.last() else {
        return Ok(Vec::new());
    };

    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read log file {}", path.display()))?;
    Ok(content.lines().map(|line| line.to_string()).collect())
}

pub fn read_ffmpeg_job_log_lines(job_number: i32, job_id: &str) -> Result<Vec<String>> {
    let dir = ffmpeg_job_log_dir(job_number, job_id)?;
    let files = collect_log_files(&dir, FFMPEG_LOG_KIND)?;

    let mut lines = Vec::new();
    for path in files {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read log file {}", path.display()))?;
        lines.extend(content.lines().map(|line| line.to_string()));
    }

    Ok(lines)
}

pub fn read_latest_ffmpeg_job_log_lines(job_number: i32, job_id: &str) -> Result<Vec<String>> {
    let dir = ffmpeg_job_log_dir(job_number, job_id)?;
    let files = collect_log_files(&dir, FFMPEG_LOG_KIND)?;
    let Some(path) = files.last() else {
        return Ok(Vec::new());
    };

    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read log file {}", path.display()))?;
    Ok(content.lines().map(|line| line.to_string()).collect())
}

pub fn write_job_toml(dir: &Path, contents: &str) -> Result<PathBuf> {
    fs::create_dir_all(dir)
        .with_context(|| format!("failed to create job directory {}", dir.display()))?;
    let path = dir.join(JOB_TOML_FILE_NAME);
    fs::write(&path, contents)
        .with_context(|| format!("failed to write job toml {}", path.display()))?;
    Ok(path)
}
