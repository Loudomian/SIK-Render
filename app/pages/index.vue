<template>
  <div class="queue-page">
    <Transition name="drop-fade">
      <div v-if="isDragging" class="drop-overlay">
        <div class="drop-message">
          <UIcon name="i-lucide-download" class="drop-icon" />
          <div class="drop-copy">
            <strong>松开以创建渲染任务</strong>
            <span>自动读取 .blend，并补全基础渲染参数</span>
          </div>
        </div>
      </div>
    </Transition>

    <section class="page-hero">
      <div class="page-hero-copy">
        <UBadge label="Render Queue" color="primary" variant="subtle" class="page-eyebrow" />
        <h1>渲染队列</h1>
        <p class="page-note">任务按优先级顺序依次执行，状态变化、日志和帧进度会实时同步。</p>
      </div>
      <div class="page-hero-actions">
        <UButton icon="i-lucide-plus" label="新建任务" color="success" variant="solid" @click="openAddJob" />
      </div>
    </section>

    <section class="queue-summary-section">
      <div class="queue-summary">
        <div v-for="item in queueStats" :key="item.label" class="surface-panel queue-summary-item">
          <span class="queue-summary-label">{{ item.label }}</span>
          <strong class="queue-summary-value">{{ item.value }}</strong>
        </div>
      </div>
    </section>

    <div v-if="jobsStore.loading" class="loading">加载中…</div>

    <UAlert v-if="retryActionError" color="error" variant="subtle" :description="retryActionError" class="surface-alert" />

    <UCard v-else-if="jobsStore.jobs.length === 0" variant="subtle" class="empty-state" :ui="{ body: 'empty-state-body' }">
      <div class="empty-state-icon">
        <UIcon name="i-lucide-film" />
      </div>
      <div class="empty-state-title">还没有渲染任务</div>
      <div class="empty-state-note">拖拽 .blend 工程到窗口，或点击"新建任务"开始。</div>
      <div class="empty-state-actions">
        <UButton icon="i-lucide-plus" label="新建任务" color="success" variant="solid" @click="openAddJob" />
        <UButton
          v-if="showInitializeTools"
          icon="i-lucide-scan-search"
          label="初始化工具"
          color="neutral"
          variant="outline"
          :loading="initializingTools"
          @click="initializeTools"
        />
      </div>
    </UCard>

    <TransitionGroup v-else name="job-list" tag="div" class="job-list">
      <JobCard
        v-for="job in jobsStore.jobs"
        :key="job.id"
        :job="job"
        @cancel="jobsStore.stopJob(job.id)"
        @remove="deleteConfirmJob = job"
        @retry="handleRetry(job)"
      />
    </TransitionGroup>

    <UModal
      :open="!!retryConfirmJob"
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
                从第 <span class="choice-card-accent">{{ retryConfirmJob?.frameStart }}</span> 帧开始渲染，直接覆盖
                <span class="choice-card-accent">{{ retryConfirmJob?.frameStart }}–{{ retryConfirmJob?.frameEnd }}</span>
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
                <template v-if="retryConfirmJob && retryFrameStatus && retryFrameStatus.nextFrame <= retryConfirmJob.frameEnd">
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
      :open="!!deleteConfirmJob"
      :close="false"
      title="删除任务"
      :ui="{ content: 'job-modal-content' }"
      @update:open="v => { if (!v) deleteConfirmJob = null }"
    >
      <template #body>
        <div class="modal-stack">
          <p class="modal-copy">
            确定删除 <strong>{{ deleteConfirmJob?.name }}</strong>？此操作不可撤销。
          </p>
          <div class="modal-actions">
            <UButton icon="i-lucide-x" label="取消" color="warning" variant="outline" @click="deleteConfirmJob = null" />
            <UButton icon="i-lucide-trash-2" label="删除" color="error" variant="outline" @click="confirmDelete" />
          </div>
        </div>
      </template>
    </UModal>

    <UModal
      :open="showAddJob"
      title="新建渲染任务"
      :dismissible="false"
      :content="{ class: 'job-modal-content job-form-modal' }"
      @update:open="v => { if (!v) closeAddJob() }"
    >
      <template #body>
        <form class="job-form" @submit.prevent="submitNewJob">
          <div class="job-form-layout">
            <div class="job-form-main">
              <section class="job-form-block surface-panel">
                <div class="form-section-header">
                  <div>
                    <h2 class="form-section-title">项目</h2>
                    <p class="form-section-note">先确定任务名称和 Blender 工程路径。</p>
                  </div>
                </div>

                <div class="job-form-stack">
                  <UFormField label="任务名称" size="lg" class="job-form-field">
                    <UTextarea v-model.trim="form.name" :rows="1" autoresize class="job-name-textarea" placeholder="Shot_010_Lighting" :ui="{ root: 'w-full' }" />
                  </UFormField>

                  <UFormField label="Blend 文件" size="lg" class="job-form-field">
                    <UTextarea v-model.trim="form.blend_file" :rows="2" autoresize class="path-textarea path-textarea-lg" placeholder="F:\projects\scene.blend" :ui="{ root: 'w-full' }" />
                  </UFormField>
                </div>
              </section>

              <section class="job-form-block surface-panel">
                <div class="form-section-header">
                  <div>
                    <h2 class="form-section-title">输出</h2>
                    <p class="form-section-note">输出模板完整展开显示，方便直接检查目录和命名规则，建议使用包含 ###### 的路径模板，方便 Blender 输出序列帧。</p>
                  </div>
                </div>

                <div class="job-form-stack">
                  <UFormField label="输出路径" size="lg" class="job-form-field">
                    <UTextarea v-model.trim="form.output_path" :rows="2" autoresize class="path-textarea path-textarea-xl" placeholder="E:\renders\scene\scene_######" :ui="{ root: 'w-full' }" />
                  </UFormField>
                </div>
              </section>
            </div>

            <aside class="job-form-sidebar surface-panel">
              <section class="job-form-sidebar-section">
                <div class="form-section-header">
                  <div>
                    <h2 class="form-section-title">渲染控制</h2>
                    <p class="form-section-note">先选择 Blender 版本，再确认优先级、输出格式和渲染帧段。</p>
                  </div>
                </div>

                <div v-if="settingsStore.blenderVersions.length" class="version-stack">
                  <div class="chip-row version-chip-row">
                    <UButton
                      v-for="b in settingsStore.blenderVersions"
                      :key="b.executable"
                      type="button"
                      size="sm"
                      :color="form.blender_executable === b.executable ? 'primary' : 'neutral'"
                      :variant="form.blender_executable === b.executable ? 'solid' : 'outline'"
                      :label="b.version"
                      @click="selectBlender(b.executable)"
                    />
                  </div>

                  <div class="job-form-control-grid job-form-control-grid-meta">
                    <UFormField label="优先级">
                      <UInputNumber v-model="form.priority" :min="0" :max="99" :ui="{ root: 'w-full' }" />
                    </UFormField>
                    <UFormField label="格式">
                      <USelect v-model="form.output_format" :items="outputFormatOptions" :ui="{ base: 'w-full' }" />
                    </UFormField>
                  </div>
                  <div class="job-form-control-grid job-form-control-grid-frames">
                    <UFormField label="起始帧">
                      <UInputNumber v-model="form.frame_start" :min="1" :ui="{ root: 'w-full' }" />
                    </UFormField>
                    <UFormField label="结束帧">
                      <UInputNumber v-model="form.frame_end" :min="1" :ui="{ root: 'w-full' }" />
                    </UFormField>
                  </div>
                </div>
                <p v-else class="hint-text">
                  未检测到 Blender，请前往
                  <UButton type="button" color="neutral" variant="link" size="sm" label="设置" @click="goToSettings" />
                  添加。
                </p>

                <div class="form-inline-actions">
                  <UButton
                    type="button"
                    color="neutral"
                    variant="outline"
                    size="sm"
                    icon="i-lucide-search"
                    :loading="inspectingProject"
                    :disabled="!canInspectProject"
                    :label="inspectingProject ? '读取中…' : '读取工程参数'"
                    @click="inspectProjectSettings(true)"
                  />
                </div>
              </section>

              <section class="job-form-sidebar-section">
                <div class="form-section-header">
                  <div>
                    <h2 class="form-section-title">工程参数</h2>
                    <p class="form-section-note">读取后会同步当前工程里的核心渲染信息。</p>
                  </div>
                </div>

                <UAlert v-if="projectSettingsMessage" color="neutral" variant="subtle" :description="projectSettingsMessage" />

                <div v-if="projectSettings" class="job-form-stats">
                  <div class="job-form-stat">
                    <span class="job-form-stat-label">渲染引擎</span>
                    <strong>{{ displayEngine(projectSettings.engine) }}</strong>
                  </div>
                  <div class="job-form-stat">
                    <span class="job-form-stat-label">工程帧段</span>
                    <strong>{{ projectSettings.frameStart }}–{{ projectSettings.frameEnd }}</strong>
                  </div>
                  <div class="job-form-stat">
                    <span class="job-form-stat-label">分辨率</span>
                    <strong>{{ projectSettings.resolutionX }}×{{ projectSettings.resolutionY }}</strong>
                  </div>
                  <div class="job-form-stat">
                    <span class="job-form-stat-label">FPS</span>
                    <strong>{{ projectSettings.fps.toFixed(1) }}</strong>
                  </div>
                </div>
                <div v-else class="job-form-empty">
                  <UIcon name="i-lucide-scan-search" class="job-form-empty-icon" />
                  <p>点击"读取工程参数"后，这里会显示 Blender 工程内的渲染信息。</p>
                </div>
              </section>

              <section v-if="notices.length" class="job-form-sidebar-section">
                <div class="form-section-header">
                  <div>
                    <h2 class="form-section-title">提示</h2>
                  </div>
                </div>

                <div class="notices-area notices-stack">
                  <UAlert
                    v-for="(n, i) in notices"
                    :key="i"
                    :color="n.type === 'warn' ? 'warning' : 'neutral'"
                    variant="subtle"
                    :description="n.text"
                  />
                </div>
              </section>
            </aside>
          </div>

          <UAlert v-if="formError" color="error" variant="subtle" :description="formError" />

          <div class="modal-actions">
            <UButton type="button" icon="i-lucide-x" label="取消" color="warning" variant="outline" @click="closeAddJob" />
            <UButton type="submit" color="neutral" variant="solid" :loading="submitting" :label="submitting ? '提交中…' : '加入渲染队列'" />
          </div>
        </form>
      </template>
    </UModal>

    <UModal
      :open="showAddJobConfirm"
      :close="false"
      title="选择渲染方式"
      :ui="{ content: 'job-modal-content' }"
      @update:open="v => { if (!v) closeAddJobConfirm() }"
    >
      <template #body>
        <div class="modal-stack">
          <p class="modal-copy">
            检测到当前输出范围已存在
            <strong>{{ addJobFrameStatus?.frameCount ?? 0 }} 帧</strong>
            <template v-if="addJobFrameStatus?.lastFrame != null">
              ，当前最后一帧为 <strong>{{ addJobFrameStatus.lastFrame }}</strong>
            </template>
            。
          </p>
          <div class="choice-grid">
            <UCard variant="subtle" class="choice-card" :ui="{ body: 'choice-card-body' }">
              <div class="choice-card-head">
                <p class="choice-card-mode">覆盖模式</p>
                <h3 class="choice-card-title">重新开始渲染</h3>
              </div>
              <p class="choice-card-desc">
                从第 <span class="choice-card-accent">{{ form.frame_start }}</span> 帧开始渲染，直接覆盖
                <span class="choice-card-accent">{{ form.frame_start }}–{{ form.frame_end }}</span>
                范围内的同名帧
              </p>
              <UButton
                color="neutral"
                variant="outline"
                label="从头覆盖"
                class="choice-card-action"
                :loading="submitting && addJobSubmitMode === 'restart'"
                :disabled="submitting"
                @click="submitPreparedJob(false)"
              />
            </UCard>

            <UCard variant="subtle" class="choice-card" :ui="{ body: 'choice-card-body' }">
              <div class="choice-card-head">
                <p class="choice-card-mode">续跑模式</p>
                <h3 class="choice-card-title">从最后一帧继续</h3>
              </div>
              <p class="choice-card-desc">
                <template v-if="addJobFrameStatus && addJobFrameStatus.nextFrame <= form.frame_end">
                  从第 <span class="choice-card-accent">{{ addJobFrameStatus.nextFrame }}</span> 帧继续渲染
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
                :loading="submitting && addJobSubmitMode === 'continue'"
                :disabled="submitting"
                @click="submitPreparedJob(true)"
              />
            </UCard>
          </div>
          <div class="modal-actions">
            <UButton icon="i-lucide-x" label="取消" color="warning" variant="outline" :disabled="submitting" @click="closeAddJobConfirm" />
          </div>
        </div>
      </template>
    </UModal>
  </div>
</template>

<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useRouter } from 'vue-router'
import type { AddJobPayload, BlendProjectSettings, RenderJob, RenderedFramesStatus } from '~/types'

const router = useRouter()
const jobsStore = useJobsStore()
const settingsStore = useSettingsStore()
const toast = useToast()

const initializingTools = ref(false)

function formatToolSource(source: string | null | undefined) {
  switch (source) {
    case 'bundled resource':
      return '打包资源'
    case 'workspace bin':
      return '内置工具'
    case 'settings':
      return '设置指定'
    case 'blender directory':
      return 'Blender 目录'
    case 'blender parent directory':
      return 'Blender 上级目录'
    case 'system PATH':
      return '系统 PATH'
    default:
      return source ?? ''
  }
}

function shortenPath(path: string | null | undefined) {
  if (!path) return ''
  const normalized = path.replace(/^\\\\\?\\/, '')
  const parts = normalized.split(/[\\/]/).filter(Boolean)
  if (parts.length <= 3 || normalized.length <= 48) {
    return normalized
  }
  return `${parts.slice(0, 2).join('\\')}\\...\\${parts.slice(-2).join('\\')}`
}

async function initializeTools(options?: { silent?: boolean }) {
  const silent = options?.silent ?? false
  if (initializingTools.value) return
  initializingTools.value = true
  try {
    const toolchain = await inspectToolchain()
    await settingsStore.refreshBlenderVersions()

    let ffmpegAutoFilled = false
    if (
      !settingsStore.settings.ffmpegExecutable
      && toolchain.ffmpegFound
      && toolchain.ffmpegExecutable
    ) {
      settingsStore.settings.ffmpegExecutable = toolchain.ffmpegExecutable
      await settingsStore.save()
      ffmpegAutoFilled = true
    }

    const blenderCount = toolchain.blenderInstalls.length
    const blenderVersions = toolchain.blenderInstalls.map((b) => b.version).join('、')
    const ffmpegSource = formatToolSource(toolchain.ffmpegSource)
    const ffmpegPath = shortenPath(toolchain.ffmpegExecutable)
    const ffmpegLabel = toolchain.ffmpegFound
      ? [
          ffmpegPath ? `FFmpeg：${ffmpegPath}` : 'FFmpeg 已找到',
          ffmpegSource ? `来源：${ffmpegSource}` : '',
        ].filter(Boolean).join('，')
      : 'FFmpeg 未找到，请前往设置页指定路径或检查打包资源 / PATH。'

    if (silent) {
      return
    }

    if (blenderCount > 0 && toolchain.ffmpegFound) {
      toast.add({
        title: `工具已就绪：Blender ${blenderCount} 个，FFmpeg 已找到`,
        description: `${blenderVersions}；${ffmpegLabel}${ffmpegAutoFilled ? '；已自动写入 FFmpeg 路径' : ''}`,
        color: 'success',
      })
    } else if (blenderCount > 0) {
      toast.add({
        title: `找到 ${blenderCount} 个 Blender，未找到 FFmpeg`,
        description: `${blenderVersions}；${ffmpegLabel}`,
        color: 'warning',
      })
    } else if (toolchain.ffmpegFound) {
      toast.add({
        title: '已找到 FFmpeg，未找到 Blender',
        description: `${ffmpegLabel}${ffmpegAutoFilled ? '；已自动写入 FFmpeg 路径' : ''}；Blender 未检测到安装，请前往设置页手动添加路径。`,
        color: 'warning',
      })
    } else {
      toast.add({
        title: '未找到 Blender 和 FFmpeg',
        description: 'Blender 未检测到安装，FFmpeg 也不可用，请前往设置页补充路径。',
        color: 'warning',
      })
    }
  } catch {
    if (!silent) {
      toast.add({ title: '扫描失败', description: '初始化工具时出错，请检查设置。', color: 'error' })
    }
  } finally {
    initializingTools.value = false
  }
}

const showInitializeTools = computed(() => {
  const hasBlender = Boolean(settingsStore.settings.defaultBlender || settingsStore.blenderVersions[0]?.executable)
  const hasFfmpeg = Boolean(settingsStore.settings.ffmpegExecutable)
  return !(hasBlender && hasFfmpeg)
})

const { validateBlendFile, inspectBlendFile, inspectRenderedFrames, inspectToolchain } = useTauri()

const retryConfirmJob = ref<RenderJob | null>(null)
const retryExistingCount = ref(0)
const retryFrameStatus = ref<RenderedFramesStatus | null>(null)
const deleteConfirmJob = ref<RenderJob | null>(null)
const retryActionError = ref('')
const retrySubmittingMode = ref<'restart' | 'continue' | 'range-restart' | 'range-continue' | null>(null)
const retryCustomStart = ref<number | null>(null)
const retryCustomEnd = ref<number | null>(null)
const retryCustomResumeFromExisting = ref(true)
const retryCustomFrameStatus = ref<RenderedFramesStatus | null>(null)
const retryCustomInspectLoading = ref(false)
let retryCustomInspectToken = 0

function confirmDelete() {
  if (deleteConfirmJob.value) jobsStore.deleteJob(deleteConfirmJob.value.id)
  deleteConfirmJob.value = null
}

async function handleRetry(job: RenderJob) {
  retryActionError.value = ''
  const status = await inspectRenderedFrames(job.outputPath, job.outputFormat, job.frameStart, job.frameEnd)
    .catch(() => ({ frameCount: 0, lastFrame: null, nextFrame: job.frameStart }))
  retryExistingCount.value = status.frameCount
  retryFrameStatus.value = normalizeRetryFrameStatus(job, status)
  retryConfirmJob.value = job
  retryCustomStart.value = job.frameStart
  retryCustomEnd.value = job.frameEnd
  retryCustomResumeFromExisting.value = true
  retryCustomFrameStatus.value = status
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
  retryConfirmJob.value = null
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
  retrySubmittingMode.value = 'continue'
  try {
    if (retryConfirmJob.value) await jobsStore.retryJob(retryConfirmJob.value)
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
  retrySubmittingMode.value = 'restart'
  try {
    if (retryConfirmJob.value) await jobsStore.retryJobFromStart(retryConfirmJob.value)
    resetRetryConfirmState()
  } catch (error) {
    retryActionError.value = error instanceof Error ? error.message : String(error)
  } finally {
    retrySubmittingMode.value = null
  }
}

const retryCustomRangeSummary = computed(() => {
  const job = retryConfirmJob.value
  const start = retryCustomStart.value
  const end = retryCustomEnd.value
  if (!job || start == null || end == null) return '设置这次要重跑的帧区间，可用于补帧或局部返修。'
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
  const job = retryConfirmJob.value
  const start = retryCustomStart.value
  const end = retryCustomEnd.value
  if (!job || start == null || end == null || start > end) {
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

async function confirmRetryCustomRange() {
  if (retrySubmittingMode.value) return
  retryActionError.value = ''
  const job = retryConfirmJob.value
  const start = retryCustomStart.value
  const end = retryCustomEnd.value
  if (!job || start == null || end == null) return
  if (start > end) {
    retryActionError.value = '起始帧不能大于结束帧。'
    return
  }

  retrySubmittingMode.value = retryCustomResumeFromExisting.value ? 'range-continue' : 'range-restart'
  try {
    if (retryCustomResumeFromExisting.value) {
      await jobsStore.retryJob(job, true, { start, end })
    } else {
      await jobsStore.retryJobFromStart(job, { start, end })
    }
    resetRetryConfirmState()
  } catch (error) {
    retryActionError.value = error instanceof Error ? error.message : String(error)
  } finally {
    retrySubmittingMode.value = null
  }
}

watch(
  [retryConfirmJob, retryCustomStart, retryCustomEnd],
  () => {
    if (!retryConfirmJob.value) return
    void refreshRetryCustomInspection()
  },
)

const isDragging = ref(false)
let unlistenDrop: (() => void) | null = null

onMounted(async () => {
  await settingsStore.load()
  if (showInitializeTools.value) {
    await initializeTools({ silent: true })
  }
  unlistenDrop = await getCurrentWindow().onDragDropEvent((event) => {
    if (event.payload.type === 'enter' || event.payload.type === 'over') {
      isDragging.value = true
    } else if (event.payload.type === 'leave') {
      isDragging.value = false
    } else if (event.payload.type === 'drop') {
      isDragging.value = false
      const blendPath = (event.payload.paths as string[]).find(p => p.toLowerCase().endsWith('.blend'))
      if (blendPath) openAddJobWithFile(blendPath).catch(console.error)
    }
  })
})

const showAddJob = ref(false)
const showAddJobConfirm = ref(false)
const submitting = ref(false)
const addJobSubmitMode = ref<'restart' | 'continue' | null>(null)
const inspectingProject = ref(false)
const formError = ref('')
const projectSettings = ref<BlendProjectSettings | null>(null)
const projectSettingsMessage = ref('')
const outputFrameStatus = ref<RenderedFramesStatus | null>(null)
const addJobFrameStatus = ref<RenderedFramesStatus | null>(null)
let inspectTimer: ReturnType<typeof setTimeout> | null = null
let outputDirTimer: ReturnType<typeof setTimeout> | null = null


const ENGINE_NAMES: Record<string, string> = {
  BLENDER_EEVEE: 'EEVEE',
  BLENDER_EEVEE_NEXT: 'EEVEE Next',
  CYCLES: 'Cycles',
  BLENDER_WORKBENCH: 'Workbench',
}
function displayEngine(engine: string) {
  return ENGINE_NAMES[engine] ?? engine
}

const SEQUENCE_FORMATS = ['PNG', 'JPEG', 'OPEN_EXR']
const outputFormatOptions = ['PNG', 'JPEG', 'OPEN_EXR']
const queueStats = computed(() => [
  { label: '总任务', value: jobsStore.jobs.length },
  { label: '运行中', value: jobsStore.runningJobs.length },
  { label: '等待中', value: jobsStore.pendingJobs.length },
  { label: '已完成', value: jobsStore.doneJobs.length },
])

function autoFillOutputPath(blendPath: string) {
  const name = (inferJobName(blendPath) || 'render').replace(/[<>:"/\\|?*]/g, '_')
  if (!form.name) form.name = name
  const dir = blendPath.replace(/[/\\][^/\\]*$/, '')
  form.output_path = `${dir}\\renders\\${name}_######`
}

const form = reactive<AddJobPayload>({
  name: '',
  blend_file: '',
  blender_executable: '',
  output_path: '',
  output_format: 'PNG',
  frame_start: 1,
  frame_end: 250,
  resume_from_existing: true,
  priority: 5,
})

function resetForm() {
  form.name = ''
  form.blend_file = ''
  form.blender_executable = settingsStore.settings.defaultBlender || settingsStore.blenderVersions[0]?.executable || ''
  form.output_path = ''
  form.output_format = 'PNG'
  form.frame_start = 1
  form.frame_end = 250
  form.resume_from_existing = true
  form.priority = 5
  formError.value = ''
  projectSettings.value = null
  projectSettingsMessage.value = ''
  outputFrameStatus.value = null
  addJobFrameStatus.value = null
}

async function ensureSettingsLoaded() {
  if (!settingsStore.blenderVersions.length && !settingsStore.settings.defaultBlender) {
    await settingsStore.load()
  }
}

async function openAddJob() {
  await ensureSettingsLoaded()
  resetForm()
  showAddJob.value = true
}

async function openAddJobWithFile(blendPath: string) {
  await ensureSettingsLoaded()
  resetForm()
  form.blend_file = blendPath
  autoFillOutputPath(blendPath)
  showAddJob.value = true
}

function closeAddJob(force = false) {
  if (submitting.value && !force) return
  showAddJob.value = false
  showAddJobConfirm.value = false
  formError.value = ''
  projectSettings.value = null
  projectSettingsMessage.value = ''
  outputFrameStatus.value = null
  addJobFrameStatus.value = null
}

function closeAddJobConfirm() {
  if (submitting.value) return
  showAddJobConfirm.value = false
  addJobFrameStatus.value = null
}

function goToSettings() {
      closeAddJob(true)
  router.push('/settings')
}

function inferJobName(path: string) {
  const filename = path.split(/[/\\]/).pop() || ''
  return filename.replace(/\.blend$/i, '')
}

function selectBlender(executable: string) {
  form.blender_executable = executable
}

const canInspectProject = computed(() => {
  return Boolean(form.blender_executable && form.blend_file && form.blend_file.toLowerCase().endsWith('.blend'))
})

const notices = computed(() => {
  const list: { type: 'warn' | 'info'; text: string }[] = []
  if (projectSettings.value?.outputFormat === 'FFMPEG') {
    list.push({ type: 'warn', text: '工程原输出为视频 (FFMPEG)，将按所选图像格式渲染序列帧。' })
  }
  if ((outputFrameStatus.value?.frameCount ?? 0) > 0) {
    const suffix = outputFrameStatus.value?.lastFrame != null ? `，最后一帧 ${outputFrameStatus.value.lastFrame}` : ''
    list.push({ type: 'warn', text: `检测到当前帧段已存在 ${outputFrameStatus.value?.frameCount} 帧${suffix}，提交时会询问从头覆盖还是断点继续。` })
  }
  return list
})

async function inspectProjectSettings(showErrors = false) {
  if (!canInspectProject.value || inspectingProject.value) return
  inspectingProject.value = true

  try {
    const settings = await inspectBlendFile(form.blender_executable, form.blend_file)
    projectSettings.value = settings
    projectSettingsMessage.value = '已从工程读取渲染参数。'
    form.frame_start = settings.frameStart
    form.frame_end = settings.frameEnd

    if (settings.outputFormat && SEQUENCE_FORMATS.includes(settings.outputFormat)) {
      form.output_format = settings.outputFormat
    }

    // Always default to blend-file-adjacent renders folder; ignore project's configured output path
    if (!form.output_path && form.blend_file) {
      autoFillOutputPath(form.blend_file)
    }
  } catch (error) {
    projectSettings.value = null
    if (showErrors) {
      projectSettingsMessage.value = error instanceof Error ? error.message : String(error)
    }
  } finally {
    inspectingProject.value = false
  }
}

async function checkOutputDir() {
  if (!form.output_path) { outputFrameStatus.value = null; return }
  try {
    outputFrameStatus.value = await inspectRenderedFrames(form.output_path, form.output_format, form.frame_start, form.frame_end)
  } catch {
    outputFrameStatus.value = null
  }
}

watch(
  [() => form.blender_executable, () => form.blend_file],
  ([newExec, newBlend], [oldExec, oldBlend]) => {
    if (newExec === oldExec && newBlend === oldBlend) return

    if (newBlend !== oldBlend && newBlend.toLowerCase().endsWith('.blend') && !form.output_path) {
      autoFillOutputPath(newBlend)
    }

    projectSettingsMessage.value = ''
    if (inspectTimer) { clearTimeout(inspectTimer); inspectTimer = null }
    projectSettings.value = null
  },
)

watch(
  [() => form.output_path, () => form.output_format, () => form.frame_start, () => form.frame_end],
  () => {
    if (outputDirTimer) { clearTimeout(outputDirTimer); outputDirTimer = null }
    outputDirTimer = setTimeout(() => void checkOutputDir(), 600)
  },
)

function buildJobPayload(resumeFromExisting: boolean): AddJobPayload {
  return {
    ...form,
    name: form.name || inferJobName(form.blend_file) || 'Untitled Render Job',
    preview_width: projectSettings.value?.resolutionX ?? null,
    preview_height: projectSettings.value?.resolutionY ?? null,
    resume_from_existing: resumeFromExisting,
  }
}

async function submitNewJob() {
  if (submitting.value) return
  formError.value = ''

  if (!form.blend_file || !form.blender_executable || !form.output_path) {
    formError.value = 'Blend file, Blender executable, and output path are required.'
    return
  }

  if (form.frame_start > form.frame_end) {
    formError.value = 'Frame start must be less than or equal to frame end.'
    return
  }

  submitting.value = true

  try {
    await validateBlendFile(form.blend_file)
    const status = await inspectRenderedFrames(form.output_path, form.output_format, form.frame_start, form.frame_end)
      .catch(() => ({ frameCount: 0, lastFrame: null, nextFrame: form.frame_start }))
    if (status.frameCount > 0) {
      addJobFrameStatus.value = status
      showAddJobConfirm.value = true
      return
    }

    await jobsStore.submitJob(buildJobPayload(true))
    closeAddJob(true)
  } catch (error) {
    formError.value = error instanceof Error ? error.message : String(error)
  } finally {
    submitting.value = false
  }
}

async function submitPreparedJob(resumeFromExisting: boolean) {
  if (submitting.value) return
  formError.value = ''
  addJobSubmitMode.value = resumeFromExisting ? 'continue' : 'restart'
  submitting.value = true
  try {
    await jobsStore.submitJob(buildJobPayload(resumeFromExisting))
    closeAddJob(true)
  } catch (error) {
    formError.value = error instanceof Error ? error.message : String(error)
  } finally {
    addJobSubmitMode.value = null
    if (showAddJob.value) {
      closeAddJobConfirm()
    }
    submitting.value = false
  }
}

onUnmounted(() => {
  unlistenDrop?.()
  if (inspectTimer) clearTimeout(inspectTimer)
  if (outputDirTimer) clearTimeout(outputDirTimer)
})
</script>
