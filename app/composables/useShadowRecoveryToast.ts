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

  function showForJob(job: RenderJob) {
    if (job.renderMode === 'quick_mp4') return

    const toastId = `shadow-buffer-full:${job.id}`
    if (activeToastIds.has(toastId)) return

    const targetScale = nextScale(job.shadowResolutionScaleOverride)
    const description = targetScale == null
      ? '阴影分辨率已经降到自动档位下限。请手动减少灯光阴影分辨率或拆分场景。'
      : `检测到阴影缓冲满了。可将本任务阴影分辨率降到 ${Math.round(targetScale * 100)}%，并从当前镜头起点重渲。`

    activeToastIds.add(toastId)
    toast.add({
      id: toastId,
      title: '阴影缓冲已满',
      description,
      icon: 'i-lucide-triangle-alert',
      color: 'warning',
      duration: 0,
      progress: false,
      close: true,
      actions: targetScale == null
        ? []
        : [{
            label: `降到 ${Math.round(targetScale * 100)}% 并重渲镜头`,
            icon: 'i-lucide-rotate-ccw',
            variant: 'solid',
            onClick: async () => {
              toast.update(toastId, {
                description: `正在应用 ${Math.round(targetScale * 100)}% 阴影分辨率，并准备从当前镜头起点重渲。`,
                actions: [],
              })

              try {
                const response = await applyShadowResolutionRecovery(job.id)
                job.shadowResolutionScaleOverride = response.scale
                job.frameStart = response.frameStart
                toast.update(toastId, {
                  title: '已应用阴影分辨率',
                  description: `已降到 ${Math.round(response.scale * 100)}%，将从第 ${response.frameStart} 帧重渲当前镜头。`,
                  color: 'success',
                  icon: 'i-lucide-check',
                  duration: 6000,
                  progress: true,
                })
                activeToastIds.delete(toastId)
              } catch (error) {
                toast.update(toastId, {
                  title: '降低阴影分辨率失败',
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
