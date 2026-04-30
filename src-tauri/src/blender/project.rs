use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

const PROJECT_SETTINGS_MARKER: &str = "__SIK_PROJECT_SETTINGS__";
const CAMERA_MARKER_RANGE_MARKER: &str = "__SIK_CAMERA_MARKER_RANGE__";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlendProjectSettings {
    pub frame_start: i32,
    pub frame_end: i32,
    pub output_path: String,
    pub output_format: String,
    pub engine: String,
    pub resolution_x: i32,
    pub resolution_y: i32,
    pub fps: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraMarkerRange {
    pub frame_start: i32,
    pub frame_end: i32,
    pub marker_name: Option<String>,
    pub camera_name: Option<String>,
}

pub async fn inspect_project_with_timeout(
    blender_executable: &Path,
    blend_file: &Path,
    timeout_seconds: u64,
) -> Result<BlendProjectSettings> {
    let script = r#"
import bpy, json
scene = bpy.context.scene
payload = {
    "frameStart": int(scene.frame_start),
    "frameEnd": int(scene.frame_end),
    "outputPath": bpy.path.abspath(scene.render.filepath or ""),
    "outputFormat": scene.render.image_settings.file_format,
    "engine": scene.render.engine,
    "resolutionX": int(scene.render.resolution_x),
    "resolutionY": int(scene.render.resolution_y),
    "fps": float(scene.render.fps / scene.render.fps_base),
}
print("__SIK_PROJECT_SETTINGS__" + json.dumps(payload))
"#
    .trim();

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_seconds.max(1)),
        crate::blender::command::inspect_project_command(blender_executable, blend_file, script)
            .output(),
    )
    .await
    .map_err(|_| {
        anyhow!(
            "Blender inspect timed out after {}s",
            timeout_seconds.max(1)
        )
    })?
    .with_context(|| {
        format!(
            "failed to launch Blender at {}",
            blender_executable.display()
        )
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Blender inspect failed: {}", stderr.trim()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let payload = stdout
        .lines()
        .find_map(|line| line.strip_prefix(PROJECT_SETTINGS_MARKER))
        .ok_or_else(|| anyhow!("Blender did not return project settings"))?;

    serde_json::from_str(payload).context("failed to parse Blender project settings")
}

pub async fn inspect_camera_marker_range_with_timeout(
    blender_executable: &Path,
    blend_file: &Path,
    frame: i32,
    fallback_frame_start: i32,
    fallback_frame_end: i32,
    timeout_seconds: u64,
) -> Result<CameraMarkerRange> {
    let script = format!(
        r#"
import bpy, json
scene = bpy.context.scene
target = int({frame})
fallback_start = int({fallback_frame_start})
fallback_end = int({fallback_frame_end})
markers = sorted(
    [m for m in scene.timeline_markers if getattr(m, "camera", None) is not None],
    key=lambda m: int(m.frame),
)
active = None
next_marker = None
for marker in markers:
    if int(marker.frame) <= target:
        active = marker
    elif int(marker.frame) > target:
        next_marker = marker
        break
if active is None and markers:
    active = markers[0]
    next_marker = markers[1] if len(markers) > 1 else None
start = int(active.frame) if active is not None else fallback_start
end = int(next_marker.frame) - 1 if next_marker is not None else fallback_end
# Clamp marker-derived shot bounds to the current render job range.
start = max(fallback_start, min(start, fallback_end))
end = max(start, min(end, fallback_end))
payload = {{
    "frameStart": start,
    "frameEnd": end,
    "markerName": active.name if active is not None else None,
    "cameraName": active.camera.name if active is not None and active.camera is not None else None,
}}
print("{marker}" + json.dumps(payload))
"#,
        frame = frame,
        fallback_frame_start = fallback_frame_start,
        fallback_frame_end = fallback_frame_end,
        marker = CAMERA_MARKER_RANGE_MARKER,
    );

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_seconds.max(1)),
        crate::blender::command::inspect_project_command(blender_executable, blend_file, &script)
            .output(),
    )
    .await
    .map_err(|_| {
        anyhow!(
            "Blender camera marker inspect timed out after {}s",
            timeout_seconds.max(1)
        )
    })?
    .with_context(|| {
        format!(
            "failed to launch Blender at {}",
            blender_executable.display()
        )
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "Blender camera marker inspect failed: {}",
            stderr.trim()
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let payload = stdout
        .lines()
        .find_map(|line| line.strip_prefix(CAMERA_MARKER_RANGE_MARKER))
        .ok_or_else(|| anyhow!("Blender did not return camera marker range"))?;

    serde_json::from_str(payload).context("failed to parse Blender camera marker range")
}

pub fn normalize_versions(
    mut versions: Vec<crate::blender::discovery::BlenderInstall>,
) -> Vec<crate::blender::discovery::BlenderInstall> {
    let mut seen = BTreeSet::new();
    versions.retain(|install| seen.insert(install.executable.clone()));
    versions.sort_by(|a, b| {
        b.version
            .cmp(&a.version)
            .then_with(|| a.executable.cmp(&b.executable))
    });
    versions
}
