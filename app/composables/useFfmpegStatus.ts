import type { FfmpegJobStatus } from '~/types'

export const FFMPEG_STATUS_LABEL: Record<FfmpegJobStatus, string> = {
  pending: '等待中',
  running: '转码中',
  done: '已完成',
  failed: '失败',
  cancelled: '已取消',
}

export const FFMPEG_STATUS_COLOR: Record<FfmpegJobStatus, 'neutral' | 'info' | 'success' | 'error' | 'warning'> = {
  pending: 'neutral',
  running: 'info',
  done: 'success',
  failed: 'error',
  cancelled: 'warning',
}
