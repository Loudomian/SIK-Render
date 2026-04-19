use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeStatus {
    Online,
    Busy,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderNode {
    pub id: String,
    pub name: String,
    pub address: String,
    pub port: u16,
    pub status: NodeStatus,
    pub is_local: bool,
}

/// Placeholder: returns local node only. LAN discovery (mdns-sd) to be added in P3.
pub fn list_nodes() -> Vec<RenderNode> {
    vec![RenderNode {
        id: "local".into(),
        name: "This Machine".into(),
        address: "127.0.0.1".into(),
        port: 0,
        status: NodeStatus::Online,
        is_local: true,
    }]
}
