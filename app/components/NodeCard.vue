<template>
  <UCard
    variant="subtle"
    class="node-card"
    :ui="{ body: 'node-card-body' }"
  >
    <div class="node-card-layout">
      <div ref="cardInfoEl" class="node-card-info">
        <div class="job-card-heading">
          <div class="job-title-stack">
            <div class="job-head-badges">
              <UBadge v-if="isLocal" label="本机" color="info" variant="subtle" />
              <UBadge
                v-else
                :label="connected ? '已连接' : '断开'"
                :color="connected ? 'success' : 'neutral'"
                variant="subtle"
              />
              <UBadge
                :label="queuePaused ? '队列暂停' : '队列运行中'"
                :color="queuePaused ? 'warning' : 'success'"
                variant="subtle"
              />
              <UBadge
                v-if="activeJob?.renderMode === 'quick_mp4'"
                label="快速 MP4"
                color="neutral"
                variant="subtle"
              />
            </div>
            <span class="node-name">{{ node.hostname }}</span>
            <p v-if="node.note" class="node-device-note">{{ node.note }}</p>
            <p class="node-note">{{ node.ipAddress }}:{{ node.port }} · v{{ node.version }}</p>
          </div>
        </div>

        <div class="job-footer node-footer">
          <div class="job-meta">
            <span class="job-meta-item">
              <span class="job-meta-label">运行中</span>
              <strong>{{ runningJobs.length }}</strong>
            </span>
            <span class="job-meta-divider" aria-hidden="true" />
            <span class="job-meta-item">
              <span class="job-meta-label">待渲染</span>
              <strong>{{ pendingCount }}</strong>
            </span>
            <span class="job-meta-divider" aria-hidden="true" />
            <span class="job-meta-item">
              <span class="job-meta-label">已完成</span>
              <strong>{{ doneCount }}</strong>
            </span>
            <span class="job-meta-divider" aria-hidden="true" />
            <span class="job-meta-item">
              <span class="job-meta-label">当前任务</span>
              <strong>{{ activeJobLabel }}</strong>
            </span>
            <template v-if="activeJob">
              <span class="job-meta-divider" aria-hidden="true" />
              <span class="job-meta-item">
                <span class="job-meta-label">当前执行</span>
                <strong>{{ executionRangeLabel }}</strong>
              </span>
            </template>
          </div>
        </div>
      </div>

      <div
        class="node-preview"
        :class="{
          'node-preview-empty': !displayPreviewUrl && !previewLoading,
          'node-preview-loading': previewLoading && !displayPreviewUrl,
        }"
        :style="previewStyle"
      >
        <img
          v-if="displayPreviewUrl"
          :key="displayPreviewUrl"
          :src="displayPreviewUrl"
          class="node-preview-image"
          :class="{ 'node-preview-image-visible': previewVisible }"
          alt="node render preview"
        />
        <div
          class="node-preview-placeholder"
          :class="{ 'node-preview-placeholder-hidden': !!displayPreviewUrl && previewVisible }"
        >
          <UIcon name="i-lucide-image" class="job-preview-icon" />
          <span>{{ previewText }}</span>
        </div>
        <UBadge
          v-if="showPreviewBadge && displayPreviewUrl"
          :label="previewBadge"
          color="neutral"
          variant="subtle"
          class="node-preview-badge"
        />
      </div>
    </div>

    <RenderProgress
      v-if="activeJob"
      class="node-card-progress"
      :frame="currentFrame"
      :total-frames="totalFrames"
      :warming-up="isActiveJobWarmingUp"
      :time-elapsed="activeJob.timeElapsed ?? undefined"
      :remaining-secs="activeJob.remainingSecs"
    />
  </UCard>
</template>

<script setup lang="ts">
import type { NodeInfo, RenderJob } from '~/types'

const props = withDefaults(defineProps<{
  node: NodeInfo
  jobs: RenderJob[]
  queuePaused: boolean
  connected?: boolean
  isLocal?: boolean
}>(), {
  connected: true,
  isLocal: false,
})

const runningJobs = computed(() => props.jobs.filter(job => job.status === 'running'))
const activeJob = computed(() => runningJobs.value[0] ?? null)
const previewJob = computed(() => activeJob.value)
const pendingCount = computed(() => props.jobs.filter(job => job.status === 'pending').length)
const doneCount = computed(() => props.jobs.filter(job => job.status === 'done').length)
const currentFrame = computed(() => activeJob.value?.currentFrame ?? 0)
const totalFrames = computed(() => {
  const job = activeJob.value
  return job?.totalFrames ?? (job ? job.frameEnd - job.frameStart + 1 : 0)
})
const activeJobLabel = computed(() => activeJob.value?.name ?? '空闲')
const executionRangeLabel = computed(() => {
  const job = activeJob.value
  return job ? `${job.frameStart}-${job.frameEnd}` : '无'
})
const absoluteCurrentFrame = computed(() => {
  const job = activeJob.value
  if (!job || currentFrame.value <= 0) return null
  return Math.min(job.frameEnd, Math.max(job.frameStart, job.frameStart + currentFrame.value - 1))
})
const isActiveJobWarmingUp = computed(() => {
  const job = activeJob.value
  if (!job) return false
  return (job.currentFrame ?? 0) <= 0 && !((job.timeElapsed ?? 0) > 0) && !((job.remainingSecs ?? 0) > 0)
})
const previewFrameToken = computed(() => {
  const job = previewJob.value
  return `${job?.id ?? 'none'}:${job?.status ?? 'none'}:${currentFrame.value}:${job?.lastRenderedFrame ?? 'none'}:${job?.finishedAt ?? 'none'}`
})
const previewSourceUrl = computed(() => {
  const job = previewJob.value
  if (!job) return null
  if (job.outputFormat === 'OPEN_EXR' || job.outputFormat === 'EXR') return null
  if (job.renderMode === 'quick_mp4' && job.status !== 'done') return null
  return `http://${props.node.ipAddress}:${props.node.port}/api/jobs/${encodeURIComponent(job.id)}/preview?t=${encodeURIComponent(previewFrameToken.value)}`
})
const displayPreviewUrl = ref<string | null>(null)
const previewVisible = ref(false)
const previewLoading = ref(false)
let previewLoadToken = 0
let previewRevealTimer = 0
const previewText = computed(() => {
  const job = previewJob.value
  if (!job) return '当前没有渲染预览'
  if (job.renderMode === 'quick_mp4' && job.status !== 'done') return '等待任务完成后可预览最终帧'
  if (job.outputFormat === 'OPEN_EXR' || job.outputFormat === 'EXR') return 'EXR 不支持预览'
  if (job.renderMode === 'quick_mp4') return previewLoading.value ? '生成最终帧预览中' : '等待最终帧预览'
  if (job.status === 'running') return previewLoading.value ? '加载渲染预览' : '等待首帧输出'
  return previewLoading.value ? '加载最近预览' : '暂无预览'
})
const previewBadge = computed(() => {
  if (!previewJob.value) return '预览'
  return absoluteCurrentFrame.value ? `第 ${absoluteCurrentFrame.value} 帧` : '预览'
})
const showPreviewBadge = computed(() => !!previewJob.value && previewJob.value.renderMode !== 'quick_mp4')
const cardInfoEl = ref<HTMLElement | null>(null)
const cardInfoHeight = ref<number | null>(null)
let cardInfoResizeObserver: ResizeObserver | null = null
const previewStyle = computed(() =>
  cardInfoHeight.value ? { '--node-preview-height': `${cardInfoHeight.value}px` } : {},
)

function resetPreview() {
  displayPreviewUrl.value = null
  previewVisible.value = false
  previewLoading.value = false
}

function preloadPreview(url: string) {
  return new Promise<void>((resolve, reject) => {
    const img = new Image()
    img.onload = () => resolve()
    img.onerror = () => reject(new Error('node preview load failed'))
    img.src = url
  })
}

async function refreshPreview() {
  const token = ++previewLoadToken
  const url = previewSourceUrl.value

  if (!url) {
    resetPreview()
    return
  }

  try {
    previewLoading.value = !displayPreviewUrl.value
    await preloadPreview(url)
    if (token !== previewLoadToken) return

    const hadPreview = !!displayPreviewUrl.value
    displayPreviewUrl.value = url
    previewLoading.value = false

    if (hadPreview) {
      previewVisible.value = true
      return
    }

    previewVisible.value = false
    await nextTick()

    window.clearTimeout(previewRevealTimer)
    previewRevealTimer = window.setTimeout(() => {
      previewVisible.value = true
    }, 24)
  } catch {
    if (token !== previewLoadToken) return
    resetPreview()
  }
}

function syncCardInfoHeight() {
  const el = cardInfoEl.value
  if (!el) return
  cardInfoHeight.value = Math.round(el.getBoundingClientRect().height)
}

watch(previewSourceUrl, () => { void refreshPreview() }, { immediate: true })

watch(
  () => [
    props.node.hostname,
    props.node.note,
    props.node.ipAddress,
    props.node.port,
    props.queuePaused,
    props.connected,
    activeJob.value?.id,
    activeJob.value?.name,
    currentFrame.value,
    totalFrames.value,
  ] as const,
  async () => {
    await nextTick()
    syncCardInfoHeight()
  },
  { flush: 'post' },
)

onMounted(() => {
  if (!cardInfoEl.value) return
  syncCardInfoHeight()
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
