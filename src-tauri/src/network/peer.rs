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
            let peer = PeerInfo {
                node: node.clone(),
                jobs,
                queue_paused,
                connected: true,
            };
            state
                .peers
                .lock()
                .await
                .insert(peer_id.to_string(), peer.clone());
            let _ = app.emit("peer-discovered", PeerDiscoveredEvent { peer });
        }
        WsMessage::JobUpdated { job } => {
            let mut peers = state.peers.lock().await;
            if let Some(peer) = peers.get_mut(peer_id) {
                if let Some(index) = peer.jobs.iter().position(|item| item.id == job.id) {
                    peer.jobs[index] = job.clone();
                } else {
                    peer.jobs.push(job.clone());
                }
            }
            drop(peers);
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
    state.peers.lock().await.remove(&peer_id);
    let _ = app.emit("peer-lost", PeerLostEvent { node_id: peer_id });
}
