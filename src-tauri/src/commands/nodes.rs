use crate::network::server::{build_node_info, list_node_interfaces as load_interfaces};
use crate::network::types::{NodeInfo, NodeInterfaceInfo, NodeJobEvent, PeerInfo};
use crate::state::AppState;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn get_node_info(app: AppHandle, state: State<'_, AppState>) -> Result<NodeInfo, String> {
    Ok(build_node_info(&app, &state).await)
}

#[tauri::command]
pub async fn get_peers(state: State<'_, AppState>) -> Result<Vec<PeerInfo>, String> {
    let peers = state.peers.lock().await;
    let mut records = crate::network::peers::load_peer_records()
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|peer| (peer.node.id.clone(), peer))
        .collect::<std::collections::HashMap<_, _>>();

    for peer in peers.values().cloned() {
        records.insert(peer.node.id.clone(), peer);
    }

    let mut peers = records.into_values().collect::<Vec<_>>();
    peers.sort_by(|a, b| {
        b.connected
            .cmp(&a.connected)
            .then_with(|| {
                b.last_seen_at
                    .unwrap_or_default()
                    .cmp(&a.last_seen_at.unwrap_or_default())
            })
            .then_with(|| a.node.hostname.cmp(&b.node.hostname))
    });

    Ok(peers)
}

#[tauri::command]
pub fn list_node_interfaces() -> Result<Vec<NodeInterfaceInfo>, String> {
    Ok(load_interfaces())
}

#[tauri::command]
pub async fn get_node_job_events(
    node_id: String,
    job_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<NodeJobEvent>, String> {
    crate::network::events::load_node_job_events(&state.pool, &node_id, &job_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn forget_peer(node_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let removed = {
        let mut peers = state.peers.lock().await;
        if peers
            .get(&node_id)
            .map(|peer| peer.connected)
            .unwrap_or(false)
        {
            return Err("cannot forget a connected node".to_string());
        }
        peers.remove(&node_id)
    };
    let removed_label = removed.as_ref().map(|peer| {
        format!(
            "{} @ {}:{}",
            peer.node.hostname, peer.node.ip_address, peer.node.port
        )
    });

    crate::network::events::delete_node_events(&state.pool, &node_id)
        .await
        .map_err(|error| error.to_string())?;
    let had_record = crate::network::peers::load_peer_record(&node_id)
        .map_err(|error| error.to_string())?
        .is_some();
    if removed.is_some() || had_record {
        crate::network::peers::delete_peer_record(&node_id).map_err(|error| error.to_string())?;
        log::info!(
            "Forgot offline render node: {}{}",
            node_id,
            removed_label
                .as_deref()
                .map(|label| format!(" ({label})"))
                .unwrap_or_default()
        );
    } else {
        log::info!("Forget offline render node requested, no local record found: {node_id}");
    }
    Ok(())
}
