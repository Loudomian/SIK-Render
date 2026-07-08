import { defineStore } from 'pinia'
import type { AddJobPayload, JobPreviewDimensionsUpdate, JobUpdatedEvent, RenderJob, RenderLogEvent, RenderProgressEvent } from '~/types'

const MAX_LOG_LINES = 5000

export const useJobsStore = defineStore('jobs', () => {
  const jobs = ref<RenderJob[]>([])
  const logs = ref<Record<string, string[]>>({})
  const renderStarted = ref<Record<string, boolean>>({})
  const loading = ref(false)
  const queuePaused = ref(true)
  const pausedJobId = ref<string | null>(null)
  const {
    listJobs,
    getQueueState,
    startQueue: invokeStartQueue,
    pauseQueue: invokePauseQueue,
    addJob,
    updateJobMetadata: invokeUpdateJobMetadata,
    updateJobTranscodeSettings: invokeUpdateJobTranscodeSettings,
    removeJob,
    cancelJob,
    resetJob,
    reorderJob,
    getJobLatestLogs: fetchJobLogs,
  } = useTauri()

  function sortJobs() {
    jobs.value = [...jobs.value].sort((a, b) =>
      a.priority - b.priority || a.createdAt - b.createdAt,
    )
  }

  async function fetchJobs() {
    loading.value = true
    try {
      jobs.value = await listJobs()
      renderStarted.value = Object.fromEntries(
        jobs.value.map(job => [
          job.id,
          job.status === 'running' && (((job.timeElapsed ?? 0) > 0) || ((job.remainingSecs ?? 0) > 0)),
        ]),
      )
      sortJobs()
    } finally {
      loading.value = false
    }
  }

  async function fetchQueueState() {
    const state = await getQueueState()
    queuePaused.value = state.paused
    pausedJobId.value = state.pausedJob ?? null
  }

  async function startQueue() {
    const state = await invokeStartQueue()
    queuePaused.value = state.paused
    pausedJobId.value = state.pausedJob ?? null
  }

  async function pauseQueue() {
    const state = await invokePauseQueue()
    queuePaused.value = state.paused
    pausedJobId.value = state.pausedJob ?? null
  }

  function applyQueueState(state: { paused: boolean; pausedJob?: string | null }) {
    queuePaused.value = state.paused
    pausedJobId.value = state.pausedJob ?? null
  }

  function applyPreviewDimensions(update: JobPreviewDimensionsUpdate) {
    const job = jobs.value.find(job => job.id === update.id)
    if (!job) return
    job.previewWidth = update.previewWidth
    job.previewHeight = update.previewHeight
  }

  async function submitJob(payload: AddJobPayload) {
    const job = await addJob(payload)
    // job-updated may arrive during the await and set status/progress first.
    // Merge the add_job response so static fields like fps and preview size are not lost.
    const existing = jobs.value.find((j) => j.id === job.id)
    if (!existing) {
      jobs.value.push(job)
    } else {
      Object.assign(existing, {
        ...job,
        status: existing.status === 'running' ? existing.status : job.status,
        currentFrame: existing.currentFrame ?? job.currentFrame,
        totalFrames: existing.totalFrames ?? job.totalFrames,
        lastRenderedFrame: existing.lastRenderedFrame ?? job.lastRenderedFrame,
        timeElapsed: existing.timeElapsed ?? job.timeElapsed,
        remainingSecs: existing.remainingSecs ?? job.remainingSecs,
      })
    }
    sortJobs()
    return job
  }

  async function reorderQueueJobs(orderedIds: string[]) {
    jobs.value = await reorderJob(orderedIds)
    sortJobs()
  }

  async function updateJobMetadata(id: string, name: string, note?: string | null) {
    const updated = await invokeUpdateJobMetadata(id, name, note ?? null)
    const index = jobs.value.findIndex(job => job.id === updated.id)
    if (index === -1) {
      jobs.value.push(updated)
    } else {
      jobs.value[index] = {
        ...jobs.value[index],
        ...updated,
      }
    }
    sortJobs()
    return updated
  }

  async function updateJobTranscodeSettings(payload: {
    id: string
    auto_transcode_mp4: boolean
    transcode_name_override: string | null
    transcode_fps_override: number | null
      transcode_output_path_override: string | null
      transcode_crf_override: number | null
      transcode_preset_override: string | null
      transcode_frame_start_override: number | null
      transcode_frame_end_override: number | null
    }) {
    const updated = await invokeUpdateJobTranscodeSettings(payload)
    const index = jobs.value.findIndex(job => job.id === updated.id)
    if (index === -1) {
      jobs.value.push(updated)
    } else {
      jobs.value[index] = {
        ...jobs.value[index],
        ...updated,
      }
    }
    sortJobs()
    return updated
  }

  async function deleteJob(id: string) {
    await removeJob(id)
    jobs.value = jobs.value.filter((j) => j.id !== id)
    delete logs.value[id]
    delete renderStarted.value[id]
  }

  async function clearCompletedJobs() {
    const ids = doneJobs.value.map(job => job.id)
    if (!ids.length) {
      return { removed: 0, failed: 0 }
    }

    const results = await Promise.allSettled(ids.map(id => deleteJob(id)))
    return {
      removed: results.filter(result => result.status === 'fulfilled').length,
      failed: results.filter(result => result.status === 'rejected').length,
    }
  }

  function _applyReset(updated: RenderJob) {
    const idx = jobs.value.findIndex((j) => j.id === updated.id)
    if (idx !== -1) jobs.value[idx] = updated
    // Clear stale log/progress state so the new run starts fresh in the UI
    delete logs.value[updated.id]
    renderStarted.value[updated.id] = false
    sortJobs()
  }

  // Continue from first missing frame (backend auto-detects via find_resume_frame)
  async function retryJob(
    job: RenderJob,
    resumeFromExisting = true,
    frameRange?: { start: number, end: number },
  ) {
    const updated = await resetJob(
      job.id,
      resumeFromExisting,
      frameRange?.start ?? null,
      frameRange?.end ?? null,
    )
    _applyReset(updated)
  }

  async function retryJobFromStart(job: RenderJob, frameRange?: { start: number, end: number }) {
    await retryJob(job, false, frameRange)
  }

  async function stopJob(id: string) {
    await cancelJob(id)
    const job = jobs.value.find((j) => j.id === id)
    if (job) job.status = 'cancelled'
    delete renderStarted.value[id]
    sortJobs()
  }

  // Fallback: file-poll progress events from the backend
  function applyProgress(event: RenderProgressEvent) {
    const job = jobs.value.find((j) => j.id === event.jobId)
    if (!job) return
    job.status = 'running'
    job.currentFrame = Math.max(job.currentFrame ?? 0, event.frame)
    job.totalFrames = event.totalFrames
    job.lastRenderedFrame = Math.min(job.frameEnd, Math.max(job.frameStart, job.frameStart + event.frame - 1))
    if (event.timeElapsed) job.timeElapsed = event.timeElapsed
    if (event.remainingSecs != null) job.remainingSecs = event.remainingSecs
    if (event.timeElapsed > 0 || event.remainingSecs != null) {
      renderStarted.value[event.jobId] = true
    }
  }

  function applyLog(event: RenderLogEvent) {
    const current = logs.value[event.jobId] ?? []
    if (current.at(-1) === event.line) return
    const existing = [...current, event.line]
    logs.value[event.jobId] = existing.slice(-MAX_LOG_LINES)
    if (/\bFra:\d+/.test(event.line) || /\bSaved:\s/i.test(event.line)) {
      renderStarted.value[event.jobId] = true
    }
  }

  function getJobLogs(id: string): string[] {
    return logs.value[id] ?? []
  }

  async function loadJobLogs(id: string) {
    if (logs.value[id]?.length) return
    const lines = await fetchJobLogs(id)
    if (lines.length) {
      logs.value[id] = lines.slice(-MAX_LOG_LINES)
      if (lines.some(line => /\bFra:\d+/.test(line) || /\bSaved:\s/i.test(line))) {
        renderStarted.value[id] = true
      }
    }
  }

  function applyJobUpdate(event: JobUpdatedEvent) {
    const index = jobs.value.findIndex((job) => job.id === event.job.id)
    if (index === -1) {
      jobs.value.push(event.job)
      sortJobs()
      return
    }
    const current = jobs.value[index]
    if (!current) return
    const shouldSort = current.priority !== event.job.priority || current.createdAt !== event.job.createdAt
    const isRunning = event.job.status === 'running'
    const storedFrame = current.currentFrame ?? 0
    const incomingFrame = event.job.currentFrame ?? 0
    // A job-updated event carries a DB snapshot that may lag behind a render-progress
    // event already applied to the store. For running jobs, never let frame counters
    // go backwards, and skip timing fields entirely when the snapshot is stale.
    const timingIsStale = isRunning && incomingFrame < storedFrame
    jobs.value[index] = {
      ...current,
      ...event.job,
      currentFrame: isRunning ? (Math.max(storedFrame, incomingFrame) || undefined) : event.job.currentFrame,
      totalFrames: event.job.totalFrames ?? (isRunning ? current.totalFrames : undefined),
      lastRenderedFrame: isRunning
        ? (current.lastRenderedFrame != null && event.job.lastRenderedFrame != null
            ? Math.max(current.lastRenderedFrame, event.job.lastRenderedFrame)
            : (current.lastRenderedFrame ?? event.job.lastRenderedFrame))
        : event.job.lastRenderedFrame,
      timeElapsed: timingIsStale
        ? current.timeElapsed
        : (event.job.timeElapsed ?? (isRunning ? current.timeElapsed : undefined)),
      remainingSecs: timingIsStale
        ? current.remainingSecs
        : (event.job.remainingSecs ?? (isRunning ? current.remainingSecs : undefined)),
    }
    if (event.job.status !== 'running') {
      delete renderStarted.value[event.job.id]
    } else if (!(event.job.id in renderStarted.value)) {
      renderStarted.value[event.job.id] = (((event.job.timeElapsed ?? 0) > 0) || ((event.job.remainingSecs ?? 0) > 0))
    }
    if (shouldSort) sortJobs()
  }

  function isJobWarmingUp(id: string): boolean {
    return !renderStarted.value[id]
  }

  const pendingJobs = computed(() => jobs.value.filter((j) => j.status === 'pending'))
  const runningJobs = computed(() => jobs.value.filter((j) => j.status === 'running'))
  const doneJobs = computed(() => jobs.value.filter((j) => j.status === 'done'))
  const queueJobs = computed(() => jobs.value.filter((j) => j.status === 'pending' || j.status === 'running'))
  const errorJobs = computed(() => jobs.value.filter((j) => j.status === 'failed' || j.status === 'cancelled' || j.status === 'interrupted'))

  return {
    jobs,
    logs,
    loading,
    queuePaused,
    pausedJobId,
    pendingJobs,
    runningJobs,
    doneJobs,
    queueJobs,
    errorJobs,
    fetchJobs,
    fetchQueueState,
    startQueue,
    pauseQueue,
    applyQueueState,
    applyPreviewDimensions,
    submitJob,
    updateJobMetadata,
    updateJobTranscodeSettings,
    reorderQueueJobs,
    retryJob,
    retryJobFromStart,
    deleteJob,
    clearCompletedJobs,
    stopJob,
    applyProgress,
    applyLog,
    getJobLogs,
    loadJobLogs,
    applyJobUpdate,
    isJobWarmingUp,
  }
})
