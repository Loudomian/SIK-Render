// Shared TypeScript types — must be kept in sync with Rust structs in src-tauri/src/queue/job.rs

export type JobStatus = 'pending' | 'running' | 'done' | 'failed' | 'cancelled' | 'interrupted'

export interface RenderJob {
  id: string
  jobNumber: number
  name: string
  blendFile: string
  blenderExecutable: string
  outputPath: string
  outputFormat: string
  frameStart: number
  frameEnd: number
  previewWidth: number | null
  previewHeight: number | null
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
  blend_file: string
  blender_executable: string
  output_path: string
  output_format: string
  frame_start: number
  frame_end: number
  preview_width?: number | null
  preview_height?: number | null
  resume_from_existing: boolean
  priority: number
}

export interface RenderProgressEvent {
  jobId: string
  frame: number
  totalFrames: number
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

export interface Mp4LogEvent {
  jobId: string
  line: string
}

export interface JobLogSummary {
  directory: string
  blenderCount: number
  ffmpegCount: number
  totalCount: number
}

export interface BlenderInstall {
  version: string
  executable: string
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

export interface Mp4ExportResult {
  outputPath: string
  fps: number
  frameCount: number
}

export type Mp4RangeMode = 'job' | 'all' | 'custom'

export interface Mp4ExportInspection {
  availableStart: number | null
  availableEnd: number | null
  selectedStart: number | null
  selectedEnd: number | null
  frameCount: number
  missingCount: number
  hasGaps: boolean
  missingSegments: string[]
  missingSegmentsTruncated: boolean
}

export interface RenderedFramesStatus {
  frameCount: number
  lastFrame: number | null
  nextFrame: number
}

export interface AppSettings {
  defaultBlender: string
  ffmpegExecutable: string
  defaultOutputDir: string
  maxConcurrentJobs: number
  theme: 'dark' | 'light'
  extraBlenderPaths: string[]
  excludedBlenderPaths: string[]
}
