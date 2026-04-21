import { invoke } from '@tauri-apps/api/core'
import type { AddJobPayload, AppSettings, BlenderInstall, BlendProjectSettings, JobLogSummary, Mp4ExportInspection, Mp4ExportResult, QueueState, RenderJob, RenderedFramesStatus, ToolchainStatus } from '~/types'

// Typed wrappers around Tauri IPC commands

const tauriApi = {
  listJobs: () =>
    invoke<RenderJob[]>('list_jobs'),

  getQueueState: () =>
    invoke<QueueState>('get_queue_state'),

  startQueue: () =>
    invoke<QueueState>('start_queue'),

  pauseQueue: () =>
    invoke<QueueState>('pause_queue'),

  addJob: (payload: AddJobPayload) =>
    invoke<RenderJob>('add_job', { payload }),

  removeJob: (id: string) =>
    invoke<void>('remove_job', { id }),

  resetJob: (id: string, resumeFromExisting: boolean, frameStart?: number | null, frameEnd?: number | null) =>
    invoke<RenderJob>('reset_job', { id, resumeFromExisting, frameStart, frameEnd }),

  updateJobPreviewDimensions: (id: string, width: number, height: number) =>
    invoke<RenderJob>('update_job_preview_dimensions', { id, width, height }),

  cancelJob: (id: string) =>
    invoke<void>('cancel_job', { id }),

  reorderJob: (orderedIds: string[]) =>
    invoke<RenderJob[]>('reorder_job', { orderedIds }),

  getBlenderVersions: () =>
    invoke<BlenderInstall[]>('get_blender_versions'),

  inspectToolchain: () =>
    invoke<ToolchainStatus>('inspect_toolchain'),

  addBlenderByPath: (path: string) =>
    invoke<BlenderInstall>('add_blender_by_path', { path }),

  hasOutputFiles: (path: string) =>
    invoke<number>('has_output_files', { path }),

  countRenderedFrames: (outputPath: string, format: string, frameStart: number, frameEnd: number) =>
    invoke<number>('count_rendered_frames', { outputPath, format, frameStart, frameEnd }),

  inspectRenderedFrames: (outputPath: string, format: string, frameStart: number, frameEnd: number) =>
    invoke<RenderedFramesStatus>('inspect_rendered_frames', { outputPath, format, frameStart, frameEnd }),

  clearRenderedFrames: (outputPath: string, format: string, frameStart: number, frameEnd: number) =>
    invoke<number>('clear_rendered_frames', { outputPath, format, frameStart, frameEnd }),

  getLastRenderedFrame: (outputPath: string, format: string, frameStart: number, frameEnd: number) =>
    invoke<string | null>('get_last_rendered_frame', { outputPath, format, frameStart, frameEnd }),

  inspectMp4Export: (
    outputPath: string,
    format: string,
    jobFrameStart: number,
    jobFrameEnd: number,
    rangeMode: string,
    customStart?: number | null,
    customEnd?: number | null,
  ) =>
    invoke<Mp4ExportInspection>('inspect_mp4_export', {
      outputPath,
      format,
      jobFrameStart,
      jobFrameEnd,
      rangeMode,
      customStart,
      customEnd,
    }),

  encodeSequenceToMp4: (
    jobId: string,
    blenderExecutable: string,
    blendFile: string,
    outputPath: string,
    format: string,
    frameStart: number,
    frameEnd: number,
    strictContiguous: boolean,
  ) =>
    invoke<Mp4ExportResult>('encode_sequence_to_mp4', {
      jobId,
      blenderExecutable,
      blendFile,
      outputPath,
      format,
      frameStart,
      frameEnd,
      strictContiguous,
    }),

  cancelMp4Export: (jobId: string) =>
    invoke<void>('cancel_mp4_export', { jobId }),

  openPath: (path: string) =>
    invoke<void>('open_path', { path }),

  validateBlendFile: (path: string) =>
    invoke<boolean>('validate_blend_file', { path }),

  inspectBlendFile: (blenderExecutable: string, path: string) =>
    invoke<BlendProjectSettings>('inspect_blend_file', { blenderExecutable, path }),

  getJobLogs: (jobId: string) =>
    invoke<string[]>('get_job_logs', { jobId }),

  getJobLatestLogs: (jobId: string) =>
    invoke<string[]>('get_job_latest_logs', { jobId }),

  getJobMp4Logs: (jobId: string) =>
    invoke<string[]>('get_job_mp4_logs', { jobId }),

  getJobLatestMp4Logs: (jobId: string) =>
    invoke<string[]>('get_job_latest_mp4_logs', { jobId }),

  getJobLogSummary: (jobId: string) =>
    invoke<JobLogSummary>('get_job_log_summary', { jobId }),

  getSettings: () =>
    invoke<AppSettings>('get_settings'),

  saveSettings: (settings: AppSettings) =>
    invoke<void>('save_settings', { settings }),
}

export const useTauri = () => tauriApi
