use crate::blender::parser;
use crate::queue::job::{JobStatus, RenderJob};
use crate::state::AppState;
use anyhow::Result;
use std::collections::BTreeSet;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Child;
use tokio::sync::Mutex;

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderProgressEvent {
    pub job_id: String,
    pub frame: u32,
    pub total_frames: u32,
    pub time_elapsed: f32,
    pub memory_mb: f32,
    pub remaining_secs: Option<f32>,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderLogEvent {
    pub job_id: String,
    pub line: String,
}

/// Max automatic crash-recovery retries before giving up.
const MAX_CRASH_RETRIES: u32 = 3;

pub fn format_to_ext(format: &str) -> &'static str {
    match format {
        "PNG" => "png",
        "JPEG" => "jpg",
        "OPEN_EXR" => "exr",
        "TIFF" => "tiff",
        "BMP" => "bmp",
        "HDR" => "hdr",
        "TARGA" => "tga",
        _ => "png",
    }
}

/// Build the output filename for a given frame using Blender's `######` pattern.
/// Returns `None` when the output path has no `#` pattern (can't predict filenames).
pub fn frame_filename(output_path: &std::path::Path, frame: i32, format: &str) -> Option<PathBuf> {
    let dir = output_path.parent().filter(|p| !p.as_os_str().is_empty())?;
    let template = output_path.file_name()?.to_str()?;
    let hash_start = template.find('#')?;
    let hash_count = template[hash_start..].chars().take_while(|&c| c == '#').count();
    let prefix = &template[..hash_start];
    let suffix_raw = &template[hash_start + hash_count..];
    let suffix = if let Some(dot) = suffix_raw.rfind('.') { &suffix_raw[..dot] } else { suffix_raw };
    let ext = format_to_ext(format);
    let frame_str = format!("{:0>width$}", frame, width = hash_count);
    Some(dir.join(format!("{}{}{}.{}", prefix, frame_str, suffix, ext)))
}

fn scan_rendered_frame_numbers(
    output_path: &std::path::Path,
    frame_start: i32,
    frame_end: i32,
    format: &str,
) -> BTreeSet<i32> {
    let dir = output_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(output_path);
    let expected_ext = format_to_ext(format).to_ascii_lowercase();
    let template = output_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let prefix_hint = template
        .find('#')
        .map(|idx| template[..idx].to_string())
        .unwrap_or_default();
    let suffix_hint = template
        .find('#')
        .map(|idx| {
            let hash_count = template[idx..]
                .chars()
                .take_while(|&ch| ch == '#')
                .count();
            let suffix_raw = &template[idx + hash_count..];
            if let Some(dot) = suffix_raw.rfind('.') {
                suffix_raw[..dot].to_string()
            } else {
                suffix_raw.to_string()
            }
        })
        .unwrap_or_default();

    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|path| path.is_file())
                .filter(|path| {
                    path.extension()
                        .map(|ext| ext.to_string_lossy().to_ascii_lowercase() == expected_ext)
                        .unwrap_or(false)
                })
                .filter(|path| {
                    let stem = path.file_stem().and_then(|name| name.to_str()).unwrap_or_default();
                    (prefix_hint.is_empty() || stem.starts_with(&prefix_hint))
                        && (suffix_hint.is_empty() || stem.ends_with(&suffix_hint))
                })
                .filter_map(|path| crate::blender::frame_path::trailing_frame_number(&path))
                .filter(|frame| (frame_start..=frame_end).contains(frame))
                .collect()
        })
        .unwrap_or_default()
}

pub fn count_job_image_files_sync(
    output_path: &std::path::Path,
    frame_start: i32,
    frame_end: i32,
    format: &str,
) -> u32 {
    if output_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.contains('#'))
        .unwrap_or(false)
    {
        return scan_rendered_frame_numbers(output_path, frame_start, frame_end, format).len() as u32;
    }

    let dir = output_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(output_path);
    let expected_ext = format_to_ext(format).to_ascii_lowercase();
    let template = output_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let prefix_hint = template
        .find('#')
        .map(|idx| template[..idx].to_string())
        .unwrap_or_default();
    let suffix_hint = template
        .find('#')
        .map(|idx| {
            let hash_count = template[idx..]
                .chars()
                .take_while(|&ch| ch == '#')
                .count();
            let suffix_raw = &template[idx + hash_count..];
            if let Some(dot) = suffix_raw.rfind('.') {
                suffix_raw[..dot].to_string()
            } else {
                suffix_raw.to_string()
            }
        })
        .unwrap_or_default();

    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|path| path.is_file())
                .filter(|path| {
                    path.extension()
                        .map(|ext| ext.to_string_lossy().to_ascii_lowercase() == expected_ext)
                        .unwrap_or(false)
                })
                .filter(|path| {
                    let stem = path.file_stem().and_then(|name| name.to_str()).unwrap_or_default();
                    (prefix_hint.is_empty() || stem.starts_with(&prefix_hint))
                        && (suffix_hint.is_empty() || stem.ends_with(&suffix_hint))
                })
                .filter_map(|path| crate::blender::frame_path::trailing_frame_number(&path))
                .filter(|frame| (frame_start..=frame_end).contains(frame))
                .count() as u32
        })
        .unwrap_or(0)
}

fn mark_progress_timestamp(progress_started_at: &std::time::Instant, last_progress_ms: &AtomicU64) {
    last_progress_ms.store(
        progress_started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
        Ordering::Relaxed,
    );
}

async fn persist_job_progress(
    state: &AppState,
    job_id: &str,
    current_frame: Option<u32>,
    total_frames: u32,
    last_rendered_frame: Option<i32>,
    time_elapsed: Option<f32>,
    remaining_secs: Option<f32>,
) {
    let _ = sqlx::query(
        "UPDATE jobs
         SET current_frame = COALESCE(?, current_frame),
             total_frames = ?,
             last_rendered_frame = COALESCE(?, last_rendered_frame),
             time_elapsed = COALESCE(?, time_elapsed),
             remaining_secs = ?
         WHERE id = ?",
    )
    .bind(current_frame.map(|value| value as i32))
    .bind(total_frames as i32)
    .bind(last_rendered_frame)
    .bind(time_elapsed)
    .bind(remaining_secs)
    .bind(job_id)
    .execute(&state.pool)
    .await;
}

fn compute_resume_frame(job: &RenderJob) -> i32 {
    if !job.resume_from_existing {
        return job.frame_start;
    }

    if let Some(last_frame) = job.last_rendered_frame {
        let last_frame = last_frame.clamp(job.frame_start - 1, job.frame_end);
        return (last_frame + 1).min(job.frame_end + 1);
    }

    job.frame_start
}

pub async fn run_job(app: AppHandle, state: AppState, job: RenderJob) -> Result<JobStatus> {
    let total_frames = job.total_frames() as u32;
    let mut resume_from_existing = job.resume_from_existing;
    let mut last_rendered_frame = job.last_rendered_frame;

    let output_dir: Option<PathBuf> = job
        .output_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.to_path_buf());

    if let Some(dir) = &output_dir {
        tokio::fs::create_dir_all(dir).await.ok();
    }

    let log_file_path = crate::app_paths::create_job_log_file(
        job.job_number,
        &job.id,
        crate::app_paths::BLENDER_LOG_KIND,
    )?;
    let log_write_lock = Arc::new(Mutex::new(()));
    let mut crash_count = 0u32;

    let job_result: Result<JobStatus> = loop {
        // Before each attempt check cancellation.
        if state.interrupted_jobs.lock().await.contains(&job.id) {
            break Err(anyhow::anyhow!("interrupted"));
        }
        if state.cancelled_jobs.lock().await.contains(&job.id) {
            break Err(anyhow::anyhow!("cancelled"));
        }

        let actual_start = if resume_from_existing {
            let mut resume_job = job.clone();
            resume_job.resume_from_existing = true;
            resume_job.last_rendered_frame = last_rendered_frame;
            compute_resume_frame(&resume_job)
        } else {
            job.frame_start
        };
        resume_from_existing = true;
        if actual_start > job.frame_end {
            persist_job_progress(
                &state,
                &job.id,
                Some(total_frames),
                total_frames,
                Some(job.frame_end),
                None,
                Some(0.0),
            )
            .await;
            break Ok(JobStatus::Done);
        }

        let already_done = (actual_start - job.frame_start).max(0) as u32;
        let file_baseline = count_job_image_files_sync(
            &job.output_path,
            job.frame_start,
            job.frame_end,
            &job.output_format,
        );

        // Emit initial progress so the UI starts at the right position.
        if already_done > 0 {
            persist_job_progress(
                &state,
                &job.id,
                Some(already_done),
                total_frames,
                Some(actual_start - 1),
                Some(0.0),
                None,
            )
            .await;
            let _ = app.emit(
                "render-progress",
                RenderProgressEvent {
                    job_id: job.id.clone(),
                    frame: already_done,
                    total_frames,
                    time_elapsed: 0.0,
                    memory_mb: 0.0,
                    remaining_secs: None,
                },
            );
        }

        let mut child = match spawn_blender(&job, actual_start) {
            Ok(c) => c,
            Err(e) => break Err(e),
        };
        {
            let mut active = state.active_jobs.lock().await;
            if let Some(pid) = child.id() {
                active.insert(job.id.clone(), pid);
            }
        }

        let cancelled_early = state.cancelled_jobs.lock().await.contains(&job.id);
        let interrupted_early = state.interrupted_jobs.lock().await.contains(&job.id);
        if cancelled_early || interrupted_early {
            if let Some(pid) = child.id() {
                let _ = AppState::kill_process_tree(pid);
            }
        }

        let stdout = child.stdout.take().expect("stdout not captured");
        let stderr = child.stderr.take().expect("stderr not captured");

        let render_running = Arc::new(AtomicBool::new(true));
        let progress_started_at = Arc::new(std::time::Instant::now());
        let last_primary_progress_ms = Arc::new(AtomicU64::new(0));
        // Per-run stderr buffer for error messages on failure.
        let stderr_buf: Arc<Mutex<VecDeque<String>>> =
            Arc::new(Mutex::new(VecDeque::with_capacity(200)));
        let job_frame_start = job.frame_start;
        let job_frame_end = job.frame_end;
        let poll_output_path = job.output_path.clone();
        let poll_output_format = job.output_format.clone();

        // ── File-poll progress (spawn_blocking → unaffected by async scheduler) ──
        let poll_running = render_running.clone();
        let poll_app = app.clone();
        let poll_job_id = job.id.clone();
        let poll_dir = output_dir.clone();
        let poll_last_primary_progress_ms = last_primary_progress_ms.clone();
        let poll_progress_started_at = progress_started_at.clone();
        let poll_task = tokio::task::spawn_blocking(move || {
            let Some(_dir) = poll_dir else { return };
            let mut last_count = already_done;
            loop {
                std::thread::sleep(std::time::Duration::from_millis(500));
                if !poll_running.load(Ordering::Relaxed) { break; }
                let raw = count_job_image_files_sync(&poll_output_path, job_frame_start, job_frame_end, &poll_output_format);
                let new_in_run = raw.saturating_sub(file_baseline);
                let count = already_done + new_in_run;
                let silent_ms = poll_progress_started_at
                    .elapsed()
                    .as_millis()
                    .saturating_sub(u128::from(poll_last_primary_progress_ms.load(Ordering::Relaxed)));
                if count > last_count && silent_ms >= 1500 {
                    last_count = count;
                    let elapsed = poll_progress_started_at.elapsed().as_secs_f32();
                    let secs_per_frame = if new_in_run > 0 { elapsed / new_in_run as f32 } else { 0.0 };
                    let remaining = if count < total_frames && secs_per_frame > 0.0 {
                        Some(secs_per_frame * (total_frames - count) as f32)
                    } else {
                        None
                    };
                    let _ = poll_app.emit(
                        "render-progress",
                        RenderProgressEvent {
                            job_id: poll_job_id.clone(),
                            frame: count.min(total_frames),
                            total_frames,
                            time_elapsed: secs_per_frame,
                            memory_mb: 0.0,
                            remaining_secs: remaining,
                        },
                    );
                }
            }
        });

        // ── Stderr drain ──────────────────────────────────────────────────────
        let stderr_buf_clone = stderr_buf.clone();
        let log_write_lock_stderr = log_write_lock.clone();
        let log_file_path_stderr = log_file_path.clone();
        let app_stderr = app.clone();
        let job_id_stderr = job.id.clone();
        let state_stderr = state.clone();
        let mut stderr_last_frame = actual_start.max(job_frame_start) as u32;
        let stderr_task = tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = app_stderr.emit(
                    "render-log",
                    RenderLogEvent { job_id: job_id_stderr.clone(), line: line.clone() },
                );
                if let Some(frame) = parser::parse_frame(&line) {
                    stderr_last_frame = frame;
                }
                if let Some(p) = parser::parse_time_progress(&line) {
                    let new_this_run = stderr_last_frame
                        .saturating_sub(actual_start.max(1) as u32)
                        .saturating_add(1);
                    let rel = (already_done + new_this_run).min(total_frames);
                    let abs_frame = (job_frame_start + rel as i32 - 1).clamp(job_frame_start, job_frame_end);
                    persist_job_progress(
                        &state_stderr,
                        &job_id_stderr,
                        None,
                        total_frames,
                        Some(abs_frame),
                        Some(p.time_elapsed),
                        p.remaining_secs,
                    )
                    .await;
                }
                let mut stderr_tail = stderr_buf_clone.lock().await;
                if stderr_tail.len() >= 200 {
                    stderr_tail.pop_front();
                }
                stderr_tail.push_back(line.clone());
                let _guard = log_write_lock_stderr.lock().await;
                let _ = crate::app_paths::append_log_line(&log_file_path_stderr, &line);
            }
        });

        // ── Stdout drain ──────────────────────────────────────────────────────
        let log_write_lock_stdout = log_write_lock.clone();
        let log_file_path_stdout = log_file_path.clone();
        let mut saved_in_run = 0u32;
        let mut stdout_last_frame = actual_start.max(job_frame_start) as u32;
        let mut latest_frame_time_secs: Option<f32> = None;
        let mut lines = BufReader::new(stdout).lines();
        while let Some(line) = lines.next_line().await? {
            let _ = app.emit(
                "render-log",
                RenderLogEvent { job_id: job.id.clone(), line: line.clone() },
            );
            {
                let _guard = log_write_lock_stdout.lock().await;
                let _ = crate::app_paths::append_log_line(&log_file_path_stdout, &line);
            }
            if let Some(frame) = parser::parse_frame(&line) {
                stdout_last_frame = frame;
            }
            if line.contains("Saved:") {
                saved_in_run += 1;
                let completed = (already_done + saved_in_run).min(total_frames);
                let abs_frame = (job_frame_start + completed as i32 - 1).clamp(job_frame_start, job_frame_end);
                last_rendered_frame = Some(abs_frame);
                let remaining = latest_frame_time_secs
                    .filter(|_| completed < total_frames)
                    .map(|secs| secs * (total_frames - completed) as f32);
                persist_job_progress(
                    &state,
                    &job.id,
                    Some(completed),
                    total_frames,
                    Some(abs_frame),
                    latest_frame_time_secs,
                    remaining,
                )
                .await;
                mark_progress_timestamp(progress_started_at.as_ref(), &last_primary_progress_ms);
                let _ = app.emit(
                    "render-progress",
                    RenderProgressEvent {
                        job_id: job.id.clone(),
                        frame: completed,
                        total_frames,
                        time_elapsed: latest_frame_time_secs.unwrap_or(0.0),
                        memory_mb: 0.0,
                        remaining_secs: remaining,
                    },
                );
            }
            if let Some(p) = parser::parse_time_progress(&line) {
                latest_frame_time_secs = Some(p.time_elapsed);
                let new_this_run = stdout_last_frame
                    .saturating_sub(actual_start.max(1) as u32)
                    .saturating_add(1);
                let rel = (already_done + new_this_run).min(total_frames);
                let abs_frame = (job_frame_start + rel as i32 - 1).clamp(job_frame_start, job_frame_end);
                persist_job_progress(
                    &state,
                    &job.id,
                    None,
                    total_frames,
                    Some(abs_frame),
                    Some(p.time_elapsed),
                    p.remaining_secs,
                )
                .await;
                mark_progress_timestamp(progress_started_at.as_ref(), &last_primary_progress_ms);
                let _ = app.emit(
                    "render-progress",
                    RenderProgressEvent {
                        job_id: job.id.clone(),
                        frame: rel,
                        total_frames,
                        time_elapsed: p.time_elapsed,
                        memory_mb: 0.0,
                        remaining_secs: p.remaining_secs,
                    },
                );
            }
        }

        let _ = stderr_task.await;
        let exit_status = child.wait().await?;

        render_running.store(false, Ordering::Relaxed);
        let _ = poll_task.await;
        state.active_jobs.lock().await.remove(&job.id);

        if exit_status.success() {
            persist_job_progress(
                &state,
                &job.id,
                Some(total_frames),
                total_frames,
                Some(job.frame_end),
                None,
                Some(0.0),
            )
            .await;
            break Ok(JobStatus::Done);
        }

        if state.interrupted_jobs.lock().await.contains(&job.id) {
            break Err(anyhow::anyhow!("interrupted"));
        }

        // Was the process killed by an explicit cancellation?
        if state.cancelled_jobs.lock().await.contains(&job.id) {
            break Err(anyhow::anyhow!("cancelled"));
        }

        // ── Crash recovery ────────────────────────────────────────────────────
        crash_count += 1;
        let mut resume_job = job.clone();
        resume_job.resume_from_existing = true;
        resume_job.last_rendered_frame = last_rendered_frame;
        let next_start = compute_resume_frame(&resume_job);
        let frames_done = (next_start - job.frame_start).max(0);
        let stderr_tail = stderr_buf
            .lock()
            .await
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");

        if next_start == actual_start && saved_in_run == 0 {
            break Err(anyhow::anyhow!(
                "Blender exited before rendering any new frame.{}",
                if stderr_tail.is_empty() {
                    String::new()
                } else {
                    format!("\nLast output:\n{}", stderr_tail)
                }
            ));
        }

        let recovery_line = if next_start > job.frame_end {
            format!(
                "[crash-recovery] Blender exited with an error (crash #{}) but all {} frames are complete.",
                crash_count, total_frames
            )
        } else if crash_count < MAX_CRASH_RETRIES {
            format!(
                "[crash-recovery] Blender exited with an error (crash #{}/{}). {} frame(s) done — resuming from frame {}…",
                crash_count, MAX_CRASH_RETRIES, frames_done, next_start
            )
        } else {
            format!(
                "[crash-recovery] Blender exited with an error (crash #{}/{}). {} frame(s) done — max retries reached, giving up.",
                crash_count, MAX_CRASH_RETRIES, frames_done
            )
        };

        log::warn!("Job {}: {}", job.id, recovery_line);
        let _ = app.emit(
            "render-log",
            RenderLogEvent { job_id: job.id.clone(), line: recovery_line.clone() },
        );
        {
            let _guard = log_write_lock.lock().await;
            let _ = crate::app_paths::append_log_line(&log_file_path, &recovery_line);
        }

        if next_start > job.frame_end {
            break Ok(JobStatus::Done);
        }

        if crash_count >= MAX_CRASH_RETRIES {
            break Err(anyhow::anyhow!(
                "Blender crashed {} time(s).{}",
                crash_count,
                if stderr_tail.is_empty() { String::new() }
                else { format!("\nLast output:\n{}", stderr_tail) }
            ));
        }

        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    };

    job_result
}

fn spawn_blender(job: &RenderJob, frame_start_actual: i32) -> Result<Child> {
    let mut command = crate::blender::command::render_command(job, frame_start_actual)
        .into_tokio_command();
    command
        .current_dir(
            job.blend_file
                .parent()
                .unwrap_or_else(|| std::path::Path::new(".")),
        )
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    command.process_group(0);
    Ok(command.spawn()?)
}
