import { invoke } from '@tauri-apps/api/core'
import type { AddFfmpegJobPayload, AddJobPayload, AppSettings, BlenderInstall, BlendProjectSettings, FfmpegJob, FolderFrameGroup, FolderFramesInspection, JobLogSummary, NodeInfo, NodeInterfaceInfo, NodeJobEvent, OutputPathTemplatePreview, PathTemplateKind, PeerInfo, QueueState, RenderJob, RenderedFramesStatus, RuntimeResetResult, ShadowRecoveryResponse, ToolchainStatus } from '~/types'

// Typed wrappers around Tauri IPC commands

const tauriApi = {
  listJobs: () =>
    invoke<RenderJob[]>('list_jobs'),

  getQueueState: () =>
    invoke<QueueState>('get_queue_state'),

  getNodeInfo: () =>
    invoke<NodeInfo>('get_node_info'),

  getPeers: () =>
    invoke<PeerInfo[]>('get_peers'),

  getNodeJobEvents: (nodeId: string, jobId: string) =>
    invoke<NodeJobEvent[]>('get_node_job_events', { nodeId, jobId }),

  forgetPeer: (nodeId: string) =>
    invoke<void>('forget_peer', { nodeId }),

  listNodeInterfaces: () =>
    invoke<NodeInterfaceInfo[]>('list_node_interfaces'),

  startQueue: () =>
    invoke<QueueState>('start_queue'),

  pauseQueue: () =>
    invoke<QueueState>('pause_queue'),

  applyShadowResolutionRecovery: (id: string) =>
    invoke<ShadowRecoveryResponse>('apply_shadow_resolution_recovery', { id }),

  addJob: (payload: AddJobPayload) =>
    invoke<RenderJob>('add_job', { payload }),

  updateJobMetadata: (id: string, name: string, note?: string | null) =>
    invoke<RenderJob>('update_job_metadata', { payload: { id, name, note } }),

  updateJobTranscodeSettings: (
    payload: {
      id: string
      auto_transcode_mp4: boolean
      transcode_name_override: string | null
      transcode_fps_override: number | null
      transcode_output_path_override: string | null
      transcode_crf_override: number | null
      transcode_preset_override: string | null
      transcode_frame_start_override: number | null
      transcode_frame_end_override: number | null
    },
  ) =>
    invoke<RenderJob>('update_job_transcode_settings', { payload }),

  updateJobFps: (id: string, fps: number) =>
    invoke<RenderJob>('update_job_fps', { payload: { id, fps } }),

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

  inspectFolderFrames: (folderPath: string, formatHint?: string | null) =>
    invoke<FolderFramesInspection>('inspect_folder_frames', {
      folderPath,
      formatHint: formatHint ?? null,
    }),

  scanFolderFrameGroups: (folderPath: string) =>
    invoke<{ groups: FolderFrameGroup[] }>('scan_folder_frame_groups', { folderPath }),

  openPath: (path: string) =>
    invoke<void>('open_path', { path }),

  validateBlendFile: (path: string) =>
    invoke<boolean>('validate_blend_file', { path }),

  inspectBlendFile: (blenderExecutable: string, path: string) =>
    invoke<BlendProjectSettings>('inspect_blend_file', { blenderExecutable, path }),

  previewOutputPathTemplate: (payload: {
    kind: PathTemplateKind
    template: string
    blend_file?: string | null
    source_path?: string | null
    frame_start: number
    frame_end: number
  }) =>
    invoke<OutputPathTemplatePreview>('preview_output_path_template', { payload }),

  pathExists: (path: string) =>
    invoke<boolean>('path_exists', { path }),

  getJobLogs: (jobId: string) =>
    invoke<string[]>('get_job_logs', { jobId }),

  getJobLatestLogs: (jobId: string) =>
    invoke<string[]>('get_job_latest_logs', { jobId }),

  getJobLogSummary: (jobId: string) =>
    invoke<JobLogSummary>('get_job_log_summary', { jobId }),

  listFfmpegJobs: () =>
    invoke<FfmpegJob[]>('list_ffmpeg_jobs'),

  getFfmpegJob: (id: string) =>
    invoke<FfmpegJob>('get_ffmpeg_job', { id }),

  addFfmpegJob: (payload: AddFfmpegJobPayload) =>
    invoke<FfmpegJob>('add_ffmpeg_job', { payload }),

  cancelFfmpegJob: (id: string) =>
    invoke<void>('cancel_ffmpeg_job', { id }),

  deleteFfmpegJob: (id: string) =>
    invoke<void>('delete_ffmpeg_job', { id }),

  reorderFfmpegJobs: (orderedIds: string[]) =>
    invoke<FfmpegJob[]>('reorder_ffmpeg_jobs', { orderedIds }),

  getFfmpegJobLogs: (jobId: string) =>
    invoke<string[]>('get_ffmpeg_job_logs', { jobId }),

  getFfmpegJobLatestLogs: (jobId: string) =>
    invoke<string[]>('get_ffmpeg_job_latest_logs', { jobId }),

  getSettings: () =>
    invoke<AppSettings>('get_settings'),

  saveSettings: (settings: AppSettings) =>
    invoke<void>('save_settings', { settings }),

  resetAppRuntimeData: () =>
    invoke<RuntimeResetResult>('reset_app_runtime_data'),
}

export const useTauri = () => tauriApi
