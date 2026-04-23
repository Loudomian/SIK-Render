use anyhow::{Context, Result};
use chrono::Utc;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const BLENDER_LOG_KIND: &str = "blender";
pub const FFMPEG_LOG_KIND: &str = "ffmpeg";

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
    let dir = tool_root_dir()?.join("logs");
    fs::create_dir_all(&dir).context("failed to create logs directory")?;
    Ok(dir)
}

pub fn ensure_runtime_layout() -> Result<()> {
    logs_dir().map(|_| ())
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
    format!("job-{job_number:04}-{}", job_log_suffix(job_id))
}

pub fn ffmpeg_job_logs_dir_name(job_number: i32, job_id: &str) -> String {
    format!("ffmpeg-job-{job_number:04}-{}", ffmpeg_job_log_suffix(job_id))
}

pub fn legacy_job_logs_dir(job_number: i32) -> Result<PathBuf> {
    Ok(logs_dir()?.join(format!("job-{job_number:04}")))
}

pub fn job_logs_dir(job_number: i32, job_id: &str) -> Result<PathBuf> {
    let dir = logs_dir()?.join(job_logs_dir_name(job_number, job_id));
    fs::create_dir_all(&dir).context("failed to create job log directory")?;
    Ok(dir)
}

pub fn ffmpeg_job_logs_dir(job_number: i32, job_id: &str) -> Result<PathBuf> {
    let dir = logs_dir()?.join(ffmpeg_job_logs_dir_name(job_number, job_id));
    fs::create_dir_all(&dir).context("failed to create ffmpeg job log directory")?;
    Ok(dir)
}

fn active_job_logs_dir(job_number: i32, job_id: &str) -> Result<PathBuf> {
    let new_dir = logs_dir()?.join(job_logs_dir_name(job_number, job_id));
    if new_dir.exists() {
        return Ok(new_dir);
    }

    let legacy_dir = legacy_job_logs_dir(job_number)?;
    if legacy_dir.exists() {
        return Ok(legacy_dir);
    }

    fs::create_dir_all(&new_dir).context("failed to create job log directory")?;
    Ok(new_dir)
}

pub fn count_job_log_files(job_number: i32, job_id: &str, kind: &str) -> Result<usize> {
    let dir = active_job_logs_dir(job_number, job_id)?;
    let count = fs::read_dir(&dir)
        .with_context(|| format!("failed to read job log directory {}", dir.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with(&format!("{kind}-")) && name.ends_with(".log"))
                .unwrap_or(false)
        })
        .count();
    Ok(count)
}

pub fn create_job_log_file(job_number: i32, job_id: &str, kind: &str) -> Result<PathBuf> {
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
    let path = active_job_logs_dir(job_number, job_id)?.join(format!("{kind}-{timestamp}.log"));
    if !path.exists() {
      fs::write(&path, "").with_context(|| format!("failed to create log file {}", path.display()))?;
    }
    Ok(path)
}

pub fn create_ffmpeg_job_log_file(job_number: i32, job_id: &str) -> Result<PathBuf> {
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
    let path = ffmpeg_job_logs_dir(job_number, job_id)?.join(format!("{FFMPEG_LOG_KIND}-{timestamp}.log"));
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
    writeln!(file, "{line}").with_context(|| format!("failed to write log file {}", path.display()))?;
    Ok(())
}

pub fn append_job_log_event(job_number: i32, job_id: &str, kind: &str, line: &str) -> Result<PathBuf> {
    let dir = active_job_logs_dir(job_number, job_id)?;
    let mut files = fs::read_dir(&dir)
        .with_context(|| format!("failed to read job log directory {}", dir.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with(&format!("{kind}-")) && name.ends_with(".log"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    files.sort();
    let path = if let Some(path) = files.pop() {
        path
    } else {
        create_job_log_file(job_number, job_id, kind)?
    };
    append_log_line(&path, line)?;
    Ok(path)
}

pub fn append_ffmpeg_job_log_event(job_number: i32, job_id: &str, line: &str) -> Result<PathBuf> {
    let dir = ffmpeg_job_logs_dir(job_number, job_id)?;
    let mut files = fs::read_dir(&dir)
        .with_context(|| format!("failed to read ffmpeg job log directory {}", dir.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with(&format!("{FFMPEG_LOG_KIND}-")) && name.ends_with(".log"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    files.sort();
    let path = if let Some(path) = files.pop() {
        path
    } else {
        create_ffmpeg_job_log_file(job_number, job_id)?
    };
    append_log_line(&path, line)?;
    Ok(path)
}

pub fn read_job_log_lines(job_number: i32, job_id: &str, kind: &str) -> Result<Vec<String>> {
    let dir = active_job_logs_dir(job_number, job_id)?;
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut files = fs::read_dir(&dir)
        .with_context(|| format!("failed to read job log directory {}", dir.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with(&format!("{kind}-")) && name.ends_with(".log"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    files.sort();

    let mut lines = Vec::new();
    for path in files {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read log file {}", path.display()))?;
        lines.extend(content.lines().map(|line| line.to_string()));
    }

    Ok(lines)
}

pub fn read_latest_job_log_lines(job_number: i32, job_id: &str, kind: &str) -> Result<Vec<String>> {
    let dir = active_job_logs_dir(job_number, job_id)?;
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut files = fs::read_dir(&dir)
        .with_context(|| format!("failed to read job log directory {}", dir.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with(&format!("{kind}-")) && name.ends_with(".log"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    files.sort();
    let Some(path) = files.last() else {
        return Ok(Vec::new());
    };

    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read log file {}", path.display()))?;
    Ok(content.lines().map(|line| line.to_string()).collect())
}

pub fn read_ffmpeg_job_log_lines(job_number: i32, job_id: &str) -> Result<Vec<String>> {
    let dir = ffmpeg_job_logs_dir(job_number, job_id)?;
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut files = fs::read_dir(&dir)
        .with_context(|| format!("failed to read ffmpeg job log directory {}", dir.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with(&format!("{FFMPEG_LOG_KIND}-")) && name.ends_with(".log"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    files.sort();

    let mut lines = Vec::new();
    for path in files {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read log file {}", path.display()))?;
        lines.extend(content.lines().map(|line| line.to_string()));
    }

    Ok(lines)
}

pub fn read_latest_ffmpeg_job_log_lines(job_number: i32, job_id: &str) -> Result<Vec<String>> {
    let dir = ffmpeg_job_logs_dir(job_number, job_id)?;
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut files = fs::read_dir(&dir)
        .with_context(|| format!("failed to read ffmpeg job log directory {}", dir.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with(&format!("{FFMPEG_LOG_KIND}-")) && name.ends_with(".log"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    files.sort();
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
    let path = dir.join("job.toml");
    fs::write(&path, contents)
        .with_context(|| format!("failed to write job toml {}", path.display()))?;
    Ok(path)
}
