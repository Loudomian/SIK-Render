use crate::commands::settings::AppSettings;
use crate::queue::job::RenderJob;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use tokio::process::Command;

/// Local single-machine variant of Flamenco's Blender command model:
/// `{exe} {argsBefore} {blendfile?} {args}`.
#[derive(Debug, Clone)]
pub struct BlenderCliCommand {
    executable: PathBuf,
    args_before: Vec<OsString>,
    blend_file: Option<PathBuf>,
    args: Vec<OsString>,
}

impl BlenderCliCommand {
    pub fn new(executable: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
            args_before: Vec::new(),
            blend_file: None,
            args: Vec::new(),
        }
    }

    pub fn arg_before(mut self, arg: impl Into<OsString>) -> Self {
        self.args_before.push(arg.into());
        self
    }

    pub fn blend_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.blend_file = Some(path.into());
        self
    }

    pub fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn into_tokio_command(self) -> Command {
        let mut command = Command::new(&self.executable);
        command.args(&self.args_before);
        if let Some(blend_file) = &self.blend_file {
            command.arg(blend_file);
        }
        command.args(&self.args);
        #[cfg(target_os = "windows")]
        {
            command.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        command
    }

    pub async fn output(self) -> std::io::Result<std::process::Output> {
        self.into_tokio_command().output().await
    }
}

fn build_render_settings_script(job: &RenderJob, settings: &AppSettings) -> String {
    let mut lines = vec![
        "import bpy".to_string(),
        "scene = bpy.context.scene".to_string(),
        "r = scene.render".to_string(),
        "image = r.image_settings".to_string(),
        "r.use_file_extension = True".to_string(),
    ];

    match job.output_format.as_str() {
        "OPEN_EXR" | "EXR" => {
            lines.extend([
                "image.file_format = 'OPEN_EXR'".to_string(),
                format!("image.color_mode = '{}'", settings.exr_color_mode),
                format!("image.color_depth = '{}'", settings.exr_color_depth),
                format!("image.exr_codec = '{}'", settings.exr_codec),
            ]);

            if matches!(settings.exr_codec.as_str(), "DWAA" | "DWAB") {
                lines.push(format!("image.quality = {}", settings.exr_quality));
            }
        }
        "PNG" => {
            lines.extend([
                "image.file_format = 'PNG'".to_string(),
                format!("image.color_mode = '{}'", settings.png_color_mode),
                format!("image.color_depth = '{}'", settings.png_color_depth),
                format!("image.compression = {}", settings.png_compression),
            ]);
        }
        _ => {
            lines.push(format!("image.file_format = '{}'", job.output_format));
        }
    }

    lines.join("; ")
}

pub fn render_command(
    job: &RenderJob,
    frame_start_actual: i32,
    settings: &AppSettings,
) -> BlenderCliCommand {
    let render_settings_script = build_render_settings_script(job, settings);
    BlenderCliCommand::new(&job.blender_executable)
        .arg_before("--background")
        .blend_file(&job.blend_file)
        .arg("--python-expr")
        .arg(render_settings_script)
        .args([
            "--render-output".into(),
            job.output_path.as_os_str().to_os_string(),
            "--render-format".into(),
            job.output_format.clone().into(),
            "-s".into(),
            frame_start_actual.to_string().into(),
            "-e".into(),
            job.frame_end.to_string().into(),
            "-a".into(),
        ])
}

pub fn inspect_project_command(
    blender_executable: &Path,
    blend_file: &Path,
    script: &str,
) -> BlenderCliCommand {
    BlenderCliCommand::new(blender_executable)
        .arg_before("--background")
        .arg_before("--disable-autoexec")
        .blend_file(blend_file)
        .arg("--python-expr")
        .arg(script)
}
