use crate::queue::job::RenderJob;
use serde::{Deserialize, Serialize};

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
pub struct PeerInfo {
    pub node: NodeInfo,
    pub jobs: Vec<RenderJob>,
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
        node: NodeInfo,
        jobs: Vec<RenderJob>,
        queue_paused: bool,
    },
    JobUpdated {
        job: RenderJob,
    },
    QueueState {
        paused: bool,
    },
    Progress {
        job_id: String,
        frame: i32,
        total_frames: i32,
        time_elapsed: f32,
        memory_mb: f32,
        remaining_secs: Option<f32>,
    },
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
    pub job: RenderJob,
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
    pub frame: i32,
    pub total_frames: i32,
    pub time_elapsed: f32,
    pub memory_mb: f32,
    pub remaining_secs: Option<f32>,
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
