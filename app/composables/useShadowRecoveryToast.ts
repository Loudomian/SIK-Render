import type { RenderJob, RenderLogEvent } from '~/types'

const SHADOW_BUFFER_PATTERN = /Shadow (?:buffer|pool) full|阴影缓冲满了/i
// Keep in sync with next_shadow_resolution_scale() in src-tauri/src/commands/jobs.rs.
const SCALE_STEPS = [0.75, 0.5, 0.3]
const activeToastIds = new Set<string>()

function nextScale(current?: number | null) {
  // Blender default is 1.0; step 1.0 → 0.75 → 0.5 → 0.3 → none.
  if (current == null) return SCALE_STEPS[0]
  return SCALE_STEPS.find((s) => s < current) ?? null
}

export function isShadowBufferWarningLine(line: string) {
  return SHADOW_BUFFER_PATTERN.test(line)
}

export function useShadowRecoveryToast() {
  const toast = useToast()
  const jobsStore = useJobsStore()
  const { applyShadowResolutionRecovery } = useTauri()
  const { t } = useI18n()

  function showForJob(job: RenderJob) {
    if (job.renderMode === 'quick_mp4') return

    const toastId = `shadow-buffer-full:${job.id}`
    if (activeToastIds.has(toastId)) return

    const targetScale = nextScale(job.shadowResolutionScaleOverride)
    const targetPercent = targetScale == null ? null : Math.round(targetScale * 100)
    const description = targetScale == null
      ? t('shadowRecovery.atLimit')
      : t('shadowRecovery.detected', { percent: targetPercent })

    activeToastIds.add(toastId)
    toast.add({
      id: toastId,
      title: t('shadowRecovery.title'),
      description,
      icon: 'i-lucide-triangle-alert',
      color: 'warning',
      duration: 0,
      progress: false,
      close: true,
      actions: targetScale == null
        ? []
        : [{
            label: t('shadowRecovery.action', { percent: targetPercent }),
            icon: 'i-lucide-rotate-ccw',
            variant: 'solid',
            onClick: async () => {
              toast.update(toastId, {
                description: t('shadowRecovery.applying', { percent: targetPercent }),
                actions: [],
              })

              try {
                const response = await applyShadowResolutionRecovery(job.id)
                job.shadowResolutionScaleOverride = response.scale
                job.frameStart = response.frameStart
                toast.update(toastId, {
                  title: t('shadowRecovery.appliedTitle'),
                  description: t('shadowRecovery.appliedDescription', {
                    percent: Math.round(response.scale * 100),
                    frame: response.frameStart,
                  }),
                  color: 'success',
                  icon: 'i-lucide-check',
                  duration: 6000,
                  progress: true,
                })
                activeToastIds.delete(toastId)
              } catch (error) {
                toast.update(toastId, {
                  title: t('shadowRecovery.failedTitle'),
                  description: error instanceof Error ? error.message : String(error),
                  color: 'error',
                  icon: 'i-lucide-circle-alert',
                  actions: [],
                })
              }
            },
          }],
      'onUpdate:open': (open: boolean) => {
        if (!open) activeToastIds.delete(toastId)
      },
    })
  }

  function handleLogEvent(event: RenderLogEvent) {
    if (!isShadowBufferWarningLine(event.line)) return
    const job = jobsStore.jobs.find(item => item.id === event.jobId)
    if (job) showForJob(job)
  }

  function handleExistingLogs(job: RenderJob, lines: string[]) {
    if (lines.some(isShadowBufferWarningLine)) showForJob(job)
  }

  return {
    handleLogEvent,
    handleExistingLogs,
    showForJob,
  }
}
