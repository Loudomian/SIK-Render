use chrono::{DateTime, Datelike, Local, Timelike};
use regex::{Captures, Regex};
use serde::Serialize;
use std::fmt;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathKind {
    BlenderRender,
    BlenderFfmpeg,
    StandaloneFfmpeg,
}

impl PathKind {
    pub fn from_input(value: &str) -> Result<Self, String> {
        match value {
            "blender" => Ok(Self::BlenderRender),
            "blender-ffmpeg" => Ok(Self::BlenderFfmpeg),
            "standalone-ffmpeg" => Ok(Self::StandaloneFfmpeg),
            _ => Err(format!("unsupported path template kind: {value}")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TemplateContext {
    pub kind: PathKind,
    pub base_dir: Option<PathBuf>,
    pub blend_file_name: Option<String>,
    pub folder_name: Option<String>,
    pub frame_start: i32,
    pub frame_end: i32,
    pub now: DateTime<Local>,
    pub username: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplatePreview {
    pub resolved_path: Option<String>,
    pub errors: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum TemplateError {
    UnknownVariable(String),
    VariableNotAvailable(String),
    MissingValue(String),
    MissingBaseDir,
    FrameInDirectory,
    FrameInFfmpegPath,
}

const DEFAULT_FRAME_PADDING: usize = 6;
const MAX_FRAME_PADDING: usize = 12;

#[derive(Debug, Clone, Copy)]
enum ParsedVariable<'a> {
    Scalar(&'a str),
    Frame { padding: usize },
}

fn parse_variable(raw: &str) -> Option<ParsedVariable<'_>> {
    match raw {
        "year" | "month" | "day" | "hour" | "date" | "user" | "frameStart" | "frameEnd"
        | "blendFileName" | "folderName" => Some(ParsedVariable::Scalar(raw)),
        "frame" => Some(ParsedVariable::Frame {
            padding: DEFAULT_FRAME_PADDING,
        }),
        _ => raw
            .strip_prefix("frame:")
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| (1..=MAX_FRAME_PADDING).contains(value))
            .map(|padding| ParsedVariable::Frame { padding }),
    }
}

impl fmt::Display for TemplateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownVariable(name) => write!(f, "未知变量: {{{name}}}"),
            Self::VariableNotAvailable(name) => match name.as_str() {
                "blendFileName" => {
                    write!(f, "{{blendFileName}} 在当前路径中不可用")
                }
                "folderName" => {
                    write!(f, "{{folderName}} 在当前路径中不可用")
                }
                "frame" => {
                    write!(f, "{{frame}} 在当前路径中不可用")
                }
                _ => write!(f, "变量 {{{name}}} 在当前路径中不可用"),
            },
            Self::MissingValue(name) => match name.as_str() {
                "blendFileName" => {
                    write!(f, "选择 Blend 文件后才可以解析 {{blendFileName}}")
                }
                "folderName" => {
                    write!(f, "选择序列帧目录后才可以解析 {{folderName}}")
                }
                _ => write!(f, "变量 {{{name}}} 当前缺少可用值"),
            },
            Self::MissingBaseDir => write!(f, "当前路径缺少可用的基准目录"),
            Self::FrameInDirectory => write!(f, "{{frame}} 只能出现在文件名部分"),
            Self::FrameInFfmpegPath => write!(f, "{{frame}} 不能用于 FFmpeg 输出路径"),
        }
    }
}

fn username() -> String {
    std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| String::from("user"))
}

pub fn blend_file_name_from_path(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|value| value.to_str())
        .map(|value| value.to_string())
}

pub fn folder_name_from_source_path(path: &Path) -> Option<String> {
    let directory = if path.is_dir() {
        path
    } else {
        path.parent().unwrap_or(path)
    };

    directory
        .file_name()
        .and_then(|value| value.to_str())
        .map(|value| value.to_string())
}

pub fn default_context(
    kind: PathKind,
    base_dir: Option<PathBuf>,
    blend_file_name: Option<String>,
    folder_name: Option<String>,
    frame_start: i32,
    frame_end: i32,
) -> TemplateContext {
    TemplateContext {
        kind,
        base_dir,
        blend_file_name,
        folder_name,
        frame_start,
        frame_end,
        now: Local::now(),
        username: username(),
    }
}

pub fn preview_output_path(template: &str, context: &TemplateContext) -> TemplatePreview {
    match resolve_internal(template, context, true) {
        Ok(path) => TemplatePreview {
            resolved_path: Some(path.to_string_lossy().to_string()),
            errors: Vec::new(),
            notes: Vec::new(),
        },
        Err(ResolveOutcome::Errors(errors)) => TemplatePreview {
            resolved_path: None,
            errors: errors.iter().map(ToString::to_string).collect(),
            notes: Vec::new(),
        },
        Err(ResolveOutcome::Notes(notes)) => TemplatePreview {
            resolved_path: None,
            errors: Vec::new(),
            notes: notes.iter().map(ToString::to_string).collect(),
        },
    }
}

pub fn resolve_output_path(template: &str, context: &TemplateContext) -> Result<PathBuf, TemplateError> {
    match resolve_internal(template, context, false) {
        Ok(path) => Ok(path),
        Err(ResolveOutcome::Errors(errors)) => Err(errors.into_iter().next().unwrap_or(TemplateError::MissingBaseDir)),
        Err(ResolveOutcome::Notes(notes)) => Err(notes.into_iter().next().unwrap_or(TemplateError::MissingBaseDir)),
    }
}

enum ResolveOutcome {
    Errors(Vec<TemplateError>),
    Notes(Vec<TemplateError>),
}

fn normalize_path(path: PathBuf) -> PathBuf {
    let mut components = Vec::new();

    for component in path.components() {
        match component {
            Component::ParentDir => {
                if matches!(components.last(), Some(Component::Normal(_))) {
                    components.pop();
                } else {
                    components.push(component);
                }
            }
            Component::CurDir => {}
            other => components.push(other),
        }
    }

    let mut normalized = PathBuf::new();
    for component in components {
        normalized.push(component.as_os_str());
    }
    normalized
}

fn resolve_internal(
    template: &str,
    context: &TemplateContext,
    preview_mode: bool,
) -> Result<PathBuf, ResolveOutcome> {
    let variable_pattern = Regex::new(r"\{([^{}]+)\}").expect("valid regex");
    let mut errors = Vec::new();
    let mut notes = Vec::new();

    for capture in variable_pattern.captures_iter(template) {
        let Some(raw) = capture.get(1).map(|value| value.as_str()) else {
            continue;
        };

        match parse_variable(raw) {
            Some(ParsedVariable::Scalar(
                "year" | "month" | "day" | "hour" | "date" | "user" | "frameStart" | "frameEnd",
            )) => {}
            Some(ParsedVariable::Scalar("blendFileName")) => {
                if context.kind == PathKind::StandaloneFfmpeg {
                    errors.push(TemplateError::VariableNotAvailable(raw.to_string()));
                } else if context.blend_file_name.is_none() {
                    notes.push(TemplateError::MissingValue(raw.to_string()));
                }
            }
            Some(ParsedVariable::Scalar("folderName")) => {
                if context.kind != PathKind::StandaloneFfmpeg {
                    errors.push(TemplateError::VariableNotAvailable(raw.to_string()));
                } else if context.folder_name.is_none() {
                    notes.push(TemplateError::MissingValue(raw.to_string()));
                }
            }
            Some(ParsedVariable::Frame { .. }) => {
                if context.kind != PathKind::BlenderRender {
                    errors.push(TemplateError::FrameInFfmpegPath);
                }
            }
            _ => errors.push(TemplateError::UnknownVariable(raw.to_string())),
        }
    }

    if context.kind == PathKind::BlenderRender {
        let file_part_index = template
            .char_indices()
            .filter(|(_, ch)| *ch == '/' || *ch == '\\')
            .map(|(index, _)| index + 1)
            .last()
            .unwrap_or(0);
        for capture in variable_pattern.captures_iter(template) {
            let Some(raw) = capture.get(1).map(|value| value.as_str()) else {
                continue;
            };
            let Some(full_match) = capture.get(0) else {
                continue;
            };

            if matches!(parse_variable(raw), Some(ParsedVariable::Frame { .. }))
                && full_match.start() < file_part_index
            {
                errors.push(TemplateError::FrameInDirectory);
                break;
            }
        }
    }

    if !errors.is_empty() {
        return Err(ResolveOutcome::Errors(errors));
    }
    if preview_mode && !notes.is_empty() {
        return Err(ResolveOutcome::Notes(notes));
    }

    let resolved = variable_pattern
        .replace_all(template, |captures: &Captures| {
            let raw = captures.get(1).map(|value| value.as_str()).unwrap_or_default();
            match parse_variable(raw) {
                Some(ParsedVariable::Scalar("year")) => format!("{:04}", context.now.year()),
                Some(ParsedVariable::Scalar("month")) => format!("{:02}", context.now.month()),
                Some(ParsedVariable::Scalar("day")) => format!("{:02}", context.now.day()),
                Some(ParsedVariable::Scalar("hour")) => format!("{:02}", context.now.hour()),
                Some(ParsedVariable::Scalar("date")) => format!(
                    "{:04}{:02}{:02}",
                    context.now.year(),
                    context.now.month(),
                    context.now.day()
                ),
                Some(ParsedVariable::Scalar("user")) => context.username.clone(),
                Some(ParsedVariable::Scalar("frameStart")) => context.frame_start.to_string(),
                Some(ParsedVariable::Scalar("frameEnd")) => context.frame_end.to_string(),
                Some(ParsedVariable::Scalar("blendFileName")) => {
                    context.blend_file_name.clone().unwrap_or_else(|| captures[0].to_string())
                }
                Some(ParsedVariable::Scalar("folderName")) => {
                    context.folder_name.clone().unwrap_or_else(|| captures[0].to_string())
                }
                Some(ParsedVariable::Frame { padding }) => "#".repeat(padding),
                _ => captures[0].to_string(),
            }
        })
        .to_string();

    let resolved_path = PathBuf::from(&resolved);
    if resolved_path.is_absolute() {
        return Ok(normalize_path(resolved_path));
    }

    let Some(base_dir) = &context.base_dir else {
        return Err(ResolveOutcome::Notes(vec![TemplateError::MissingBaseDir]));
    };

    Ok(normalize_path(base_dir.join(resolved_path)))
}
