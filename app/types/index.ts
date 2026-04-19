// Shared TypeScript types — must be kept in sync with Rust structs in src-tauri/src/queue/job.rs

export type JobStatus = 'pending' | 'running' | 'done' | 'failed' | 'cancelled'

export interface RenderJob {
  id: string
  name: string
  blendFile: string
  blenderExecutable: string
  outputPath: string
  outputFormat: string
  frameStart: number
  frameEnd: number
  status: JobStatus
  priority: number
  createdAt: number
  startedAt: number | null
  finishedAt: number | null
}

export interface AddJobPayload {
  name: string
  blend_file: string
  blender_executable: string
  output_path: string
  output_format: string
  frame_start: number
  frame_end: number
  priority: number
}

export interface RenderProgressEvent {
  jobId: string
  frame: number
  totalFrames: number
  timeElapsed: number
  memoryMb: number
}

export interface BlenderInstall {
  version: string
  executable: string
}

export interface AppSettings {
  defaultBlender: string
  defaultOutputDir: string
  maxConcurrentJobs: number
  theme: 'dark' | 'light'
}
