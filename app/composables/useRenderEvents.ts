import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { RenderProgressEvent } from '~/types'

/**
 * Subscribe to render progress events emitted by the Rust backend.
 * Returns an unlisten function — call it on component unmount.
 */
export const useRenderEvents = () => {
  const onProgress = async (
    handler: (event: RenderProgressEvent) => void,
  ): Promise<UnlistenFn> => {
    return listen<RenderProgressEvent>('render-progress', (e) => {
      handler(e.payload)
    })
  }

  return { onProgress }
}
