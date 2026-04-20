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
                :label="STATUS_LABEL[job.status] ?? job.status"
                :color="statusColor"
                variant="subtle"
              />
              <UBadge :label="`优先级 ${job.priority}`" color="neutral" variant="subtle" />
            </div>
            <span class="job-name"><span class="job-number">#{{ job.jobNumber }}</span> {{ job.name }}</span>
          </div>
        </div>

        <div class="job-footer">
          <div class="job-meta">
            <span class="job-meta-item">
              <span class="job-meta-label">帧范围</span>
              <strong>{{ job.frameStart }}–{{ job.frameEnd }}</strong>
            </span>
            <span class="job-meta-divider" aria-hidden="true" />
            <span class="job-meta-item">
              <span class="job-meta-label">渲染时间</span>
              <strong>{{ renderTime }}</strong>
            </span>
          </div>

          <div class="job-actions" @dblclick.stop>
            <UButton v-if="job.status === 'running' || job.status === 'pending'" icon="i-lucide-x" label="取消" color="warning" variant="outline" size="sm" @click="$emit('cancel')" />
            <UButton
              v-if="job.status === 'failed' || job.status === 'cancelled' || job.status === 'interrupted'"
              icon="i-lucide-rotate-ccw"
              :label="job.status === 'cancelled' || job.status === 'interrupted' ? '继续' : '重试'"
              :color="job.status === 'cancelled' || job.status === 'interrupted' ? 'warning' : 'neutral'"
              variant="outline"
              size="sm"
              @click="$emit('retry')"
            />
            <UButton
              v-if="job.status === 'done' || job.status === 'failed' || job.status === 'cancelled' || job.status === 'interrupted'"
              icon="i-lucide-trash-2"
              label="删除"
              color="error"
              variant="outline"
              size="sm"
              @click="$emit('remove')"
            />
            <UButton
              icon="i-lucide-folder-open"
              label="输出目录"
              color="neutral"
              variant="outline"
              size="sm"
              @click="openOutput"
            />
            <UButton :to="`/jobs/${job.id}`" icon="i-lucide-external-link" label="详情" color="neutral" variant="outline" size="sm" />
          </div>
        </div>
      </div>

      <div
        class="job-preview"
        :class="{
          'job-preview-empty': !displayPreviewUrl && !previewLoading,
          'job-preview-clickable': !!displayPreviewUrl,
          'job-preview-loading': previewLoading && !displayPreviewUrl,
        }"
        :style="previewStyle"
        @click="displayPreviewUrl && (lightboxOpen = true)"
      >
        <img
          v-if="displayPreviewUrl"
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
          :label="`第 ${previewFrame} 帧`"
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
      :time-elapsed="job.timeElapsed"
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

const props = defineProps<{ job: RenderJob }>()
defineEmits(['cancel', 'remove', 'retry'])
const router = useRouter()
const jobsStore = useJobsStore()
const { openPath, getLastRenderedFrame, updateJobPreviewDimensions } = useTauri()

const STATUS_LABEL: Record<string, string> = {
  pending: '等待中',
  running: '渲染中',
  done: '已完成',
  failed: '失败',
  cancelled: '已取消',
  interrupted: '已中断',
}

const STATUS_COLOR: Record<string, 'neutral' | 'info' | 'success' | 'error' | 'warning'> = {
  pending: 'neutral',
  running: 'info',
  done: 'success',
  failed: 'error',
  cancelled: 'warning',
  interrupted: 'warning',
}

const statusColor = computed(() => STATUS_COLOR[props.job.status] ?? 'neutral')

const renderTime = computed(() => {
  const job = props.job
  if (!job.startedAt) return '未开始'
  return formatDuration((job.finishedAt ?? Date.now()) - job.startedAt)
})
const previewText = computed(() => {
  if (props.job.outputFormat === 'OPEN_EXR') return 'EXR 不支持预览'
  return props.job.status === 'running' ? '等待首帧输出' : '暂无已渲染帧'
})

const previewUrl = ref<string | null>(null)
const displayPreviewUrl = ref<string | null>(null)
const previewFrame = ref<number | null>(null)
const previewAspect = ref(aspectFromDimensions(props.job.previewWidth, props.job.previewHeight))
const previewVisible = ref(false)
const previewLoading = ref(false)
const lightboxOpen = ref(false)
const cardInfoEl = ref<HTMLElement | null>(null)
const cardInfoHeight = ref<number | null>(null)
let cardInfoResizeObserver: ResizeObserver | null = null
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

function formatDuration(ms: number) {
  const s = Math.round(ms / 1000)
  if (s < 60) return `${s}s`
  const m = Math.floor(s / 60)
  if (m < 60) return `${m}m ${s % 60}s`
  return `${Math.floor(m / 60)}h ${m % 60}m`
}

function openDetails() {
  router.push(`/jobs/${props.job.id}`)
}

function openOutput() {
  openPath(props.job.outputPath)
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
  previewFrame.value = null
  applyStoredPreviewAspect()
  previewVisible.value = false
  previewLoading.value = false
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
  if (job.outputFormat === 'OPEN_EXR') {
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
    const frame = match ? parseInt(match[1]) : null
    const { width, height } = await preloadPreview(url)

    if (token !== previewLoadToken) return
    await revealPreview(url, frame, width, height)
  } catch {
    if (token !== previewLoadToken) return
    resetPreview()
  }
}

watch(
  () => [props.job.id, props.job.status, props.job.currentFrame, props.job.outputPath, props.job.outputFormat] as const,
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

onMounted(() => {
  if (!cardInfoEl.value) return
  cardInfoResizeObserver = new ResizeObserver((entries) => {
    const entry = entries[0]
    if (!entry) return
    cardInfoHeight.value = Math.round(entry.contentRect.height)
  })
  cardInfoResizeObserver.observe(cardInfoEl.value)
})

onUnmounted(() => {
  cardInfoResizeObserver?.disconnect()
  window.clearTimeout(previewRevealTimer)
})
</script>
