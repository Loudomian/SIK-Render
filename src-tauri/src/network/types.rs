use crate::queue::job::{RenderJob, RenderMode};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const SERVICE_TYPE: &str = "_sik-render._tcp.local.";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeInfo {
    pub id: String,
    pub hostname: String,
    pub note: String,
    pub version: String,
    pub ip_address: String,
    pub port: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteJobSnapshot {
    pub id: String,
    pub job_number: i32,
    pub name: String,
    pub note: Option<String>,
    pub status: crate::queue::job::JobStatus,
    pub render_mode: RenderMode,
    pub output_format: String,
    pub output_path: PathBuf,
    pub blend_file: PathBuf,
    pub original_frame_start: i32,
    pub original_frame_end: i32,
    pub frame_start: i32,
    pub frame_end: i32,
    pub crash_count: i32,
    pub shadow_resolution_scale_override: Option<f32>,
    pub last_rendered_frame: Option<i32>,
    pub preview_width: Option<i32>,
    pub preview_height: Option<i32>,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
    #[allow(dead_code)]
    #[serde(default, skip_serializing)]
    pub current_frame: Option<i32>,
    #[allow(dead_code)]
    #[serde(default, skip_serializing)]
    pub total_frames: Option<i32>,
    #[allow(dead_code)]
    #[serde(default, skip_serializing)]
    pub time_elapsed: Option<f32>,
    #[allow(dead_code)]
    #[serde(default, skip_serializing)]
    pub remaining_secs: Option<f32>,
}

impl RemoteJobSnapshot {
    pub fn from_job(job: &RenderJob) -> Self {
        Self {
            id: job.id.clone(),
            job_number: job.job_number,
            name: job.name.clone(),
            note: job.note.clone(),
            status: job.status.clone(),
            render_mode: job.render_mode.clone(),
            output_format: job.output_format.clone(),
            output_path: job.output_path.clone(),
            blend_file: job.blend_file.clone(),
            original_frame_start: job.original_frame_start,
            original_frame_end: job.original_frame_end,
            frame_start: job.frame_start,
            frame_end: job.frame_end,
            crash_count: job.crash_count,
            shadow_resolution_scale_override: job.shadow_resolution_scale_override,
            last_rendered_frame: job.last_rendered_frame,
            preview_width: job.preview_width,
            preview_height: job.preview_height,
            created_at: job.created_at,
            started_at: job.started_at,
            finished_at: job.finished_at,
            current_frame: job.current_frame,
            total_frames: job.total_frames,
            time_elapsed: job.time_elapsed,
            remaining_secs: job.remaining_secs,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerInfo {
    pub node: NodeInfo,
    pub jobs: Vec<RemoteJobSnapshot>,
    pub queue_paused: bool,
    pub connected: bool,
    #[serde(default)]
    pub first_seen_at: Option<i64>,
    #[serde(default)]
    pub last_seen_at: Option<i64>,
    #[serde(default)]
    pub last_connected_at: Option<i64>,
    #[serde(default)]
    pub last_disconnected_at: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeInterfaceInfo {
    pub name: String,
    pub ip_address: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    Hello {
        #[serde(default)]
        protocol_version: u32,
        node: NodeInfo,
        jobs: Vec<RemoteJobSnapshot>,
        queue_paused: bool,
    },
    JobUpdated {
        job: RemoteJobSnapshot,
    },
    QueueState {
        paused: bool,
    },
    Progress {
        job_id: String,
        frame: u32,
        total_frames: u32,
        time_elapsed: f32,
        memory_mb: f32,
        remaining_secs: Option<f32>,
    },
    Log {
        job_id: String,
        line: String,
    },
    Ping,
    Pong,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerDiscoveredEvent {
    pub peer: PeerInfo,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerLostEvent {
    pub node_id: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerJobUpdatedEvent {
    pub node_id: String,
    pub job: RemoteJobSnapshot,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerQueueStateEvent {
    pub node_id: String,
    pub paused: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerProgressEvent {
    pub node_id: String,
    pub job_id: String,
    pub frame: u32,
    pub total_frames: u32,
    pub time_elapsed: f32,
    pub memory_mb: f32,
    pub remaining_secs: Option<f32>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerLogEvent {
    pub node_id: String,
    pub job_id: String,
    pub line: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodeJobEventKind {
    JobDiscovered,
    StatusChanged,
    CrashRetry,
    ShadowRecovery,
    RangeChanged,
    NodeConnected,
    NodeDisconnected,
    Progress,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeJobEventLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeJobEvent {
    pub id: String,
    pub node_id: String,
    pub job_id: String,
    pub timestamp: i64,
    pub kind: NodeJobEventKind,
    pub level: NodeJobEventLevel,
    pub title: String,
    pub message: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerJobEventPayload {
    pub event: NodeJobEvent,
}
