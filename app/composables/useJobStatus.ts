import type { JobStatus } from '~/types'

export const JOB_STATUS_LABEL: Record<JobStatus, string> = {
  pending: '等待中',
  running: '渲染中',
  done: '已完成',
  failed: '失败',
  cancelled: '已取消',
  interrupted: '已中断',
}

export const JOB_STATUS_COLOR: Record<JobStatus, 'neutral' | 'info' | 'success' | 'error' | 'warning'> = {
  pending: 'neutral',
  running: 'info',
  done: 'success',
  failed: 'error',
  cancelled: 'warning',
  interrupted: 'warning',
}
