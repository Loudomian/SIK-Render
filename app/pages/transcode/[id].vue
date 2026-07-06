<template>
  <div v-if="job" class="detail-page">
    <section class="page-hero detail-hero">
      <div class="page-hero-copy detail-title">
        <div class="detail-heading-stack">
          <div class="detail-title-row">
            <div class="detail-title-main">
              <UBreadcrumb
                as="h1"
                :items="[
                  { label: t('transcodeDetails.breadcrumb'), to: '/transcode' },
                  { label: `#${job.jobNumber} ${job.name}` },
                ]"
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
                <UBadge :label="statusLabel(job.status)" :color="statusColor(job.status)" variant="subtle" />
                <UBadge :label="job.sourceType === 'blender_job' ? t('ffmpegCard.sourceBlenderJob') : t('ffmpegCard.sourceFolder')" color="neutral" variant="subtle" />
              </div>
            </div>
            <div class="detail-header-actions">
              <UButton
                v-if="job.status === 'running'"
                icon="i-lucide-square"
                :label="t('common.cancel')"
                color="warning"
                variant="outline"
                size="md"
                @click="handleCancel"
              />
              <UButton
                v-if="job.sourceBlenderJobId"
                :to="`/jobs/${job.sourceBlenderJobId}`"
                icon="i-lucide-arrow-right"
                :label="t('transcodeDetails.sourceJob')"
                color="neutral"
                variant="outline"
                size="md"
              />
              <UButton
                v-if="job.status !== 'running'"
                icon="i-lucide-trash-2"
                :label="t('common.delete')"
                color="error"
                variant="outline"
                size="md"
                @click="handleDelete"
              />
            </div>
          </div>
        </div>
      </div>
    </section>

    <section class="detail-content">
      <div class="detail-grid">
        <UCard variant="subtle" :ui="{ root: 'detail-section detail-full', body: 'detail-card-body' }">
        <h2 class="detail-card-title">{{ t('transcodeDetails.filePaths') }}</h2>
        <div class="detail-info-stack">
          <section class="detail-info-item">
            <div class="detail-path-chip">
              <span class="detail-path-label">{{ t('transcodeDetails.inputPath') }}</span>
              <button class="detail-path-text" type="button" :title="job.inputPath" @click="openPath(job.inputPath)">
                {{ job.inputPath }}
              </button>
              <UTooltip :text="t('jobDetails.copyPath')" :content="{ side: 'top', sideOffset: 6 }">
                <UButton
                  icon="i-lucide-copy"
                  color="neutral"
                  variant="ghost"
                  size="xs"
                  square
                  @click="copyPath(job.inputPath)"
                />
              </UTooltip>
            </div>
          </section>

          <section class="detail-info-item">
            <div class="detail-path-stack">
              <div class="detail-path-chip">
                <span class="detail-path-label">{{ t('transcodeDetails.outputPath') }}</span>
                <button class="detail-path-text" type="button" :title="job.outputPath" @click="openPath(resolveOutputDirectory(job.outputPath))">
                  {{ job.outputPath }}
                </button>
                <UTooltip :text="t('jobDetails.copyPath')" :content="{ side: 'top', sideOffset: 6 }">
                  <UButton
                    v-if="job.outputPath"
                    icon="i-lucide-copy"
                    color="neutral"
                    variant="ghost"
                    size="xs"
                    square
                    @click="copyPath(job.outputPath)"
                  />
                </UTooltip>
              </div>
            </div>
          </section>
        </div>
        </UCard>

        <UCard variant="subtle" :ui="{ root: 'detail-section detail-full', body: 'detail-card-body' }">
        <div class="stat-row">
          <div class="stat-item">
            <p class="stat-label">{{ t('transcodeDetails.stats.frameSegment') }}</p>
            <p class="stat-value">{{ job.frameStart }} – {{ job.frameEnd }}</p>
          </div>
          <div class="stat-item">
            <p class="stat-label">{{ t('transcodeDetails.stats.outputFps') }}</p>
            <p class="stat-value">{{ job.fps.toFixed(3) }}</p>
          </div>
          <div class="stat-item">
            <p class="stat-label">CRF / Preset</p>
            <p class="stat-value">{{ job.crf }} / {{ job.preset }}</p>
          </div>
          <div class="stat-item">
            <p class="stat-label">{{ t('transcodeDetails.stats.progress') }}</p>
            <p class="stat-value">{{ job.progressFrame ?? 0 }} / {{ job.totalFrames ?? totalFrames }}</p>
          </div>
        </div>
        <div class="stat-row">
          <div class="stat-item">
            <p class="stat-label">{{ t('transcodeDetails.stats.fileSize') }}</p>
            <p class="stat-value">{{ formatBytes(job.outputSizeBytes) }}</p>
          </div>
          <div class="stat-item">
            <p class="stat-label">{{ t('transcodeDetails.stats.videoDuration') }}</p>
            <p class="stat-value">{{ formatDuration(job.outputDurationSecs) }}</p>
          </div>
          <div class="stat-item">
            <p class="stat-label">{{ t('transcodeDetails.stats.created') }}</p>
            <p class="stat-value">{{ formatTime(job.createdAt) }}</p>
          </div>
          <div class="stat-item">
            <p class="stat-label">{{ t('transcodeDetails.stats.started') }}</p>
            <p class="stat-value">{{ job.startedAt ? formatTime(job.startedAt) : '—' }}</p>
          </div>
          <div class="stat-item">
            <p class="stat-label">{{ t('transcodeDetails.stats.finished') }}</p>
            <p class="stat-value">{{ job.finishedAt ? formatTime(job.finishedAt) : '—' }}</p>
          </div>
        </div>
        <template v-if="job.status === 'running'">
          <div class="stat-row">
            <div class="stat-item" style="flex: 1">
              <p class="stat-label">{{ t('transcodeDetails.stats.transcodeProgress') }}</p>
              <UProgress
                :value="job.progressFrame ?? 0"
                :max="job.totalFrames ?? totalFrames"
                size="sm"
                class="detail-progress"
              />
              <p class="stat-value stat-progress-note">{{ job.progressFrame ?? 0 }} / {{ job.totalFrames ?? totalFrames }} {{ t('transcodeDetails.stats.frames') }}</p>
            </div>
          </div>
        </template>
        </UCard>

        <UCard variant="subtle" :ui="{ root: 'detail-section detail-full log-section', body: 'detail-card-body' }">
          <div class="log-header">
            <h2 class="detail-card-title log-title">{{ t('transcodeDetails.logs.title') }}</h2>
            <div class="log-header-actions">
              <UButton
                :label="showAllLogs ? t('jobDetails.logs.latest') : t('jobDetails.logs.all')"
                :icon="showAllLogs ? 'i-lucide-clock' : 'i-lucide-layers'"
                color="neutral"
                variant="subtle"
                size="sm"
                :loading="logsLoading"
                @click="toggleLogScope"
              />
              <UButton
                icon="i-lucide-copy"
                :label="t('common.copy')"
                color="neutral"
                variant="subtle"
                size="sm"
                @click="copyLogs"
              />
              <UButton
                v-if="job.outputPath"
                icon="i-lucide-folder-open"
                :label="t('common.outputDirectory')"
                color="neutral"
                variant="subtle"
                size="sm"
                @click="openPath(resolveOutputDirectory(job.outputPath))"
              />
            </div>
          </div>
          <div class="log-surface">
            <div ref="logEl" class="log-panel" @scroll="onLogScroll">
              <span v-if="logLines.length === 0" class="log-empty">{{ t('transcodeDetails.logs.empty') }}</span>
              <div v-for="(entry, index) in displayLogLines" :key="index" class="log-line">
                <div class="log-line-inner" :class="{ 'log-line-inner-no-timestamp': !entry.timestamp }">
                  <span v-if="entry.timestamp" class="log-line-timestamp">{{ entry.timestamp }}</span>
                  <span class="log-line-text">{{ entry.content || '\u00A0' }}</span>
                </div>
              </div>
            </div>
          </div>
        </UCard>
      </div>
    </section>
  </div>

  <div v-else-if="loadError" class="detail-page">
    <UCard variant="subtle" class="empty-state" :ui="{ body: 'empty-state-body' }">
      <div class="empty-state-icon"><UIcon name="i-lucide-alert-circle" /></div>
      <div class="empty-state-title">{{ t('transcodeDetails.error.title') }}</div>
      <div class="empty-state-note">{{ loadError }}</div>
      <div class="empty-state-actions">
        <UButton icon="i-lucide-arrow-left" :label="t('transcodeDetails.error.back')" color="neutral" variant="outline" to="/transcode" />
      </div>
    </UCard>
  </div>

  <div v-else class="detail-page">
    <UCard variant="subtle" class="empty-state" :ui="{ body: 'empty-state-body' }">
      <div class="empty-state-icon"><UIcon name="i-lucide-loader-circle" /></div>
      <div class="empty-state-title">{{ t('common.loading') }}</div>
    </UCard>
  </div>
</template>

<script setup lang="ts">
import type { TranscodeLogEvent } from '~/types'
import { FFMPEG_STATUS_COLOR, useFfmpegStatusLabel } from '~/composables/useFfmpegStatus'
import { useDateFormatters } from '~/utils/date-format'
import { parseLogLine } from '~/utils/log-line'
import { resolveOutputDirectory } from '~/utils/output-path'

const route = useRoute()
const router = useRouter()
const toast = useToast()
const transcodeStore = useTranscodeStore()
const { t } = useI18n()
const { formatTimestamp } = useDateFormatters()
const { openPath, getFfmpegJobLogs } = useTauri()
const { onTranscodeProgress, onTranscodeLog, onFfmpegJobUpdated } = useRenderEvents()

const jobId = computed(() => String(route.params.id ?? ''))
const job = computed(() => transcodeStore.getFfmpegJobById(jobId.value))
const totalFrames = computed(() => {
  const current = job.value
  return current ? current.frameEnd - current.frameStart + 1 : 0
})
const loadError = ref<string | null>(null)
const showAllLogs = ref(false)
const logsLoading = ref(false)
const allLogsLoaded = ref(false)
const allLogLines = ref<string[]>([])
const logLines = computed(() =>
  showAllLogs.value ? allLogLines.value : (transcodeStore.logs[jobId.value] ?? []),
)
const displayLogLines = computed(() => logLines.value.map(line => parseLogLine(line)))
const unlisteners: Array<() => void> = []
const logEl = ref<HTMLDivElement | null>(null)
const pinToBottom = ref(true)

const translatedStatusLabel = useFfmpegStatusLabel()
const statusLabel = translatedStatusLabel

function statusColor(status: keyof typeof FFMPEG_STATUS_COLOR) {
  return FFMPEG_STATUS_COLOR[status]
}

const formatTime = formatTimestamp

function formatBytes(value: number | null) {
  if (value == null || value <= 0) return '—'
  if (value < 1024) return `${value} B`
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`
  if (value < 1024 * 1024 * 1024) return `${(value / (1024 * 1024)).toFixed(1)} MB`
  return `${(value / (1024 * 1024 * 1024)).toFixed(1)} GB`
}

function formatDuration(value: number | null) {
  if (value == null || value <= 0) return '—'
  if (value < 60) return `${value.toFixed(1)}s`
  const mins = Math.floor(value / 60)
  const secs = value % 60
  return `${mins}m ${secs.toFixed(1)}s`
}

async function handleCancel() {
  if (!job.value) return
  try {
    await transcodeStore.cancelFfmpegJob(job.value.id)
  } catch (error) {
    toast.add({
      title: t('transcodeQueue.toast.cancelFailedTitle'),
      description: error instanceof Error ? error.message : String(error),
      color: 'error',
    })
  }
}

async function handleDelete() {
  if (!job.value) return
  try {
    await transcodeStore.deleteFfmpegJob(job.value.id)
    router.push('/transcode')
  } catch (error) {
    toast.add({
      title: t('transcodeQueue.toast.deleteFailedTitle'),
      description: error instanceof Error ? error.message : String(error),
      color: 'error',
    })
  }
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

async function loadLogs() {
  await transcodeStore.loadFfmpegJobLogs(jobId.value)
}

async function toggleLogScope() {
  const nextShowAll = !showAllLogs.value
  showAllLogs.value = nextShowAll
  if (!nextShowAll || allLogsLoaded.value) return

  logsLoading.value = true
  try {
    allLogLines.value = await getFfmpegJobLogs(jobId.value)
    allLogsLoaded.value = true
  } catch (error) {
    showAllLogs.value = false
    toast.add({
      title: t('transcodeDetails.logs.loadAllFailed'),
      description: error instanceof Error ? error.message : String(error),
      color: 'error',
    })
  } finally {
    logsLoading.value = false
  }
}

async function copyLogs() {
  try {
    await navigator.clipboard.writeText(logLines.value.join('\n'))
    toast.add({
      title: t('transcodeDetails.logs.copied'),
      color: 'success',
    })
  } catch (error) {
    toast.add({
      title: t('transcodeDetails.logs.copyFailed'),
      description: error instanceof Error ? error.message : String(error),
      color: 'error',
    })
  }
}

function onLogScroll() {
  const el = logEl.value
  if (!el) return
  pinToBottom.value = el.scrollTop + el.clientHeight >= el.scrollHeight - 40
}

function handleTranscodeLogEvent(event: TranscodeLogEvent) {
  transcodeStore.applyLog(event)
  if (event.jobId !== jobId.value || !allLogsLoaded.value) return
  allLogLines.value = [...allLogLines.value, event.line]
}

watch(
  () => logLines.value.length,
  async () => {
    if (!pinToBottom.value) return
    await nextTick()
    if (logEl.value) {
      logEl.value.scrollTop = logEl.value.scrollHeight
    }
  },
)

onMounted(async () => {
  await transcodeStore.fetchFfmpegJobs()
  if (!job.value) {
    try {
      await transcodeStore.fetchFfmpegJob(jobId.value)
    } catch (error) {
      loadError.value = error instanceof Error ? error.message : t('transcodeDetails.error.missing')
      return
    }
  }

  await loadLogs()
  loadError.value = null

  unlisteners.push(await onTranscodeProgress(event => transcodeStore.applyProgress(event)))
  unlisteners.push(await onTranscodeLog(handleTranscodeLogEvent))
  unlisteners.push(await onFfmpegJobUpdated(event => transcodeStore.applyFfmpegJobUpdate(event)))
})

onUnmounted(() => {
  for (const unlisten of unlisteners) {
    unlisten()
  }
})
</script>
