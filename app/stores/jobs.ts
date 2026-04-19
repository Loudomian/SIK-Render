import { defineStore } from 'pinia'
import type { AddJobPayload, RenderJob, RenderProgressEvent } from '~/types'

export const useJobsStore = defineStore('jobs', () => {
  const jobs = ref<RenderJob[]>([])
  const loading = ref(false)
  const { listJobs, addJob, removeJob, cancelJob } = useTauri()

  async function fetchJobs() {
    loading.value = true
    jobs.value = await listJobs()
    loading.value = false
  }

  async function submitJob(payload: AddJobPayload) {
    const job = await addJob(payload)
    jobs.value.push(job)
    return job
  }

  async function deleteJob(id: string) {
    await removeJob(id)
    jobs.value = jobs.value.filter((j) => j.id !== id)
  }

  async function stopJob(id: string) {
    await cancelJob(id)
    const job = jobs.value.find((j) => j.id === id)
    if (job) job.status = 'cancelled'
  }

  function applyProgress(event: RenderProgressEvent) {
    const job = jobs.value.find((j) => j.id === event.jobId)
    if (job) {
      job.status = 'running'
      // Store latest frame on job for display; no extra property needed
      ;(job as any)._frame = event.frame
      ;(job as any)._totalFrames = event.totalFrames
    }
  }

  const pendingJobs = computed(() => jobs.value.filter((j) => j.status === 'pending'))
  const runningJobs = computed(() => jobs.value.filter((j) => j.status === 'running'))
  const doneJobs = computed(() => jobs.value.filter((j) => j.status === 'done'))

  return {
    jobs,
    loading,
    pendingJobs,
    runningJobs,
    doneJobs,
    fetchJobs,
    submitJob,
    deleteJob,
    stopJob,
    applyProgress,
  }
})
