use crate::network::types::{NodeInfo, NodeInterfaceInfo, WsMessage};
use crate::queue::job::{DbRenderJob, RenderJob};
use crate::state::AppState;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path as AxumPath, State as AxumState,
    },
    http::{header, StatusCode},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use std::{
    collections::HashSet,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
};
use tauri::AppHandle;

pub fn start(app: AppHandle, state: AppState) {
    tauri::async_runtime::spawn(async move {
        let port = node_port(&state);
        let router = Router::new()
            .route("/api/node", get(handle_node_info))
            .route("/api/jobs", get(handle_jobs))
            .route("/api/jobs/{id}/preview", get(handle_job_preview))
            .route("/api/queue", get(handle_queue))
            .route("/ws", get(handle_ws_upgrade))
            .with_state((app, state));

        let bind_addr = SocketAddr::from(([0, 0, 0, 0], port));
        let listener = match tokio::net::TcpListener::bind(bind_addr).await {
            Ok(listener) => listener,
            Err(error) => {
                log::error!("Node server failed to bind {bind_addr}: {error}");
                return;
            }
        };

        log::info!("Node server listening on {bind_addr}");
        if let Err(error) = axum::serve(listener, router).await {
            log::error!("Node server exited: {error}");
        }
    });
}

async fn handle_node_info(
    AxumState((app, state)): AxumState<(AppHandle, AppState)>,
) -> Json<NodeInfo> {
    Json(build_node_info(&app, &state).await)
}

async fn handle_jobs(
    AxumState((_, state)): AxumState<(AppHandle, AppState)>,
) -> Json<Vec<RenderJob>> {
    Json(load_all_jobs(&state).await.unwrap_or_default())
}

async fn handle_queue(
    AxumState((_, state)): AxumState<(AppHandle, AppState)>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "paused": state.is_queue_paused() }))
}

async fn handle_job_preview(
    AxumPath(id): AxumPath<String>,
    AxumState((_, state)): AxumState<(AppHandle, AppState)>,
) -> impl IntoResponse {
    let Ok(job) = crate::queue::scheduler::load_job(&state.pool, &id).await else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let Some(path) = resolve_preview_path(&job) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let Ok(bytes) = tokio::fs::read(&path).await else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let content_type = content_type_for_path(&path);
    ([(header::CONTENT_TYPE, content_type)], bytes).into_response()
}

async fn handle_ws_upgrade(
    ws: WebSocketUpgrade,
    AxumState((app, state)): AxumState<(AppHandle, AppState)>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, app, state))
}

async fn handle_ws(socket: WebSocket, app: AppHandle, state: AppState) {
    let mut rx = state.ws_broadcaster.subscribe();
    let (mut sender, _reader) = socket.split();

    let hello = WsMessage::Hello {
        node: build_node_info(&app, &state).await,
        jobs: load_all_jobs(&state).await.unwrap_or_default(),
        queue_paused: state.is_queue_paused(),
    };

    if let Ok(text) = serde_json::to_string(&hello) {
        if sender.send(Message::Text(text.into())).await.is_err() {
            return;
        }
    }

    loop {
        match rx.recv().await {
            Ok(msg) => {
                let Ok(text) = serde_json::to_string(&msg) else {
                    continue;
                };
                if sender.send(Message::Text(text.into())).await.is_err() {
                    break;
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
        }
    }
}

pub async fn build_node_info(_app: &AppHandle, state: &AppState) -> NodeInfo {
    let settings = state.cached_settings().unwrap_or_default();
    NodeInfo {
        id: (*state.node_id).clone(),
        hostname: hostname(),
        note: settings.node_note,
        version: env!("CARGO_PKG_VERSION").to_string(),
        ip_address: select_node_ip(state).to_string(),
        port: node_port(state),
    }
}

pub fn node_port(state: &AppState) -> u16 {
    state.cached_settings().unwrap_or_default().node_port
}

pub fn select_node_ip(state: &AppState) -> Ipv4Addr {
    let settings = state.cached_settings().unwrap_or_default();
    let preferred = settings.node_interface_address.trim().to_string();

    let candidates = list_node_interfaces()
        .into_iter()
        .filter_map(|interface| interface.ip_address.parse::<Ipv4Addr>().ok())
        .collect::<Vec<_>>();

    if let Some(ip) = candidates
        .iter()
        .copied()
        .find(|ip| matches_configured_network(*ip, &preferred))
    {
        return ip;
    }

    if let Some(ip) = candidates.first().copied() {
        return ip;
    }

    match local_ip_address::local_ip() {
        Ok(IpAddr::V4(ip)) => ip,
        _ => Ipv4Addr::LOCALHOST,
    }
}

pub fn list_node_interfaces() -> Vec<NodeInterfaceInfo> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();

    if let Ok(interfaces) = local_ip_address::list_afinet_netifas() {
        for (name, ip) in interfaces {
            let IpAddr::V4(ip) = ip else {
                continue;
            };

            if !is_valid_node_interface(&name, ip) || !seen.insert(ip) {
                continue;
            }

            result.push(NodeInterfaceInfo {
                name,
                ip_address: ip.to_string(),
            });
        }
    }

    if result.is_empty() {
        if let Ok(IpAddr::V4(ip)) = local_ip_address::local_ip() {
            if !ip.is_loopback() {
                result.push(NodeInterfaceInfo {
                    name: String::from("默认网卡"),
                    ip_address: ip.to_string(),
                });
            }
        }
    }

    result
}

fn is_valid_node_interface(name: &str, ip: Ipv4Addr) -> bool {
    if ip.is_loopback() || ip.is_unspecified() || ip.is_link_local() {
        return false;
    }

    !is_virtual_interface_name(name)
}

fn is_virtual_interface_name(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    [
        "bluetooth",
        "docker",
        "hyper-v",
        "isatap",
        "npcap",
        "pseudo",
        "tailscale",
        "tap",
        "teredo",
        "tunnel",
        "tun",
        "vethernet",
        "virtual",
        "virtualbox",
        "vmware",
        "vbox",
        "wsl",
        "zerotier",
    ]
    .iter()
    .any(|keyword| name.contains(keyword))
}

fn matches_configured_network(candidate: Ipv4Addr, preferred: &str) -> bool {
    if preferred.is_empty() {
        return false;
    }

    if let Ok(configured) = preferred.parse::<Ipv4Addr>() {
        let candidate = candidate.octets();
        let configured = configured.octets();
        return candidate[..3] == configured[..3];
    }

    candidate.to_string().starts_with(preferred)
}

fn hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| String::from("unknown"))
}

fn resolve_preview_path(job: &RenderJob) -> Option<PathBuf> {
    if job.render_mode.is_quick_mp4() {
        if let Ok(path) = crate::app_paths::job_preview_image_path(job.job_number, &job.id) {
            if path.exists() {
                return Some(path);
            }
        }
        return None;
    }

    match job.output_format.to_ascii_uppercase().as_str() {
        "OPEN_EXR" | "EXR" => return None,
        _ => {}
    }

    if let Some(last_frame) = job.last_rendered_frame {
        if let Some(path) = crate::blender::process::frame_filename(
            &job.output_path,
            last_frame,
            &job.output_format,
        ) {
            if path.exists() {
                return Some(path);
            }
        }
    }

    crate::commands::blender::get_last_rendered_frame(
        job.output_path.to_string_lossy().to_string(),
        job.output_format.clone(),
        job.frame_start,
        job.frame_end,
    )
    .map(PathBuf::from)
    .filter(|path| path.exists())
}

fn content_type_for_path(path: &std::path::Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "gif" => "image/gif",
        "mp4" => "video/mp4",
        "m4v" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "mkv" => "video/x-matroska",
        _ => "application/octet-stream",
    }
}

async fn load_all_jobs(state: &AppState) -> anyhow::Result<Vec<RenderJob>> {
    let rows = sqlx::query_as::<_, DbRenderJob>(
        "SELECT id, job_number, name, note, crash_count,
                auto_transcode_mp4, transcode_name_override, transcode_fps_override,
                transcode_output_path_override, transcode_crf_override,
                transcode_preset_override, transcode_frame_start_override,
                transcode_frame_end_override, fps, blend_file, blender_exec,
                output_path, output_format, render_mode, shadow_resolution_scale_override,
                original_frame_start, original_frame_end, frame_start, frame_end,
                preview_width, preview_height, resume_from_existing,
                status, priority, created_at, started_at, finished_at,
                current_frame, total_frames, last_rendered_frame,
                time_elapsed, remaining_secs
         FROM jobs ORDER BY priority ASC, created_at ASC",
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(rows.into_iter().map(RenderJob::from).collect())
}
