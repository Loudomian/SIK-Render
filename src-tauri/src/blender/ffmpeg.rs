use std::ffi::OsString;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};
use tokio::process::Command as TokioCommand;

/// Local single-machine variant of Flamenco's FFmpeg command model:
/// `{exe} {argsBefore} {inputArgs} {args} {outputFile}`.
#[derive(Debug, Clone)]
pub struct FfmpegCliCommand {
    executable: PathBuf,
    args_before: Vec<OsString>,
    input_args: Vec<OsString>,
    args: Vec<OsString>,
    output_file: PathBuf,
}

impl FfmpegCliCommand {
    pub fn new(executable: impl Into<PathBuf>, output_file: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
            args_before: Vec::new(),
            input_args: Vec::new(),
            args: Vec::new(),
            output_file: output_file.into(),
        }
    }

    pub fn arg_before(mut self, arg: impl Into<OsString>) -> Self {
        self.args_before.push(arg.into());
        self
    }

    pub fn input_arg(mut self, arg: impl Into<OsString>) -> Self {
        self.input_args.push(arg.into());
        self
    }

    pub fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn into_tokio_command(self) -> TokioCommand {
        let mut command = TokioCommand::new(&self.executable);
        command.args(&self.args_before);
        command.args(&self.input_args);
        command.args(&self.args);
        command.arg(&self.output_file);
        command
    }
}

#[derive(Debug, Clone)]
pub struct FfmpegExecutableLookup {
    pub executable: Option<PathBuf>,
    pub source: Option<&'static str>,
}

pub fn find_ffmpeg_executable(
    app: Option<&AppHandle>,
    configured_executable: Option<&Path>,
    blender_executable: &Path,
) -> FfmpegExecutableLookup {
    let candidates = ffmpeg_candidates(app, configured_executable, blender_executable);
    let executable = candidates
        .iter()
        .find_map(|(path, source)| path.exists().then(|| (path.clone(), *source)));

    FfmpegExecutableLookup {
        executable: executable.as_ref().map(|(path, _)| path.clone()),
        source: executable.as_ref().map(|(_, source)| *source),
    }
}

fn ffmpeg_candidates(
    app: Option<&AppHandle>,
    configured_executable: Option<&Path>,
    blender_executable: &Path,
) -> Vec<(PathBuf, &'static str)> {
    let mut candidates = Vec::new();

    if let Some(path) = configured_executable {
        candidates.push((path.to_path_buf(), "settings"));
    }

    if let Some(app) = app {
        if let Ok(resource_dir) = app.path().resource_dir() {
            for path in binary_candidates_in(&resource_dir.join("bin")) {
                candidates.push((path, "bundled resource"));
            }
            for path in binary_candidates_in(&resource_dir) {
                candidates.push((path, "bundled resource"));
            }
        }
    }

    let manifest_bin_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bin");
    for path in binary_candidates_in(&manifest_bin_dir) {
        candidates.push((path, "workspace bin"));
    }

    if let Some(dir) = blender_executable.parent() {
        for path in binary_candidates_in(dir) {
            candidates.push((path, "blender directory"));
        }

        if let Some(parent) = dir.parent() {
            for path in binary_candidates_in(parent) {
                candidates.push((path, "blender parent directory"));
            }
        }
    }

    for path in command_search_paths(ffmpeg_command_name()) {
        candidates.push((path, "system PATH"));
    }

    dedupe_candidates(candidates)
}

pub fn concat_to_mp4_command(
    ffmpeg_executable: &Path,
    concat_index: &Path,
    fps: f32,
    output_file: &Path,
) -> FfmpegCliCommand {
    FfmpegCliCommand::new(ffmpeg_executable, output_file)
        .arg_before("-hide_banner")
        .arg_before("-y")
        .input_arg("-f")
        .input_arg("concat")
        .input_arg("-safe")
        .input_arg("0")
        .input_arg("-i")
        .input_arg(concat_index.as_os_str().to_os_string())
        .arg("-r")
        .arg(format!("{fps:.6}"))
        .arg("-an")
        .arg("-c:v")
        .arg("libx264")
        .arg("-preset")
        .arg("medium")
        .arg("-crf")
        .arg("18")
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg("-movflags")
        .arg("+faststart")
}

fn binary_candidates_in(dir: &Path) -> Vec<PathBuf> {
    ffmpeg_binary_names()
        .iter()
        .map(|name| dir.join(name))
        .collect()
}

fn ffmpeg_binary_names() -> &'static [&'static str] {
    #[cfg(target_os = "windows")]
    {
        &["ffmpeg.exe", "ffmpeg"]
    }
    #[cfg(not(target_os = "windows"))]
    {
        &["ffmpeg"]
    }
}

fn ffmpeg_command_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "where.exe"
    }
    #[cfg(not(target_os = "windows"))]
    {
        "which"
    }
}

fn dedupe_candidates(candidates: Vec<(PathBuf, &'static str)>) -> Vec<(PathBuf, &'static str)> {
    let mut seen = std::collections::HashSet::new();
    candidates
        .into_iter()
        .filter(|(path, _)| seen.insert(path.clone()))
        .collect()
}

fn command_search_paths(program: &str) -> Vec<PathBuf> {
    let lookup_name = ffmpeg_binary_names().first().copied().unwrap_or("ffmpeg");
    let Ok(output) = std::process::Command::new(program)
        .arg(lookup_name)
        .output()
    else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect()
}
