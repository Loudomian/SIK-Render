use crate::network::server::{build_node_info, list_node_interfaces as load_interfaces};
use crate::network::types::{NodeInfo, NodeInterfaceInfo, PeerInfo};
use crate::state::AppState;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn get_node_info(app: AppHandle, state: State<'_, AppState>) -> Result<NodeInfo, String> {
    Ok(build_node_info(&app, &state).await)
}

#[tauri::command]
pub async fn get_peers(state: State<'_, AppState>) -> Result<Vec<PeerInfo>, String> {
    let peers = state.peers.lock().await;
    Ok(peers.values().cloned().collect())
}

#[tauri::command]
pub fn list_node_interfaces() -> Result<Vec<NodeInterfaceInfo>, String> {
    Ok(load_interfaces())
}
