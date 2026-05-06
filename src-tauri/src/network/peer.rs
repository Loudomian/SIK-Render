use crate::network::types::{
    PeerDiscoveredEvent, PeerInfo, PeerJobUpdatedEvent, PeerLostEvent, PeerProgressEvent,
    PeerQueueStateEvent, WsMessage,
};
use crate::state::AppState;
use futures_util::StreamExt;
use tauri::{AppHandle, Emitter};
use tokio_tungstenite::connect_async;

pub fn spawn_connection(app: AppHandle, state: AppState, peer_id: String, ip: String, port: u16) {
    tauri::async_runtime::spawn(async move {
        {
            let peers = state.peers.lock().await;
            if peers.contains_key(&peer_id) {
                return;
            }
        }

        let url = format!("ws://{ip}:{port}/ws");
        log::info!("Connecting to peer {peer_id} @ {url}");

        match connect_async(&url).await {
            Ok((ws_stream, _)) => {
                let (_, mut read) = ws_stream.split();

                while let Some(msg) = read.next().await {
                    match msg {
                        Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                            match serde_json::from_str::<WsMessage>(&text) {
                                Ok(ws_msg) => {
                                    handle_message(app.clone(), state.clone(), &peer_id, ws_msg)
                                        .await;
                                }
                                Err(error) => {
                                    log::warn!("WS parse error from {peer_id}: {error}");
                                }
                            }
                        }
                        Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => break,
                        Err(error) => {
                            log::warn!("WS error from {peer_id}: {error}");
                            break;
                        }
                        _ => {}
                    }
                }

                remove(app.clone(), state.clone(), peer_id).await;
            }
            Err(error) => {
                log::warn!("Cannot connect to peer {peer_id}: {error}");
            }
        }
    });
}

async fn handle_message(app: AppHandle, state: AppState, peer_id: &str, msg: WsMessage) {
    match msg {
        WsMessage::Hello {
            node,
            jobs,
            queue_paused,
        } => {
            let incoming_peer = PeerInfo {
                node: node.clone(),
                jobs,
                queue_paused,
                connected: true,
                first_seen_at: None,
                last_seen_at: None,
                last_connected_at: None,
                last_disconnected_at: None,
            };
            let peer = {
                let mut peers = state.peers.lock().await;
                let previous = peers.get(peer_id);
                let peer = crate::network::peers::peer_record_for_connect(incoming_peer, previous);
                peers.insert(peer_id.to_string(), peer.clone());
                peer
            };
            if let Err(error) = crate::network::peers::save_peer_record(&peer) {
                log::warn!("Failed to persist peer {peer_id}: {error}");
            }
            crate::network::events::record_node_connected_events(&app, peer_id, &peer.jobs);
            for job in &peer.jobs {
                crate::network::events::seed_job_events(&app, peer_id, job);
            }
            let _ = app.emit("peer-discovered", PeerDiscoveredEvent { peer });
        }
        WsMessage::JobUpdated { job } => {
            let mut updated_peer = None;
            let mut previous_job = None;
            let mut should_persist = false;
            let mut peers = state.peers.lock().await;
            if let Some(peer) = peers.get_mut(peer_id) {
                if let Some(index) = peer.jobs.iter().position(|item| item.id == job.id) {
                    previous_job = Some(peer.jobs[index].clone());
                    should_persist = previous_job
                        .as_ref()
                        .map(|previous| previous.status != job.status)
                        .unwrap_or(false);
                    peer.jobs[index] = job.clone();
                } else {
                    peer.jobs.push(job.clone());
                }
                peer.last_seen_at = Some(chrono::Utc::now().timestamp_millis());
                updated_peer = Some(peer.clone());
            }
            drop(peers);
            crate::network::events::record_job_snapshot_events(
                &app,
                peer_id,
                previous_job.as_ref(),
                &job,
            );
            if should_persist {
                if let Some(peer) = updated_peer {
                    if let Err(error) = crate::network::peers::save_peer_record(&peer) {
                        log::warn!("Failed to persist peer {peer_id}: {error}");
                    }
                }
            }
            let _ = app.emit(
                "peer-job-updated",
                PeerJobUpdatedEvent {
                    node_id: peer_id.to_string(),
                    job,
                },
            );
        }
        WsMessage::QueueState { paused } => {
            let mut peers = state.peers.lock().await;
            if let Some(peer) = peers.get_mut(peer_id) {
                peer.queue_paused = paused;
            }
            drop(peers);
            let _ = app.emit(
                "peer-queue-state",
                PeerQueueStateEvent {
                    node_id: peer_id.to_string(),
                    paused,
                },
            );
        }
        WsMessage::Progress {
            job_id,
            frame,
            total_frames,
            time_elapsed,
            memory_mb,
            remaining_secs,
        } => {
            let _ = app.emit(
                "peer-progress",
                PeerProgressEvent {
                    node_id: peer_id.to_string(),
                    job_id,
                    frame,
                    total_frames,
                    time_elapsed,
                    memory_mb,
                    remaining_secs,
                },
            );
        }
    }
}

pub async fn remove(app: AppHandle, state: AppState, peer_id: String) {
    let removed = state.peers.lock().await.remove(&peer_id);
    if let Some(peer) = removed {
        let peer = crate::network::peers::peer_record_for_disconnect(peer);
        crate::network::events::record_node_disconnected_events(&app, &peer_id, &peer.jobs);
        if let Err(error) = crate::network::peers::save_peer_record(&peer) {
            log::warn!("Failed to persist peer disconnect {peer_id}: {error}");
        }
    }
    let _ = app.emit("peer-lost", PeerLostEvent { node_id: peer_id });
}
