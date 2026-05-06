use crate::network::types::PeerInfo;
use anyhow::{Context, Result};
use chrono::Utc;
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

fn peer_file_candidates_for_peer(peer: &PeerInfo) -> Result<Vec<PathBuf>> {
    Ok(crate::network::file_names::node_file_candidates_for_info(
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

pub fn load_peer_records() -> Result<Vec<PeerInfo>> {
    let dir = crate::app_paths::node_peers_dir()?;
    let mut peers = std::collections::HashMap::<String, PeerInfo>::new();

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
            .and_then(|content| {
                toml::from_str::<PeerInfo>(&content)
                    .with_context(|| format!("failed to parse node peer record {}", path.display()))
            }) {
            Ok(mut peer) => {
                peer.connected = false;
                let id = peer.node.id.clone();
                match peers.get(&id) {
                    Some(existing)
                        if existing.last_seen_at.unwrap_or_default()
                            >= peer.last_seen_at.unwrap_or_default() => {}
                    _ => {
                        peers.insert(id, peer);
                    }
                }
            }
            Err(error) => {
                log::warn!("{error}");
            }
        }
    }

    let mut peers = peers.into_values().collect::<Vec<_>>();
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
        let mut peer = toml::from_str::<PeerInfo>(&content)
            .with_context(|| format!("failed to parse node peer record {}", path.display()))?;
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

        let mut peer = match toml::from_str::<PeerInfo>(&content) {
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
    let content = toml::to_string_pretty(peer).context("failed to serialize node peer record")?;
    std::fs::write(&path, content)
        .with_context(|| format!("failed to write node peer record {}", path.display()))?;
    remove_stale_peer_files(peer, &path)?;
    Ok(())
}

fn remove_stale_peer_files(peer: &PeerInfo, current_path: &PathBuf) -> Result<()> {
    for path in peer_file_candidates_for_peer(peer)? {
        if path == *current_path || !path.exists() {
            continue;
        }

        let should_remove = std::fs::read_to_string(&path)
            .ok()
            .and_then(|content| toml::from_str::<PeerInfo>(&content).ok())
            .map(|stored_peer| stored_peer.node.id == peer.node.id)
            .unwrap_or(false);

        if should_remove {
            std::fs::remove_file(&path).with_context(|| {
                format!("failed to remove stale node peer record {}", path.display())
            })?;
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
                .and_then(|content| toml::from_str::<PeerInfo>(&content).ok())
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
            .and_then(|content| toml::from_str::<PeerInfo>(&content).ok())
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
