import type { FfmpegJobStatus } from '~/types'

export const FFMPEG_STATUS_KEY: Record<FfmpegJobStatus, string> = {
  pending: 'ffmpegStatus.pending',
  running: 'ffmpegStatus.running',
  done: 'ffmpegStatus.done',
  failed: 'ffmpegStatus.failed',
  cancelled: 'ffmpegStatus.cancelled',
}

export const FFMPEG_STATUS_COLOR: Record<FfmpegJobStatus, 'neutral' | 'info' | 'success' | 'error' | 'warning'> = {
  pending: 'neutral',
  running: 'info',
  done: 'success',
  failed: 'error',
  cancelled: 'warning',
}

export function useFfmpegStatusLabel() {
  const { t } = useI18n()
  return (status: FfmpegJobStatus) => t(FFMPEG_STATUS_KEY[status] ?? status)
}
