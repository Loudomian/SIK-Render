use crate::network::types::{NodeJobEvent, NodeJobEventKind, NodeJobEventLevel, PeerJobEventPayload};
use crate::queue::job::{JobStatus, RenderJob};
use anyhow::{Context, Result};
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Emitter};

const MAX_EVENTS_PER_JOB: usize = 1_000;
const MAX_STORED_EVENTS_PER_FILE: usize = 5_000;
const COMPACT_AFTER_EVENTS: usize = 6_000;

#[derive(Default)]
struct EventIndex {
    ids: HashSet<String>,
    semantic_keys: HashSet<String>,
    event_count: usize,
}

fn node_file_path(node_id: &str) -> Result<PathBuf> {
    let dir = crate::app_paths::node_events_dir()?;
    if let Some(peer) = peer_record(node_id) {
        return Ok(crate::network::file_names::node_file_path_for_info(
            &dir,
            &peer.node.hostname,
            &peer.node.ip_address,
            &peer.node.id,
            "jsonl",
        ));
    }

    Ok(crate::network::file_names::node_file_path(
        &dir, node_id, "jsonl",
    ))
}

fn node_file_candidates(node_id: &str) -> Result<Vec<PathBuf>> {
    let dir = crate::app_paths::node_events_dir()?;
    if let Some(peer) = peer_record(node_id) {
        return Ok(crate::network::file_names::node_file_candidates_for_info(
            &dir,
            &peer.node.hostname,
            &peer.node.ip_address,
            &peer.node.id,
            "jsonl",
        ));
    }

    Ok(crate::network::file_names::node_file_candidates(
        &dir, node_id, "jsonl",
    ))
}

fn event_indexes() -> &'static Mutex<HashMap<PathBuf, EventIndex>> {
    static INDEXES: OnceLock<Mutex<HashMap<PathBuf, EventIndex>>> = OnceLock::new();
    INDEXES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn peer_record(node_id: &str) -> Option<crate::network::types::PeerInfo> {
    match crate::network::peers::load_peer_record(node_id) {
        Ok(peer) => peer,
        Err(error) => {
            log::warn!("Failed to resolve peer record for node events {node_id}: {error}");
            None
        }
    }
}

pub fn load_node_job_events(node_id: &str, job_id: &str) -> Result<Vec<NodeJobEvent>> {
    let mut seen = HashSet::new();
    let mut events = read_node_events(node_id)?
        .into_iter()
        .filter(|event| event.node_id == node_id && event.job_id == job_id)
        .filter(|event| event.kind != NodeJobEventKind::Progress)
        .filter(|event| seen.insert(event_semantic_key(event)))
        .collect::<Vec<_>>();

    events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    if events.len() > MAX_EVENTS_PER_JOB {
        Ok(events.split_off(events.len() - MAX_EVENTS_PER_JOB))
    } else {
        Ok(events)
    }
}

fn append_event_if_missing(event: &NodeJobEvent) -> Result<bool> {
    let path = node_file_path(&event.node_id)?;
    let event_key = event_semantic_key(event);
    let should_compact = {
        let mut indexes = event_indexes()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let index = indexes.entry(path.clone()).or_insert_with(|| {
            build_event_index(&event.node_id).unwrap_or_else(|error| {
                log::warn!(
                    "Failed to build node event index {}: {error}",
                    event.node_id
                );
                EventIndex::default()
            })
        });

        if index.ids.contains(&event.id) || index.semantic_keys.contains(&event_key) {
            return Ok(false);
        }

        index.event_count + 1 > COMPACT_AFTER_EVENTS
    };

    if should_compact {
        compact_node_events(&event.node_id)?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("failed to open node events {}", path.display()))?;
    let line = serde_json::to_string(event).context("failed to serialize node event")?;
    writeln!(file, "{line}")
        .with_context(|| format!("failed to write node events {}", path.display()))?;
    let mut indexes = event_indexes()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let index = indexes.entry(path).or_insert_with(|| EventIndex::default());
    index.ids.insert(event.id.clone());
    index.semantic_keys.insert(event_key);
    index.event_count += 1;
    Ok(true)
}

pub fn delete_node_events(node_id: &str) -> Result<()> {
    let primary = node_file_path(node_id)?;
    for path in node_file_candidates(node_id)? {
        if !path.exists() {
            continue;
        }

        if path == primary {
            fs::remove_file(&path)
                .with_context(|| format!("failed to remove node events {}", path.display()))?;
        } else {
            rewrite_legacy_events_without_node(&path, node_id)?;
        }
    }

    event_indexes()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(&primary);
    Ok(())
}

fn read_node_events(node_id: &str) -> Result<Vec<NodeJobEvent>> {
    let mut events = Vec::new();
    for path in node_file_candidates(node_id)? {
        if !path.exists() {
            continue;
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read node events {}", path.display()))?;
        events.extend(content.lines().filter_map(|line| match parse_event(line) {
            Some(event) => Some(event),
            None => {
                log::warn!("Failed to parse node event {}", path.display());
                None
            }
        }));
    }
    Ok(events)
}

fn build_event_index(node_id: &str) -> Result<EventIndex> {
    let mut index = EventIndex::default();
    for event in read_node_events(node_id)?
        .into_iter()
        .filter(|event| event.node_id == node_id)
    {
        index.ids.insert(event.id.clone());
        index.semantic_keys.insert(event_semantic_key(&event));
        index.event_count += 1;
    }
    Ok(index)
}

fn compact_node_events(node_id: &str) -> Result<()> {
    let path = node_file_path(node_id)?;
    let mut seen = HashSet::new();
    let mut events = read_node_events(node_id)?
        .into_iter()
        .filter(|event| event.node_id == node_id)
        .filter(|event| seen.insert(event_semantic_key(event)))
        .collect::<Vec<_>>();

    events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp).then_with(|| a.id.cmp(&b.id)));
    if events.len() > MAX_STORED_EVENTS_PER_FILE {
        events = events.split_off(events.len() - MAX_STORED_EVENTS_PER_FILE);
    }

    let mut content = String::new();
    for event in &events {
        let line = serde_json::to_string(event).context("failed to serialize node event")?;
        content.push_str(&line);
        content.push('\n');
    }
    fs::write(&path, content)
        .with_context(|| format!("failed to compact node events {}", path.display()))?;
    for candidate in node_file_candidates(node_id)? {
        if candidate != path && candidate.exists() {
            rewrite_legacy_events_without_node(&candidate, node_id)?;
        }
    }

    let mut index = EventIndex::default();
    for event in events {
        index.ids.insert(event.id.clone());
        index.semantic_keys.insert(event_semantic_key(&event));
        index.event_count += 1;
    }
    event_indexes()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(path, index);
    Ok(())
}

fn rewrite_legacy_events_without_node(path: &PathBuf, node_id: &str) -> Result<()> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read legacy node events {}", path.display()))?;
    let mut retained = Vec::new();
    let mut removed_any = false;

    for line in content.lines() {
        match parse_event(line) {
            Some(event) if event.node_id == node_id => {
                removed_any = true;
            }
            Some(_) => retained.push(line.to_string()),
            None => retained.push(line.to_string()),
        }
    }

    if !removed_any {
        return Ok(());
    }

    if retained.is_empty() {
        fs::remove_file(path)
            .with_context(|| format!("failed to remove legacy node events {}", path.display()))?;
    } else {
        fs::write(path, format!("{}\n", retained.join("\n")))
            .with_context(|| format!("failed to rewrite legacy node events {}", path.display()))?;
    }
    Ok(())
}

fn parse_event(line: &str) -> Option<NodeJobEvent> {
    serde_json::from_str::<NodeJobEvent>(line).ok()
}

fn event_semantic_key(event: &NodeJobEvent) -> String {
    match event.kind {
        NodeJobEventKind::StatusChanged => format!(
            "{}:{}:{}:{}",
            event.node_id,
            event.job_id,
            event_kind_key(&event.kind),
            event.id,
        ),
        _ => event.id.clone(),
    }
}

fn event_kind_key(kind: &NodeJobEventKind) -> &'static str {
    match kind {
        NodeJobEventKind::JobDiscovered => "job_discovered",
        NodeJobEventKind::StatusChanged => "status_changed",
        NodeJobEventKind::CrashRetry => "crash_retry",
        NodeJobEventKind::ShadowRecovery => "shadow_recovery",
        NodeJobEventKind::RangeChanged => "range_changed",
        NodeJobEventKind::NodeConnected => "node_connected",
        NodeJobEventKind::NodeDisconnected => "node_disconnected",
        NodeJobEventKind::Progress => "progress",
    }
}

fn append_and_emit(app: &AppHandle, event: NodeJobEvent) {
    match append_event_if_missing(&event) {
        Ok(true) => {
            let _ = app.emit("peer-job-event", PeerJobEventPayload { event });
        }
        Ok(false) => {}
        Err(error) => {
            log::warn!("Failed to persist node event {}: {error}", event.id);
        }
    }
}

pub fn seed_job_events(app: &AppHandle, node_id: &str, job: &RenderJob) {
    append_and_emit(app, job_discovered_event(node_id, job));

    if job.status != JobStatus::Pending {
        append_and_emit(
            app,
            status_changed_event(
                node_id,
                &JobStatus::Pending,
                &job.status,
                job,
                job.started_at.or(job.finished_at).unwrap_or(job.created_at),
            ),
        );
    }

    if job.crash_count > 0 {
        append_and_emit(
            app,
            crash_retry_event(
                node_id,
                job,
                job.crash_count,
                job.started_at.unwrap_or(job.created_at),
            ),
        );
    }

    if let Some(scale) = job.shadow_resolution_scale_override {
        append_and_emit(
            app,
            shadow_recovery_event(
                node_id,
                job,
                scale,
                job.started_at.unwrap_or(job.created_at),
            ),
        );
    }
}

pub fn record_job_snapshot_events(
    app: &AppHandle,
    node_id: &str,
    previous: Option<&RenderJob>,
    incoming: &RenderJob,
) {
    let Some(previous) = previous else {
        seed_job_events(app, node_id, incoming);
        return;
    };

    if previous.status != incoming.status {
        append_and_emit(
            app,
            status_changed_event(
                node_id,
                &previous.status,
                &incoming.status,
                incoming,
                incoming.finished_at.unwrap_or_else(now),
            ),
        );
    }

    if incoming.crash_count > previous.crash_count {
        append_and_emit(
            app,
            crash_retry_event(node_id, incoming, incoming.crash_count, now()),
        );
    }

    if incoming.shadow_resolution_scale_override.is_some()
        && incoming.shadow_resolution_scale_override != previous.shadow_resolution_scale_override
    {
        append_and_emit(
            app,
            shadow_recovery_event(
                node_id,
                incoming,
                incoming
                    .shadow_resolution_scale_override
                    .unwrap_or_default(),
                now(),
            ),
        );
    }

    if incoming.frame_start != previous.frame_start || incoming.frame_end != previous.frame_end {
        append_and_emit(app, range_changed_event(node_id, previous, incoming));
    }
}

pub fn record_node_connected_events(app: &AppHandle, node_id: &str, jobs: &[RenderJob]) {
    let timestamp = now();
    for job in jobs.iter().filter(|job| is_running_job(job)) {
        append_and_emit(app, node_connected_event(node_id, job, timestamp));
    }
}

pub fn record_node_disconnected_events(app: &AppHandle, node_id: &str, jobs: &[RenderJob]) {
    let timestamp = now();
    for job in jobs.iter().filter(|job| is_running_job(job)) {
        append_and_emit(app, node_disconnected_event(node_id, job, timestamp));
    }
}

fn job_discovered_event(node_id: &str, job: &RenderJob) -> NodeJobEvent {
    NodeJobEvent {
        id: format!("job_discovered:{node_id}:{}", job.id),
        node_id: node_id.to_string(),
        job_id: job.id.clone(),
        timestamp: job.created_at,
        kind: NodeJobEventKind::JobDiscovered,
        level: NodeJobEventLevel::Info,
        title: "接收到远端任务".to_string(),
        message: format!("节点同步了 #{} {}。", job.job_number, job.name),
    }
}

fn status_changed_event(
    node_id: &str,
    previous: &JobStatus,
    next: &JobStatus,
    job: &RenderJob,
    timestamp: i64,
) -> NodeJobEvent {
    NodeJobEvent {
        id: format!(
            "status_changed:{node_id}:{}:{}:{}",
            job.id,
            status_key(previous),
            status_key(next)
        ),
        node_id: node_id.to_string(),
        job_id: job.id.clone(),
        timestamp,
        kind: NodeJobEventKind::StatusChanged,
        level: status_level(next),
        title: format!("#{} 状态变为{}", job.job_number, status_label(next)),
        message: format!(
            "任务 #{} {} 从{}变为{}。",
            job.job_number,
            job.name,
            status_label(previous),
            status_label(next)
        ),
    }
}

fn crash_retry_event(
    node_id: &str,
    job: &RenderJob,
    crash_count: i32,
    timestamp: i64,
) -> NodeJobEvent {
    NodeJobEvent {
        id: format!("crash_retry:{node_id}:{}:{crash_count}", job.id),
        node_id: node_id.to_string(),
        job_id: job.id.clone(),
        timestamp,
        kind: NodeJobEventKind::CrashRetry,
        level: if job.status == JobStatus::Failed {
            NodeJobEventLevel::Error
        } else {
            NodeJobEventLevel::Warning
        },
        title: format!("Blender 崩溃重试 {crash_count} 次"),
        message: if job.status == JobStatus::Failed {
            "远端任务在达到重试上限后失败。".to_string()
        } else {
            "远端任务已记录崩溃并由节点继续恢复渲染。".to_string()
        },
    }
}

fn shadow_recovery_event(
    node_id: &str,
    job: &RenderJob,
    scale: f32,
    timestamp: i64,
) -> NodeJobEvent {
    NodeJobEvent {
        id: format!("shadow_recovery:{node_id}:{}:{:.3}", job.id, scale),
        node_id: node_id.to_string(),
        job_id: job.id.clone(),
        timestamp,
        kind: NodeJobEventKind::ShadowRecovery,
        level: NodeJobEventLevel::Warning,
        title: format!("阴影分辨率降到 {}%", (scale * 100.0).round() as i32),
        message: "远端任务已应用阴影恢复参数。通常表示此前检测到阴影池满或手动执行了降级恢复。"
            .to_string(),
    }
}

fn range_changed_event(node_id: &str, previous: &RenderJob, next: &RenderJob) -> NodeJobEvent {
    NodeJobEvent {
        id: format!(
            "range_changed:{node_id}:{}:{}:{}",
            next.id, next.frame_start, next.frame_end
        ),
        node_id: node_id.to_string(),
        job_id: next.id.clone(),
        timestamp: now(),
        kind: NodeJobEventKind::RangeChanged,
        level: NodeJobEventLevel::Info,
        title: format!("执行区间调整为 {}-{}", next.frame_start, next.frame_end),
        message: format!(
            "上一执行区间为 {}-{}。",
            previous.frame_start, previous.frame_end
        ),
    }
}

fn node_connected_event(node_id: &str, job: &RenderJob, timestamp: i64) -> NodeJobEvent {
    NodeJobEvent {
        id: format!("node_connected:{node_id}:{}:{timestamp}", job.id),
        node_id: node_id.to_string(),
        job_id: job.id.clone(),
        timestamp,
        kind: NodeJobEventKind::NodeConnected,
        level: NodeJobEventLevel::Info,
        title: "节点重新连接".to_string(),
        message: format!("任务 #{} {} 所在节点已重新连接。", job.job_number, job.name),
    }
}

fn node_disconnected_event(node_id: &str, job: &RenderJob, timestamp: i64) -> NodeJobEvent {
    NodeJobEvent {
        id: format!("node_disconnected:{node_id}:{}:{timestamp}", job.id),
        node_id: node_id.to_string(),
        job_id: job.id.clone(),
        timestamp,
        kind: NodeJobEventKind::NodeDisconnected,
        level: NodeJobEventLevel::Warning,
        title: "节点连接断开".to_string(),
        message: format!(
            "任务 #{} {} 所在节点断开连接；可能是工具退出、崩溃或网络中断。",
            job.job_number, job.name
        ),
    }
}

fn is_running_job(job: &RenderJob) -> bool {
    matches!(job.status, JobStatus::Running)
}

fn status_label(status: &JobStatus) -> &'static str {
    match status {
        JobStatus::Pending => "等待中",
        JobStatus::Running => "渲染中",
        JobStatus::Done => "已完成",
        JobStatus::Failed => "失败",
        JobStatus::Cancelled => "已取消",
        JobStatus::Interrupted => "已中断",
    }
}

fn status_key(status: &JobStatus) -> &'static str {
    match status {
        JobStatus::Pending => "pending",
        JobStatus::Running => "running",
        JobStatus::Done => "done",
        JobStatus::Failed => "failed",
        JobStatus::Cancelled => "cancelled",
        JobStatus::Interrupted => "interrupted",
    }
}

fn status_level(status: &JobStatus) -> NodeJobEventLevel {
    match status {
        JobStatus::Done => NodeJobEventLevel::Success,
        JobStatus::Failed => NodeJobEventLevel::Error,
        JobStatus::Cancelled | JobStatus::Interrupted => NodeJobEventLevel::Warning,
        JobStatus::Pending | JobStatus::Running => NodeJobEventLevel::Info,
    }
}

fn now() -> i64 {
    Utc::now().timestamp_millis()
}
