use crate::network::types::{
    PeerDiscoveredEvent, PeerInfo, PeerJobUpdatedEvent, PeerLogEvent, PeerLostEvent,
    PeerProgressEvent, PeerQueueStateEvent, WsMessage,
};
use crate::state::AppState;
use futures_util::StreamExt;
use tauri::{AppHandle, Emitter};
use tokio_tungstenite::connect_async;

pub fn spawn_connection(app: AppHandle, state: AppState, peer_id: String, ip: String, port: u16) {
    tauri::async_runtime::spawn(async move {
        const MAX_RETRIES: u32 = 6;
        let mut delay_secs = 5u64;

        for attempt in 0..=MAX_RETRIES {
            if attempt == 0 {
                let peers = state.peers.lock().await;
                if peers.contains_key(&peer_id) {
                    return;
                }
            } else {
                tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
                delay_secs = (delay_secs * 2).min(120);
                match crate::network::peers::load_peer_record(&peer_id) {
                    Ok(None) => {
                        log::info!("Peer {peer_id} was forgotten, stopping reconnect");
                        return;
                    }
                    Ok(Some(_)) => {}
                    Err(error) => {
                        log::warn!("Failed to inspect peer record {peer_id}: {error}");
                    }
                }
            }

            let url = format!("ws://{ip}:{port}/ws");
            log::info!("Connecting to peer {peer_id} @ {url} (attempt {attempt})");

            match connect_async(&url).await {
                Ok((ws_stream, _)) => {
                    delay_secs = 5;
                    let (_, mut read) = ws_stream.split();

                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                                match serde_json::from_str::<WsMessage>(&text) {
                                    Ok(ws_msg) => {
                                        handle_message(
                                            app.clone(),
                                            state.clone(),
                                            &peer_id,
                                            ws_msg,
                                        )
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

                    remove(app.clone(), state.clone(), peer_id.clone()).await;
                    log::info!("Peer {peer_id} disconnected, will retry");
                }
                Err(error) => {
                    log::warn!("Cannot connect to peer {peer_id} (attempt {attempt}): {error}");
                }
            }
        }

        log::info!("Giving up reconnecting to peer {peer_id} after {MAX_RETRIES} attempts");
    });
}

async fn handle_message(app: AppHandle, state: AppState, peer_id: &str, msg: WsMessage) {
    match msg {
        WsMessage::Hello {
            protocol_version: _,
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
            crate::network::events::record_node_connected_events(
                &state.pool,
                &app,
                peer_id,
                &peer.jobs,
            )
            .await;
            for job in &peer.jobs {
                crate::network::events::seed_job_events(&state.pool, &app, peer_id, job).await;
            }
            let pool = state.pool.clone();
            let node_id = peer_id.to_string();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = crate::network::events::trim_node_events(&pool, &node_id).await
                {
                    log::warn!("Failed to trim node events {node_id}: {error}");
                }
            });
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
                &state.pool,
                &app,
                peer_id,
                previous_job.as_ref(),
                &job,
            )
            .await;
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
            {
                let mut peers = state.peers.lock().await;
                if let Some(peer) = peers.get_mut(peer_id) {
                    if let Some(job) = peer.jobs.iter_mut().find(|job| job.id == job_id) {
                        job.status = crate::queue::job::JobStatus::Running;
                        job.current_frame = Some(frame as i32);
                        job.total_frames = Some(total_frames as i32);
                        if time_elapsed > 0.0 {
                            job.time_elapsed = Some(time_elapsed);
                        }
                        if remaining_secs.map(|value| value > 0.0).unwrap_or(false) {
                            job.remaining_secs = remaining_secs;
                        } else if frame >= total_frames {
                            job.remaining_secs = Some(0.0);
                        }
                        peer.last_seen_at = Some(chrono::Utc::now().timestamp_millis());
                    }
                }
            }
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
        WsMessage::Log { job_id, line } => {
            let _ = app.emit(
                "peer-log",
                PeerLogEvent {
                    node_id: peer_id.to_string(),
                    job_id,
                    line,
                },
            );
        }
        WsMessage::Ping => {}
        WsMessage::Pong => {}
    }
}

pub async fn remove(app: AppHandle, state: AppState, peer_id: String) {
    let removed = state.peers.lock().await.remove(&peer_id);
    if let Some(peer) = removed {
        let peer = crate::network::peers::peer_record_for_disconnect(peer);
        crate::network::events::record_node_disconnected_events(
            &state.pool,
            &app,
            &peer_id,
            &peer.jobs,
        )
        .await;
        if let Err(error) = crate::network::peers::save_peer_record(&peer) {
            log::warn!("Failed to persist peer disconnect {peer_id}: {error}");
        }
    }
    let _ = app.emit("peer-lost", PeerLostEvent { node_id: peer_id });
}
