import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { JobUpdatedEvent, RenderLogEvent, RenderProgressEvent } from '~/types'

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

  return { onProgress, onJobUpdated, onLog }
}
