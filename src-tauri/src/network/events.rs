use crate::network::types::{
    NodeJobEvent, NodeJobEventKind, NodeJobEventLevel, PeerJobEventPayload, RemoteJobSnapshot,
};
use crate::queue::job::JobStatus;
use anyhow::Result;
use chrono::Utc;
use sqlx::{Row, SqlitePool};
use tauri::{AppHandle, Emitter};

const MAX_EVENTS_PER_JOB: i64 = 1_000;
const MAX_EVENTS_PER_NODE: i64 = 5_000;

pub async fn load_node_job_events(
    pool: &SqlitePool,
    node_id: &str,
    job_id: &str,
) -> Result<Vec<NodeJobEvent>> {
    let rows = sqlx::query(
        "SELECT id, node_id, job_id, timestamp, kind, level, title, message
         FROM node_job_events
         WHERE node_id = ? AND job_id = ?
         ORDER BY timestamp ASC
         LIMIT ?",
    )
    .bind(node_id)
    .bind(job_id)
    .bind(MAX_EVENTS_PER_JOB)
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(node_event_from_row).collect()
}

async fn append_event_if_missing(pool: &SqlitePool, event: &NodeJobEvent) -> Result<bool> {
    let result = sqlx::query(
        "INSERT OR IGNORE INTO node_job_events
            (id, node_id, job_id, timestamp, kind, level, title, message)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&event.id)
    .bind(&event.node_id)
    .bind(&event.job_id)
    .bind(event.timestamp)
    .bind(event_kind_key(&event.kind))
    .bind(event_level_key(&event.level))
    .bind(&event.title)
    .bind(&event.message)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn delete_node_events(pool: &SqlitePool, node_id: &str) -> Result<()> {
    sqlx::query("DELETE FROM node_job_events WHERE node_id = ?")
        .bind(node_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn trim_node_events(pool: &SqlitePool, node_id: &str) -> Result<()> {
    sqlx::query(
        "DELETE FROM node_job_events
         WHERE node_id = ?
           AND id NOT IN (
             SELECT id FROM node_job_events
             WHERE node_id = ?
             ORDER BY timestamp DESC, id DESC
             LIMIT ?
           )",
    )
    .bind(node_id)
    .bind(node_id)
    .bind(MAX_EVENTS_PER_NODE)
    .execute(pool)
    .await?;
    Ok(())
}

async fn append_and_emit(pool: &SqlitePool, app: &AppHandle, event: NodeJobEvent) {
    match append_event_if_missing(pool, &event).await {
        Ok(true) => {
            let _ = app.emit("peer-job-event", PeerJobEventPayload { event });
        }
        Ok(false) => {}
        Err(error) => {
            log::warn!("Failed to persist node event {}: {error}", event.id);
        }
    }
}

pub async fn seed_job_events(
    pool: &SqlitePool,
    app: &AppHandle,
    node_id: &str,
    job: &RemoteJobSnapshot,
) {
    append_and_emit(pool, app, job_discovered_event(node_id, job)).await;

    if job.status != JobStatus::Pending {
        append_and_emit(
            pool,
            app,
            status_changed_event(
                node_id,
                &JobStatus::Pending,
                &job.status,
                job,
                job.started_at.or(job.finished_at).unwrap_or(job.created_at),
            ),
        )
        .await;
    }

    if job.crash_count > 0 {
        append_and_emit(
            pool,
            app,
            crash_retry_event(
                node_id,
                job,
                job.crash_count,
                job.started_at.unwrap_or(job.created_at),
            ),
        )
        .await;
    }

    if let Some(scale) = job.shadow_resolution_scale_override {
        append_and_emit(
            pool,
            app,
            shadow_recovery_event(
                node_id,
                job,
                scale,
                job.started_at.unwrap_or(job.created_at),
            ),
        )
        .await;
    }
}

pub async fn record_job_snapshot_events(
    pool: &SqlitePool,
    app: &AppHandle,
    node_id: &str,
    previous: Option<&RemoteJobSnapshot>,
    incoming: &RemoteJobSnapshot,
) {
    let Some(previous) = previous else {
        seed_job_events(pool, app, node_id, incoming).await;
        return;
    };

    if previous.status != incoming.status {
        append_and_emit(
            pool,
            app,
            status_changed_event(
                node_id,
                &previous.status,
                &incoming.status,
                incoming,
                incoming.finished_at.unwrap_or_else(now),
            ),
        )
        .await;
    }

    if incoming.crash_count > previous.crash_count {
        append_and_emit(
            pool,
            app,
            crash_retry_event(node_id, incoming, incoming.crash_count, now()),
        )
        .await;
    }

    if incoming.shadow_resolution_scale_override.is_some()
        && incoming.shadow_resolution_scale_override != previous.shadow_resolution_scale_override
    {
        append_and_emit(
            pool,
            app,
            shadow_recovery_event(
                node_id,
                incoming,
                incoming
                    .shadow_resolution_scale_override
                    .unwrap_or_default(),
                now(),
            ),
        )
        .await;
    }

    if incoming.frame_start != previous.frame_start || incoming.frame_end != previous.frame_end {
        append_and_emit(pool, app, range_changed_event(node_id, previous, incoming)).await;
    }
}

pub async fn record_node_connected_events(
    pool: &SqlitePool,
    app: &AppHandle,
    node_id: &str,
    jobs: &[RemoteJobSnapshot],
) {
    let timestamp = now();
    for job in jobs.iter().filter(|job| is_running_job(job)) {
        append_and_emit(pool, app, node_connected_event(node_id, job, timestamp)).await;
    }
}

pub async fn record_node_disconnected_events(
    pool: &SqlitePool,
    app: &AppHandle,
    node_id: &str,
    jobs: &[RemoteJobSnapshot],
) {
    let timestamp = now();
    for job in jobs.iter().filter(|job| is_running_job(job)) {
        append_and_emit(pool, app, node_disconnected_event(node_id, job, timestamp)).await;
    }
}

fn node_event_from_row(row: sqlx::sqlite::SqliteRow) -> Result<NodeJobEvent> {
    let kind: String = row.try_get("kind")?;
    let level: String = row.try_get("level")?;
    Ok(NodeJobEvent {
        id: row.try_get("id")?,
        node_id: row.try_get("node_id")?,
        job_id: row.try_get("job_id")?,
        timestamp: row.try_get("timestamp")?,
        kind: parse_event_kind(&kind),
        level: parse_event_level(&level),
        title: row.try_get("title")?,
        message: row.try_get("message")?,
    })
}

fn job_discovered_event(node_id: &str, job: &RemoteJobSnapshot) -> NodeJobEvent {
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
    job: &RemoteJobSnapshot,
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
    job: &RemoteJobSnapshot,
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
    job: &RemoteJobSnapshot,
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

fn range_changed_event(
    node_id: &str,
    previous: &RemoteJobSnapshot,
    next: &RemoteJobSnapshot,
) -> NodeJobEvent {
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

fn node_connected_event(
    node_id: &str,
    job: &RemoteJobSnapshot,
    timestamp: i64,
) -> NodeJobEvent {
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

fn node_disconnected_event(
    node_id: &str,
    job: &RemoteJobSnapshot,
    timestamp: i64,
) -> NodeJobEvent {
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

fn is_running_job(job: &RemoteJobSnapshot) -> bool {
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

fn event_level_key(level: &NodeJobEventLevel) -> &'static str {
    match level {
        NodeJobEventLevel::Info => "info",
        NodeJobEventLevel::Success => "success",
        NodeJobEventLevel::Warning => "warning",
        NodeJobEventLevel::Error => "error",
    }
}

fn parse_event_kind(value: &str) -> NodeJobEventKind {
    match value {
        "status_changed" => NodeJobEventKind::StatusChanged,
        "crash_retry" => NodeJobEventKind::CrashRetry,
        "shadow_recovery" => NodeJobEventKind::ShadowRecovery,
        "range_changed" => NodeJobEventKind::RangeChanged,
        "node_connected" => NodeJobEventKind::NodeConnected,
        "node_disconnected" => NodeJobEventKind::NodeDisconnected,
        "progress" => NodeJobEventKind::Progress,
        _ => NodeJobEventKind::JobDiscovered,
    }
}

fn parse_event_level(value: &str) -> NodeJobEventLevel {
    match value {
        "success" => NodeJobEventLevel::Success,
        "warning" => NodeJobEventLevel::Warning,
        "error" => NodeJobEventLevel::Error,
        _ => NodeJobEventLevel::Info,
    }
}

fn now() -> i64 {
    Utc::now().timestamp_millis()
}
