use crate::path_template::{
    blend_file_name_from_path, default_context, folder_name_from_source_path, preview_output_path,
    PathKind,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct PreviewOutputPathTemplatePayload {
    pub kind: String,
    pub template: String,
    pub blend_file: Option<String>,
    pub source_path: Option<String>,
    pub frame_start: i32,
    pub frame_end: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputPathTemplatePreview {
    pub resolved_path: Option<String>,
    pub errors: Vec<String>,
    pub notes: Vec<String>,
}

#[tauri::command]
pub async fn preview_output_path_template(
    payload: PreviewOutputPathTemplatePayload,
) -> Result<OutputPathTemplatePreview, String> {
    let kind = PathKind::from_input(payload.kind.trim())?;
    let blend_file = payload.blend_file.as_deref().map(PathBuf::from);
    let source_path = payload.source_path.as_deref().map(PathBuf::from);
    let base_dir = match kind {
        PathKind::BlenderRender | PathKind::BlenderFfmpeg => blend_file
            .as_ref()
            .and_then(|path| path.parent().map(|value| value.to_path_buf())),
        PathKind::StandaloneFfmpeg => source_path.as_ref().map(|path| {
            if path.is_dir() {
                path.clone()
            } else {
                path.parent().unwrap_or(path).to_path_buf()
            }
        }),
    };
    let blend_file_name = blend_file.as_deref().and_then(blend_file_name_from_path);
    let folder_name = source_path
        .as_deref()
        .and_then(folder_name_from_source_path);

    let preview = preview_output_path(
        payload.template.trim(),
        &default_context(
            kind,
            base_dir,
            blend_file_name,
            folder_name,
            payload.frame_start,
            payload.frame_end,
        ),
    );

    Ok(OutputPathTemplatePreview {
        resolved_path: preview.resolved_path,
        errors: preview.errors,
        notes: preview.notes,
    })
}
