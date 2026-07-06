use serde::{Deserialize, Serialize};
use std::fs;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub default_blender: String,
    #[serde(default)]
    pub ffmpeg_executable: String,
    pub blend_inspect_timeout_seconds: u64,
    #[serde(default = "default_transcode_crf")]
    pub transcode_crf: u32,
    #[serde(default = "default_transcode_preset")]
    pub transcode_preset: String,
    #[serde(default = "default_ffmpeg_max_concurrent")]
    pub ffmpeg_max_concurrent: u32,
    #[serde(default = "default_render_output_path_template")]
    pub render_output_path_template: String,
    #[serde(default = "default_blender_transcode_output_path_template")]
    pub blender_transcode_output_path_template: String,
    #[serde(default = "default_standalone_transcode_output_path_template")]
    pub standalone_transcode_output_path_template: String,
    #[serde(default = "default_png_color_mode")]
    pub png_color_mode: String,
    #[serde(default = "default_png_color_depth")]
    pub png_color_depth: u32,
    #[serde(default = "default_png_compression")]
    pub png_compression: u32,
    #[serde(default = "default_exr_color_mode")]
    pub exr_color_mode: String,
    #[serde(default = "default_exr_color_depth")]
    pub exr_color_depth: u32,
    #[serde(default = "default_exr_codec")]
    pub exr_codec: String,
    #[serde(default = "default_exr_quality")]
    pub exr_quality: u32,
    pub theme: String,
    #[serde(default = "default_locale")]
    pub locale: String,
    #[serde(default)]
    pub extra_blender_paths: Vec<String>,
    #[serde(default)]
    pub excluded_blender_paths: Vec<String>,
    #[serde(default = "default_max_crash_retries")]
    pub max_crash_retries: u32,
    #[serde(default = "default_node_port")]
    pub node_port: u16,
    #[serde(default = "default_node_interface_address")]
    pub node_interface_address: String,
    #[serde(default)]
    pub node_note: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            default_blender: String::new(),
            ffmpeg_executable: String::new(),
            blend_inspect_timeout_seconds: default_blend_inspect_timeout_seconds(),
            transcode_crf: default_transcode_crf(),
            transcode_preset: default_transcode_preset(),
            ffmpeg_max_concurrent: default_ffmpeg_max_concurrent(),
            render_output_path_template: default_render_output_path_template(),
            blender_transcode_output_path_template: default_blender_transcode_output_path_template(
            ),
            standalone_transcode_output_path_template:
                default_standalone_transcode_output_path_template(),
            png_color_mode: default_png_color_mode(),
            png_color_depth: default_png_color_depth(),
            png_compression: default_png_compression(),
            exr_color_mode: default_exr_color_mode(),
            exr_color_depth: default_exr_color_depth(),
            exr_codec: default_exr_codec(),
            exr_quality: default_exr_quality(),
            theme: "dark".into(),
            locale: default_locale(),
            extra_blender_paths: Vec::new(),
            excluded_blender_paths: Vec::new(),
            max_crash_retries: default_max_crash_retries(),
            node_port: default_node_port(),
            node_interface_address: default_node_interface_address(),
            node_note: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SettingsFile {
    #[serde(default)]
    tools: ToolsSettings,
    #[serde(default)]
    ui: UiSettings,
    #[serde(default)]
    blender: BlenderSettings,
    #[serde(default)]
    output_paths: OutputPathSettings,
    #[serde(default)]
    blender_output: BlenderOutputSettings,
    #[serde(default)]
    network: NetworkSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ToolsSettings {
    #[serde(default)]
    default_blender: String,
    #[serde(default)]
    ffmpeg_executable: String,
    #[serde(default = "default_blend_inspect_timeout_seconds")]
    blend_inspect_timeout_seconds: u64,
    #[serde(default = "default_transcode_crf")]
    transcode_crf: u32,
    #[serde(default = "default_transcode_preset")]
    transcode_preset: String,
    #[serde(default = "default_ffmpeg_max_concurrent")]
    ffmpeg_max_concurrent: u32,
    #[serde(default = "default_max_crash_retries")]
    max_crash_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct OutputPathSettings {
    #[serde(default = "default_render_output_path_template")]
    render_output_path_template: String,
    #[serde(default = "default_blender_transcode_output_path_template")]
    blender_transcode_output_path_template: String,
    #[serde(default = "default_standalone_transcode_output_path_template")]
    standalone_transcode_output_path_template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct BlenderOutputSettings {
    #[serde(default = "default_png_color_mode")]
    png_color_mode: String,
    #[serde(default = "default_png_color_depth")]
    png_color_depth: u32,
    #[serde(default = "default_png_compression")]
    png_compression: u32,
    #[serde(default = "default_exr_color_mode")]
    exr_color_mode: String,
    #[serde(default = "default_exr_color_depth")]
    exr_color_depth: u32,
    #[serde(default = "default_exr_codec")]
    exr_codec: String,
    #[serde(default = "default_exr_quality")]
    exr_quality: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UiSettings {
    #[serde(default = "default_theme")]
    theme: String,
    #[serde(default = "default_locale")]
    locale: String,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            locale: default_locale(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct BlenderSettings {
    #[serde(default)]
    extra_blender_paths: Vec<String>,
    #[serde(default)]
    excluded_blender_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct NetworkSettings {
    #[serde(default = "default_node_port")]
    port: u16,
    #[serde(default = "default_node_interface_address")]
    interface_address: String,
    #[serde(default)]
    note: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum SettingsFileCompat {
    Grouped(SettingsFile),
    Flat(AppSettings),
}

fn default_blend_inspect_timeout_seconds() -> u64 {
    300
}

fn default_max_crash_retries() -> u32 {
    3
}

fn default_node_port() -> u16 {
    47878
}

fn default_node_interface_address() -> String {
    String::from("192.168.1.1")
}

fn default_transcode_crf() -> u32 {
    18
}

fn default_transcode_preset() -> String {
    String::from("medium")
}

fn default_ffmpeg_max_concurrent() -> u32 {
    2
}

fn default_theme() -> String {
    String::from("dark")
}

fn default_locale() -> String {
    String::from("zh-CN")
}

fn default_render_output_path_template() -> String {
    String::from("./{blendFileName}_{frameStart}-{frameEnd}/{blendFileName}_{frame}")
}

fn default_blender_transcode_output_path_template() -> String {
    String::from("./transcode/{blendFileName}_{frameStart}-{frameEnd}.mp4")
}

fn default_standalone_transcode_output_path_template() -> String {
    String::from("../transcode/{folderName}_{frameStart}-{frameEnd}.mp4")
}

fn default_png_color_mode() -> String {
    String::from("RGB")
}

fn default_png_color_depth() -> u32 {
    8
}

fn default_png_compression() -> u32 {
    15
}

fn default_exr_color_mode() -> String {
    String::from("RGB")
}

fn default_exr_color_depth() -> u32 {
    16
}

fn default_exr_codec() -> String {
    String::from("DWAA")
}

fn default_exr_quality() -> u32 {
    98
}

fn normalize_theme(theme: String) -> String {
    match theme.as_str() {
        "light" => String::from("light"),
        "system" => String::from("system"),
        _ => default_theme(),
    }
}

fn normalize_locale(locale: String) -> String {
    match locale.as_str() {
        "en-US" => String::from("en-US"),
        _ => default_locale(),
    }
}

fn normalize_blend_inspect_timeout_seconds(seconds: u64) -> u64 {
    seconds.clamp(30, 800)
}

fn normalize_max_crash_retries(retries: u32) -> u32 {
    retries.min(10)
}

fn normalize_ffmpeg_max_concurrent(value: u32) -> u32 {
    value.clamp(1, 8)
}

fn normalize_node_port(port: u16) -> u16 {
    if port == 0 {
        default_node_port()
    } else {
        port
    }
}

fn normalize_node_interface_address(address: String) -> String {
    let trimmed = address.trim();
    if trimmed.is_empty() {
        default_node_interface_address()
    } else {
        trimmed.to_string()
    }
}

fn normalize_node_note(note: String) -> String {
    note.trim().chars().take(80).collect()
}

fn normalize_png_color_mode(mode: &str) -> String {
    match mode {
        "BW" | "RGB" | "RGBA" => mode.to_string(),
        _ => default_png_color_mode(),
    }
}

fn normalize_png_color_depth(depth: u32) -> u32 {
    match depth {
        8 | 16 => depth,
        _ => default_png_color_depth(),
    }
}

fn normalize_png_compression(value: u32) -> u32 {
    value.min(100)
}

fn normalize_exr_color_mode(mode: &str) -> String {
    match mode {
        "BW" | "RGB" | "RGBA" => mode.to_string(),
        _ => default_exr_color_mode(),
    }
}

fn normalize_exr_color_depth(depth: u32) -> u32 {
    match depth {
        16 | 32 => depth,
        _ => default_exr_color_depth(),
    }
}

fn normalize_exr_codec(codec: &str) -> String {
    match codec {
        "NONE" | "ZIP" | "PIZ" | "DWAA" | "DWAB" | "ZIPS" | "RLE" | "PXR24" | "B44" | "B44A" => {
            codec.to_string()
        }
        _ => default_exr_codec(),
    }
}

fn normalize_exr_quality(value: u32) -> u32 {
    value.min(100)
}

fn normalize_preset(preset: &str) -> String {
    match preset {
        "ultrafast" | "superfast" | "veryfast" | "faster" | "fast" | "medium" | "slow"
        | "slower" | "veryslow" => preset.to_string(),
        _ => default_transcode_preset(),
    }
}

fn normalize_output_path_template(template: String, fallback: fn() -> String) -> String {
    let trimmed = template.trim();
    if trimmed.is_empty() {
        fallback()
    } else {
        trimmed.to_string()
    }
}

impl From<SettingsFile> for AppSettings {
    fn from(value: SettingsFile) -> Self {
        Self {
            default_blender: value.tools.default_blender,
            ffmpeg_executable: value.tools.ffmpeg_executable,
            blend_inspect_timeout_seconds: normalize_blend_inspect_timeout_seconds(
                value.tools.blend_inspect_timeout_seconds,
            ),
            transcode_crf: value.tools.transcode_crf.min(51),
            transcode_preset: normalize_preset(&value.tools.transcode_preset),
            ffmpeg_max_concurrent: normalize_ffmpeg_max_concurrent(
                value.tools.ffmpeg_max_concurrent,
            ),
            render_output_path_template: normalize_output_path_template(
                value.output_paths.render_output_path_template,
                default_render_output_path_template,
            ),
            blender_transcode_output_path_template: normalize_output_path_template(
                value.output_paths.blender_transcode_output_path_template,
                default_blender_transcode_output_path_template,
            ),
            standalone_transcode_output_path_template: normalize_output_path_template(
                value.output_paths.standalone_transcode_output_path_template,
                default_standalone_transcode_output_path_template,
            ),
            png_color_mode: normalize_png_color_mode(&value.blender_output.png_color_mode),
            png_color_depth: normalize_png_color_depth(value.blender_output.png_color_depth),
            png_compression: normalize_png_compression(value.blender_output.png_compression),
            exr_color_mode: normalize_exr_color_mode(&value.blender_output.exr_color_mode),
            exr_color_depth: normalize_exr_color_depth(value.blender_output.exr_color_depth),
            exr_codec: normalize_exr_codec(&value.blender_output.exr_codec),
            exr_quality: normalize_exr_quality(value.blender_output.exr_quality),
            theme: normalize_theme(value.ui.theme),
            locale: normalize_locale(value.ui.locale),
            extra_blender_paths: value.blender.extra_blender_paths,
            excluded_blender_paths: value.blender.excluded_blender_paths,
            max_crash_retries: normalize_max_crash_retries(value.tools.max_crash_retries),
            node_port: normalize_node_port(value.network.port),
            node_interface_address: normalize_node_interface_address(
                value.network.interface_address,
            ),
            node_note: normalize_node_note(value.network.note),
        }
    }
}

impl From<AppSettings> for SettingsFile {
    fn from(value: AppSettings) -> Self {
        Self {
            tools: ToolsSettings {
                default_blender: value.default_blender,
                ffmpeg_executable: value.ffmpeg_executable,
                blend_inspect_timeout_seconds: normalize_blend_inspect_timeout_seconds(
                    value.blend_inspect_timeout_seconds,
                ),
                transcode_crf: value.transcode_crf.min(51),
                transcode_preset: value.transcode_preset.clone(),
                ffmpeg_max_concurrent: normalize_ffmpeg_max_concurrent(value.ffmpeg_max_concurrent),
                max_crash_retries: normalize_max_crash_retries(value.max_crash_retries),
            },
            ui: UiSettings {
                theme: value.theme,
                locale: value.locale,
            },
            blender: BlenderSettings {
                extra_blender_paths: value.extra_blender_paths,
                excluded_blender_paths: value.excluded_blender_paths,
            },
            output_paths: OutputPathSettings {
                render_output_path_template: normalize_output_path_template(
                    value.render_output_path_template,
                    default_render_output_path_template,
                ),
                blender_transcode_output_path_template: normalize_output_path_template(
                    value.blender_transcode_output_path_template,
                    default_blender_transcode_output_path_template,
                ),
                standalone_transcode_output_path_template: normalize_output_path_template(
                    value.standalone_transcode_output_path_template,
                    default_standalone_transcode_output_path_template,
                ),
            },
            blender_output: BlenderOutputSettings {
                png_color_mode: normalize_png_color_mode(&value.png_color_mode),
                png_color_depth: normalize_png_color_depth(value.png_color_depth),
                png_compression: normalize_png_compression(value.png_compression),
                exr_color_mode: normalize_exr_color_mode(&value.exr_color_mode),
                exr_color_depth: normalize_exr_color_depth(value.exr_color_depth),
                exr_codec: normalize_exr_codec(&value.exr_codec),
                exr_quality: normalize_exr_quality(value.exr_quality),
            },
            network: NetworkSettings {
                port: normalize_node_port(value.node_port),
                interface_address: normalize_node_interface_address(value.node_interface_address),
                note: normalize_node_note(value.node_note),
            },
        }
    }
}

fn settings_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let _ = app;
    crate::app_paths::config_path().map_err(|error| error.to_string())
}

fn legacy_settings_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    crate::db::app_data_dir(app)
        .map(|dir| dir.join("settings.json"))
        .map_err(|error| error.to_string())
}

fn read_settings_from_disk(app: &AppHandle) -> Result<AppSettings, String> {
    let path = settings_path(app)?;
    if !path.exists() {
        let legacy_path = legacy_settings_path(app)?;
        if legacy_path.exists() {
            let content = fs::read_to_string(&legacy_path).map_err(|error| error.to_string())?;
            let settings: AppSettings =
                serde_json::from_str(&content).map_err(|error| error.to_string())?;
            save_settings(app.clone(), settings.clone())?;
            let _ = fs::remove_file(&legacy_path);
            return Ok(settings);
        }

        let settings = AppSettings::default();
        save_settings(app.clone(), settings.clone())?;
        return Ok(settings);
    }

    let content = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    match toml::from_str::<SettingsFileCompat>(&content) {
        Ok(SettingsFileCompat::Grouped(settings)) => Ok(settings.into()),
        Ok(SettingsFileCompat::Flat(settings)) => {
            save_settings(app.clone(), settings.clone())?;
            Ok(settings)
        }
        Err(error) => {
            log::error!(
                "failed to parse settings file {}: {}",
                path.display(),
                error
            );
            let fallback = AppSettings::default();
            save_settings(app.clone(), fallback.clone())?;
            Ok(fallback)
        }
    }
}

#[cfg(target_os = "windows")]
fn replace_file_atomically(
    source: &std::path::Path,
    destination: &std::path::Path,
) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let source_wide = source
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let destination_wide = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();

    unsafe {
        MoveFileExW(
            PCWSTR(source_wide.as_ptr()),
            PCWSTR(destination_wide.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
        .map_err(|error| error.to_string())
    }
}

#[cfg(not(target_os = "windows"))]
fn replace_file_atomically(
    source: &std::path::Path,
    destination: &std::path::Path,
) -> Result<(), String> {
    fs::rename(source, destination).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Result<AppSettings, String> {
    if let Some(state) = app.try_state::<crate::state::AppState>() {
        if let Some(settings) = state.cached_settings() {
            return Ok(settings);
        }
    }

    let settings = read_settings_from_disk(&app)?;
    if let Some(state) = app.try_state::<crate::state::AppState>() {
        state.set_cached_settings(settings.clone());
    }
    Ok(settings)
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: AppSettings) -> Result<(), String> {
    let path = settings_path(&app)?;
    let mut settings = settings;
    settings.theme = normalize_theme(settings.theme);
    settings.locale = normalize_locale(settings.locale);
    settings.blend_inspect_timeout_seconds =
        normalize_blend_inspect_timeout_seconds(settings.blend_inspect_timeout_seconds);
    settings.transcode_crf = settings.transcode_crf.min(51);
    settings.transcode_preset = normalize_preset(&settings.transcode_preset);
    settings.ffmpeg_max_concurrent =
        normalize_ffmpeg_max_concurrent(settings.ffmpeg_max_concurrent);
    settings.render_output_path_template = normalize_output_path_template(
        settings.render_output_path_template,
        default_render_output_path_template,
    );
    settings.blender_transcode_output_path_template = normalize_output_path_template(
        settings.blender_transcode_output_path_template,
        default_blender_transcode_output_path_template,
    );
    settings.standalone_transcode_output_path_template = normalize_output_path_template(
        settings.standalone_transcode_output_path_template,
        default_standalone_transcode_output_path_template,
    );
    settings.png_color_mode = normalize_png_color_mode(&settings.png_color_mode);
    settings.png_color_depth = normalize_png_color_depth(settings.png_color_depth);
    settings.png_compression = normalize_png_compression(settings.png_compression);
    settings.exr_color_mode = normalize_exr_color_mode(&settings.exr_color_mode);
    settings.exr_color_depth = normalize_exr_color_depth(settings.exr_color_depth);
    settings.exr_codec = normalize_exr_codec(&settings.exr_codec);
    settings.exr_quality = normalize_exr_quality(settings.exr_quality);
    settings.max_crash_retries = normalize_max_crash_retries(settings.max_crash_retries);
    settings.node_port = normalize_node_port(settings.node_port);
    settings.node_interface_address =
        normalize_node_interface_address(settings.node_interface_address);
    settings.node_note = normalize_node_note(settings.node_note);
    let content = toml::to_string_pretty(&SettingsFile::from(settings.clone()))
        .map_err(|error| error.to_string())?;
    let tmp_path = path.with_extension("toml.tmp");
    fs::write(&tmp_path, content).map_err(|error| error.to_string())?;
    replace_file_atomically(&tmp_path, &path)?;
    if let Some(state) = app.try_state::<crate::state::AppState>() {
        state.set_cached_settings(settings);
    }
    Ok(())
}
