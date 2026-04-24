<template>
  <div v-if="job" class="detail-page">
    <section class="page-hero detail-hero">
        <div class="page-hero-copy detail-title">
          <div class="detail-title-row">
            <div class="detail-heading-stack">
              <div class="detail-heading-line">
                <UBadge :label="statusLabel(job.status)" :color="statusColor(job.status)" variant="subtle" />
                <UBadge :label="`#${job.jobNumber}`" color="neutral" variant="subtle" />
                <UBadge :label="job.sourceType === 'blender_job' ? '来自 Blender Job' : '来自文件夹'" color="neutral" variant="subtle" />
              </div>
              <UBreadcrumb
                as="h1"
                :items="[
                  { label: '转码队列', to: '/transcode' },
                  { label: job.name },
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
            </div>
            <div class="detail-header-actions">
              <UButton
                v-if="job.status === 'running'"
                icon="i-lucide-square"
                label="取消"
                color="warning"
                variant="outline"
                size="md"
                @click="handleCancel"
              />
              <UButton
                v-if="job.status !== 'running'"
                icon="i-lucide-trash-2"
                label="删除"
                color="error"
                variant="outline"
                size="md"
                @click="handleDelete"
              />
            </div>
          </div>
        </div>
    </section>

    <section class="detail-content">
      <div class="detail-grid">
        <UCard variant="subtle" :ui="{ root: 'detail-section detail-full', body: 'detail-card-body' }">
        <h2 class="detail-card-title">文件路径</h2>
        <div class="detail-info-stack">
          <section class="detail-info-item">
            <h3 class="detail-info-label">输入路径</h3>
            <div class="surface-panel path-row detail-path-row">
              <span class="path-text" :title="job.inputPath">{{ job.inputPath }}</span>
              <UTooltip text="在文件管理器中显示" :content="{ side: 'top', sideOffset: 6 }">
                <UButton
                  icon="i-lucide-external-link"
                  color="neutral"
                  variant="ghost"
                  size="xs"
                  square
                  @click="openPath(job.inputPath)"
                />
              </UTooltip>
            </div>
          </section>

          <section class="detail-info-item">
            <h3 class="detail-info-label">输出路径</h3>
            <div class="detail-path-stack">
              <div class="surface-panel path-row detail-path-row">
                <span class="path-text" :title="job.outputPath">{{ job.outputPath }}</span>
                <UTooltip text="打开输出目录" :content="{ side: 'top', sideOffset: 6 }">
                  <UButton
                    v-if="job.outputPath"
                    icon="i-lucide-external-link"
                    color="neutral"
                    variant="ghost"
                    size="xs"
                    square
                    @click="openPath(job.outputPath)"
                  />
                </UTooltip>
              </div>
            </div>
          </section>

          <section class="detail-info-item">
            <h3 class="detail-info-label">来源渲染任务</h3>
            <div class="surface-panel path-row detail-path-row">
              <template v-if="sourceJob">
                <span class="path-text" :title="`#${sourceJob.jobNumber} ${sourceJob.name}`">#{{ sourceJob.jobNumber }} {{ sourceJob.name }}</span>
                <UTooltip text="前往渲染任务" :content="{ side: 'top', sideOffset: 6 }">
                  <UButton
                    :to="`/jobs/${sourceJob.id}`"
                    icon="i-lucide-arrow-right"
                    color="neutral"
                    variant="ghost"
                    size="xs"
                    square
                  />
                </UTooltip>
              </template>
              <template v-else-if="job.sourceBlenderJobId">
                <span class="path-text" :title="job.sourceBlenderJobId">{{ job.sourceBlenderJobId }}</span>
                <UTooltip text="前往渲染任务" :content="{ side: 'top', sideOffset: 6 }">
                  <UButton
                    :to="`/jobs/${job.sourceBlenderJobId}`"
                    icon="i-lucide-arrow-right"
                    color="neutral"
                    variant="ghost"
                    size="xs"
                    square
                  />
                </UTooltip>
              </template>
              <template v-else>
                <span class="path-text">—</span>
              </template>
            </div>
          </section>
        </div>
        </UCard>

        <UCard variant="subtle" :ui="{ root: 'detail-section detail-full', body: 'detail-card-body' }">
        <div class="stat-row">
          <div class="stat-item">
            <p class="stat-label">帧段</p>
            <p class="stat-value">{{ job.frameStart }} – {{ job.frameEnd }}</p>
          </div>
          <div class="stat-item">
            <p class="stat-label">输出 FPS</p>
            <p class="stat-value">{{ job.fps.toFixed(3) }}</p>
          </div>
          <div class="stat-item">
            <p class="stat-label">Preset / CRF</p>
            <p class="stat-value">{{ job.preset }} / {{ job.crf }}</p>
          </div>
          <div class="stat-item">
            <p class="stat-label">进度</p>
            <p class="stat-value">{{ job.progressFrame ?? 0 }} / {{ job.totalFrames ?? totalFrames }}</p>
          </div>
        </div>
        <div class="stat-row">
          <div class="stat-item">
            <p class="stat-label">文件大小</p>
            <p class="stat-value">{{ formatBytes(job.outputSizeBytes) }}</p>
          </div>
          <div class="stat-item">
            <p class="stat-label">视频时长</p>
            <p class="stat-value">{{ formatDuration(job.outputDurationSecs) }}</p>
          </div>
          <div class="stat-item">
            <p class="stat-label">创建</p>
            <p class="stat-value">{{ formatTime(job.createdAt) }}</p>
          </div>
          <div class="stat-item">
            <p class="stat-label">开始</p>
            <p class="stat-value">{{ job.startedAt ? formatTime(job.startedAt) : '—' }}</p>
          </div>
          <div class="stat-item">
            <p class="stat-label">结束</p>
            <p class="stat-value">{{ job.finishedAt ? formatTime(job.finishedAt) : '—' }}</p>
          </div>
        </div>
        <template v-if="job.status === 'running'">
          <div class="stat-row">
            <div class="stat-item" style="flex: 1">
              <p class="stat-label">转码进度</p>
              <UProgress
                :value="job.progressFrame ?? 0"
                :max="job.totalFrames ?? totalFrames"
                size="sm"
                class="detail-progress"
              />
              <p class="stat-value stat-progress-note">{{ job.progressFrame ?? 0 }} / {{ job.totalFrames ?? totalFrames }} 帧</p>
            </div>
          </div>
        </template>
        </UCard>

        <UCard variant="subtle" :ui="{ root: 'detail-section detail-full log-section', body: 'detail-card-body' }">
          <div class="log-header">
            <h2 class="detail-card-title log-title">转码日志</h2>
            <div class="log-header-actions">
              <UBadge :label="`${logLines.length} 行`" color="neutral" variant="subtle" />
              <UButton
                :label="showAllLogs ? '最新日志' : '全部日志'"
                :icon="showAllLogs ? 'i-lucide-clock' : 'i-lucide-layers'"
                color="neutral"
                variant="subtle"
                size="md"
                :loading="logsLoading"
                @click="toggleLogScope"
              />
              <UButton
                icon="i-lucide-copy"
                label="复制"
                color="neutral"
                variant="subtle"
                size="md"
                @click="copyLogs"
              />
              <UButton
                v-if="job.outputPath"
                icon="i-lucide-folder-open"
                label="输出目录"
                color="neutral"
                variant="subtle"
                size="md"
                @click="openPath(job.outputPath)"
              />
            </div>
          </div>
          <div class="log-surface">
            <div ref="logEl" class="log-panel" @scroll="onLogScroll">
              <span v-if="logLines.length === 0" class="log-empty">当前还没有转码日志。</span>
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
      <div class="empty-state-title">加载失败</div>
      <div class="empty-state-note">{{ loadError }}</div>
      <div class="empty-state-actions">
        <UButton icon="i-lucide-arrow-left" label="返回转码队列" color="neutral" variant="outline" to="/transcode" />
      </div>
    </UCard>
  </div>

  <div v-else class="detail-page">
    <UCard variant="subtle" class="empty-state" :ui="{ body: 'empty-state-body' }">
      <div class="empty-state-icon"><UIcon name="i-lucide-loader-circle" /></div>
      <div class="empty-state-title">加载中…</div>
    </UCard>
  </div>
</template>

<script setup lang="ts">
import type { TranscodeLogEvent } from '~/types'
import { FFMPEG_STATUS_COLOR, FFMPEG_STATUS_LABEL } from '~/composables/useFfmpegStatus'
import { parseLogLine } from '~/utils/log-line'

const route = useRoute()
const router = useRouter()
const toast = useToast()
const transcodeStore = useTranscodeStore()
const jobsStore = useJobsStore()
const { openPath, getFfmpegJobLogs } = useTauri()
const { onTranscodeProgress, onTranscodeLog, onFfmpegJobUpdated } = useRenderEvents()

const jobId = computed(() => String(route.params.id ?? ''))
const job = computed(() => transcodeStore.getFfmpegJobById(jobId.value))
const sourceJob = computed(() => {
  const currentJob = job.value
  if (!currentJob?.sourceBlenderJobId) return null
  return jobsStore.jobs.find(entry => entry.id === currentJob.sourceBlenderJobId) ?? null
})
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

function statusLabel(status: keyof typeof FFMPEG_STATUS_LABEL) {
  return FFMPEG_STATUS_LABEL[status]
}

function statusColor(status: keyof typeof FFMPEG_STATUS_COLOR) {
  return FFMPEG_STATUS_COLOR[status]
}

function formatTime(ms: number) {
  return new Date(ms).toLocaleString()
}

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
      title: '取消 FFmpeg Job 失败',
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
      title: '删除 FFmpeg Job 失败',
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
      title: '读取全部日志失败',
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
      title: '已复制日志',
      color: 'success',
    })
  } catch (error) {
    toast.add({
      title: '复制日志失败',
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
  await Promise.all([
    transcodeStore.fetchFfmpegJobs(),
    jobsStore.jobs.length ? Promise.resolve() : jobsStore.fetchJobs(),
  ])
  if (!job.value) {
    try {
      await transcodeStore.fetchFfmpegJob(jobId.value)
    } catch (error) {
      loadError.value = error instanceof Error ? error.message : '任务不存在或已被删除'
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
