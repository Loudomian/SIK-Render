import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
  FfmpegJobUpdatedEvent,
  JobUpdatedEvent,
  QueueState,
  RenderLogEvent,
  RenderProgressEvent,
  TranscodeLogEvent,
  TranscodeProgressEvent,
} from '~/types'

export const useRenderEvents = () => {
  const onProgress = async (
    handler: (event: RenderProgressEvent) => void,
  ): Promise<UnlistenFn> => {
    return listen<RenderProgressEvent>('render-progress', (e) => {
      handler(e.payload)
    })
  }

  const onJobUpdated = async (
    handler: (event: JobUpdatedEvent) => void,
  ): Promise<UnlistenFn> => {
    return listen<JobUpdatedEvent>('job-updated', (e) => {
      handler(e.payload)
    })
  }

  const onLog = async (
    handler: (event: RenderLogEvent) => void,
  ): Promise<UnlistenFn> => {
    return listen<RenderLogEvent>('render-log', (e) => {
      handler(e.payload)
    })
  }

  const onQueueState = async (
    handler: (event: QueueState) => void,
  ): Promise<UnlistenFn> => {
    return listen<QueueState>('queue-state', (e) => {
      handler(e.payload)
    })
  }

  const onTranscodeProgress = async (
    handler: (event: TranscodeProgressEvent) => void,
  ): Promise<UnlistenFn> => {
    return listen<TranscodeProgressEvent>('transcode-progress', (e) => {
      handler(e.payload)
    })
  }

  const onTranscodeLog = async (
    handler: (event: TranscodeLogEvent) => void,
  ): Promise<UnlistenFn> => {
    return listen<TranscodeLogEvent>('transcode-log', (e) => {
      handler(e.payload)
    })
  }

  const onFfmpegJobUpdated = async (
    handler: (event: FfmpegJobUpdatedEvent) => void,
  ): Promise<UnlistenFn> => {
    return listen<FfmpegJobUpdatedEvent>('ffmpeg-job-updated', (e) => {
      handler(e.payload)
    })
  }

  return {
    onProgress,
    onJobUpdated,
    onLog,
    onQueueState,
    onTranscodeProgress,
    onTranscodeLog,
    onFfmpegJobUpdated,
  }
}
