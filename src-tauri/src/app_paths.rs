use anyhow::{Context, Result};
use chrono::Local;
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

pub fn job_logs_dir(job_number: i32) -> Result<PathBuf> {
    let dir = logs_dir()?.join(format!("job-{job_number:04}"));
    fs::create_dir_all(&dir).context("failed to create job log directory")?;
    Ok(dir)
}

pub fn count_job_log_files(job_number: i32, kind: &str) -> Result<usize> {
    let dir = job_logs_dir(job_number)?;
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

pub fn create_job_log_file(job_number: i32, kind: &str) -> Result<PathBuf> {
    let timestamp = Local::now().format("%Y%m%d-%H%M%S");
    let path = job_logs_dir(job_number)?.join(format!("{kind}-{timestamp}.log"));
    if !path.exists() {
      fs::write(&path, "").with_context(|| format!("failed to create log file {}", path.display()))?;
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

pub fn append_job_log_event(job_number: i32, kind: &str, line: &str) -> Result<PathBuf> {
    let dir = job_logs_dir(job_number)?;
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
    let path = files
        .pop()
        .unwrap_or_else(|| create_job_log_file(job_number, kind).expect("failed to create job log file"));
    append_log_line(&path, line)?;
    Ok(path)
}

pub fn read_job_log_lines(job_number: i32, kind: &str) -> Result<Vec<String>> {
    let dir = logs_dir()?.join(format!("job-{job_number:04}"));
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

pub fn read_latest_job_log_lines(job_number: i32, kind: &str) -> Result<Vec<String>> {
    let dir = logs_dir()?.join(format!("job-{job_number:04}"));
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
