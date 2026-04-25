<template>
  <div v-if="job" class="detail-page">
    <section class="page-hero detail-hero">
      <div class="page-hero-copy detail-title">
        <UContextMenu
          :items="buildJobContextMenuItems(job)"
          :ui="{ content: 'detail-context-menu-content' }"
        >
          <div class="detail-context-menu-target" data-context-menu>
            <div class="detail-heading-stack">
              <div class="detail-heading-line">
                <UBadge
                  :label="STATUS_LABEL[job.status] ?? job.status"
                  :color="statusBadgeColor"
                  variant="subtle"
                />
                <UBadge v-if="orderBadgeLabel" :label="orderBadgeLabel" color="neutral" variant="subtle" />
                <UBadge
                  v-if="job.crashCount > 0"
                  :label="`崩溃 ${job.crashCount} 次`"
                  color="warning"
                  variant="subtle"
                />
              </div>
              <div class="detail-title-row">
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
                <div class="detail-header-actions">
                  <UFieldGroup size="md" class="detail-action-fieldgroup">
                    <UButton
                      :icon="transcodePrimaryAction.icon"
                      :label="transcodePrimaryAction.label"
                      :color="transcodePrimaryAction.color"
                      variant="subtle"
                      size="md"
                      :loading="transcodePrimaryAction.loading"
                      :disabled="transcodePrimaryAction.disabled"
                      @click="handlePrimaryTranscodeAction"
                    />
                    <UDropdownMenu
                      :items="transcodeActionItems"
                      arrow
                      :content="{ side: 'bottom', align: 'end', sideOffset: 8 }"
                    >
                      <UButton
                        icon="i-lucide-chevron-down"
                        color="neutral"
                        variant="outline"
                        size="md"
                        square
                      />
                      <template #auto-transcode-trailing>
                        <USwitch
                          :model-value="autoTranscodeEnabled"
                          color="neutral"
                          :disabled="updatingAutoTranscode || !transcodeSupported"
                          @pointerdown.stop
                          @click.stop
                          @update:model-value="handleAutoTranscodeSwitchUpdate"
                        />
                      </template>
                    </UDropdownMenu>
                  </UFieldGroup>
                  <UButton
                    v-if="job.status === 'failed' || job.status === 'cancelled' || job.status === 'interrupted' || job.status === 'done'"
                    icon="i-lucide-rotate-ccw"
                    :label="job.status === 'cancelled' || job.status === 'interrupted' ? '继续' : '重新渲染'"
                    :color="job.status === 'cancelled' || job.status === 'interrupted' ? 'warning' : 'neutral'"
                    variant="outline"
                    size="md"
                    @click="handleRetry"
                  />
                  <UButton
                    v-if="job.status === 'running' || job.status === 'pending'"
                    icon="i-lucide-x"
                    label="取消"
                    color="warning"
                    variant="outline"
                    size="md"
                    @click="jobsStore.stopJob(job.id)"
                  />
                  <UButton
                    v-if="job.status === 'done' || job.status === 'failed' || job.status === 'cancelled' || job.status === 'interrupted'"
                    icon="i-lucide-trash-2"
                    label="删除"
                    color="error"
                    variant="outline"
                    size="md"
                    @click="showDeleteConfirm = true"
                  />
                </div>
              </div>
              <p v-if="job.note" class="page-note detail-note">{{ job.note }}</p>
            </div>
          </div>
        </UContextMenu>
      </div>
    </section>

    <UModal
      v-model:open="showDeleteConfirm"
      :close="false"
      title="删除任务"
      :ui="{ content: 'job-modal-content' }"
    >
      <template #body>
        <div class="modal-stack">
          <p class="modal-copy">
            确定删除 <strong>{{ job.name }}</strong>？此操作不可撤销。
          </p>
          <div class="modal-actions">
            <UButton icon="i-lucide-x" label="取消" color="warning" variant="outline" @click="showDeleteConfirm = false" />
            <UButton icon="i-lucide-trash-2" label="删除" color="error" variant="outline" @click="removeAndBack" />
          </div>
        </div>
      </template>
    </UModal>

    <JobMetadataModal
      v-model:open="metadataDialogOpen"
      :job="metadataJob"
    />

    <TranscodeSubmitModal
      v-if="transcodeModalOpen && job"
      :open="transcodeModalOpen"
      :initial-config="effectiveTranscodeConfig"
      :blender-job-id="job.id"
      :blender-job-name="job.name"
      :blender-job-fps="job.fps ?? null"
      :blender-job-frame-start="job.frameStart"
      :blender-job-frame-end="job.frameEnd"
      :blender-job-output-path="job.outputPath"
      @submit="handleTranscodeSubmit"
      @close="transcodeModalOpen = false"
      @update:open="transcodeModalOpen = $event"
    />

    <TranscodeSubmitModal
      v-if="transcodeSettingsModalOpen && job"
      :open="transcodeSettingsModalOpen"
      mode="settings"
      :initial-config="effectiveTranscodeConfig"
      :base-config="baseTranscodeConfig"
      :blender-job-id="job.id"
      :blender-job-name="job.name"
      :blender-job-fps="job.fps ?? null"
      :blender-job-frame-start="job.frameStart"
      :blender-job-frame-end="job.frameEnd"
      :blender-job-output-path="job.outputPath"
      @save-settings="handleTranscodeSettingsSave"
      @close="transcodeSettingsModalOpen = false"
      @update:open="transcodeSettingsModalOpen = $event"
    />

    <UModal
      :open="showRetryConfirm"
      :close="false"
      title="选择渲染方式"
      :ui="{ content: 'job-modal-content retry-modal-content' }"
      @update:open="v => { if (!v) closeRetryConfirm() }"
    >
      <template #body>
        <div class="modal-stack">
          <p class="modal-copy">
            <template v-if="retryExistingCount > 0">
              检测到当前输出范围已存在
              <strong>{{ retryExistingCount }} 帧</strong>
              <template v-if="retryFrameStatus?.lastFrame != null">
                ，当前最后一帧为 <strong>{{ retryFrameStatus.lastFrame }}</strong>
              </template>
              。
            </template>
            <template v-else>
              当前任务帧段里还没有检测到已渲染输出，你可以直接重开、继续，或改成指定区间渲染。
            </template>
          </p>
          <div class="choice-grid retry-choice-grid">
            <UCard variant="subtle" class="choice-card" :ui="{ body: 'choice-card-body' }">
              <div class="choice-card-head">
                <p class="choice-card-mode">覆盖模式</p>
                <h3 class="choice-card-title">重新开始渲染</h3>
              </div>
              <p class="choice-card-desc">
                从第 <span class="choice-card-accent">{{ job?.frameStart }}</span> 帧开始渲染，直接覆盖
                <span class="choice-card-accent">{{ job?.frameStart }}–{{ job?.frameEnd }}</span>
                范围内的同名帧
              </p>
              <UButton
                color="neutral"
                variant="outline"
                label="从头覆盖"
                class="choice-card-action"
                :loading="retrySubmittingMode === 'restart'"
                :disabled="retrySubmittingMode !== null"
                @click="confirmRetryFromStart"
              />
            </UCard>

            <UCard variant="subtle" class="choice-card" :ui="{ body: 'choice-card-body' }">
              <div class="choice-card-head">
                <p class="choice-card-mode">续跑模式</p>
                <h3 class="choice-card-title">从最后一帧继续</h3>
              </div>
              <p class="choice-card-desc">
                <template v-if="job && retryFrameStatus && retryFrameStatus.nextFrame <= job.frameEnd">
                  从第 <span class="choice-card-accent">{{ retryFrameStatus.nextFrame }}</span> 帧继续渲染
                </template>
                <template v-else>
                  当前帧段已完整存在，继续将直接完成
                </template>
              </p>
              <UButton
                color="neutral"
                variant="outline"
                label="继续渲染"
                class="choice-card-action"
                :loading="retrySubmittingMode === 'continue'"
                :disabled="retrySubmittingMode !== null"
                @click="confirmRetryContinue"
              />
            </UCard>

            <UCard variant="subtle" class="choice-card" :ui="{ body: 'choice-card-body' }">
              <div class="choice-card-head">
                <p class="choice-card-mode">区间模式</p>
                <h3 class="choice-card-title">指定区间渲染</h3>
              </div>
              <div class="choice-card-fields">
                <UFormField label="起始帧">
                  <UInputNumber v-model="retryCustomStart" :min="1" />
                </UFormField>
                <UFormField label="结束帧">
                  <UInputNumber v-model="retryCustomEnd" :min="1" />
                </UFormField>
              </div>
              <USwitch
                v-model="retryCustomResumeFromExisting"
                color="neutral"
                label="从已有帧续跑"
                :description="retryCustomResumeFromExisting ? '开启后会在所选区间内查找最后一帧，并从下一帧继续。' : '关闭后会从所选区间起始帧重新覆盖渲染。'"
                class="choice-card-switch"
              />
              <p v-if="retryCustomInspectLoading" class="choice-card-inline-note">正在检查所选区间…</p>
              <p v-else-if="retryCustomRangeSummary" class="choice-card-inline-note">{{ retryCustomRangeSummary }}</p>
              <UButton
                color="neutral"
                variant="outline"
                :label="retryCustomResumeFromExisting ? '按区间续跑' : '按区间覆盖'"
                class="choice-card-action"
                :loading="retrySubmittingMode === 'range-continue' || retrySubmittingMode === 'range-restart'"
                :disabled="retrySubmittingMode !== null"
                @click="confirmRetryCustomRange"
              />
            </UCard>
          </div>
          <div class="modal-actions">
            <UButton icon="i-lucide-x" label="取消" color="warning" variant="outline" :disabled="retrySubmittingMode !== null" @click="closeRetryConfirm" />
          </div>
        </div>
      </template>
    </UModal>

    <section class="detail-content">
      <div class="detail-grid">
        <UCard variant="subtle" :ui="{ root: 'detail-section detail-full', body: 'detail-card-body' }">
        <h2 class="detail-card-title">文件路径</h2>
        <div class="detail-info-stack">
          <section class="detail-info-item">
            <h3 class="detail-info-label">工程文件</h3>
            <div class="surface-panel path-row detail-path-row">
              <span class="path-text" :title="job.blendFile">{{ job.blendFile }}</span>
              <UTooltip text="在文件管理器中显示" :content="{ side: 'top', sideOffset: 6 }">
                <UButton icon="i-lucide-external-link" color="neutral" variant="ghost" size="xs" square @click="openPath(job.blendFile)" />
              </UTooltip>
            </div>
          </section>
          <section class="detail-info-item">
            <h3 class="detail-info-label">输出路径</h3>
            <div class="detail-path-stack">
              <div class="surface-panel path-row detail-path-row">
                <span class="path-text" :title="job.outputPath">{{ job.outputPath }}</span>
                <UTooltip text="打开输出目录" :content="{ side: 'top', sideOffset: 6 }">
                  <UButton icon="i-lucide-external-link" color="neutral" variant="ghost" size="xs" square @click="openPath(job.outputPath)" />
                </UTooltip>
              </div>
            </div>
          </section>
        </div>
        </UCard>

        <UCard variant="subtle" :ui="{ root: 'detail-section detail-full', body: 'detail-card-body' }">
        <div class="stat-row">
          <div class="stat-item">
            <p class="stat-label">格式</p>
            <p class="stat-value">{{ job.outputFormat }}</p>
          </div>
          <div class="stat-item">
            <p class="stat-label">帧范围</p>
            <p class="stat-value">{{ job.frameStart }} – {{ job.frameEnd }}（共 {{ job.frameEnd - job.frameStart + 1 }} 帧）</p>
          </div>
          <div class="stat-item">
            <p class="stat-label">Blender</p>
            <p class="stat-value">{{ blenderVersion }}</p>
          </div>
          <div v-if="crashCount" class="stat-item">
            <p class="stat-label">崩溃恢复</p>
            <p class="stat-value">{{ crashCount }} 次</p>
          </div>
        </div>
        <div class="stat-row">
          <div class="stat-item">
            <p class="stat-label">开始</p>
            <p class="stat-value">{{ formatTime(job.startedAt ?? job.createdAt) }}</p>
          </div>
          <div class="stat-item">
            <p class="stat-label">完成</p>
            <p class="stat-value">{{ job.finishedAt ? formatTime(job.finishedAt) : '—' }}</p>
          </div>
          <div class="stat-item">
            <p class="stat-label">耗时</p>
            <p class="stat-value">{{ duration }}</p>
          </div>
        </div>
        <template v-if="job.status === 'running'">
          <div class="stat-row">
            <div class="stat-item detail-progress-stat">
              <p class="stat-label">渲染进度</p>
              <RenderProgress
                class="detail-render-progress"
                :frame="job.currentFrame ?? 0"
                :total-frames="job.totalFrames ?? (job.frameEnd - job.frameStart + 1)"
                :warming-up="jobsStore.isJobWarmingUp(job.id)"
                :time-elapsed="job.timeElapsed ?? undefined"
                :remaining-secs="job.remainingSecs"
              />
            </div>
          </div>
        </template>
        </UCard>

        <UCard variant="subtle" :ui="{ root: 'detail-section detail-full preview-card', body: 'detail-card-body' }">
        <h2 class="detail-card-title">帧预览</h2>
        <div
          class="surface-panel preview-thumb-wrap"
          :class="{ 'preview-thumb-clickable': !!previewUrl }"
          :style="previewStyle"
          @click="previewUrl && (lightboxOpen = true)"
        >
          <img v-if="previewUrl" :src="previewUrl" class="preview-thumb" alt="last frame" />
          <div v-else class="preview-thumb-empty">
            <UIcon name="i-lucide-image" class="preview-thumb-icon" />
            <span>{{ job.outputFormat === 'OPEN_EXR' ? 'EXR 不支持预览' : '暂无已渲染帧' }}</span>
          </div>
          <UBadge
            v-if="previewFrame && previewUrl"
            :label="`第 ${previewFrame} 帧`"
            color="neutral"
            variant="subtle"
            class="preview-frame-label"
          />
        </div>
        </UCard>

        <UCard variant="subtle" :ui="{ root: 'detail-section detail-full log-section', body: 'detail-card-body' }">
          <div class="log-header">
            <h2 class="detail-card-title log-title">输出日志</h2>
            <div class="log-header-actions">
              <UButton
                :label="showAllLogs ? '最新日志' : '全部日志'"
                :icon="showAllLogs ? 'i-lucide-clock' : 'i-lucide-layers'"
                color="neutral"
                variant="subtle"
                size="sm"
                :loading="logsLoading"
                @click="toggleLogScope"
              />
              <UButton
                v-if="logSummary?.directory"
                icon="i-lucide-folder-open"
                label="打开日志目录"
                color="neutral"
                variant="subtle"
                size="sm"
                @click="openPath(logSummary.directory)"
              />
            </div>
          </div>
          <div class="log-surface">
            <div class="log-panel" ref="logEl" @scroll="onLogScroll">
              <span v-if="logLines.length === 0" class="log-empty">
                {{ job.status === 'pending' ? '等待开始…' : '暂无输出内容。' }}
              </span>
              <div v-for="(entry, i) in displayJobLogs" :key="i" class="log-line">
                <div class="log-line-inner" :class="{ 'log-line-inner-no-timestamp': !entry.timestamp }">
                  <span v-if="entry.timestamp" class="log-line-timestamp">{{ entry.timestamp }}</span>
                  <span class="log-line-text">{{ entry.content || '\u00A0' }}</span>
                </div>
              </div>
            </div>
          </div>
        </UCard>
      </div>

      <div v-if="warnings.length" class="detail-warnings">
        <UAlert
          v-for="(w, i) in warnings"
          :key="i"
          icon="i-lucide-triangle-alert"
          color="warning"
          variant="subtle"
          title="渲染警告"
          :description="w"
        />
      </div>

      <p v-if="retryActionError" class="form-error">{{ retryActionError }}</p>
    </section>

    <UModal v-model:open="lightboxOpen" :close="false" :ui="{ content: 'preview-lightbox' }">
      <template #body>
        <div @click="lightboxOpen = false">
          <div @click.stop>
          <img :src="previewUrl!" class="preview-lightbox-img" alt="frame preview" />
          </div>
        </div>
      </template>
    </UModal>
  </div>

  <div v-else class="empty">找不到该任务。</div>
</template>

<script setup lang="ts">
import { convertFileSrc } from '@tauri-apps/api/core'
import type { DropdownMenuItem } from '@nuxt/ui'
import type { AddFfmpegJobPayload, JobLogSummary, RenderJob, RenderJobTranscodeConfig, RenderedFramesStatus } from '~/types'
import { JOB_STATUS_COLOR, JOB_STATUS_LABEL } from '~/composables/useJobStatus'
import { resolveBaseRenderJobTranscodeConfig, resolveEffectiveRenderJobTranscodeConfig } from '~/composables/useTranscodeConfig'
import { parseLogLine } from '~/utils/log-line'

const route = useRoute()
const router = useRouter()
const toast = useToast()
const jobsStore = useJobsStore()
const transcodeStore = useTranscodeStore()

const settingsStore = useSettingsStore()

const { openPath, inspectRenderedFrames, getLastRenderedFrame, getJobLogSummary, getJobLogs, updateJobPreviewDimensions } = useTauri()
const { onProgress, onJobUpdated, onLog, onFfmpegJobUpdated } = useRenderEvents()
const STATUS_LABEL = JOB_STATUS_LABEL

const jobId = computed(() => route.params.id as string)
const job = computed(() => jobsStore.jobs.find((j) => j.id === jobId.value))
const jobLogs = computed(() => jobsStore.getJobLogs(jobId.value))
const showAllLogs = ref(false)
const logsLoading = ref(false)
const allLogsLoaded = ref(false)
const allLogLines = ref<string[]>([])
const logLines = computed(() =>
  showAllLogs.value ? allLogLines.value : jobLogs.value,
)
const displayJobLogs = computed(() => logLines.value.map(line => parseLogLine(line)))
const relatedFfmpegJobs = computed(() => transcodeStore.getRelatedJobs(jobId.value))
const primaryTranscodeJob = computed(() =>
  relatedFfmpegJobs.value.find(entry => entry.status === 'running')
  ?? relatedFfmpegJobs.value.find(entry => entry.status === 'pending')
  ?? relatedFfmpegJobs.value[0]
  ?? null,
)
const detailBreadcrumbItems = computed(() => {
  const currentJob = job.value
  if (!currentJob) return []
  return [
    { label: '渲染队列', to: '/' },
    { label: `#${currentJob.jobNumber} ${currentJob.name}` },
  ]
})
const statusBadgeColor = computed(() => JOB_STATUS_COLOR[job.value?.status ?? 'pending'] ?? 'neutral')
const orderBadgeLabel = computed(() => {
  if (!job.value || job.value.status === 'running') return null
  const queue = jobsStore.jobs.filter(item => item.status !== 'running')
  const index = queue.findIndex(item => item.id === job.value?.id)
  return index === -1 ? null : `顺序 ${index + 1}`
})
const metadataDialogOpen = ref(false)
const metadataJob = computed(() => job.value ?? null)
const transcodeModalOpen = ref(false)
const transcodeSettingsModalOpen = ref(false)
const blenderVersion = computed(() => {
  const exe = job.value?.blenderExecutable
  if (!exe) return '—'
  const match = settingsStore.blenderVersions.find((b) => b.executable === exe)
  return match ? `Blender ${match.version}` : exe
})
const baseTranscodeConfig = computed<RenderJobTranscodeConfig | null>(() => {
  if (!job.value) return null
  return resolveBaseRenderJobTranscodeConfig(job.value, settingsStore.settings)
})
const effectiveTranscodeConfig = computed<RenderJobTranscodeConfig | null>(() => {
  if (!job.value) return null
  return resolveEffectiveRenderJobTranscodeConfig(job.value, settingsStore.settings)
})
const transcodeSupported = computed(() => {
  const format = job.value?.outputFormat
  return Boolean(job.value) && format !== 'OPEN_EXR' && format !== 'EXR'
})
const autoTranscodeEnabled = computed(() => transcodeSupported.value && Boolean(job.value?.autoTranscodeMp4))
const transcodePrimaryAction = computed(() => {
  const currentJob = job.value
  const currentTranscodeJob = primaryTranscodeJob.value
  if (!currentJob) {
    return {
      label: '提交转码',
      icon: 'i-lucide-film',
      color: 'neutral' as const,
      loading: false,
      disabled: true,
    }
  }

  if (!currentTranscodeJob) {
    if (!transcodeSupported.value) {
      return {
        label: 'EXR 禁用转码',
        icon: 'i-lucide-ban',
        color: 'neutral' as const,
        loading: false,
        disabled: true,
      }
    }

    return {
      label: '提交转码',
      icon: 'i-lucide-film',
      color: 'neutral' as const,
      loading: false,
      disabled: currentJob.status === 'running',
    }
  }

  const statusMap = {
    pending: { icon: 'i-lucide-loader-circle', color: 'warning' as const, loading: false },
    running: { icon: 'i-lucide-loader-circle', color: 'info' as const, loading: true },
    done: { icon: 'i-lucide-circle-check-big', color: 'success' as const, loading: false },
    failed: { icon: 'i-lucide-triangle-alert', color: 'error' as const, loading: false },
    cancelled: { icon: 'i-lucide-square', color: 'warning' as const, loading: false },
  }[currentTranscodeJob.status]

  return {
    label: '查看转码',
    icon: statusMap.icon,
    color: statusMap.color,
    loading: statusMap.loading,
    disabled: false,
  }
})

function openMetadataDialog() {
  if (!job.value) return
  metadataDialogOpen.value = true
}

function buildJobContextMenuItems(currentJob: RenderJob) {
  return [
    {
      label: '编辑项目信息',
      icon: 'i-lucide-notebook-pen',
      onSelect: () => openMetadataDialog(),
    },
  ]
}

const updatingAutoTranscode = ref(false)
const logSummary = ref<JobLogSummary | null>(null)
const detailUnlisteners: Array<() => void> = []
let logSummaryTimer: ReturnType<typeof setTimeout> | null = null

const LOG_WARNINGS: Array<{ pattern: RegExp; message: string }> = [
  {
    pattern: /Shadow buffer full/i,
    message: '阴影缓冲区已满（Shadow buffer full）：部分阴影可能缺失，光照结果不准确，建议在渲染属性中增大阴影贴图分辨率或减少灯光数量。',
  },
]

const crashCount = computed(() => job.value?.crashCount ?? 0)

const transcodeActionItems = computed<DropdownMenuItem[][]>(() => {
  const items: DropdownMenuItem[][] = [[
    {
      slot: 'auto-transcode',
      label: '渲染完成后自动转码',
      icon: 'i-lucide-clapperboard',
      loading: updatingAutoTranscode.value,
      disabled: updatingAutoTranscode.value || !transcodeSupported.value,
      onSelect: (event: Event) => {
        event.preventDefault()
        void handleAutoTranscodeToggle(!autoTranscodeEnabled.value)
      },
    },
  ], [
    {
      label: '转码设置',
      icon: 'i-lucide-sliders',
      disabled: !job.value || !transcodeSupported.value,
      onSelect: () => {
        transcodeSettingsModalOpen.value = true
      },
    },
    {
      label: '立即提交转码',
      icon: 'i-lucide-film',
      disabled: job.value?.status === 'running' || !transcodeSupported.value,
      onSelect: () => {
        void submitTranscodeForJob()
      },
    },
  ], [
    {
      label: '前往转码队列',
      icon: 'i-lucide-list-video',
      onSelect: () => router.push('/transcode'),
    },
  ]]

  if (primaryTranscodeJob.value) {
    items.push([
      {
        label: '查看 FFmpeg Job',
        icon: 'i-lucide-file-text',
        onSelect: () => router.push(`/transcode/${primaryTranscodeJob.value?.id}`),
      },
    ])
  }

  return items
})

const warnings = computed(() => {
  const found: string[] = []
  const logs = jobLogs.value
  for (const { pattern, message } of LOG_WARNINGS) {
    if (logs.some((line) => pattern.test(line))) {
      found.push(message)
    }
  }
  return found
})

// ── Frame preview ──────────────────────────────────────────────────────────
const previewUrl = ref<string | null>(null)
const previewFrame = ref<number | null>(null)
const previewAspect = ref<string | null>(null)
const lightboxOpen = ref(false)
const previewStyle = computed(() =>
  previewAspect.value ? { '--preview-aspect': previewAspect.value } : undefined,
)
const previewFrameEnd = computed(() => {
  const j = job.value
  if (!j) return null
  if (j.lastRenderedFrame != null) {
    return Math.min(j.frameEnd, Math.max(j.frameStart, j.lastRenderedFrame))
  }
  if (j.status !== 'running') return j.frameEnd
  const progressed = Math.max(j.currentFrame ?? 0, 0)
  const capped = j.frameStart + progressed - 1
  return Math.min(j.frameEnd, Math.max(j.frameStart, capped))
})

function aspectFromDimensions(width: number | null | undefined, height: number | null | undefined) {
  if (!width || !height || width <= 0 || height <= 0) return null
  return `${width} / ${height}`
}

function applyStoredPreviewAspect() {
  previewAspect.value = aspectFromDimensions(job.value?.previewWidth, job.value?.previewHeight)
}

async function preloadPreview(url: string) {
  return new Promise<{ width: number, height: number }>((resolve, reject) => {
    const img = new Image()
    img.onload = () => resolve({ width: img.naturalWidth, height: img.naturalHeight })
    img.onerror = () => reject(new Error('preview load failed'))
    img.src = url
  })
}

async function syncStoredPreviewDimensions(width: number, height: number) {
  const j = job.value
  if (!j) return
  const currentKey = j.previewWidth && j.previewHeight ? `${j.previewWidth}x${j.previewHeight}` : null
  const nextKey = `${width}x${height}`
  if (currentKey === nextKey) return

  try {
    await updateJobPreviewDimensions(j.id, width, height)
  } catch {
    // Ignore persistence failures; preview display should still work.
  }
}

async function refreshPreview() {
  const j = job.value
  if (!j || j.outputFormat === 'OPEN_EXR') {
    previewUrl.value = null
    previewFrame.value = null
    applyStoredPreviewAspect()
    return
  }
  try {
    const path = await getLastRenderedFrame(
      j.outputPath,
      j.outputFormat,
      j.frameStart,
      previewFrameEnd.value ?? j.frameEnd,
    )
    if (!path) {
      previewUrl.value = null
      previewFrame.value = null
      applyStoredPreviewAspect()
      return
    }
    const url = `${convertFileSrc(path)}?t=${Date.now()}`
    const { width, height } = await preloadPreview(url)
    previewAspect.value = `${width} / ${height}`
    previewUrl.value = url
    void syncStoredPreviewDimensions(width, height)
    // Extract frame number from filename
    const match = path.match(/(\d+)\.[^.]+$/)
    const frameToken = match?.[1]
    previewFrame.value = frameToken ? parseInt(frameToken) : null
  } catch {
    previewUrl.value = null
    previewFrame.value = null
    applyStoredPreviewAspect()
  }
}

watch(
  () => jobLogs.value.length,
  () => { if (job.value?.status === 'running') refreshPreview() },
)

watch(
  () => job.value?.status,
  (newStatus, oldStatus) => {
    if (oldStatus === 'running' && newStatus !== 'running') refreshPreview()
  },
)

watch(
  () => [job.value?.previewWidth, job.value?.previewHeight] as const,
  () => {
    if (!previewUrl.value) {
      applyStoredPreviewAspect()
    }
  },
  { immediate: true },
)

// ─────────────────────────────────────────────────────────────────────────────

const logEl = ref<HTMLDivElement | null>(null)
const pinToBottom = ref(true)

watch(
  () => logLines.value.length,
  async () => {
    if (!pinToBottom.value) return
    await nextTick()
    if (logEl.value) logEl.value.scrollTop = logEl.value.scrollHeight
  },
)

async function toggleLogScope() {
  const nextShowAll = !showAllLogs.value
  showAllLogs.value = nextShowAll
  if (!nextShowAll || allLogsLoaded.value) return

  logsLoading.value = true
  try {
    allLogLines.value = await getJobLogs(jobId.value)
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

function scheduleLogSummaryRefresh(delay = 180) {
  if (logSummaryTimer) clearTimeout(logSummaryTimer)
  logSummaryTimer = setTimeout(() => {
    void refreshLogSummary()
  }, delay)
}

async function refreshLogSummary() {
  try {
    logSummary.value = await getJobLogSummary(jobId.value)
  } catch {
    logSummary.value = null
  }
}

watch(
  () => jobLogs.value.length,
  () => {
    scheduleLogSummaryRefresh()
  },
)

watch(
  () => job.value?.status,
  () => {
    scheduleLogSummaryRefresh(40)
  },
)

function onLogScroll() {
  const el = logEl.value
  if (!el) return
  pinToBottom.value = el.scrollTop + el.clientHeight >= el.scrollHeight - 40
}

function formatTime(ms: number) {
  return new Date(ms).toLocaleString()
}

const duration = computed(() => {
  const j = job.value
  if (!j?.startedAt) return '—'
  const ms = (j.finishedAt ?? Date.now()) - j.startedAt
  const secs = Math.round(ms / 1000)
  if (secs < 60) return `${secs}s`
  const m = Math.floor(secs / 60)
  const s = secs % 60
  if (m < 60) return `${m}m ${s}s`
  return `${Math.floor(m / 60)}h ${m % 60}m ${s}s`
})

onMounted(async () => {
  await Promise.all([
    settingsStore.load(),
    jobsStore.fetchJobs(),
    transcodeStore.fetchFfmpegJobs(),
  ])
  detailUnlisteners.push(await onProgress((event) => {
    jobsStore.applyProgress(event)
  }))
  detailUnlisteners.push(await onJobUpdated((event) => {
    jobsStore.applyJobUpdate(event)
  }))
  detailUnlisteners.push(await onLog((event) => {
    jobsStore.applyLog(event)
    if (event.jobId !== jobId.value || !allLogsLoaded.value) return
    allLogLines.value = [...allLogLines.value, event.line]
  }))
  detailUnlisteners.push(await onFfmpegJobUpdated((event) => {
    transcodeStore.applyFfmpegJobUpdate(event)
  }))
  await Promise.all([
    jobsStore.loadJobLogs(jobId.value),
    refreshLogSummary(),
  ])
  await refreshPreview()
})

onUnmounted(() => {
  for (const unlisten of detailUnlisteners) {
    unlisten()
  }
  if (logSummaryTimer) clearTimeout(logSummaryTimer)
})

const showDeleteConfirm = ref(false)
const showRetryConfirm = ref(false)
const retryExistingCount = ref(0)
const retryFrameStatus = ref<RenderedFramesStatus | null>(null)
const retryActionError = ref('')
const retrySubmittingMode = ref<'restart' | 'continue' | 'range-restart' | 'range-continue' | null>(null)
const retryCustomStart = ref<number | null>(null)
const retryCustomEnd = ref<number | null>(null)
const retryCustomResumeFromExisting = ref(true)
const retryCustomFrameStatus = ref<RenderedFramesStatus | null>(null)
const retryCustomInspectLoading = ref(false)
let retryCustomInspectToken = 0

async function handleAutoTranscodeToggle(value: boolean) {
  const currentJob = job.value
  if (!currentJob || updatingAutoTranscode.value) return
  if (currentJob.outputFormat === 'OPEN_EXR' || currentJob.outputFormat === 'EXR') return

  updatingAutoTranscode.value = true
  try {
    await jobsStore.updateJobTranscodeSettings({
      id: currentJob.id,
      auto_transcode_mp4: value,
      transcode_name_override: currentJob.transcodeNameOverride,
      transcode_fps_override: currentJob.transcodeFpsOverride,
      transcode_output_path_override: currentJob.transcodeOutputPathOverride,
      transcode_crf_override: currentJob.transcodeCrfOverride,
      transcode_preset_override: currentJob.transcodePresetOverride,
    })
  } finally {
    updatingAutoTranscode.value = false
  }
}

function handleAutoTranscodeSwitchUpdate(value: boolean) {
  void handleAutoTranscodeToggle(value)
}

async function handleRetry() {
  const j = job.value
  if (!j) return
  retryActionError.value = ''
  const status = await inspectRenderedFrames(j.outputPath, j.outputFormat, j.frameStart, j.frameEnd)
    .catch(() => ({ frameCount: 0, lastFrame: null, nextFrame: j.frameStart }))
  retryExistingCount.value = status.frameCount
  retryFrameStatus.value = normalizeRetryFrameStatus(j, status)
  retryCustomStart.value = j.frameStart
  retryCustomEnd.value = j.frameEnd
  retryCustomResumeFromExisting.value = true
  retryCustomFrameStatus.value = status
  showRetryConfirm.value = true
  void refreshRetryCustomInspection()
}

function normalizeRetryFrameStatus(job: RenderJob, status: RenderedFramesStatus): RenderedFramesStatus {
  if (job.lastRenderedFrame == null) return status
  const lastFrame = Math.min(job.frameEnd, Math.max(job.frameStart, job.lastRenderedFrame))
  return {
    frameCount: status.frameCount,
    lastFrame,
    nextFrame: Math.min(job.frameEnd + 1, lastFrame + 1),
  }
}

function resetRetryConfirmState() {
  showRetryConfirm.value = false
  retryFrameStatus.value = null
  retrySubmittingMode.value = null
  retryCustomStart.value = null
  retryCustomEnd.value = null
  retryCustomFrameStatus.value = null
  retryCustomInspectLoading.value = false
}

function closeRetryConfirm() {
  if (retrySubmittingMode.value) return
  resetRetryConfirmState()
}

async function submitTranscodeForJob() {
  const currentJob = job.value
  if (!currentJob || currentJob.status === 'running' || currentJob.outputFormat === 'OPEN_EXR' || currentJob.outputFormat === 'EXR') return
  retryActionError.value = ''
  transcodeModalOpen.value = true
}

async function handleTranscodeSettingsSave(payload: {
  transcode_name_override: string | null
  transcode_fps_override: number | null
  transcode_output_path_override: string | null
  transcode_crf_override: number | null
  transcode_preset_override: string | null
}) {
  const currentJob = job.value
  if (!currentJob) return

  try {
    await jobsStore.updateJobTranscodeSettings({
      id: currentJob.id,
      auto_transcode_mp4: currentJob.autoTranscodeMp4,
      ...payload,
    })
    transcodeSettingsModalOpen.value = false
  } catch (error) {
    retryActionError.value = error instanceof Error ? error.message : String(error)
  }
}

async function handleTranscodeSubmit(payload: AddFfmpegJobPayload) {
  try {
    const ffmpegJob = await transcodeStore.submitFfmpegJob(payload)
    transcodeModalOpen.value = false
    router.push(`/transcode/${ffmpegJob.id}`)
  } catch (error) {
    retryActionError.value = error instanceof Error ? error.message : String(error)
  }
}

async function handlePrimaryTranscodeAction() {
  if (primaryTranscodeJob.value) {
    await router.push(`/transcode/${primaryTranscodeJob.value.id}`)
    return
  }
  await submitTranscodeForJob()
}

async function confirmRetryContinue() {
  if (retrySubmittingMode.value) return
  retryActionError.value = ''
  const j = job.value
  retrySubmittingMode.value = 'continue'
  try {
    if (j) await jobsStore.retryJob(j)
    resetRetryConfirmState()
  } catch (error) {
    retryActionError.value = error instanceof Error ? error.message : String(error)
  } finally {
    retrySubmittingMode.value = null
  }
}

async function confirmRetryFromStart() {
  if (retrySubmittingMode.value) return
  retryActionError.value = ''
  const j = job.value
  retrySubmittingMode.value = 'restart'
  try {
    if (j) await jobsStore.retryJobFromStart(j)
    resetRetryConfirmState()
  } catch (error) {
    retryActionError.value = error instanceof Error ? error.message : String(error)
  } finally {
    retrySubmittingMode.value = null
  }
}

const retryCustomRangeSummary = computed(() => {
  const j = job.value
  const start = retryCustomStart.value
  const end = retryCustomEnd.value
  if (!j || start == null || end == null) return '设置这次要重跑的帧区间，可用于补帧或局部返修。'
  if (start > end) return '起始帧不能大于结束帧。'
  const status = retryCustomFrameStatus.value
  if (!status) return `将渲染 ${start}–${end}。`
  if (status.frameCount === 0) return `所选区间 ${start}–${end} 内还没有已渲染帧。`
  if (retryCustomResumeFromExisting.value) {
    if (status.nextFrame <= end) {
      return `区间内已存在 ${status.frameCount} 帧，最后一帧 ${status.lastFrame ?? '—'}，将从 ${status.nextFrame} 继续。`
    }
    return `区间 ${start}–${end} 已完整存在，继续后会直接完成。`
  }
  return `区间内已存在 ${status.frameCount} 帧，关闭续跑后会从 ${start} 开始覆盖。`
})

async function refreshRetryCustomInspection() {
  const j = job.value
  const start = retryCustomStart.value
  const end = retryCustomEnd.value
  if (!j || !showRetryConfirm.value || start == null || end == null || start > end) {
    retryCustomFrameStatus.value = null
    retryCustomInspectLoading.value = false
    return
  }

  const token = ++retryCustomInspectToken
  retryCustomInspectLoading.value = true
  try {
    const status = await inspectRenderedFrames(j.outputPath, j.outputFormat, start, end)
      .catch(() => ({ frameCount: 0, lastFrame: null, nextFrame: start }))
    if (token !== retryCustomInspectToken) return
    retryCustomFrameStatus.value = status
  } finally {
    if (token === retryCustomInspectToken) {
      retryCustomInspectLoading.value = false
    }
  }
}

async function confirmRetryCustomRange() {
  if (retrySubmittingMode.value) return
  retryActionError.value = ''
  const j = job.value
  const start = retryCustomStart.value
  const end = retryCustomEnd.value
  if (!j || start == null || end == null) return
  if (start > end) {
    retryActionError.value = '起始帧不能大于结束帧。'
    return
  }
  if (start < j.frameStart || end > j.frameEnd) {
    retryActionError.value = `帧范围必须在任务范围 ${j.frameStart}–${j.frameEnd} 内。`
    return
  }

  retrySubmittingMode.value = retryCustomResumeFromExisting.value ? 'range-continue' : 'range-restart'
  try {
    if (retryCustomResumeFromExisting.value) {
      await jobsStore.retryJob(j, true, { start, end })
    } else {
      await jobsStore.retryJobFromStart(j, { start, end })
    }
    resetRetryConfirmState()
  } catch (error) {
    retryActionError.value = error instanceof Error ? error.message : String(error)
  } finally {
    retrySubmittingMode.value = null
  }
}

watch(
  () => [showRetryConfirm.value, retryCustomStart.value, retryCustomEnd.value] as const,
  ([open]) => {
    if (!open) return
    void refreshRetryCustomInspection()
  },
)

async function removeAndBack() {
  const j = job.value
  if (!j) return
  await jobsStore.deleteJob(j.id)
  router.push('/')
}
</script>
