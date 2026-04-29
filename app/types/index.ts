// Shared TypeScript types — must be kept in sync with Rust structs in src-tauri/src/queue/job.rs.
// IPC request payloads keep Rust field names (snake_case) so Tauri command args map directly.
// IPC responses are serde-serialized Rust structs with `rename_all = "camelCase"`.

export type JobStatus = 'pending' | 'running' | 'done' | 'failed' | 'cancelled' | 'interrupted'
export type RenderMode = 'image_sequence' | 'quick_mp4'

export interface RenderJob {
  id: string
  jobNumber: number
  name: string
  note: string | null
  crashCount: number
  autoTranscodeMp4: boolean
  transcodeNameOverride: string | null
  transcodeFpsOverride: number | null
  transcodeOutputPathOverride: string | null
  transcodeCrfOverride: number | null
  transcodePresetOverride: string | null
  transcodeFrameStartOverride: number | null
  transcodeFrameEndOverride: number | null
  fps?: number | null
  blendFile: string
  blenderExecutable: string
  outputPath: string
  outputFormat: string
  renderMode: RenderMode
  originalFrameStart: number
  originalFrameEnd: number
  frameStart: number
  frameEnd: number
  previewWidth: number | null
  previewHeight: number | null
  previewImagePath: string | null
  resumeFromExisting: boolean
  status: JobStatus
  priority: number
  createdAt: number
  startedAt: number | null
  finishedAt: number | null
  currentFrame?: number | null
  totalFrames?: number | null
  lastRenderedFrame?: number | null
  timeElapsed?: number | null
  remainingSecs?: number | null
}

export interface AddJobPayload {
  name: string
  note?: string | null
  render_mode: RenderMode
  auto_transcode_mp4: boolean
  transcode_name_override?: string | null
  transcode_fps_override?: number | null
  transcode_output_path_override?: string | null
  transcode_crf_override?: number | null
  transcode_preset_override?: string | null
  transcode_frame_start_override?: number | null
  transcode_frame_end_override?: number | null
  fps?: number | null
  blend_file: string
  blender_executable: string
  output_path: string
  output_format: string
  frame_start: number
  frame_end: number
  preview_width?: number | null
  preview_height?: number | null
  resume_from_existing: boolean
  initial_current_frame?: number | null
  initial_total_frames?: number | null
  initial_last_rendered_frame?: number | null
  priority: number
}

export interface RenderProgressEvent {
  jobId: string
  frame: number
  totalFrames: number
  // Estimated single-frame render seconds used by the UI for "单帧".
  timeElapsed: number
  memoryMb: number
  remainingSecs: number | null
}

export interface JobUpdatedEvent {
  job: RenderJob
}

export interface RenderLogEvent {
  jobId: string
  line: string
}

export type FfmpegJobStatus = 'pending' | 'running' | 'done' | 'failed' | 'cancelled'
export type FfmpegJobSourceType = 'blender_job' | 'folder'

export interface FfmpegJob {
  id: string
  jobNumber: number
  name: string
  sourceType: FfmpegJobSourceType
  sourceBlenderJobId: string | null
  inputPath: string
  frameStart: number
  frameEnd: number
  fps: number
  outputPath: string
  crf: number
  preset: string
  status: FfmpegJobStatus
  priority: number
  createdAt: number
  startedAt: number | null
  finishedAt: number | null
  progressFrame: number | null
  totalFrames: number | null
  outputSizeBytes: number | null
  outputDurationSecs: number | null
}

export interface AddFfmpegJobPayload {
  name: string
  source_type: 'blender_job' | 'folder'
  source_blender_job_id?: string | null
  input_path: string
  frame_start: number
  frame_end: number
  fps: number
  output_path: string
  crf: number
  preset: string
}

export interface RenderJobTranscodeConfig {
  name: string
  fps: number
  outputPath: string
  outputDir: string
  outputStem: string
  crf: number
  preset: string
}

export type PathTemplateKind = 'blender' | 'blender-ffmpeg' | 'standalone-ffmpeg'

export interface OutputPathTemplatePreview {
  resolvedPath: string | null
  errors: string[]
  notes: string[]
}

export interface TranscodeProgressEvent {
  jobId: string
  frame: number
  totalFrames: number
  encodeSpeed: number | null
}

export interface TranscodeLogEvent {
  jobId: string
  line: string
}

export interface FfmpegJobUpdatedEvent {
  job: FfmpegJob
}

export interface FolderFramesInspection {
  detectedFormat: string | null
  frameStart: number | null
  frameEnd: number | null
  frameCount: number
  missingCount: number
  hasGaps: boolean
  folderName: string
}

export interface FolderFrameGroup {
  name: string
  inputPath: string
  frameStart: number
  frameEnd: number
  frameCount: number
  detectedFormat: string
}

export interface JobLogSummary {
  directory: string
  blenderCount: number
  ffmpegCount: number
  totalCount: number
}

export interface QueueState {
  paused: boolean
  pausedJob: string | null
}

export interface BlenderInstall {
  version: string
  executable: string
}

export interface ToolchainStatus {
  blenderInstalls: BlenderInstall[]
  ffmpegFound: boolean
  ffmpegExecutable: string | null
  ffmpegSource: string | null
}

export interface BlendProjectSettings {
  frameStart: number
  frameEnd: number
  outputPath: string
  outputFormat: string
  engine: string
  resolutionX: number
  resolutionY: number
  fps: number
}

export interface RenderedFramesStatus {
  frameCount: number
  lastFrame: number | null
  nextFrame: number
}

export interface AppSettings {
  defaultBlender: string
  ffmpegExecutable: string
  blendInspectTimeoutSeconds: number
  transcodeCrf: number
  transcodePreset: string
  ffmpegMaxConcurrent: number
  renderOutputPathTemplate: string
  blenderTranscodeOutputPathTemplate: string
  standaloneTranscodeOutputPathTemplate: string
  pngColorMode: 'BW' | 'RGB' | 'RGBA'
  pngColorDepth: number
  pngCompression: number
  exrColorMode: 'BW' | 'RGB' | 'RGBA'
  exrColorDepth: number
  exrCodec: 'NONE' | 'ZIP' | 'PIZ' | 'DWAA' | 'DWAB' | 'ZIPS' | 'RLE' | 'PXR24' | 'B44' | 'B44A'
  exrQuality: number
  theme: 'dark' | 'light' | 'system'
  extraBlenderPaths: string[]
  excludedBlenderPaths: string[]
  maxCrashRetries: number
  nodePort: number
  nodeInterfaceAddress: string
  nodeNote: string
}

export interface NodeInfo {
  id: string
  hostname: string
  note: string
  version: string
  ipAddress: string
  port: number
}

export interface NodeInterfaceInfo {
  name: string
  ipAddress: string
}

export interface PeerInfo {
  node: NodeInfo
  jobs: RenderJob[]
  queuePaused: boolean
  connected: boolean
}

export interface PeerDiscoveredEvent {
  peer: PeerInfo
}

export interface PeerLostEvent {
  nodeId: string
}

export interface PeerJobUpdatedEvent {
  nodeId: string
  job: RenderJob
}

export interface PeerQueueStateEvent {
  nodeId: string
  paused: boolean
}

export interface PeerProgressEvent {
  nodeId: string
  jobId: string
  frame: number
  totalFrames: number
  timeElapsed: number
  memoryMb: number
  remainingSecs: number | null
}
