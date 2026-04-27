import type { RenderedFramesStatus, RenderJob } from '~/types'

type RetrySubmittingMode = 'restart' | 'continue' | 'range-restart' | 'range-continue' | null
type RetryPreviewMode = 'continue' | 'restart' | null

function normalizeRetryFrameStatus(job: RenderJob, status: RenderedFramesStatus): RenderedFramesStatus {
  if (job.lastRenderedFrame == null) return status
  const lastFrame = Math.min(job.frameEnd, Math.max(job.frameStart, job.lastRenderedFrame))
  return {
    frameCount: status.frameCount,
    lastFrame,
    nextFrame: Math.min(job.frameEnd + 1, lastFrame + 1),
  }
}

export function useJobRetry() {
  const jobsStore = useJobsStore()
  const { inspectRenderedFrames } = useTauri()

  const showRetryConfirm = ref(false)
  const retryConfirmJob = ref<RenderJob | null>(null)
  const retryIsQuickMp4 = ref(false)
  const retryFrameStatus = ref<RenderedFramesStatus | null>(null)
  const retryActionError = ref('')
  const retrySubmittingMode = ref<RetrySubmittingMode>(null)
  const retryCustomStart = ref<number | null>(null)
  const retryCustomEnd = ref<number | null>(null)
  const retryFullRangePreviewMode = ref<RetryPreviewMode>(null)
  const retryCustomResumeFromExisting = ref(true)
  const retryCustomPreviewMode = ref<RetryPreviewMode>(null)
  const retryCustomFrameStatus = ref<RenderedFramesStatus | null>(null)
  const retryCustomInspectLoading = ref(false)
  const retryAutoTranscodeEnabled = ref(false)
  const retryTranscodeRangeMode = ref<'current' | 'original'>('current')
  const retryOriginalTranscodeFrameStart = ref<number | null>(null)
  const retryOriginalTranscodeFrameEnd = ref<number | null>(null)
  let retryCustomInspectToken = 0
  let retryCloseCleanupTimer: ReturnType<typeof setTimeout> | null = null

  function clearRetryConfirmState() {
    retryConfirmJob.value = null
    retryIsQuickMp4.value = false
    retryFrameStatus.value = null
    retrySubmittingMode.value = null
    retryCustomStart.value = null
    retryCustomEnd.value = null
    retryCustomFrameStatus.value = null
    retryCustomInspectLoading.value = false
    retryFullRangePreviewMode.value = null
    retryCustomPreviewMode.value = null
    retryAutoTranscodeEnabled.value = false
    retryTranscodeRangeMode.value = 'current'
    retryOriginalTranscodeFrameStart.value = null
    retryOriginalTranscodeFrameEnd.value = null
  }

  function cancelRetryCloseCleanup() {
    if (!retryCloseCleanupTimer) return
    clearTimeout(retryCloseCleanupTimer)
    retryCloseCleanupTimer = null
  }

  function beginCloseRetryConfirm() {
    showRetryConfirm.value = false
    cancelRetryCloseCleanup()
    retryCloseCleanupTimer = setTimeout(() => {
      clearRetryConfirmState()
      retryCloseCleanupTimer = null
    }, 220)
  }

  async function handleRetry(job: RenderJob | null | undefined) {
    if (!job) return
    cancelRetryCloseCleanup()
    retryActionError.value = ''
    retryIsQuickMp4.value = job.renderMode === 'quick_mp4'
    retryConfirmJob.value = job

    if (job.renderMode === 'quick_mp4') {
      retryFrameStatus.value = null
      retryCustomStart.value = job.frameStart
      retryCustomEnd.value = job.frameEnd
      retryCustomResumeFromExisting.value = false
      retryCustomFrameStatus.value = null
      retryAutoTranscodeEnabled.value = false
      retryOriginalTranscodeFrameStart.value = null
      retryOriginalTranscodeFrameEnd.value = null
      retryTranscodeRangeMode.value = 'current'
      showRetryConfirm.value = true
      return
    }

    const status = await inspectRenderedFrames(job.outputPath, job.outputFormat, job.frameStart, job.frameEnd)
      .catch(() => ({ frameCount: 0, lastFrame: null, nextFrame: job.frameStart }))
    retryFrameStatus.value = normalizeRetryFrameStatus(job, status)
    retryCustomStart.value = job.frameStart
    retryCustomEnd.value = job.frameEnd
    retryCustomResumeFromExisting.value = job.status !== 'done'
    retryCustomFrameStatus.value = status
    retryAutoTranscodeEnabled.value = job.autoTranscodeMp4
    retryOriginalTranscodeFrameStart.value = job.transcodeFrameStartOverride ?? job.originalFrameStart
    retryOriginalTranscodeFrameEnd.value = job.transcodeFrameEndOverride ?? job.originalFrameEnd
    retryTranscodeRangeMode.value =
      job.transcodeFrameStartOverride != null && job.transcodeFrameEndOverride != null ? 'original' : 'current'
    showRetryConfirm.value = true
    void refreshRetryCustomInspection()
  }

  function closeRetryConfirm() {
    if (retrySubmittingMode.value) return
    beginCloseRetryConfirm()
  }

  function clearRetryPreviewOnLeave(event: MouseEvent | FocusEvent, target: 'full' | 'custom') {
    const currentTarget = event.currentTarget
    const relatedTarget = event.relatedTarget
    if (currentTarget instanceof HTMLElement && relatedTarget instanceof Node) {
      const group = currentTarget.closest('.choice-card-toggle-group')
      if (group?.contains(relatedTarget)) return
    }

    if (target === 'full') {
      retryFullRangePreviewMode.value = null
      return
    }

    retryCustomPreviewMode.value = null
  }

  async function confirmRetryContinue() {
    if (retrySubmittingMode.value) return
    retryActionError.value = ''
    retrySubmittingMode.value = 'continue'
    try {
      const job = retryConfirmJob.value
      if (job) {
        await persistRetryTranscodeSettings(job, { start: job.frameStart, end: job.frameEnd })
        await jobsStore.retryJob(job)
      }
      beginCloseRetryConfirm()
    } catch (error) {
      retryActionError.value = error instanceof Error ? error.message : String(error)
    } finally {
      retrySubmittingMode.value = null
    }
  }

  async function confirmRetryFromStart() {
    if (retrySubmittingMode.value) return
    retryActionError.value = ''
    retrySubmittingMode.value = 'restart'
    try {
      const job = retryConfirmJob.value
      if (job) {
        await persistRetryTranscodeSettings(job, { start: job.frameStart, end: job.frameEnd })
        await jobsStore.retryJobFromStart(job)
      }
      beginCloseRetryConfirm()
    } catch (error) {
      retryActionError.value = error instanceof Error ? error.message : String(error)
    } finally {
      retrySubmittingMode.value = null
    }
  }

  const retryCustomActionResumeMode = computed(() => {
    if (retryCustomPreviewMode.value === 'continue') return true
    if (retryCustomPreviewMode.value === 'restart') return false
    return retryCustomResumeFromExisting.value
  })

  const retryFullRangeLabel = computed(() => {
    const job = retryConfirmJob.value
    if (!job) return '当前片段'
    return `${job.frameStart}–${job.frameEnd}`
  })

  const retryFullRangeSummary = computed(() => {
    const job = retryConfirmJob.value
    if (!job) return ''
    if (job.renderMode === 'quick_mp4') {
      return `会直接覆盖并重渲 ${job.frameStart}–${job.frameEnd} 的整段 MP4 输出。`
    }
    if (retryFullRangePreviewMode.value === 'restart') {
      return `整段覆盖会从第 ${job.frameStart} 帧重新渲染到 ${job.frameEnd} 帧。`
    }
    if (retryFullRangePreviewMode.value === 'continue') {
      if (job.status === 'done') {
        return '当前任务已完成，不能整段续跑。'
      }
      if (retryFrameStatus.value && retryFrameStatus.value.nextFrame <= job.frameEnd) {
        return `整段续跑会从第 ${retryFrameStatus.value.nextFrame} 帧开始。`
      }
      return '当前范围的帧已齐全，整段续跑会直接完成。'
    }
    if (job.status === 'done') {
      return '当前任务已完成；如需再次输出，请用整段覆盖或改为指定区间。'
    }
    if (retryFrameStatus.value && retryFrameStatus.value.nextFrame <= job.frameEnd) {
      return `整段续跑会从第 ${retryFrameStatus.value.nextFrame} 帧开始。`
    }
    return '当前范围的帧已齐全，整段续跑会直接完成。'
  })

  const retryCustomActionDescription = computed(() =>
    retryCustomActionResumeMode.value
      ? '区间续跑会自动查找断点，并从下一帧继续。'
      : '区间覆盖会从起始帧开始重新渲染。',
  )

  function buildRetryCustomRangeSummary(resumeFromExisting: boolean) {
    const job = retryConfirmJob.value
    const start = retryCustomStart.value
    const end = retryCustomEnd.value
    if (!job || start == null || end == null) return '输入起止帧后，可选择续跑或覆盖。'
    if (start > end) return '起始帧不能大于结束帧。'
    const status = retryCustomFrameStatus.value
    if (!status) return `将处理区间 ${start}–${end}。`
    if (status.frameCount === 0) return `区间 ${start}–${end} 还没有已渲染帧。`
    if (resumeFromExisting) {
      if (status.nextFrame <= end) {
        return `已找到 ${status.frameCount} 帧，最后一帧 ${status.lastFrame ?? '—'}，将从 ${status.nextFrame} 继续。`
      }
      return `区间 ${start}–${end} 已完整，继续后会直接完成。`
    }
    return `已找到 ${status.frameCount} 帧，覆盖后会从 ${start} 重新开始。`
  }

  const retryCustomRangeSummary = computed(() => {
    return buildRetryCustomRangeSummary(retryCustomActionResumeMode.value)
  })

  const retryOriginalTranscodeRangeLabel = computed(() => {
    const start = retryOriginalTranscodeFrameStart.value ?? retryConfirmJob.value?.originalFrameStart ?? 1
    const end = retryOriginalTranscodeFrameEnd.value ?? retryConfirmJob.value?.originalFrameEnd ?? start
    return `${start}–${end}`
  })

  const retrySavedTranscodeRangeTitle = computed(() => {
    const job = retryConfirmJob.value
    if (!job) return '原始片段'
    const hasSavedOverride = job.transcodeFrameStartOverride != null && job.transcodeFrameEndOverride != null
    return hasSavedOverride ? '已保存片段' : '原始片段'
  })

  const retryCurrentTargetRangeLabel = computed(() => {
    const job = retryConfirmJob.value
    if (!job) return '当前范围'
    const start = retryCustomStart.value ?? job.frameStart
    const end = retryCustomEnd.value ?? job.frameEnd
    return `${start}–${end}`
  })

  const retryTranscodeSummary = computed(() => {
    if (!retryConfirmJob.value) return ''
    if (!retryAutoTranscodeEnabled.value) {
      return '关闭后，本次补渲不会自动转码。'
    }
    if (retryTranscodeRangeMode.value === 'original') {
      if (retryOriginalTranscodeRangeLabel.value === retryCurrentTargetRangeLabel.value) {
        return `完成后自动转码 ${retryOriginalTranscodeRangeLabel.value}。`
      }
      return `完成后自动转码${retrySavedTranscodeRangeTitle.value} ${retryOriginalTranscodeRangeLabel.value}。`
    }
    return `完成后自动转码目标片段 ${retryCurrentTargetRangeLabel.value}。`
  })

  async function persistRetryTranscodeSettings(currentJob: RenderJob, nextRange: { start: number, end: number }) {
    if (currentJob.renderMode === 'quick_mp4') return
    const originalStart = retryOriginalTranscodeFrameStart.value ?? currentJob.originalFrameStart
    const originalEnd = retryOriginalTranscodeFrameEnd.value ?? currentJob.originalFrameEnd
    const useOriginalRange =
      retryAutoTranscodeEnabled.value
      && retryTranscodeRangeMode.value === 'original'
      && (originalStart !== nextRange.start || originalEnd !== nextRange.end)

    await jobsStore.updateJobTranscodeSettings({
      id: currentJob.id,
      auto_transcode_mp4: retryAutoTranscodeEnabled.value,
      transcode_name_override: currentJob.transcodeNameOverride,
      transcode_fps_override: currentJob.transcodeFpsOverride,
      transcode_output_path_override: currentJob.transcodeOutputPathOverride,
      transcode_crf_override: currentJob.transcodeCrfOverride,
      transcode_preset_override: currentJob.transcodePresetOverride,
      transcode_frame_start_override: useOriginalRange ? originalStart : null,
      transcode_frame_end_override: useOriginalRange ? originalEnd : null,
    })
  }

  async function refreshRetryCustomInspection() {
    const job = retryConfirmJob.value
    const start = retryCustomStart.value
    const end = retryCustomEnd.value
    if (!job || job.renderMode === 'quick_mp4' || !showRetryConfirm.value || start == null || end == null || start > end) {
      retryCustomFrameStatus.value = null
      retryCustomInspectLoading.value = false
      return
    }

    const token = ++retryCustomInspectToken
    retryCustomInspectLoading.value = true
    try {
      const status = await inspectRenderedFrames(job.outputPath, job.outputFormat, start, end)
        .catch(() => ({ frameCount: 0, lastFrame: null, nextFrame: start }))
      if (token !== retryCustomInspectToken) return
      retryCustomFrameStatus.value = status
    } finally {
      if (token === retryCustomInspectToken) {
        retryCustomInspectLoading.value = false
      }
    }
  }

  async function confirmRetryCustomRange(resumeFromExisting = retryCustomResumeFromExisting.value) {
    if (retrySubmittingMode.value) return
    retryActionError.value = ''
    const job = retryConfirmJob.value
    const start = retryCustomStart.value
    const end = retryCustomEnd.value
    retryCustomResumeFromExisting.value = resumeFromExisting
    if (!job || start == null || end == null) return
    if (start > end) {
      retryActionError.value = '起始帧不能大于结束帧。'
      return
    }
    if (start < job.originalFrameStart || end > job.originalFrameEnd) {
      retryActionError.value = `帧范围必须在原始片段 ${job.originalFrameStart}–${job.originalFrameEnd} 内。`
      return
    }

    retrySubmittingMode.value = retryCustomResumeFromExisting.value ? 'range-continue' : 'range-restart'
    try {
      await persistRetryTranscodeSettings(job, { start, end })
      if (retryCustomResumeFromExisting.value) {
        await jobsStore.retryJob(job, true, { start, end })
      } else {
        await jobsStore.retryJobFromStart(job, { start, end })
      }
      beginCloseRetryConfirm()
    } catch (error) {
      retryActionError.value = error instanceof Error ? error.message : String(error)
    } finally {
      retrySubmittingMode.value = null
    }
  }

  watch(
    () => [showRetryConfirm.value, retryConfirmJob.value, retryCustomStart.value, retryCustomEnd.value] as const,
    ([open]) => {
      if (!open || !retryConfirmJob.value) return
      void refreshRetryCustomInspection()
    },
  )

  return {
    showRetryConfirm,
    retryConfirmJob,
    retryIsQuickMp4,
    retryFrameStatus,
    retryActionError,
    retrySubmittingMode,
    retryCustomStart,
    retryCustomEnd,
    retryFullRangePreviewMode,
    retryCustomResumeFromExisting,
    retryCustomPreviewMode,
    retryCustomFrameStatus,
    retryCustomInspectLoading,
    retryAutoTranscodeEnabled,
    retryTranscodeRangeMode,
    retryOriginalTranscodeFrameStart,
    retryOriginalTranscodeFrameEnd,
    retryCustomActionResumeMode,
    retryFullRangeLabel,
    retryFullRangeSummary,
    retryCustomActionDescription,
    retryCustomRangeSummary,
    retryOriginalTranscodeRangeLabel,
    retrySavedTranscodeRangeTitle,
    retryCurrentTargetRangeLabel,
    retryTranscodeSummary,
    clearRetryConfirmState,
    cancelRetryCloseCleanup,
    beginCloseRetryConfirm,
    handleRetry,
    closeRetryConfirm,
    clearRetryPreviewOnLeave,
    confirmRetryContinue,
    confirmRetryFromStart,
    buildRetryCustomRangeSummary,
    persistRetryTranscodeSettings,
    refreshRetryCustomInspection,
    confirmRetryCustomRange,
  }
}
