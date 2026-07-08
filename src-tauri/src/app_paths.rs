use anyhow::{Context, Result};
use chrono::Local;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const BLENDER_LOG_KIND: &str = "blender";
pub const FFMPEG_LOG_KIND: &str = "ffmpeg";

const JOBS_ROOT_DIR_NAME: &str = "jobs";
const LEGACY_LOGS_DIR_NAME: &str = "logs";
const APP_LOGS_DIR_NAME: &str = "app";
const BLENDER_ROOT_DIR_NAME: &str = "blender";
const FFMPEG_ROOT_DIR_NAME: &str = "ffmpeg";
const LOG_DIR_NAME: &str = "log";
const JOB_TOML_FILE_NAME: &str = "job.toml";
const JOB_PREVIEW_FILE_NAME: &str = "preview.jpg";
const NODE_ID_FILE_NAME: &str = "node-id.toml";
const LEGACY_NODE_ID_FILE_NAME: &str = "node-id.txt";
const NODES_DIR_NAME: &str = "nodes";
const NODE_PEERS_DIR_NAME: &str = "peers";
const DB_FILE_NAME: &str = "sik-render.sqlite3";
const CONFIG_FILE_NAME: &str = "sik-render.toml";
const APP_VENDOR_DIR_NAME: &str = "SIKFilm";
const APP_PRODUCT_DIR_NAME: &str = "Render";

#[derive(serde::Deserialize, serde::Serialize)]
struct NodeIdFile {
    id: String,
}

pub fn tool_root_dir() -> Result<PathBuf> {
    if cfg!(debug_assertions) {
        return PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .map(Path::to_path_buf)
            .context("failed to resolve workspace root");
    }

    roaming_app_dir()
}

pub fn runtime_reset_targets() -> Result<Vec<PathBuf>> {
    let root = tool_root_dir()?;
    let db_path = root.join(DB_FILE_NAME);
    Ok(dedup_runtime_reset_targets(vec![
        root.join(CONFIG_FILE_NAME),
        db_path.clone(),
        root.join(format!("{DB_FILE_NAME}-wal")),
        root.join(format!("{DB_FILE_NAME}-shm")),
        root.join(format!("{DB_FILE_NAME}-journal")),
        root.join(NODE_ID_FILE_NAME),
        root.join(LEGACY_NODE_ID_FILE_NAME),
        root.join(JOBS_ROOT_DIR_NAME),
        root.join(LEGACY_LOGS_DIR_NAME),
        root.join("Logs"),
        root.join(NODES_DIR_NAME),
    ]))
}

fn dedup_runtime_reset_targets(targets: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut deduped = Vec::new();
    for target in targets {
        let key = if cfg!(windows) {
            target.to_string_lossy().to_lowercase()
        } else {
            target.to_string_lossy().into_owned()
        };
        if deduped.iter().any(|existing: &PathBuf| {
            let existing_key = if cfg!(windows) {
                existing.to_string_lossy().to_lowercase()
            } else {
                existing.to_string_lossy().into_owned()
            };
            existing_key == key
        }) {
            continue;
        }
        deduped.push(target);
    }
    deduped
}

pub fn reset_runtime_data() -> Result<(PathBuf, Vec<PathBuf>, Vec<(PathBuf, String)>)> {
    let root = tool_root_dir()?;
    let root = root
        .canonicalize()
        .unwrap_or_else(|_| root.to_path_buf());
    let mut removed = Vec::new();
    let mut failed = Vec::new();

    for target in runtime_reset_targets()? {
        if !target.exists() {
            continue;
        }

        let resolved = target
            .canonicalize()
            .unwrap_or_else(|_| target.to_path_buf());
        if !resolved.starts_with(&root) {
            failed.push((
                target,
                format!("refusing to remove path outside runtime root {}", root.display()),
            ));
            continue;
        }

        let result = if resolved.is_dir() {
            fs::remove_dir_all(&resolved)
        } else {
            fs::remove_file(&resolved)
        };

        match result {
            Ok(()) => removed.push(resolved),
            Err(error) => failed.push((resolved, error.to_string())),
        }
    }

    ensure_runtime_layout()?;
    let _ = read_or_create_node_id()?;

    Ok((root, removed, failed))
}

fn roaming_app_dir() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    if let Some(app_data) = std::env::var_os("APPDATA") {
        let dir = PathBuf::from(app_data)
            .join(APP_VENDOR_DIR_NAME)
            .join(APP_PRODUCT_DIR_NAME);
        fs::create_dir_all(&dir)
            .with_context(|| format!("failed to create app data directory {}", dir.display()))?;
        return Ok(dir);
    }

    let dir = std::env::current_dir()
        .context("failed to resolve current directory")?
        .join(APP_VENDOR_DIR_NAME)
        .join(APP_PRODUCT_DIR_NAME);
    fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create app data directory {}", dir.display()))?;
    Ok(dir)
}

pub fn config_path() -> Result<PathBuf> {
    Ok(tool_root_dir()?.join(CONFIG_FILE_NAME))
}

pub fn database_path() -> Result<PathBuf> {
    Ok(tool_root_dir()?.join(DB_FILE_NAME))
}

pub fn read_or_create_node_id() -> Result<String> {
    let root = tool_root_dir()?;
    let path = root.join(NODE_ID_FILE_NAME);
    if path.exists() {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read node id {}", path.display()))?;
        if let Ok(file) = toml::from_str::<NodeIdFile>(&content) {
            let trimmed = file.id.trim();
            if !trimmed.is_empty() {
                return Ok(trimmed.to_string());
            }
        }

        let legacy_plain_id = content.trim();
        if !legacy_plain_id.is_empty() {
            write_node_id_file(&path, legacy_plain_id)?;
            return Ok(legacy_plain_id.to_string());
        }
    }

    let legacy_path = root.join(LEGACY_NODE_ID_FILE_NAME);
    if legacy_path.exists() {
        let value = fs::read_to_string(&legacy_path)
            .with_context(|| format!("failed to read legacy node id {}", legacy_path.display()))?;
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            write_node_id_file(&path, trimmed)?;
            let _ = fs::remove_file(&legacy_path);
            return Ok(trimmed.to_string());
        }
    }

    let node_id = uuid::Uuid::new_v4().to_string();
    write_node_id_file(&path, &node_id)?;
    Ok(node_id)
}

fn write_node_id_file(path: &Path, node_id: &str) -> Result<()> {
    let content = toml::to_string_pretty(&NodeIdFile {
        id: node_id.to_string(),
    })
    .context("failed to serialize node id")?;
    fs::write(path, content).with_context(|| format!("failed to write node id {}", path.display()))
}

pub fn logs_dir() -> Result<PathBuf> {
    Ok(tool_root_dir()?.join(LEGACY_LOGS_DIR_NAME))
}

pub fn app_logs_dir() -> Result<PathBuf> {
    let dir = logs_dir()?
        .join(APP_LOGS_DIR_NAME)
        .join(env!("CARGO_PKG_VERSION"));
    fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create app log directory {}", dir.display()))?;
    Ok(dir)
}

pub fn node_peers_dir() -> Result<PathBuf> {
    let dir = tool_root_dir()?
        .join(NODES_DIR_NAME)
        .join(NODE_PEERS_DIR_NAME);
    fs::create_dir_all(&dir).context("failed to create node peers directory")?;
    Ok(dir)
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
    let _ = app_logs_dir()?;
    let _ = node_peers_dir()?;
    let _ = blender_jobs_root_dir()?;
    let _ = ffmpeg_jobs_root_dir()?;
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
    format!(
        "ffmpeg_job_{job_number:04}_{}",
        ffmpeg_job_log_suffix(job_id)
    )
}

fn blender_job_dir_path(job_number: i32, job_id: &str) -> Result<PathBuf> {
    Ok(blender_jobs_root_dir()?.join(job_logs_dir_name(job_number, job_id)))
}

fn ffmpeg_job_dir_path(job_number: i32, job_id: &str) -> Result<PathBuf> {
    Ok(ffmpeg_jobs_root_dir()?.join(ffmpeg_job_logs_dir_name(job_number, job_id)))
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
    Ok(target)
}

fn ensure_ffmpeg_job_layout(job_number: i32, job_id: &str) -> Result<PathBuf> {
    let target = ffmpeg_job_dir_path(job_number, job_id)?;
    fs::create_dir_all(&target).context("failed to create ffmpeg job directory")?;
    Ok(target)
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
        fs::write(&path, "")
            .with_context(|| format!("failed to create log file {}", path.display()))?;
    }
    Ok(path)
}

pub fn create_ffmpeg_job_log_file(job_number: i32, job_id: &str) -> Result<PathBuf> {
    let timestamp = Local::now().format("%Y%m%d-%H%M%S");
    let path =
        ffmpeg_job_log_dir(job_number, job_id)?.join(format!("{FFMPEG_LOG_KIND}_{timestamp}.log"));
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
    writeln!(file, "{rendered}")
        .with_context(|| format!("failed to write log file {}", path.display()))?;
    Ok(())
}

pub fn append_job_log_event(
    job_number: i32,
    job_id: &str,
    kind: &str,
    line: &str,
) -> Result<PathBuf> {
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
