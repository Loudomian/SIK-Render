use crate::network::types::{NodeInfo, PeerInfo, RemoteJobSnapshot};
use anyhow::{Context, Result};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn peer_file_path(node_id: &str) -> Result<PathBuf> {
    Ok(crate::network::file_names::node_file_path(
        &crate::app_paths::node_peers_dir()?,
        node_id,
        "toml",
    ))
}

fn peer_file_path_for_peer(peer: &PeerInfo) -> Result<PathBuf> {
    Ok(crate::network::file_names::node_file_path_for_info(
        &crate::app_paths::node_peers_dir()?,
        &peer.node.hostname,
        &peer.node.ip_address,
        &peer.node.id,
        "toml",
    ))
}

fn peer_file_candidates(node_id: &str) -> Result<Vec<PathBuf>> {
    Ok(crate::network::file_names::node_file_candidates(
        &crate::app_paths::node_peers_dir()?,
        node_id,
        "toml",
    ))
}

#[derive(Serialize, Deserialize)]
struct RemotePeerToml {
    node: PeerNodeSection,
    connection: PeerConnectionSection,
    jobs: Vec<RemoteJobToml>,
}

#[derive(Serialize, Deserialize)]
struct PeerNodeSection {
    id: String,
    hostname: String,
    version: String,
    ip_address: String,
    port: u16,
    note: String,
}

#[derive(Serialize, Deserialize)]
struct PeerConnectionSection {
    #[serde(skip_serializing_if = "Option::is_none")]
    first_seen_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_seen_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_connected_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_disconnected_at: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct RemoteJobToml {
    job: RemoteJobJobSection,
    source: RemoteJobSourceSection,
    output: RemoteJobOutputSection,
    result: RemoteJobResultSection,
    timing: RemoteJobTimingSection,
}

#[derive(Serialize, Deserialize)]
struct RemoteJobJobSection {
    id: String,
    number: i32,
    name: String,
    note: String,
}

#[derive(Serialize, Deserialize)]
struct RemoteJobSourceSection {
    blend_file: String,
}

#[derive(Serialize, Deserialize)]
struct RemoteJobOutputSection {
    path: String,
    format: String,
    render_mode: String,
    frame_start: i32,
    frame_end: i32,
    original_frame_start: i32,
    original_frame_end: i32,
}

#[derive(Serialize, Deserialize)]
struct RemoteJobResultSection {
    status: String,
    crash_count: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_rendered_frame: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shadow_resolution_scale_override: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    preview_width: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    preview_height: Option<i32>,
}

#[derive(Serialize, Deserialize)]
struct RemoteJobTimingSection {
    created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    finished_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    duration_secs: Option<i64>,
}

pub fn load_peer_records() -> Result<Vec<PeerInfo>> {
    let dir = crate::app_paths::node_peers_dir()?;
    let mut peers = std::collections::HashMap::<String, (PeerInfo, PathBuf)>::new();

    for entry in std::fs::read_dir(&dir)
        .with_context(|| format!("failed to read node peers directory {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file()
            || path
                .extension()
                .and_then(|value| value.to_str())
                .map(|value| !value.eq_ignore_ascii_case("toml"))
                .unwrap_or(true)
        {
            continue;
        }

        match std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read node peer record {}", path.display()))
            .and_then(|content| parse_peer_record_content(&path, &content)) {
            Ok(mut peer) => {
                peer.connected = false;
                let id = peer.node.id.clone();
                match peers.get(&id) {
                    Some((existing, _))
                        if existing.last_seen_at.unwrap_or_default()
                            >= peer.last_seen_at.unwrap_or_default() => {}
                    _ => {
                        peers.insert(id, (peer, path));
                    }
                }
            }
            Err(error) => {
                log::warn!("{error}");
            }
        }
    }

    for (peer, current_path) in peers.values() {
        if let Err(error) = remove_stale_peer_files(peer, current_path) {
            log::warn!("Failed to remove stale peer records for {}: {error}", peer.node.id);
        }
    }

    let mut peers = peers
        .into_values()
        .map(|(peer, _)| peer)
        .collect::<Vec<_>>();
    peers.sort_by(|a, b| {
        b.last_seen_at
            .unwrap_or_default()
            .cmp(&a.last_seen_at.unwrap_or_default())
    });
    Ok(peers)
}

pub fn load_peer_record(node_id: &str) -> Result<Option<PeerInfo>> {
    for path in peer_file_candidates(node_id)? {
        if !path.exists() {
            continue;
        }

        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read node peer record {}", path.display()))?;
        let mut peer = parse_peer_record_content(&path, &content)?;
        if peer.node.id != node_id {
            continue;
        }
        peer.connected = false;
        return Ok(Some(peer));
    }

    load_peer_record_by_scan(node_id)
}

fn load_peer_record_by_scan(node_id: &str) -> Result<Option<PeerInfo>> {
    let dir = crate::app_paths::node_peers_dir()?;
    for entry in std::fs::read_dir(&dir)
        .with_context(|| format!("failed to read node peers directory {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file()
            || path
                .extension()
                .and_then(|value| value.to_str())
                .map(|value| !value.eq_ignore_ascii_case("toml"))
                .unwrap_or(true)
        {
            continue;
        }

        let content = match std::fs::read_to_string(&path) {
            Ok(content) => content,
            Err(error) => {
                log::warn!(
                    "Failed to read node peer record {}: {error}",
                    path.display()
                );
                continue;
            }
        };

        let mut peer = match parse_peer_record_content(&path, &content) {
            Ok(peer) => peer,
            Err(error) => {
                log::warn!(
                    "Failed to parse node peer record {}: {error}",
                    path.display()
                );
                continue;
            }
        };

        if peer.node.id == node_id {
            peer.connected = false;
            return Ok(Some(peer));
        }
    }

    Ok(None)
}

pub fn save_peer_record(peer: &PeerInfo) -> Result<()> {
    let path = peer_file_path_for_peer(peer)?;
    let content = toml::to_string_pretty(&peer_info_to_toml(peer))
        .context("failed to serialize node peer record")?;
    std::fs::write(&path, content)
        .with_context(|| format!("failed to write node peer record {}", path.display()))?;
    remove_stale_peer_files(peer, &path)?;
    Ok(())
}

fn remove_stale_peer_files(peer: &PeerInfo, current_path: &PathBuf) -> Result<()> {
    for path in peer_file_candidates_with_scan(&peer.node.id)? {
        if path == *current_path || !path.exists() {
            continue;
        }

        let should_remove = std::fs::read_to_string(&path)
            .ok()
            .and_then(|content| parse_peer_record_content(&path, &content).ok())
            .map(|stored_peer| stored_peer.node.id == peer.node.id)
            .unwrap_or(false);

        if should_remove {
            std::fs::remove_file(&path).with_context(|| {
                format!("failed to remove stale node peer record {}", path.display())
            })?;
            log::info!("Removed stale render node peer record: {}", path.display());
        }
    }
    Ok(())
}

pub fn delete_peer_record(node_id: &str) -> Result<()> {
    let primary = peer_file_path(node_id)?;
    for path in peer_file_candidates_with_scan(node_id)? {
        if !path.exists() {
            continue;
        }

        let should_remove = if path == primary {
            true
        } else {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|content| parse_peer_record_content(&path, &content).ok())
                .map(|peer| peer.node.id == node_id)
                .unwrap_or(false)
        };

        if should_remove {
            std::fs::remove_file(&path)
                .with_context(|| format!("failed to remove node peer record {}", path.display()))?;
        }
    }
    Ok(())
}

fn peer_file_candidates_with_scan(node_id: &str) -> Result<Vec<PathBuf>> {
    let mut paths = peer_file_candidates(node_id)?;
    let dir = crate::app_paths::node_peers_dir()?;
    for entry in std::fs::read_dir(&dir)
        .with_context(|| format!("failed to read node peers directory {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file()
            || path
                .extension()
                .and_then(|value| value.to_str())
                .map(|value| !value.eq_ignore_ascii_case("toml"))
                .unwrap_or(true)
            || paths.contains(&path)
        {
            continue;
        }

        let matches_node = std::fs::read_to_string(&path)
            .ok()
            .and_then(|content| parse_peer_record_content(&path, &content).ok())
            .map(|peer| peer.node.id == node_id)
            .unwrap_or(false);

        if matches_node {
            paths.push(path);
        }
    }
    Ok(paths)
}

pub fn peer_record_for_connect(mut peer: PeerInfo, previous: Option<&PeerInfo>) -> PeerInfo {
    let now = Utc::now().timestamp_millis();
    let persisted_first_seen_at = if previous.and_then(|value| value.first_seen_at).is_none() {
        match load_peer_record(&peer.node.id) {
            Ok(Some(record)) => record.first_seen_at,
            Ok(None) => None,
            Err(error) => {
                log::warn!("Failed to load persisted peer {}: {error}", peer.node.id);
                None
            }
        }
    } else {
        None
    };
    peer.connected = true;
    peer.first_seen_at = previous
        .and_then(|value| value.first_seen_at)
        .or(persisted_first_seen_at)
        .or(Some(now));
    peer.last_seen_at = Some(now);
    peer.last_connected_at = Some(now);
    peer.last_disconnected_at = previous.and_then(|value| value.last_disconnected_at);
    peer
}

pub fn peer_record_for_disconnect(mut peer: PeerInfo) -> PeerInfo {
    let now = Utc::now().timestamp_millis();
    peer.connected = false;
    peer.last_seen_at = Some(now);
    peer.last_disconnected_at = Some(now);
    peer
}

fn parse_peer_record_content(path: &std::path::Path, content: &str) -> Result<PeerInfo> {
    match toml::from_str::<RemotePeerToml>(content) {
        Ok(raw) => return Ok(peer_info_from_toml(raw)),
        Err(new_error) => match toml::from_str::<PeerInfo>(content) {
            Ok(peer) => Ok(peer),
            Err(old_error) => Err(anyhow::anyhow!(
                "failed to parse node peer record {} as segmented ({new_error}) or legacy ({old_error})",
                path.display()
            )),
        },
    }
}

fn peer_info_to_toml(peer: &PeerInfo) -> RemotePeerToml {
    RemotePeerToml {
        node: PeerNodeSection {
            id: peer.node.id.clone(),
            hostname: peer.node.hostname.clone(),
            version: peer.node.version.clone(),
            ip_address: peer.node.ip_address.clone(),
            port: peer.node.port,
            note: peer.node.note.clone(),
        },
        connection: PeerConnectionSection {
            first_seen_at: peer.first_seen_at.map(to_rfc3339),
            last_seen_at: peer.last_seen_at.map(to_rfc3339),
            last_connected_at: peer.last_connected_at.map(to_rfc3339),
            last_disconnected_at: peer.last_disconnected_at.map(to_rfc3339),
        },
        jobs: peer.jobs.iter().map(remote_job_to_toml).collect(),
    }
}

fn remote_job_to_toml(job: &RemoteJobSnapshot) -> RemoteJobToml {
    RemoteJobToml {
        job: RemoteJobJobSection {
            id: job.id.clone(),
            number: job.job_number,
            name: job.name.clone(),
            note: job.note.clone().unwrap_or_default(),
        },
        source: RemoteJobSourceSection {
            blend_file: job.blend_file.to_string_lossy().to_string(),
        },
        output: RemoteJobOutputSection {
            path: job.output_path.to_string_lossy().to_string(),
            format: job.output_format.clone(),
            render_mode: render_mode_key(job.render_mode),
            frame_start: job.frame_start,
            frame_end: job.frame_end,
            original_frame_start: job.original_frame_start,
            original_frame_end: job.original_frame_end,
        },
        result: RemoteJobResultSection {
            status: status_key(&job.status).to_string(),
            crash_count: job.crash_count,
            last_rendered_frame: job.last_rendered_frame,
            shadow_resolution_scale_override: job.shadow_resolution_scale_override,
            preview_width: job.preview_width,
            preview_height: job.preview_height,
        },
        timing: RemoteJobTimingSection {
            created_at: to_rfc3339(job.created_at),
            started_at: job.started_at.map(to_rfc3339),
            finished_at: job.finished_at.map(to_rfc3339),
            duration_secs: duration_secs(job.started_at, job.finished_at),
        },
    }
}

fn peer_info_from_toml(raw: RemotePeerToml) -> PeerInfo {
    PeerInfo {
        node: NodeInfo {
            id: raw.node.id,
            hostname: raw.node.hostname,
            note: raw.node.note,
            version: raw.node.version,
            ip_address: raw.node.ip_address,
            port: raw.node.port,
        },
        jobs: raw.jobs.into_iter().map(remote_job_from_toml).collect(),
        queue_paused: false,
        connected: false,
        first_seen_at: raw.connection.first_seen_at.as_deref().and_then(parse_rfc3339),
        last_seen_at: raw.connection.last_seen_at.as_deref().and_then(parse_rfc3339),
        last_connected_at: raw
            .connection
            .last_connected_at
            .as_deref()
            .and_then(parse_rfc3339),
        last_disconnected_at: raw
            .connection
            .last_disconnected_at
            .as_deref()
            .and_then(parse_rfc3339),
    }
}

fn remote_job_from_toml(raw: RemoteJobToml) -> RemoteJobSnapshot {
    RemoteJobSnapshot {
        id: raw.job.id,
        job_number: raw.job.number,
        name: raw.job.name,
        note: if raw.job.note.is_empty() {
            None
        } else {
            Some(raw.job.note)
        },
        status: parse_job_status(&raw.result.status),
        render_mode: parse_render_mode(&raw.output.render_mode),
        output_format: raw.output.format,
        output_path: PathBuf::from(raw.output.path),
        blend_file: PathBuf::from(raw.source.blend_file),
        original_frame_start: raw.output.original_frame_start,
        original_frame_end: raw.output.original_frame_end,
        frame_start: raw.output.frame_start,
        frame_end: raw.output.frame_end,
        crash_count: raw.result.crash_count,
        shadow_resolution_scale_override: raw.result.shadow_resolution_scale_override,
        last_rendered_frame: raw.result.last_rendered_frame,
        preview_width: raw.result.preview_width,
        preview_height: raw.result.preview_height,
        created_at: parse_rfc3339(&raw.timing.created_at).unwrap_or_default(),
        started_at: raw.timing.started_at.as_deref().and_then(parse_rfc3339),
        finished_at: raw.timing.finished_at.as_deref().and_then(parse_rfc3339),
        current_frame: None,
        total_frames: None,
        time_elapsed: None,
        remaining_secs: None,
    }
}

fn to_rfc3339(timestamp_ms: i64) -> String {
    chrono::DateTime::<Utc>::from_timestamp_millis(timestamp_ms)
        .unwrap_or_else(Utc::now)
        .to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn parse_rfc3339(value: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|timestamp| timestamp.timestamp_millis())
}

fn duration_secs(started_at: Option<i64>, finished_at: Option<i64>) -> Option<i64> {
    match (started_at, finished_at) {
        (Some(started_at), Some(finished_at)) if finished_at >= started_at => {
            Some((finished_at - started_at) / 1000)
        }
        _ => None,
    }
}

fn render_mode_key(value: crate::queue::job::RenderMode) -> String {
    match value {
        crate::queue::job::RenderMode::ImageSequence => String::from("image_sequence"),
        crate::queue::job::RenderMode::QuickMp4 => String::from("quick_mp4"),
    }
}

fn parse_render_mode(value: &str) -> crate::queue::job::RenderMode {
    match value {
        "quick_mp4" => crate::queue::job::RenderMode::QuickMp4,
        _ => crate::queue::job::RenderMode::ImageSequence,
    }
}

fn status_key(status: &crate::queue::job::JobStatus) -> &'static str {
    match status {
        crate::queue::job::JobStatus::Pending => "pending",
        crate::queue::job::JobStatus::Running => "running",
        crate::queue::job::JobStatus::Done => "done",
        crate::queue::job::JobStatus::Failed => "failed",
        crate::queue::job::JobStatus::Cancelled => "cancelled",
        crate::queue::job::JobStatus::Interrupted => "interrupted",
    }
}

fn parse_job_status(value: &str) -> crate::queue::job::JobStatus {
    match value {
        "running" => crate::queue::job::JobStatus::Running,
        "done" => crate::queue::job::JobStatus::Done,
        "failed" => crate::queue::job::JobStatus::Failed,
        "cancelled" => crate::queue::job::JobStatus::Cancelled,
        "interrupted" => crate::queue::job::JobStatus::Interrupted,
        _ => crate::queue::job::JobStatus::Pending,
    }
}
