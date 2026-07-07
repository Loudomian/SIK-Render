<template>
  <UCard
    variant="subtle"
    class="node-card"
    :class="{ 'node-card-openable': !!activeJobDetailPath }"
    :ui="{ body: 'node-card-body' }"
    :tabindex="activeJobDetailPath ? 0 : undefined"
    @dblclick="openActiveJobDetails"
    @keydown.enter="openActiveJobDetails"
  >
    <div class="node-card-layout">
      <div ref="cardInfoEl" class="node-card-info">
        <div class="job-card-heading">
          <div class="job-title-stack">
            <div class="job-head-badges">
              <UBadge v-if="isLocal" :label="t('nodeCard.local')" color="info" variant="subtle" />
              <UBadge
                v-else
                :label="connected ? t('nodeCard.connected') : t('nodeCard.disconnected')"
                :color="connected ? 'success' : 'neutral'"
                variant="subtle"
              />
              <UBadge
                v-if="showQueueState"
                :label="queuePaused ? t('nodeCard.queuePaused') : t('nodeCard.queueRunning')"
                :color="queuePaused ? 'warning' : 'success'"
                variant="subtle"
              />
              <UBadge
                v-if="activeJob?.renderMode === 'quick_mp4'"
                :label="t('jobCard.quickMp4')"
                color="neutral"
                variant="subtle"
              />
              <UBadge
                v-if="activeJob && activeJob.crashCount > 0"
                :label="t('jobCard.crashCount', { count: activeJob.crashCount })"
                color="warning"
                variant="subtle"
              />
              <UBadge
                v-if="activeJob?.shadowResolutionScaleOverride != null"
                :label="t('nodeCard.shadowScale', { percent: Math.round(activeJob.shadowResolutionScaleOverride * 100) })"
                color="warning"
                variant="subtle"
              />
            </div>
            <span class="node-name">{{ node.hostname }}</span>
            <p v-if="node.note" class="job-note node-device-note">{{ node.note }}</p>
            <p class="job-note node-note">{{ nodeAddressLabel }}</p>
          </div>
        </div>

        <div class="job-footer node-footer">
          <div class="job-meta">
            <span class="job-meta-item">
              <span class="job-meta-label">{{ t('nodeCard.running') }}</span>
              <strong>{{ runningJobs.length }}</strong>
            </span>
            <span class="job-meta-divider" aria-hidden="true" />
            <span class="job-meta-item">
              <span class="job-meta-label">{{ t('nodeCard.pending') }}</span>
              <strong>{{ pendingCount }}</strong>
            </span>
            <span class="job-meta-divider" aria-hidden="true" />
            <span class="job-meta-item">
              <span class="job-meta-label">{{ t('nodeCard.done') }}</span>
              <strong>{{ doneCount }}</strong>
            </span>
            <span class="job-meta-divider" aria-hidden="true" />
            <span class="job-meta-item">
              <span class="job-meta-label">{{ activeJob ? t('nodeCard.currentJob') : t('nodeCard.latestJob') }}</span>
              <strong>{{ activeJobLabel }}</strong>
            </span>
            <template v-if="detailJob">
              <span class="job-meta-divider" aria-hidden="true" />
              <span class="job-meta-item">
                <span class="job-meta-label">{{ activeJob ? t('jobCard.currentExecution') : t('nodeCard.executionRange') }}</span>
                <strong>{{ executionRangeLabel }}</strong>
              </span>
            </template>
          </div>
          <div v-if="activeJobDetailPath || showForgetAction" class="job-actions" data-no-drag @dblclick.stop>
            <UTooltip v-if="activeJobDetailPath" :text="t('nodeCard.detailsTooltip')" arrow :content="{ side: 'bottom', sideOffset: 8 }">
              <UButton
                icon="i-lucide-external-link"
                :label="activeJob ? t('nodeCard.currentDetails') : t('nodeCard.latestJob')"
                color="neutral"
                variant="outline"
                size="sm"
                @click.stop="goToActiveJobDetails"
              />
            </UTooltip>
            <UTooltip v-if="showForgetAction" :text="t('nodeCard.forgetTooltip')" arrow :content="{ side: 'bottom', sideOffset: 8 }">
              <UButton
                icon="i-lucide-trash-2"
                color="error"
                variant="outline"
                size="sm"
                square
                :loading="forgetting"
                @click.stop="forgetNode"
              />
            </UTooltip>
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
import type { NodeInfo, RemoteJobSnapshot } from '~/types'
import { useDateFormatters } from '~/utils/date-format'

const props = withDefaults(defineProps<{
  node: NodeInfo
  jobs: RemoteJobSnapshot[]
  queuePaused: boolean
  connected?: boolean
  isLocal?: boolean
  lastSeenAt?: number | null
}>(), {
  connected: true,
  isLocal: false,
  lastSeenAt: null,
})
const router = useRouter()
const nodesStore = useNodesStore()
const { t } = useI18n()
const { formatShortTimestamp } = useDateFormatters()

const isLiveNode = computed(() => props.isLocal || props.connected)
const showQueueState = computed(() => isLiveNode.value)
const runningJobs = computed(() => isLiveNode.value ? props.jobs.filter(job => job.status === 'running') : [])
const activeJob = computed(() => runningJobs.value[0] ?? null)
const latestJob = computed(() => {
  let latest: RemoteJobSnapshot | null = null
  for (const job of props.jobs) {
    if (!latest || jobSortTime(job) > jobSortTime(latest)) {
      latest = job
    }
  }
  return latest
})
const detailJob = computed(() => activeJob.value ?? latestJob.value)
const previewJob = computed(() => activeJob.value ?? latestJob.value)
const pendingCount = computed(() => props.jobs.filter(job => job.status === 'pending').length)
const doneCount = computed(() => props.jobs.filter(job => job.status === 'done').length)
const currentFrame = computed(() => activeJob.value?.currentFrame ?? 0)
const totalFrames = computed(() => {
  const job = activeJob.value
  return job?.totalFrames ?? (job ? job.frameEnd - job.frameStart + 1 : 0)
})
const activeJobLabel = computed(() => detailJob.value?.name ?? t('nodeCard.idle'))
const nodeAddressLabel = computed(() => {
  const base = `${props.node.ipAddress}:${props.node.port} · v${props.node.version}`
  if (props.connected || !props.lastSeenAt) return base
  return `${base} · ${t('nodeCard.lastSeen', { time: formatShortTimestamp(props.lastSeenAt) })}`
})
const activeJobDetailPath = computed(() => {
  const job = detailJob.value
  if (!job) return null
  return props.isLocal
    ? `/jobs/${job.id}`
    : `/nodes/${encodeURIComponent(props.node.id)}/jobs/${encodeURIComponent(job.id)}`
})
const executionRangeLabel = computed(() => {
  const job = detailJob.value
  return job ? `${job.frameStart}-${job.frameEnd}` : t('common.none')
})

function openActiveJobDetails(event?: Event) {
  const target = event?.target as HTMLElement | null
  if (target?.closest('a, button, [data-no-drag]')) return
  goToActiveJobDetails()
}

function goToActiveJobDetails() {
  if (!activeJobDetailPath.value) return
  void router.push(activeJobDetailPath.value)
}

function jobSortTime(job: RemoteJobSnapshot) {
  return job.startedAt ?? job.finishedAt ?? job.createdAt
}

async function forgetNode() {
  if (!showForgetAction.value || forgetting.value) return
  try {
    forgetting.value = true
    await nodesStore.forgetNode(props.node.id)
  } finally {
    forgetting.value = false
  }
}

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
  if (!isLiveNode.value) return null
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
  if (!job) return t('nodeCard.preview.empty')
  if (job.renderMode === 'quick_mp4' && job.status !== 'done') return t('nodeCard.preview.waitingFinalFrame')
  if (job.outputFormat === 'OPEN_EXR' || job.outputFormat === 'EXR') return t('nodeCard.preview.exrUnsupported')
  if (job.renderMode === 'quick_mp4') return previewLoading.value ? t('nodeCard.preview.generatingFinalFrame') : t('nodeCard.preview.waitingFinalPreview')
  if (job.status === 'running') return previewLoading.value ? t('nodeCard.preview.loadingRenderPreview') : t('nodeCard.preview.waitingFirstFrame')
  return previewLoading.value ? t('nodeCard.preview.loadingLatestPreview') : t('nodeCard.preview.emptyPreview')
})
const previewBadge = computed(() => {
  if (!previewJob.value) return t('nodeCard.preview.preview')
  return absoluteCurrentFrame.value
    ? t('nodeCard.preview.frameBadge', { frame: absoluteCurrentFrame.value })
    : t('nodeCard.preview.preview')
})
const showPreviewBadge = computed(() => !!previewJob.value && previewJob.value.renderMode !== 'quick_mp4')
const { cardInfoEl, cardInfoHeight, syncHeightAfterTick } = useCardInfoHeight()
const forgetting = ref(false)
const showForgetAction = computed(() => !props.isLocal && !props.connected)
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
  () => { void syncHeightAfterTick() },
  { flush: 'post' },
)

onUnmounted(() => {
  window.clearTimeout(previewRevealTimer)
})
</script>
