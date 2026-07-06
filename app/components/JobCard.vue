<template>
  <UCard
    class="job-card job-card-openable"
    :class="`status-${job.status}`"
    variant="subtle"
    :ui="{ body: 'job-card-body' }"
    tabindex="0"
    @dblclick="openDetails"
    @keydown.enter="openDetails"
  >
    <div class="job-card-layout">
      <div ref="cardInfoEl" class="job-card-info">
        <div class="job-card-heading">
          <div class="job-title-stack">
            <div class="job-head-badges">
              <UBadge
                :label="statusLabel(job.status)"
                :color="statusColor"
                variant="subtle"
              />
              <UBadge
                v-if="job.renderMode === 'quick_mp4'"
                :label="t('jobCard.quickMp4')"
                color="neutral"
                variant="subtle"
              />
              <UBadge v-if="orderBadgeLabel" :label="orderBadgeLabel" color="neutral" variant="subtle" />
              <UBadge
                v-if="job.crashCount > 0"
                :label="t('jobCard.crashCount', { count: job.crashCount })"
                color="warning"
                variant="subtle"
              />
            </div>
            <span class="job-name"><span class="job-number">#{{ job.jobNumber }}</span> {{ job.name }}</span>
            <p v-if="job.note" class="job-note">{{ job.note }}</p>
          </div>
        </div>

        <div class="job-footer">
          <div class="job-meta">
            <span class="job-meta-item">
              <span class="job-meta-label">{{ t('jobCard.frameRange') }}</span>
              <strong>{{ displayFrameRange }}</strong>
            </span>
            <template v-if="showCurrentExecutionRange">
              <span class="job-meta-divider" aria-hidden="true" />
              <span class="job-meta-item">
                <span class="job-meta-label">{{ t('jobCard.currentExecution') }}</span>
                <strong>{{ currentExecutionRange }}</strong>
              </span>
            </template>
            <span class="job-meta-divider" aria-hidden="true" />
            <span class="job-meta-item">
              <span class="job-meta-label">{{ t('jobCard.renderTime') }}</span>
              <strong>{{ renderTime }}</strong>
            </span>
            <span class="job-meta-divider" aria-hidden="true" />
            <span class="job-meta-item">
              <span class="job-meta-label">{{ finishedAtLabel }}</span>
              <strong>{{ completedAt }}</strong>
            </span>
          </div>

          <div class="job-actions" data-no-drag @dblclick.stop>
            <UTooltip v-if="job.status === 'running' || job.status === 'pending'" :text="t('jobCard.actions.cancelTooltip')" arrow :content="{ side: 'bottom', sideOffset: 8 }">
              <UButton icon="i-lucide-x" :label="t('common.cancel')" color="warning" variant="outline" size="sm" @click="$emit('cancel')" />
            </UTooltip>
            <UTooltip
              v-if="job.status === 'done' || job.status === 'failed' || job.status === 'cancelled' || job.status === 'interrupted'"
              :text="retryButtonTooltip"
              arrow
              :content="{ side: 'bottom', sideOffset: 8 }"
            >
              <UButton
                icon="i-lucide-rotate-ccw"
                :label="retryButtonLabel"
                :color="retryButtonColor"
                variant="outline"
                size="sm"
                @click="$emit('retry')"
              />
            </UTooltip>
            <UTooltip
              v-if="job.status === 'done' || job.status === 'failed' || job.status === 'cancelled' || job.status === 'interrupted'"
              :text="t('jobCard.actions.deleteTooltip')"
              arrow
              :content="{ side: 'bottom', sideOffset: 8 }"
            >
              <UButton
                icon="i-lucide-trash-2"
                :label="t('common.delete')"
                color="error"
                variant="outline"
                size="sm"
                @click="$emit('remove')"
              />
            </UTooltip>
            <UTooltip :text="t('jobCard.actions.openOutputTooltip')" arrow :content="{ side: 'bottom', sideOffset: 8 }">
              <UButton
                icon="i-lucide-folder-open"
                :label="t('common.outputDirectory')"
                color="neutral"
                variant="outline"
                size="sm"
                @click="openOutput"
              />
            </UTooltip>
            <UTooltip :text="t('jobCard.actions.detailsTooltip')" arrow :content="{ side: 'bottom', sideOffset: 8 }">
              <UButton :to="`/jobs/${job.id}`" icon="i-lucide-external-link" :label="t('common.details')" color="neutral" variant="outline" size="sm" />
            </UTooltip>
          </div>
        </div>
      </div>

      <div
        class="job-preview"
        :class="{
          'job-preview-empty': !displayPreviewUrl && !displayPreviewVideoUrl && !previewLoading,
          'job-preview-clickable': !!displayPreviewUrl && !displayPreviewVideoUrl,
          'job-preview-loading': previewLoading && !displayPreviewUrl && !displayPreviewVideoUrl,
        }"
        :data-no-drag="displayPreviewUrl || displayPreviewVideoUrl ? '' : null"
        :style="previewStyle"
        @click="displayPreviewUrl && !displayPreviewVideoUrl && (lightboxOpen = true)"
      >
        <video
          v-if="displayPreviewVideoUrl"
          ref="previewVideoEl"
          :src="displayPreviewVideoUrl"
          class="job-preview-image job-preview-video"
          :class="{ 'job-preview-image-visible': previewVisible }"
          muted
          playsinline
          preload="metadata"
          @loadedmetadata="syncQuickMp4VideoFrame"
        />
        <img
          v-else-if="displayPreviewUrl"
          :src="displayPreviewUrl"
          class="job-preview-image"
          :class="{ 'job-preview-image-visible': previewVisible }"
          alt="last rendered frame preview"
        />
        <div
          class="job-preview-placeholder"
          :class="{ 'job-preview-placeholder-hidden': !!displayPreviewUrl && previewVisible }"
        >
          <UIcon name="i-lucide-image" class="job-preview-icon" />
          <span>{{ previewText }}</span>
        </div>
        <UBadge
          v-if="previewFrame && displayPreviewUrl"
          :label="t('jobCard.preview.frameBadge', { frame: previewFrame })"
          color="neutral"
          variant="subtle"
          class="job-preview-badge"
        />
      </div>
    </div>

    <RenderProgress
      v-if="job.status === 'running'"
      class="job-card-progress"
      :frame="job.currentFrame ?? 0"
      :total-frames="job.totalFrames ?? (job.frameEnd - job.frameStart + 1)"
      :warming-up="jobsStore.isJobWarmingUp(job.id)"
      :time-elapsed="job.timeElapsed ?? undefined"
      :remaining-secs="job.remainingSecs"
    />

    <UModal v-model:open="lightboxOpen" :close="false" :ui="{ content: 'preview-lightbox' }">
      <template #body>
        <div @click="lightboxOpen = false">
          <div @click.stop>
            <img v-if="previewUrl" :src="previewUrl" class="preview-lightbox-img" alt="frame preview" />
          </div>
        </div>
      </template>
    </UModal>
  </UCard>
</template>

<script setup lang="ts">
import { convertFileSrc } from '@tauri-apps/api/core'
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import type { RenderJob } from '~/types'
import { JOB_STATUS_COLOR, useJobStatusLabel } from '~/composables/useJobStatus'
import { RENDER_QUEUE_ORDER_HIDDEN_STATUSES, resolveQueueOrder, useQueueOrderLabel } from '~/composables/useQueueOrder'
import { formatDuration, useDateFormatters } from '~/utils/date-format'
import { resolveOutputDirectory } from '~/utils/output-path'
import { captureVideoPoster } from '~/utils/video-preview'

const props = defineProps<{ job: RenderJob }>()
defineEmits(['cancel', 'remove', 'retry'])
const router = useRouter()
const jobsStore = useJobsStore()
const { openPath, getLastRenderedFrame, updateJobPreviewDimensions, pathExists } = useTauri()
const { t } = useI18n()
const { formatShortTimestamp } = useDateFormatters()

const statusLabel = useJobStatusLabel()
const queueOrderLabel = useQueueOrderLabel()
const statusColor = computed(() => JOB_STATUS_COLOR[props.job.status] ?? 'neutral')

const retryButtonLabel = computed(() => {
  if (props.job.status === 'interrupted') return t('jobCard.actions.continue')
  if (props.job.status === 'failed') return t('jobCard.actions.retry')
  return t('jobCard.actions.rerender')
})

const retryButtonColor = computed(() =>
  props.job.status === 'interrupted' ? 'warning' as const : 'neutral' as const,
)

const retryButtonTooltip = computed(() => {
  if (props.job.status === 'interrupted') return t('jobCard.actions.continueTooltip')
  if (props.job.status === 'failed') return t('jobCard.actions.retryTooltip')
  return t('jobCard.actions.rerenderTooltip')
})
const queueOrder = computed(() => {
  return resolveQueueOrder(jobsStore.jobs, props.job, RENDER_QUEUE_ORDER_HIDDEN_STATUSES)
})
const orderBadgeLabel = computed(() => queueOrderLabel(queueOrder.value))

const renderTime = computed(() => {
  const job = props.job
  if (!job.startedAt) return t('jobCard.notStarted')
  return formatDuration((job.finishedAt ?? Date.now()) - job.startedAt)
})
const displayFrameRange = computed(() => `${props.job.originalFrameStart}–${props.job.originalFrameEnd}`)
const currentExecutionRange = computed(() => `${props.job.frameStart}–${props.job.frameEnd}`)
const showCurrentExecutionRange = computed(() =>
  props.job.originalFrameStart !== props.job.frameStart || props.job.originalFrameEnd !== props.job.frameEnd,
)
const finishedAtLabel = computed(() => {
  switch (props.job.status) {
    case 'done':
      return t('jobCard.finishedAt.done')
    case 'cancelled':
      return t('jobCard.finishedAt.cancelled')
    case 'interrupted':
      return t('jobCard.finishedAt.interrupted')
    case 'failed':
      return t('jobCard.finishedAt.failed')
    case 'running':
      return t('jobCard.finishedAt.running')
    default:
      return t('jobCard.finishedAt.default')
  }
})
const completedAt = computed(() => {
  if (!props.job.finishedAt) {
    return props.job.status === 'running' ? t('jobCard.running') : t('jobCard.unfinished')
  }
  return formatShortTimestamp(props.job.finishedAt)
})
const previewText = computed(() => {
  if (props.job.renderMode === 'quick_mp4') {
    return props.job.status === 'done'
      ? t('jobCard.preview.generatedFinalFrame')
      : t('jobCard.preview.waitingFinalFrame')
  }
  if (props.job.outputFormat === 'OPEN_EXR') return t('jobCard.preview.exrUnsupported')
  return props.job.status === 'running' ? t('jobCard.preview.waitingFirstFrame') : t('jobCard.preview.emptyRenderedFrame')
})

const previewUrl = ref<string | null>(null)
const displayPreviewUrl = ref<string | null>(null)
const displayPreviewVideoUrl = ref<string | null>(null)
const previewFrame = ref<number | null>(null)
const previewAspect = ref(aspectFromDimensions(props.job.previewWidth, props.job.previewHeight))
const previewVisible = ref(false)
const previewLoading = ref(false)
const lightboxOpen = ref(false)
const previewVideoEl = ref<HTMLVideoElement | null>(null)
const { cardInfoEl, cardInfoHeight, syncHeightAfterTick } = useCardInfoHeight()
let previewLoadToken = 0
let previewRevealTimer = 0
let previewPersistKey: string | null = null
const previewStyle = computed(() =>
  ({
    ...(previewAspect.value ? { '--preview-aspect': previewAspect.value } : {}),
    ...(cardInfoHeight.value ? { '--preview-height': `${cardInfoHeight.value}px` } : {}),
  }),
)
const previewFrameEnd = computed(() => {
  const job = props.job
  if (job.lastRenderedFrame != null) {
    return Math.min(job.frameEnd, Math.max(job.frameStart, job.lastRenderedFrame))
  }
  if (job.status !== 'running') return job.frameEnd
  const progressed = Math.max(job.currentFrame ?? 0, 0)
  const capped = job.frameStart + progressed - 1
  return Math.min(job.frameEnd, Math.max(job.frameStart, capped))
})

function openDetails() {
  router.push(`/jobs/${props.job.id}`)
}

function openOutput() {
  openPath(resolveOutputDirectory(props.job.outputPath))
}

function aspectFromDimensions(width: number | null | undefined, height: number | null | undefined) {
  if (!width || !height || width <= 0 || height <= 0) return null
  return `${width} / ${height}`
}

function applyStoredPreviewAspect() {
  previewAspect.value = aspectFromDimensions(props.job.previewWidth, props.job.previewHeight)
}

function resetPreview() {
  previewUrl.value = null
  displayPreviewUrl.value = null
  displayPreviewVideoUrl.value = null
  previewFrame.value = null
  applyStoredPreviewAspect()
  previewVisible.value = false
  previewLoading.value = false
}

function syncQuickMp4VideoFrame() {
  const video = previewVideoEl.value
  if (!video) return

  const targetTime = Number.isFinite(video.duration) && video.duration > 0
    ? Math.max(video.duration - 0.05, 0)
    : 0

  const finalize = () => {
    previewAspect.value = `${video.videoWidth || 16} / ${video.videoHeight || 9}`
    previewVisible.value = true
    previewLoading.value = false
    video.pause()
  }

  const requestFrame = (video as HTMLVideoElement & {
    requestVideoFrameCallback?: (callback: () => void) => number
  }).requestVideoFrameCallback

  const queueFinalize = () => {
    if (requestFrame) {
      requestFrame.call(video, () => finalize())
      return
    }

    window.setTimeout(finalize, 0)
  }

  if (Math.abs(video.currentTime - targetTime) < 0.001) {
    queueFinalize()
    return
  }

  const handleSeeked = () => {
    video.removeEventListener('seeked', handleSeeked)
    queueFinalize()
  }

  video.addEventListener('seeked', handleSeeked, { once: true })
  video.currentTime = targetTime
}

function preloadPreview(url: string) {
  return new Promise<{ width: number, height: number }>((resolve, reject) => {
    const img = new Image()
    img.onload = () => resolve({ width: img.naturalWidth, height: img.naturalHeight })
    img.onerror = () => reject(new Error('preview load failed'))
    img.src = url
  })
}

async function revealPreview(url: string, frame: number | null, width: number, height: number) {
  const hadPreview = !!displayPreviewUrl.value

  previewUrl.value = url
  displayPreviewUrl.value = url
  displayPreviewVideoUrl.value = null
  previewFrame.value = frame
  previewAspect.value = `${width} / ${height}`
  void syncStoredPreviewDimensions(width, height)

  if (hadPreview) {
    previewVisible.value = true
    previewLoading.value = false
    return
  }

  previewVisible.value = false
  previewLoading.value = false
  await nextTick()

  window.clearTimeout(previewRevealTimer)
  previewRevealTimer = window.setTimeout(() => {
    previewVisible.value = true
  }, 24)
}

async function syncStoredPreviewDimensions(width: number, height: number) {
  const nextKey = `${width}x${height}`
  const currentKey = props.job.previewWidth && props.job.previewHeight
    ? `${props.job.previewWidth}x${props.job.previewHeight}`
    : null

  if (currentKey === nextKey || previewPersistKey === nextKey) return

  previewPersistKey = nextKey
  try {
    await updateJobPreviewDimensions(props.job.id, width, height)
  } catch {
    // Ignore persistence failures; preview display should still work.
  } finally {
    if (previewPersistKey === nextKey) {
      previewPersistKey = null
    }
  }
}

async function refreshPreview() {
  const token = ++previewLoadToken
  const job = props.job
  if (job.renderMode === 'quick_mp4') {
    if (job.status !== 'done' || !job.outputPath) {
      resetPreview()
      return
    }

    try {
      previewLoading.value = !displayPreviewUrl.value

      if (!(await pathExists(job.outputPath))) {
        if (token !== previewLoadToken) return
        resetPreview()
        return
      }

      if (job.previewImagePath && await pathExists(job.previewImagePath).catch(() => false)) {
        const previewImageUrl = `${convertFileSrc(job.previewImagePath)}?t=${Date.now()}`
        const { width, height } = await preloadPreview(previewImageUrl)

        if (token !== previewLoadToken) return
        await revealPreview(previewImageUrl, null, width, height)
        return
      }

      const videoUrl = `${convertFileSrc(job.outputPath)}?t=${Date.now()}`
      const poster = await captureVideoPoster(videoUrl)

      if (token !== previewLoadToken) return
      if (!poster) {
        previewUrl.value = null
        displayPreviewUrl.value = null
        displayPreviewVideoUrl.value = videoUrl
        previewFrame.value = null
        applyStoredPreviewAspect()
        previewVisible.value = false
        previewLoading.value = false
        return
      }

      await revealPreview(poster.dataUrl, null, poster.width, poster.height)
      return
    } catch {
      if (token !== previewLoadToken) return
      resetPreview()
      return
    }
  }

  if (job.outputFormat === 'OPEN_EXR' || job.outputFormat === 'EXR') {
    resetPreview()
    return
  }

  try {
    previewLoading.value = !displayPreviewUrl.value
    const path = await getLastRenderedFrame(
      job.outputPath,
      job.outputFormat,
      job.frameStart,
      previewFrameEnd.value,
    )

    if (!path) {
      if (token !== previewLoadToken) return
      resetPreview()
      return
    }

    const url = `${convertFileSrc(path)}?t=${Date.now()}`
    const match = path.match(/(\d+)\.[^.]+$/)
    const frameToken = match?.[1]
    const frame = frameToken ? parseInt(frameToken) : null
    const { width, height } = await preloadPreview(url)

    if (token !== previewLoadToken) return
    await revealPreview(url, frame, width, height)
  } catch {
    if (token !== previewLoadToken) return
    resetPreview()
  }
}

watch(
  () => [props.job.id, props.job.status, props.job.currentFrame, props.job.outputPath, props.job.outputFormat, props.job.previewImagePath] as const,
  () => { void refreshPreview() },
  { immediate: true },
)

watch(
  () => [props.job.previewWidth, props.job.previewHeight] as const,
  () => {
    if (!displayPreviewUrl.value || !previewVisible.value) {
      applyStoredPreviewAspect()
    }
  },
  { immediate: true },
)

watch(
  () => [
    props.job.name,
    props.job.note,
    props.job.status,
    props.job.startedAt,
    props.job.finishedAt,
    props.job.currentFrame,
    props.job.totalFrames,
  ] as const,
  () => { void syncHeightAfterTick() },
  { flush: 'post' },
)

onUnmounted(() => {
  window.clearTimeout(previewRevealTimer)
})
</script>
