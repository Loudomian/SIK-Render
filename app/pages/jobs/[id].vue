<template>
  <div v-if="job" class="detail-page">
    <section class="page-hero detail-hero">
      <div class="page-hero-copy detail-title">
        <UButton to="/" icon="i-lucide-arrow-left" label="返回" color="neutral" variant="outline" size="sm" class="detail-back-btn" />
        <div class="detail-title-row">
          <div class="detail-heading-stack">
            <div class="detail-heading-line">
              <UBadge
                :label="STATUS_LABEL[job.status] ?? job.status"
                :color="statusBadgeColor"
                variant="subtle"
              />
              <UBadge :label="`优先级 ${job.priority}`" color="neutral" variant="subtle" />
            </div>
            <h1><span class="job-number">#{{ job.jobNumber }}</span> {{ job.name }}</h1>
          </div>
        </div>
      </div>
      <div class="detail-header-actions">
        <UButton
          v-if="exportingMp4"
          icon="i-lucide-square"
          :label="cancelingMp4 ? '中断中…' : '中断转码'"
          :loading="cancelingMp4"
          color="warning"
          variant="outline"
          size="sm"
          @click="handleCancelExportMp4"
        />
        <UButton
          v-else
          icon="i-lucide-film"
          label="转码"
          :disabled="job.status === 'running'"
          color="neutral"
          variant="outline"
          size="sm"
          @click="openMp4Dialog"
        />
        <UButton
          v-if="exportedMp4Path"
          icon="i-lucide-external-link"
          label="打开视频"
          color="neutral"
          variant="outline"
          size="sm"
          @click="openPath(exportedMp4Path)"
        />
        <UButton
          v-if="job.status === 'failed' || job.status === 'cancelled' || job.status === 'interrupted' || job.status === 'done'"
          icon="i-lucide-rotate-ccw"
          :label="job.status === 'cancelled' || job.status === 'interrupted' ? '继续' : '重试'"
          :color="job.status === 'cancelled' || job.status === 'interrupted' ? 'warning' : 'neutral'"
          variant="outline"
          size="sm"
          @click="handleRetry"
        />
        <UButton
          v-if="job.status === 'running' || job.status === 'pending'"
          icon="i-lucide-x"
          label="取消"
          color="warning"
          variant="outline"
          size="sm"
          @click="jobsStore.stopJob(job.id)"
        />
        <UButton
          v-if="job.status === 'done' || job.status === 'failed' || job.status === 'cancelled' || job.status === 'interrupted'"
          icon="i-lucide-trash-2"
          label="删除"
          color="error"
          variant="outline"
          size="sm"
          @click="showDeleteConfirm = true"
        />
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

    <UModal
      :open="showMp4Dialog"
      :close="false"
      title="转码"
      :ui="{ content: 'job-modal-content mp4-modal-content' }"
      @update:open="v => { if (!v) closeMp4Dialog() }"
    >
      <template #body>
        <div class="modal-stack mp4-modal-stack">
          <section class="surface-panel mp4-config-section">
            <div class="form-section-header">
              <div>
                <h2 class="form-section-title">转码范围</h2>
                <p class="form-section-note">从当前任务输出目录中匹配序列帧，并导出为 H.264 MP4。</p>
              </div>
            </div>

            <div class="mp4-range-mode-grid">
              <UButton
                v-for="option in mp4RangeOptions"
                :key="option.value"
                :label="option.label"
                size="sm"
                color="neutral"
                :variant="mp4RangeMode === option.value ? 'solid' : 'outline'"
                class="mp4-range-mode-button"
                @click="mp4RangeMode = option.value"
              />
            </div>

            <div v-if="mp4RangeMode === 'custom'" class="job-form-control-grid mp4-custom-grid">
              <UFormField label="起始帧">
                <UInputNumber v-model="mp4CustomStart" :min="1" />
              </UFormField>
              <UFormField label="结束帧">
                <UInputNumber v-model="mp4CustomEnd" :min="1" />
              </UFormField>
            </div>

            <div class="job-form-stats mp4-target-grid">
              <div class="job-form-stat">
                <span class="job-form-stat-label">目标帧段</span>
                <strong>{{ formatFrameRange(mp4Inspection?.selectedStart, mp4Inspection?.selectedEnd) }}</strong>
              </div>
              <div class="job-form-stat">
                <span class="job-form-stat-label">编码格式</span>
                <strong>MP4 / H.264</strong>
              </div>
            </div>

            <div class="mp4-strict-row">
              <USwitch
                v-model="mp4AllowMissingFrames"
                color="neutral"
                label="允许跳过缺帧"
                :description="mp4AllowMissingFrames ? '开启后会跳过缺失帧，只转码当前实际存在的序列。' : '关闭后会严格检查连续性，检测到缺帧时禁止开始转码。'"
                class="mp4-strict-switch"
              />
            </div>
          </section>

          <section class="surface-panel mp4-config-section">
            <div class="form-section-header">
              <div>
                <h2 class="form-section-title">帧检查</h2>
                <p class="form-section-note">确认当前目录里能参与转码的帧数量、范围和缺帧情况。</p>
              </div>
            </div>

            <div v-if="mp4InspectionLoading" class="job-form-empty">
              <UIcon name="i-lucide-loader-circle" class="job-form-empty-icon spin" />
              <p>正在检查可转码的帧范围…</p>
            </div>

            <div v-else-if="mp4Inspection" class="mp4-inspection-stack">
              <div class="job-form-stats mp4-stats-grid">
                <div class="job-form-stat">
                  <span class="job-form-stat-label">已匹配帧</span>
                  <strong>{{ mp4Inspection.frameCount }}</strong>
                </div>
                <div class="job-form-stat">
                  <span class="job-form-stat-label">可用范围</span>
                  <strong>{{ formatFrameRange(mp4Inspection.availableStart, mp4Inspection.availableEnd) }}</strong>
                </div>
                <div class="job-form-stat">
                  <span class="job-form-stat-label">转码范围</span>
                  <strong>{{ formatFrameRange(mp4Inspection.selectedStart, mp4Inspection.selectedEnd) }}</strong>
                </div>
                <div class="job-form-stat">
                  <span class="job-form-stat-label">缺失帧</span>
                  <strong>{{ mp4Inspection.missingCount }}</strong>
                </div>
              </div>

              <div v-if="mp4Inspection.hasGaps" class="mp4-missing-stack">
                <span class="job-form-stat-label">缺失片段</span>
                <p class="mp4-missing-list">
                  {{ mp4Inspection.missingSegments.join('，') }}
                  <template v-if="mp4Inspection.missingSegmentsTruncated"> 等更多</template>
                </p>
              </div>

              <p v-else class="hint-text">当前帧段连续，可以直接开始转码。</p>
            </div>

            <div v-else class="job-form-empty">
              <UIcon name="i-lucide-film" class="job-form-empty-icon" />
              <p>当前还没有可用的帧检查结果。</p>
            </div>
          </section>

          <div class="mp4-config-grid">
            <UAlert
              v-if="mp4Inspection?.hasGaps"
              color="warning"
              variant="subtle"
              :description="mp4StrictContiguous ? '当前启用了严格连续帧，必须先补齐缺失帧后才能开始转码。' : '当前会跳过缺失帧，只使用实际存在的图像序列进行转码。'"
            />
            <UAlert
              v-if="mp4Inspection && !mp4Inspection.frameCount"
              color="warning"
              variant="subtle"
              description="当前选择的范围内没有找到可用于转码的序列帧。"
            />
          </div>

          <p v-if="mp4ActionError" class="form-error">{{ mp4ActionError }}</p>

          <div class="modal-actions">
            <UButton icon="i-lucide-x" label="取消" color="warning" variant="outline" @click="closeMp4Dialog" />
            <UButton
              label="开始转码"
              color="neutral"
              variant="solid"
              :loading="exportingMp4"
              :disabled="!canStartMp4Export"
              @click="confirmExportMp4"
            />
          </div>
        </div>
      </template>
    </UModal>

    <RenderProgress
      v-if="job.status === 'running'"
      class="detail-progress"
      :frame="job.currentFrame ?? 0"
      :total-frames="job.totalFrames ?? (job.frameEnd - job.frameStart + 1)"
      :warming-up="jobsStore.isJobWarmingUp(job.id)"
      :time-elapsed="job.timeElapsed"
      :remaining-secs="job.remainingSecs"
    />

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
        <div class="detail-meta-grid">
          <section class="detail-meta-column">
            <h2 class="detail-card-title">渲染设置</h2>
            <dl class="detail-spec-list">
              <dt>格式</dt>
              <dd>{{ job.outputFormat }}</dd>
              <dt>帧范围</dt>
              <dd>{{ job.frameStart }} – {{ job.frameEnd }}（共 {{ job.frameEnd - job.frameStart + 1 }} 帧）</dd>
              <dt>Blender</dt>
              <dd>{{ blenderVersion }}</dd>
              <template v-if="crashCount">
                <dt>崩溃恢复</dt>
                <dd>{{ crashCount }} 次</dd>
              </template>
            </dl>
          </section>

          <section class="detail-meta-column">
            <h2 class="detail-card-title">任务时间</h2>
            <dl class="detail-spec-list">
              <dt>开始</dt>
              <dd>{{ formatTime(job.startedAt ?? job.createdAt) }}</dd>
              <dt>完成</dt>
              <dd>{{ job.finishedAt ? formatTime(job.finishedAt) : '—' }}</dd>
              <dt>耗时</dt>
              <dd>{{ duration }}</dd>
            </dl>
          </section>
        </div>
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
    </div>

    <UModal v-model:open="lightboxOpen" :close="false" :ui="{ content: 'preview-lightbox' }">
      <template #body>
        <div @click="lightboxOpen = false">
          <div @click.stop>
          <img :src="previewUrl!" class="preview-lightbox-img" alt="frame preview" />
          </div>
        </div>
      </template>
    </UModal>

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

    <UCard variant="subtle" :ui="{ root: 'detail-section log-section', body: 'detail-card-body' }">
      <div class="log-header">
        <h2 class="detail-card-title log-title">输出日志</h2>
        <div class="log-header-actions">
          <UButton
            :label="`当前 ${activeLogLineCount} 行`"
            color="neutral"
            variant="subtle"
            size="sm"
            class="log-header-chip log-header-stat"
          />
          <UButton
            :label="`Blender ${logSummary?.blenderCount ?? 0} 份`"
            color="neutral"
            variant="subtle"
            size="sm"
            class="log-header-chip log-header-stat"
          />
          <UButton
            :label="`FFMPEG ${logSummary?.ffmpegCount ?? 0} 份`"
            color="neutral"
            variant="subtle"
            size="sm"
            class="log-header-chip log-header-stat"
          />
          <UButton
            v-if="logSummary?.directory"
            icon="i-lucide-folder-open"
            label="打开日志目录"
            color="neutral"
            variant="subtle"
            size="sm"
            class="log-header-chip log-header-button"
            @click="openPath(logSummary.directory)"
          />
        </div>
      </div>
      <UTabs
        v-model="activeLogTab"
        :items="logTabs"
        :content="false"
        color="neutral"
        variant="link"
        size="md"
        class="log-tabs"
      />
      <div v-if="activeLogTab === 'blender'" ref="logEl" class="log-panel" @scroll="onLogScroll">
        <span v-if="jobLogs.length === 0" class="log-empty">
          {{ job.status === 'pending' ? '等待开始…' : '暂无输出内容。' }}
        </span>
        <div v-for="(line, i) in jobLogs" :key="i" class="log-line">{{ line }}</div>
      </div>
      <div v-else ref="mp4LogEl" class="log-panel" @scroll="onMp4LogScroll">
        <span v-if="mp4Logs.length === 0" class="log-empty">
          {{ exportingMp4 ? 'ffmpeg 已启动，等待日志…' : '尚未开始转码。' }}
        </span>
        <div v-for="(line, i) in mp4Logs" :key="`mp4-${i}`" class="log-line">{{ line }}</div>
      </div>
    </UCard>
  </div>

  <div v-else class="empty">找不到该任务。</div>
</template>

<script setup lang="ts">
import { convertFileSrc } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { JobLogSummary, Mp4ExportInspection, Mp4LogEvent, Mp4RangeMode, RenderJob, RenderedFramesStatus } from '~/types'

const route = useRoute()
const router = useRouter()
const jobsStore = useJobsStore()

const settingsStore = useSettingsStore()

const { openPath, inspectRenderedFrames, inspectMp4Export, getLastRenderedFrame, encodeSequenceToMp4, cancelMp4Export, getJobLatestMp4Logs, getJobLogSummary, updateJobPreviewDimensions } = useTauri()

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

const jobId = computed(() => route.params.id as string)
const job = computed(() => jobsStore.jobs.find((j) => j.id === jobId.value))
const jobLogs = computed(() => jobsStore.getJobLogs(jobId.value))
const statusBadgeColor = computed(() => STATUS_COLOR[job.value?.status ?? 'pending'] ?? 'neutral')
const blenderVersion = computed(() => {
  const exe = job.value?.blenderExecutable
  if (!exe) return '—'
  const match = settingsStore.blenderVersions.find((b) => b.executable === exe)
  return match ? `Blender ${match.version}` : exe
})

const exportingMp4 = ref(false)
const cancelingMp4 = ref(false)
const exportedMp4Path = ref('')
const showMp4Dialog = ref(false)
const mp4RangeMode = ref<Mp4RangeMode>('job')
const mp4CustomStart = ref<number | null>(null)
const mp4CustomEnd = ref<number | null>(null)
const mp4StrictContiguous = ref(false)
const mp4Inspection = ref<Mp4ExportInspection | null>(null)
const mp4InspectionLoading = ref(false)
const mp4ActionError = ref('')
const mp4Logs = ref<string[]>([])
const logSummary = ref<JobLogSummary | null>(null)
const mp4LogEl = ref<HTMLDivElement | null>(null)
const pinMp4LogToBottom = ref(true)
let unlistenMp4Log: UnlistenFn | null = null
let mp4InspectToken = 0
let logSummaryTimer: ReturnType<typeof setTimeout> | null = null
const activeLogTab = ref<'blender' | 'ffmpeg'>('blender')

const LOG_WARNINGS: Array<{ pattern: RegExp; message: string }> = [
  {
    pattern: /Shadow buffer full/i,
    message: '阴影缓冲区已满（Shadow buffer full）：部分阴影可能缺失，光照结果不准确，建议在渲染属性中增大阴影贴图分辨率或减少灯光数量。',
  },
]

const crashCount = computed(() =>
  jobLogs.value.filter(line => line.includes('[crash-recovery]')).length
)

const logTabs = computed(() => [
  { label: 'Blender', value: 'blender' },
  { label: 'FFMPEG', value: 'ffmpeg' },
])

const activeLogLineCount = computed(() =>
  activeLogTab.value === 'ffmpeg' ? mp4Logs.value.length : jobLogs.value.length
)

const mp4RangeOptions: Array<{ label: string, value: Mp4RangeMode }> = [
  { label: '当前任务帧段', value: 'job' },
  { label: '输出目录全部匹配帧', value: 'all' },
  { label: '自定义帧段', value: 'custom' },
]

const canStartMp4Export = computed(() => {
  if (exportingMp4.value || mp4InspectionLoading.value) return false
  if (!mp4Inspection.value?.frameCount) return false
  if (mp4StrictContiguous.value && mp4Inspection.value.hasGaps) return false
  if (mp4RangeMode.value === 'custom' && (mp4CustomStart.value == null || mp4CustomEnd.value == null)) return false
  return true
})

const mp4AllowMissingFrames = computed({
  get: () => !mp4StrictContiguous.value,
  set: (value: boolean) => {
    mp4StrictContiguous.value = !value
  },
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
    previewFrame.value = match ? parseInt(match[1]) : null
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

watch(
  () => [showMp4Dialog.value, mp4RangeMode.value, mp4CustomStart.value, mp4CustomEnd.value] as const,
  () => {
    if (!showMp4Dialog.value) return
    void refreshMp4Inspection()
  },
)

// ─────────────────────────────────────────────────────────────────────────────

const logEl = ref<HTMLDivElement | null>(null)
const pinToBottom = ref(true)

watch(
  () => jobLogs.value.length,
  async () => {
    if (!pinToBottom.value) return
    await nextTick()
    if (logEl.value) logEl.value.scrollTop = logEl.value.scrollHeight
  },
)

watch(
  () => mp4Logs.value.length,
  async () => {
    if (!pinMp4LogToBottom.value) return
    await nextTick()
    if (mp4LogEl.value) mp4LogEl.value.scrollTop = mp4LogEl.value.scrollHeight
  },
)

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
  () => [jobLogs.value.length, mp4Logs.value.length] as const,
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

watch(
  activeLogTab,
  async (tab) => {
    await nextTick()
    if (tab === 'ffmpeg' && mp4LogEl.value) {
      mp4LogEl.value.scrollTop = mp4LogEl.value.scrollHeight
    }
    if (tab === 'blender' && logEl.value) {
      logEl.value.scrollTop = logEl.value.scrollHeight
    }
  },
)

function onLogScroll() {
  const el = logEl.value
  if (!el) return
  pinToBottom.value = el.scrollTop + el.clientHeight >= el.scrollHeight - 40
}

function onMp4LogScroll() {
  const el = mp4LogEl.value
  if (!el) return
  pinMp4LogToBottom.value = el.scrollTop + el.clientHeight >= el.scrollHeight - 40
}

function formatTime(ms: number) {
  return new Date(ms).toLocaleString()
}

function formatFrameRange(start: number | null | undefined, end: number | null | undefined) {
  if (start == null || end == null) return '—'
  return `${start}–${end}`
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
  if (!settingsStore.blenderVersions.length) settingsStore.load()
  unlistenMp4Log = await listen<Mp4LogEvent>('mp4-log', (event) => {
    if (event.payload.jobId !== jobId.value) return
    mp4Logs.value.push(event.payload.line)
  })
  await Promise.all([
    jobsStore.loadJobLogs(jobId.value),
    getJobLatestMp4Logs(jobId.value).then((lines) => {
      if (lines.length) mp4Logs.value = lines
    }),
    refreshLogSummary(),
  ])
  await refreshPreview()
})

onUnmounted(() => {
  unlistenMp4Log?.()
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

function openMp4Dialog() {
  const j = job.value
  if (!j || exportingMp4.value) return
  mp4ActionError.value = ''
  mp4RangeMode.value = 'job'
  mp4CustomStart.value = j.frameStart
  mp4CustomEnd.value = j.frameEnd
  mp4StrictContiguous.value = false
  showMp4Dialog.value = true
  void refreshMp4Inspection()
}

function closeMp4Dialog() {
  showMp4Dialog.value = false
  mp4ActionError.value = ''
}

async function refreshMp4Inspection() {
  const j = job.value
  if (!j || !showMp4Dialog.value) return

  const token = ++mp4InspectToken
  mp4InspectionLoading.value = true
  mp4ActionError.value = ''

  try {
    const result = await inspectMp4Export(
      j.outputPath,
      j.outputFormat,
      j.frameStart,
      j.frameEnd,
      mp4RangeMode.value,
      mp4RangeMode.value === 'custom' ? mp4CustomStart.value : null,
      mp4RangeMode.value === 'custom' ? mp4CustomEnd.value : null,
    )
    if (token !== mp4InspectToken) return
    mp4Inspection.value = result
  } catch (error) {
    if (token !== mp4InspectToken) return
    mp4Inspection.value = null
    mp4ActionError.value = error instanceof Error ? error.message : String(error)
  } finally {
    if (token === mp4InspectToken) {
      mp4InspectionLoading.value = false
    }
  }
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

async function confirmExportMp4() {
  const j = job.value
  const inspection = mp4Inspection.value
  if (!j || exportingMp4.value || !inspection?.selectedStart || !inspection.selectedEnd) return

  mp4ActionError.value = ''
  exportingMp4.value = true
  cancelingMp4.value = false
  exportedMp4Path.value = ''
  mp4Logs.value = []
  pinMp4LogToBottom.value = true
  activeLogTab.value = 'ffmpeg'
  closeMp4Dialog()

  try {
    const result = await encodeSequenceToMp4(
      j.id,
      j.blenderExecutable,
      j.blendFile,
      j.outputPath,
      j.outputFormat,
      inspection.selectedStart,
      inspection.selectedEnd,
      mp4StrictContiguous.value,
    )
    exportedMp4Path.value = result.outputPath
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    if (message !== 'MP4 export cancelled' && !mp4Logs.value.some(line => line.includes(message))) {
      mp4Logs.value.push(`[ffmpeg] ${message}`)
    }
    mp4ActionError.value = message
  } finally {
    exportingMp4.value = false
    cancelingMp4.value = false
  }
}

async function handleCancelExportMp4() {
  const j = job.value
  if (!j || !exportingMp4.value || cancelingMp4.value) return

  cancelingMp4.value = true
  try {
    await cancelMp4Export(j.id)
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    if (!mp4Logs.value.some(line => line.includes(message))) {
      mp4Logs.value.push(`[ffmpeg] ${message}`)
    }
    cancelingMp4.value = false
  }
}
</script>
