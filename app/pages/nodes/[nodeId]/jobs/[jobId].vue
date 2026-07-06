<template>
  <div v-if="loading" class="empty">{{ t('nodeJobDetails.loading') }}</div>

  <div v-else-if="peer && job" class="detail-page node-job-detail-page">
    <section class="page-hero detail-hero">
      <div class="page-hero-copy detail-title">
        <div class="detail-heading-stack">
          <div class="detail-title-row">
            <div class="detail-title-main">
              <UBreadcrumb
                as="h1"
                :items="detailBreadcrumbItems"
                :ui="{
                  root: 'detail-breadcrumb',
                  list: 'detail-breadcrumb-list',
                  item: 'detail-breadcrumb-item',
                  link: 'detail-breadcrumb-link',
                  linkLabel: 'detail-breadcrumb-label',
                  separator: 'detail-breadcrumb-separator-wrap',
                  separatorIcon: 'detail-breadcrumb-separator',
                }"
              >
                <template #separator>
                  <span class="detail-breadcrumb-separator" aria-hidden="true">&gt;</span>
                </template>
                <template #item-label="{ item, active }">
                  <span :class="active ? 'detail-breadcrumb-current' : 'detail-breadcrumb-ancestor'">
                    {{ item.label }}
                  </span>
                </template>
              </UBreadcrumb>
              <div class="detail-title-badges">
                <UBadge :label="statusLabel(job.status)" :color="statusBadgeColor" variant="subtle" />
                <UBadge :label="peer.connected ? t('nodeJobDetails.nodeOnline') : t('nodeJobDetails.nodeDisconnected')" :color="peer.connected ? 'success' : 'neutral'" variant="subtle" />
                <UBadge v-if="job.renderMode === 'quick_mp4'" :label="t('jobCard.quickMp4')" color="neutral" variant="subtle" />
                <UBadge v-if="job.crashCount > 0" :label="t('jobCard.crashCount', { count: job.crashCount })" color="warning" variant="subtle" />
                <UBadge v-if="shadowScaleLabel" :label="shadowScaleLabel" color="warning" variant="subtle" />
              </div>
            </div>
            <div class="detail-header-actions" />
          </div>
          <p v-if="job.note" class="page-note detail-note">{{ job.note }}</p>
          <p class="page-note detail-note">{{ peer.node.hostname }} · {{ peer.node.ipAddress }}:{{ peer.node.port }}</p>
        </div>
      </div>
    </section>

    <section class="detail-content">
      <div class="detail-grid">
        <UCard variant="subtle" :ui="{ root: 'detail-section detail-full', body: 'detail-card-body' }">
          <h2 class="detail-card-title">{{ t('jobDetails.filePaths') }}</h2>
          <div class="detail-info-stack">
            <section class="detail-info-item">
              <div class="detail-path-chip">
                <span class="detail-path-label">{{ t('jobDetails.blendFile') }}</span>
                <span class="detail-path-text" :title="job.blendFile">{{ job.blendFile }}</span>
                <UTooltip :text="t('jobDetails.copyPath')" :content="{ side: 'top', sideOffset: 6 }">
                  <UButton icon="i-lucide-copy" color="neutral" variant="ghost" size="xs" square @click="copyPath(job.blendFile)" />
                </UTooltip>
              </div>
            </section>
            <section class="detail-info-item">
              <div class="detail-path-chip">
                <span class="detail-path-label">{{ t('jobDetails.outputPath') }}</span>
                <span class="detail-path-text" :title="job.outputPath">{{ job.outputPath }}</span>
                <UTooltip :text="t('jobDetails.copyPath')" :content="{ side: 'top', sideOffset: 6 }">
                  <UButton icon="i-lucide-copy" color="neutral" variant="ghost" size="xs" square @click="copyPath(job.outputPath)" />
                </UTooltip>
              </div>
            </section>
          </div>
        </UCard>

        <UCard variant="subtle" :ui="{ root: 'detail-section detail-full', body: 'detail-card-body' }">
          <div class="stat-row">
            <div class="stat-item">
              <p class="stat-label">{{ t('jobDetails.stats.format') }}</p>
              <p class="stat-value">{{ outputModeLabel }}</p>
            </div>
            <div class="stat-item">
              <p class="stat-label">{{ t('jobDetails.stats.frameRange') }}</p>
              <p class="stat-value">{{ originalFrameRangeLabel }} ({{ t('jobDetails.stats.totalFrames', { count: originalFrameTotal }) }})</p>
            </div>
            <div v-if="showCurrentExecutionRange" class="stat-item">
              <p class="stat-label">{{ t('jobDetails.stats.currentExecution') }}</p>
              <p class="stat-value">{{ currentExecutionRangeLabel }} ({{ t('jobDetails.stats.totalFrames', { count: currentExecutionTotal }) }})</p>
            </div>
            <div class="stat-item">
              <p class="stat-label">{{ t('nodeJobDetails.currentJob') }}</p>
              <p class="stat-value">{{ currentJobLabel }}</p>
            </div>
            <div v-if="job.crashCount > 0" class="stat-item">
              <p class="stat-label">{{ t('jobDetails.stats.crashRecovery') }}</p>
              <p class="stat-value">{{ t('jobDetails.stats.crashTimes', { count: job.crashCount }) }}</p>
            </div>
            <div v-if="shadowScaleLabel" class="stat-item">
              <p class="stat-label">{{ t('nodeJobDetails.shadowScaleLabel') }}</p>
              <p class="stat-value">{{ shadowScaleValue }}</p>
            </div>
          </div>
          <div class="stat-row">
            <div class="stat-item">
              <p class="stat-label">{{ t('jobDetails.stats.started') }}</p>
              <p class="stat-value">{{ formatTime(job.startedAt ?? job.createdAt) }}</p>
            </div>
            <div class="stat-item">
              <p class="stat-label">{{ t('jobDetails.stats.finished') }}</p>
              <p class="stat-value">{{ job.finishedAt ? formatTime(job.finishedAt) : '-' }}</p>
            </div>
            <div class="stat-item">
              <p class="stat-label">{{ t('jobDetails.stats.duration') }}</p>
              <p class="stat-value">{{ duration }}</p>
            </div>
          </div>
          <template v-if="job.status === 'running'">
            <div class="stat-row">
              <div class="stat-item detail-progress-stat">
                <p class="stat-label">{{ t('jobDetails.stats.renderProgress') }}</p>
                <RenderProgress
                  class="detail-render-progress"
                  :frame="job.currentFrame ?? 0"
                  :total-frames="job.totalFrames ?? (job.frameEnd - job.frameStart + 1)"
                  :warming-up="isWarmingUp"
                  :time-elapsed="job.timeElapsed ?? undefined"
                  :remaining-secs="job.remainingSecs"
                />
              </div>
            </div>
          </template>
        </UCard>

        <UCard variant="subtle" :ui="{ root: 'detail-section detail-full preview-card', body: 'detail-card-body' }">
          <div class="preview-card-head">
            <h2 class="detail-card-title preview-card-title">{{ t('nodeJobDetails.nodePreview') }}</h2>
          </div>
          <div
            class="surface-panel preview-thumb-wrap"
            :class="{ 'preview-thumb-clickable': !!displayPreviewUrl, 'node-job-preview-loading': previewLoading && !displayPreviewUrl }"
            :style="previewStyle"
          >
            <img v-if="displayPreviewUrl" :src="displayPreviewUrl" class="preview-thumb" alt="node render preview" />
            <div v-else class="preview-thumb-empty">
              <UIcon name="i-lucide-image" class="preview-thumb-icon" />
              <span>{{ previewText }}</span>
            </div>
            <UBadge
              v-if="previewFrameLabel && displayPreviewUrl"
              :label="previewFrameLabel"
              color="neutral"
              variant="subtle"
              class="preview-frame-label"
            />
          </div>
        </UCard>

        <UCard variant="subtle" :ui="{ root: 'detail-section detail-full log-section node-event-section', body: 'detail-card-body' }">
          <div class="log-header">
            <h2 class="detail-card-title log-title">{{ t('nodeJobDetails.eventsTitle') }}</h2>
            <div class="log-header-actions" />
          </div>
          <div class="node-event-surface">
            <div ref="eventPanelEl" class="node-event-panel">
              <span v-if="displayEvents.length === 0" class="log-empty">
                {{ t('nodeJobDetails.eventsEmpty') }}
              </span>
              <div v-for="event in displayEvents" :key="event.id" class="node-event-row">
                <span class="node-event-marker" :class="`node-event-marker-${event.level}`" aria-hidden="true" />
                <div class="node-event-main">
                  <div class="node-event-head">
                    <span class="node-event-title">{{ event.title }}</span>
                    <span class="node-event-time">{{ formatShortTimestamp(event.timestamp) }}</span>
                  </div>
                  <p class="node-event-message">{{ event.message }}</p>
                </div>
              </div>
            </div>
          </div>
        </UCard>

        <UCard variant="subtle" :ui="{ root: 'detail-section detail-full log-section node-event-section', body: 'detail-card-body' }">
          <div class="log-header">
            <h2 class="detail-card-title log-title">{{ t('nodeJobDetails.remoteLogsTitle') }}</h2>
            <div class="log-header-actions" />
          </div>
          <div class="node-event-surface">
            <pre ref="remoteLogPanelEl" class="node-event-panel remote-log-panel"><span v-if="remoteLogs.length === 0" class="log-empty">{{ t('nodeJobDetails.remoteLogsEmpty') }}</span><template v-else>{{ remoteLogs.join('\n') }}</template></pre>
          </div>
        </UCard>
      </div>
    </section>
  </div>

  <div v-else class="empty">{{ t('nodeJobDetails.notFound') }}</div>
</template>

<script setup lang="ts">
import { JOB_STATUS_COLOR, useJobStatusLabel } from '~/composables/useJobStatus'
import { formatShortTimestamp, formatTimestamp } from '~/utils/date-format'

const route = useRoute()
const nodesStore = useNodesStore()
const toast = useToast()
const { t } = useI18n()
const loading = ref(true)
const durationNow = ref(Date.now())
const eventPanelEl = ref<HTMLElement | null>(null)
const remoteLogPanelEl = ref<HTMLElement | null>(null)

const statusLabel = useJobStatusLabel()
const nodeId = computed(() => route.params.nodeId as string)
const jobId = computed(() => route.params.jobId as string)
const peer = computed(() => nodesStore.peers[nodeId.value] ?? null)
const job = computed(() => peer.value?.jobs.find(item => item.id === jobId.value) ?? null)
const queueJobs = computed(() => peer.value?.jobs ?? [])
const statusBadgeColor = computed(() => JOB_STATUS_COLOR[job.value?.status ?? 'pending'] ?? 'neutral')
const remoteLogs = computed(() => nodesStore.getPeerLogs(nodeId.value, jobId.value))
const displayEvents = computed(() => {
  const seen = new Set<string>()
  return queueJobs.value
    .flatMap(queueJob => nodesStore.getJobEvents(nodeId.value, queueJob.id))
    .filter(event => event.kind !== 'progress')
    .filter(shouldDisplayEvent)
    .map(normalizeDisplayEvent)
    .filter((event) => {
      const key = event.kind === 'status_changed'
        ? `${event.nodeId}:${event.jobId}:${event.kind}:${event.title.replace(/^#\d+\s+/, '')}`
        : event.id
      if (seen.has(key)) return false
      seen.add(key)
      return true
    })
    .sort((a, b) => b.timestamp - a.timestamp)
})
const detailBreadcrumbItems = computed(() => {
  const currentPeer = peer.value
  if (!currentPeer) return []
  return [
    { label: t('nav.nodes'), to: '/nodes' },
    { label: currentPeer.node.hostname },
  ]
})
const shadowScaleLabel = computed(() => {
  const scale = job.value?.shadowResolutionScaleOverride
  return scale == null ? null : t('nodeCard.shadowScale', { percent: Math.round(scale * 100) })
})
const shadowScaleValue = computed(() => {
  const scale = job.value?.shadowResolutionScaleOverride
  return scale == null ? null : t('nodeJobDetails.shadowScale', { percent: Math.round(scale * 100) })
})
const outputModeLabel = computed(() => {
  if (job.value?.renderMode === 'quick_mp4') return t('jobDetails.preview.quickMp4Output')
  return job.value?.outputFormat ?? '-'
})
const originalFrameRangeLabel = computed(() => {
  const currentJob = job.value
  if (!currentJob) return '-'
  return `${currentJob.originalFrameStart} - ${currentJob.originalFrameEnd}`
})
const originalFrameTotal = computed(() => {
  const currentJob = job.value
  if (!currentJob) return 0
  return currentJob.originalFrameEnd - currentJob.originalFrameStart + 1
})
const currentExecutionRangeLabel = computed(() => {
  const currentJob = job.value
  if (!currentJob) return '-'
  return `${currentJob.frameStart} - ${currentJob.frameEnd}`
})
const currentExecutionTotal = computed(() => {
  const currentJob = job.value
  if (!currentJob) return 0
  return currentJob.frameEnd - currentJob.frameStart + 1
})
const showCurrentExecutionRange = computed(() => {
  const currentJob = job.value
  if (!currentJob) return false
  return currentJob.originalFrameStart !== currentJob.frameStart || currentJob.originalFrameEnd !== currentJob.frameEnd
})
const currentJobLabel = computed(() => {
  const currentJob = job.value
  if (!currentJob) return '-'
  return `#${currentJob.jobNumber} ${currentJob.name}`
})
const isWarmingUp = computed(() => {
  const currentJob = job.value
  if (!currentJob) return false
  return (currentJob.currentFrame ?? 0) <= 0
    && !((currentJob.timeElapsed ?? 0) > 0)
    && !((currentJob.remainingSecs ?? 0) > 0)
})
const duration = computed(() => {
  const currentJob = job.value
  if (!currentJob?.startedAt) return '-'
  const ms = (currentJob.finishedAt ?? durationNow.value) - currentJob.startedAt
  const secs = Math.round(ms / 1000)
  if (secs < 60) return `${secs}s`
  const minutes = Math.floor(secs / 60)
  const restSecs = secs % 60
  if (minutes < 60) return `${minutes}m ${restSecs}s`
  return `${Math.floor(minutes / 60)}h ${minutes % 60}m ${restSecs}s`
})

const currentFrame = computed(() => job.value?.currentFrame ?? 0)
const previewToken = computed(() => {
  const currentJob = job.value
  return `${currentJob?.id ?? 'none'}:${currentJob?.status ?? 'none'}:${currentFrame.value}:${currentJob?.lastRenderedFrame ?? 'none'}:${currentJob?.finishedAt ?? 'none'}`
})
const previewSourceUrl = computed(() => {
  const currentPeer = peer.value
  const currentJob = job.value
  if (!currentPeer || !currentJob) return null
  if (currentJob.outputFormat === 'OPEN_EXR' || currentJob.outputFormat === 'EXR') return null
  if (currentJob.renderMode === 'quick_mp4' && currentJob.status !== 'done') return null
  return `http://${currentPeer.node.ipAddress}:${currentPeer.node.port}/api/jobs/${encodeURIComponent(currentJob.id)}/preview?t=${encodeURIComponent(previewToken.value)}`
})
const displayPreviewUrl = ref<string | null>(null)
const previewLoading = ref(false)
const previewError = ref(false)
const loadedEventKeys = new Set<string>()
let previewLoadToken = 0
let queueEventsLoadTimer = 0
let durationRefreshTimer = 0
const previewAspect = computed(() => {
  const currentJob = job.value
  if (!currentJob?.previewWidth || !currentJob.previewHeight) return null
  return `${currentJob.previewWidth} / ${currentJob.previewHeight}`
})
const previewStyle = computed(() => previewAspect.value ? { '--preview-aspect': previewAspect.value } : undefined)
const previewFrameLabel = computed(() => {
  const currentJob = job.value
  if (!currentJob || currentJob.renderMode === 'quick_mp4') return null
  const current = currentJob.currentFrame ?? 0
  if (current <= 0) return null
  const frame = Math.min(currentJob.frameEnd, Math.max(currentJob.frameStart, currentJob.frameStart + current - 1))
  return t('nodeJobDetails.preview.frameBadge', { frame })
})
const previewText = computed(() => {
  const currentJob = job.value
  if (!currentJob) return t('nodeJobDetails.preview.empty')
  if (currentJob.renderMode === 'quick_mp4' && currentJob.status !== 'done') return t('nodeJobDetails.preview.waitingFinalFrame')
  if (currentJob.outputFormat === 'OPEN_EXR' || currentJob.outputFormat === 'EXR') return t('nodeJobDetails.preview.exrUnsupported')
  if (!peer.value?.connected) return t('nodeJobDetails.preview.disconnected')
  if (previewError.value) return t('nodeJobDetails.preview.failed')
  return previewLoading.value ? t('nodeJobDetails.preview.loading') : t('nodeJobDetails.preview.empty')
})

function formatTime(timestamp: number) {
  return formatTimestamp(timestamp)
}

async function copyPath(path: string) {
  if (!path) return
  try {
    await navigator.clipboard.writeText(path)
    toast.add({ title: t('jobDetails.copy.success'), color: 'success' })
  } catch (error) {
    toast.add({
      title: t('jobDetails.copy.failed'),
      description: error instanceof Error ? error.message : String(error),
      color: 'error',
    })
  }
}

function normalizeDisplayEvent(event: ReturnType<typeof nodesStore.getJobEvents>[number]) {
  const eventJob = queueJobs.value.find(item => item.id === event.jobId)
  if (!eventJob || event.kind !== 'status_changed') return event

  const statusTitle = event.title.replace(/^#\d+\s+/, '')
  return {
    ...event,
    title: `#${eventJob.jobNumber} ${statusTitle}`,
    message: /^任务\s+#\d+/.test(event.message)
      ? event.message
      : `任务 #${eventJob.jobNumber} ${eventJob.name} ${event.message.replace(/^任务/, '')}`,
  }
}

function shouldDisplayEvent(event: ReturnType<typeof nodesStore.getJobEvents>[number]) {
  if (event.kind !== 'node_connected' && event.kind !== 'node_disconnected') return true
  const eventJob = queueJobs.value.find(item => item.id === event.jobId)
  return eventJob?.status === 'running'
}

async function loadNodeQueueEvents() {
  const currentPeer = peer.value
  if (!currentPeer) return

  await Promise.all(currentPeer.jobs.map(async (queueJob) => {
    const key = `${currentPeer.node.id}:${queueJob.id}`
    if (loadedEventKeys.has(key)) return
    loadedEventKeys.add(key)
    await nodesStore.loadJobEvents(currentPeer.node.id, queueJob.id)
  }))
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
    displayPreviewUrl.value = null
    previewLoading.value = false
    previewError.value = false
    return
  }

  try {
    previewLoading.value = !displayPreviewUrl.value
    previewError.value = false
    await preloadPreview(url)
    if (token !== previewLoadToken) return
    displayPreviewUrl.value = url
    previewLoading.value = false
  } catch {
    if (token !== previewLoadToken) return
    displayPreviewUrl.value = null
    previewLoading.value = false
    previewError.value = true
  }
}

watch(previewSourceUrl, () => { void refreshPreview() }, { immediate: true })

onMounted(async () => {
  durationRefreshTimer = window.setInterval(() => {
    durationNow.value = Date.now()
  }, 30_000)

  try {
    await nodesStore.init()
    await loadNodeQueueEvents()
  } finally {
    loading.value = false
  }
})

watch(
  () => queueJobs.value.map(queueJob => queueJob.id).join('|'),
  () => {
    window.clearTimeout(queueEventsLoadTimer)
    queueEventsLoadTimer = window.setTimeout(() => { void loadNodeQueueEvents() }, 120)
  },
)

watch(
  () => displayEvents.value.length,
  async () => {
    await nextTick()
    const panel = eventPanelEl.value
    if (!panel) return
    panel.scrollTop = 0
  },
  { flush: 'post' },
)

watch(
  () => remoteLogs.value.length,
  async () => {
    await nextTick()
    const panel = remoteLogPanelEl.value
    if (!panel) return
    panel.scrollTop = panel.scrollHeight
  },
  { flush: 'post' },
)

onUnmounted(() => {
  window.clearInterval(durationRefreshTimer)
  window.clearTimeout(queueEventsLoadTimer)
})
</script>
