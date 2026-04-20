import { defineStore } from 'pinia'
import type { AddJobPayload, JobUpdatedEvent, RenderJob, RenderLogEvent, RenderProgressEvent } from '~/types'

export const useJobsStore = defineStore('jobs', () => {
  const jobs = ref<RenderJob[]>([])
  const logs = ref<Record<string, string[]>>({})
  const renderStarted = ref<Record<string, boolean>>({})
  const loading = ref(false)
  const { listJobs, addJob, removeJob, cancelJob, resetJob, getJobLatestLogs: fetchJobLogs } = useTauri()

  function jobOrderWeight(status: RenderJob['status']) {
    switch (status) {
      case 'running':
        return 0
      case 'pending':
        return 1
      default:
        return 2
    }
  }

  function jobSortTimestamp(job: RenderJob) {
    if (job.status === 'running') return job.startedAt ?? job.createdAt
    if (job.status === 'pending') return job.createdAt
    return job.finishedAt ?? job.startedAt ?? job.createdAt
  }

  function sortJobs() {
    jobs.value = [...jobs.value].sort((a, b) =>
      jobOrderWeight(a.status) - jobOrderWeight(b.status)
      || jobSortTimestamp(b) - jobSortTimestamp(a)
      || a.priority - b.priority
      || b.createdAt - a.createdAt,
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

  async function submitJob(payload: AddJobPayload) {
    const job = await addJob(payload)
    // job-updated may have arrived during the await and set status to 'running';
    // only push if not already tracked to avoid overwriting the newer status.
    if (!jobs.value.find((j) => j.id === job.id)) {
      jobs.value.push(job)
    }
    sortJobs()
    return job
  }

  async function deleteJob(id: string) {
    await removeJob(id)
    jobs.value = jobs.value.filter((j) => j.id !== id)
    delete logs.value[id]
    delete renderStarted.value[id]
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
    sortJobs()
  }

  function applyLog(event: RenderLogEvent) {
    if (!logs.value[event.jobId]) logs.value[event.jobId] = []
    logs.value[event.jobId].push(event.line)
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
      logs.value[id] = lines
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
    jobs.value[index] = {
      ...current,
      ...event.job,
      currentFrame: event.job.currentFrame ?? (event.job.status === 'running' ? current.currentFrame : undefined),
      totalFrames: event.job.totalFrames ?? (event.job.status === 'running' ? current.totalFrames : undefined),
      lastRenderedFrame: event.job.lastRenderedFrame ?? (event.job.status === 'running' ? current.lastRenderedFrame : undefined),
      timeElapsed: event.job.timeElapsed ?? (event.job.status === 'running' ? current.timeElapsed : undefined),
      remainingSecs: event.job.remainingSecs ?? (event.job.status === 'running' ? current.remainingSecs : undefined),
    }
    if (event.job.status !== 'running') {
      delete renderStarted.value[event.job.id]
    } else if (!(event.job.id in renderStarted.value)) {
      renderStarted.value[event.job.id] = (((event.job.timeElapsed ?? 0) > 0) || ((event.job.remainingSecs ?? 0) > 0))
    }
    sortJobs()
  }

  function isJobWarmingUp(id: string): boolean {
    return !renderStarted.value[id]
  }

  const pendingJobs = computed(() => jobs.value.filter((j) => j.status === 'pending'))
  const runningJobs = computed(() => jobs.value.filter((j) => j.status === 'running'))
  const doneJobs = computed(() => jobs.value.filter((j) => j.status === 'done'))

  return {
    jobs,
    logs,
    loading,
    pendingJobs,
    runningJobs,
    doneJobs,
    fetchJobs,
    submitJob,
    retryJob,
    retryJobFromStart,
    deleteJob,
    stopJob,
    applyProgress,
    applyLog,
    getJobLogs,
    loadJobLogs,
    applyJobUpdate,
    isJobWarmingUp,
  }
})
