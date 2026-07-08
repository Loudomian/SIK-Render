import { defineStore } from 'pinia'
import type {
  AddFfmpegJobPayload,
  FfmpegJob,
  FfmpegJobUpdatedEvent,
  TranscodeLogEvent,
  TranscodeProgressEvent,
} from '~/types'

const MAX_LOG_LINES = 5000
const TERMINAL_FFMPEG_JOB_STATUSES = new Set<FfmpegJob['status']>(['done', 'failed', 'cancelled'])

export const useTranscodeStore = defineStore('transcode', () => {
  const ffmpegJobs = ref<FfmpegJob[]>([])
  const logs = ref<Record<string, string[]>>({})
  const loading = ref(false)
  const {
    listFfmpegJobs,
    getFfmpegJob,
    addFfmpegJob: invokeAddFfmpegJob,
    cancelFfmpegJob: invokeCancelFfmpegJob,
    deleteFfmpegJob: invokeDeleteFfmpegJob,
    reorderFfmpegJobs: invokeReorderFfmpegJobs,
    getFfmpegJobLatestLogs,
  } = useTauri()

  function sortJobs() {
    ffmpegJobs.value = [...ffmpegJobs.value].sort(
      (a, b) => a.priority - b.priority || a.createdAt - b.createdAt,
    )
  }

  function setLogLines(jobId: string, lines: string[]) {
    logs.value[jobId] = lines.slice(-MAX_LOG_LINES)
  }

  function appendLogLine(jobId: string, line: string) {
    const existing = [...(logs.value[jobId] ?? []), line]
    logs.value[jobId] = existing.slice(-MAX_LOG_LINES)
  }

  async function fetchFfmpegJobs() {
    loading.value = true
    try {
      ffmpegJobs.value = await listFfmpegJobs()
      sortJobs()
    } finally {
      loading.value = false
    }
  }

  async function fetchFfmpegJob(id: string) {
    const job = await getFfmpegJob(id)
    applyFfmpegJobUpdate({ job })
    return job
  }

  async function submitFfmpegJob(payload: AddFfmpegJobPayload) {
    const job = await invokeAddFfmpegJob(payload)
    applyFfmpegJobUpdate({ job })
    return job
  }

  async function cancelFfmpegJob(id: string) {
    await invokeCancelFfmpegJob(id)
  }

  async function deleteFfmpegJob(id: string) {
    await invokeDeleteFfmpegJob(id)
    ffmpegJobs.value = ffmpegJobs.value.filter(job => job.id !== id)
    delete logs.value[id]
  }

  async function clearCompletedJobs() {
    const ids = ffmpegJobs.value.filter(job => job.status === 'done').map(job => job.id)
    if (!ids.length) {
      return { removed: 0, failed: 0 }
    }

    const results = await Promise.allSettled(ids.map(id => deleteFfmpegJob(id)))
    return {
      removed: results.filter(result => result.status === 'fulfilled').length,
      failed: results.filter(result => result.status === 'rejected').length,
    }
  }

  async function reorderPendingJobs(orderedIds: string[]) {
    ffmpegJobs.value = await invokeReorderFfmpegJobs(orderedIds)
    sortJobs()
  }

  async function loadFfmpegJobLogs(id: string) {
    if (logs.value[id]?.length) return logs.value[id]
    const lines = await getFfmpegJobLatestLogs(id)
    setLogLines(id, lines)
    return logs.value[id] ?? []
  }

  function applyProgress(event: TranscodeProgressEvent) {
    const job = ffmpegJobs.value.find(entry => entry.id === event.jobId)
    if (!job) return
    if (TERMINAL_FFMPEG_JOB_STATUSES.has(job.status)) return
    job.status = 'running'
    job.progressFrame = Math.max(job.progressFrame ?? 0, event.frame)
    job.totalFrames = event.totalFrames
  }

  function applyLog(event: TranscodeLogEvent) {
    appendLogLine(event.jobId, event.line)
  }

  function applyFfmpegJobUpdate(event: FfmpegJobUpdatedEvent) {
    const index = ffmpegJobs.value.findIndex(job => job.id === event.job.id)
    if (index === -1) {
      ffmpegJobs.value.push(event.job)
      sortJobs()
    } else {
      const current = ffmpegJobs.value[index]
      if (!current) return
      const shouldSort = current.priority !== event.job.priority || current.createdAt !== event.job.createdAt
      ffmpegJobs.value[index] = {
        ...current,
        ...event.job,
        progressFrame: event.job.status === 'running'
          ? Math.max(current.progressFrame ?? 0, event.job.progressFrame ?? 0)
          : event.job.progressFrame,
      }
      if (shouldSort) sortJobs()
    }
  }

  function getFfmpegJobById(id: string) {
    return ffmpegJobs.value.find(job => job.id === id) ?? null
  }

  function getRelatedJobs(blenderJobId: string) {
    return ffmpegJobs.value
      .filter(job => job.sourceBlenderJobId === blenderJobId)
      .sort((a, b) => b.createdAt - a.createdAt)
  }

  return {
    ffmpegJobs,
    logs,
    loading,
    fetchFfmpegJobs,
    fetchFfmpegJob,
    submitFfmpegJob,
    cancelFfmpegJob,
    deleteFfmpegJob,
    clearCompletedJobs,
    reorderPendingJobs,
    loadFfmpegJobLogs,
    applyProgress,
    applyLog,
    applyFfmpegJobUpdate,
    getFfmpegJobById,
    getRelatedJobs,
  }
})
