import type { JobStatus } from '~/types'

export const JOB_STATUS_KEY: Record<JobStatus, string> = {
  pending: 'jobStatus.pending',
  running: 'jobStatus.running',
  done: 'jobStatus.done',
  failed: 'jobStatus.failed',
  cancelled: 'jobStatus.cancelled',
  interrupted: 'jobStatus.interrupted',
}

export const JOB_STATUS_COLOR: Record<JobStatus, 'neutral' | 'info' | 'success' | 'error' | 'warning'> = {
  pending: 'neutral',
  running: 'info',
  done: 'success',
  failed: 'error',
  cancelled: 'warning',
  interrupted: 'warning',
}

export function useJobStatusLabel() {
  const { t } = useI18n()
  return (status: JobStatus) => t(JOB_STATUS_KEY[status] ?? status)
}
