<template>
  <UModal
    :open="open"
    :close="false"
    :title="modalTitle"
    :ui="{ content: 'job-modal-content transcode-submit-modal' }"
    @update:open="handleOpenChange"
  >
    <template #body>
      <div class="modal-stack">
        <div class="transcode-submit-stack">
          <section v-if="showManualFolderSourceSection" class="surface-panel transcode-submit-section">
            <div class="transcode-submit-head">
              <div>
                <p class="choice-card-mode">输入项</p>
                <h2 class="choice-card-title">序列帧来源</h2>
              </div>
            </div>

            <div class="transcode-submit-fields">
              <UFormField label="序列帧目录">
                <div class="transcode-submit-path-row">
                  <UTextarea
                    v-model.trim="selectedSourceFolder"
                    :rows="1"
                    autoresize
                    :disabled="saving || sourceLoading"
                    class="w-full path-textarea"
                    placeholder="F:\项目\潜行瞬鲨1-250"
                  />
                  <UButton
                    icon="i-lucide-folder-open"
                    label="选择"
                    color="neutral"
                    variant="outline"
                    :disabled="saving || sourceLoading"
                    :loading="sourceLoading"
                    @click="browseSourceFolder"
                  />
                </div>
              </UFormField>

              <UFormField v-if="sourceItems.length > 1" label="序列选择">
                <USelect
                  v-model="selectedSourceInputPath"
                  :items="sourceItems"
                  value-key="value"
                  label-key="label"
                  :disabled="saving || sourceLoading"
                  trailing-icon="i-lucide-chevron-down"
                />
              </UFormField>

              <UAlert
                v-if="selectedSourceOption"
                color="neutral"
                variant="subtle"
                :title="`${selectedSourceOption.name} · ${selectedSourceOption.frameStart}–${selectedSourceOption.frameEnd}`"
                :description="`输入序列：${selectedSourceOption.inputPath}`"
              />
              <p v-else class="hint-text">先选择一个包含序列帧的目录，再继续配置输出参数。</p>
            </div>
          </section>

          <section class="surface-panel transcode-submit-section">
            <div class="transcode-submit-head">
              <div>
                <p class="choice-card-mode">基础项</p>
                <h2 class="choice-card-title">输出配置</h2>
              </div>
            </div>

            <div class="transcode-submit-fields">
              <div v-if="showBlenderFrameRangeSection" class="transcode-submit-inline-grid">
                <UFormField label="转码起始帧">
                  <UInputNumber
                    v-model="selectedFrameStart"
                    :min="blenderFrameRangeMin"
                    :max="blenderFrameRangeMax"
                    :disabled="saving"
                    orientation="horizontal"
                    decrement-icon="i-lucide-minus"
                    increment-icon="i-lucide-plus"
                    :ui="{ root: 'w-full' }"
                  />
                </UFormField>

                <UFormField label="转码结束帧">
                  <UInputNumber
                    v-model="selectedFrameEnd"
                    :min="blenderFrameRangeMin"
                    :max="blenderFrameRangeMax"
                    :disabled="saving"
                    orientation="horizontal"
                    decrement-icon="i-lucide-minus"
                    increment-icon="i-lucide-plus"
                    :ui="{ root: 'w-full' }"
                  />
                </UFormField>
              </div>
              <UFormField label="任务名">
                <UTextarea
                  v-model.trim="form.name"
                  :rows="1"
                  autoresize
                  :disabled="saving"
                  class="w-full"
                  placeholder="潜行瞬鲨_1-250_转码"
                />
              </UFormField>

              <UFormField label="输出目录">
                <div class="transcode-submit-path-row">
                  <UTextarea
                    v-model.trim="form.outputDir"
                    :rows="1"
                    autoresize
                    :disabled="saving"
                    class="w-full path-textarea"
                    placeholder="F:\项目\转码"
                  />
                  <UButton
                    icon="i-lucide-folder-open"
                    label="更改"
                    color="neutral"
                    variant="outline"
                    :disabled="saving"
                    @click="browseOutputDirectory"
                  />
                </div>
              </UFormField>

              <UFormField label="输出文件名">
                <div class="transcode-submit-filename-row">
                  <UTextarea
                    v-model.trim="form.outputStem"
                    :rows="1"
                    autoresize
                    :disabled="saving"
                    class="w-full"
                    placeholder="潜行瞬鲨_1-250"
                  />
                  <span class="transcode-submit-suffix">.mp4</span>
                </div>
              </UFormField>
            </div>
          </section>

          <section class="surface-panel transcode-submit-section">
            <div class="transcode-submit-head">
              <div>
                <p class="choice-card-mode">高级项</p>
                <h2 class="choice-card-title">编码参数</h2>
              </div>
            </div>

            <div class="transcode-submit-inline-grid">
              <UFormField label="输出 FPS">
                <UInputNumber
                  v-model="form.fps"
                  :min="1"
                  :max="240"
                  :step="1"
                  :disabled="saving"
                  orientation="horizontal"
                  decrement-icon="i-lucide-minus"
                  increment-icon="i-lucide-plus"
                  :ui="{ root: 'w-full' }"
                />
              </UFormField>

              <UFormField label="CRF">
                <UInputNumber
                  v-model="form.crf"
                  :min="0"
                  :max="51"
                  :step="1"
                  :disabled="saving"
                  orientation="horizontal"
                  decrement-icon="i-lucide-minus"
                  increment-icon="i-lucide-plus"
                  :ui="{ root: 'w-full' }"
                />
              </UFormField>

              <UFormField label="Preset">
                <USelect
                  v-model="form.preset"
                  :items="presetOptions"
                  :disabled="saving"
                  trailing-icon="i-lucide-chevron-down"
                  :ui="{ base: 'w-full' }"
                />
              </UFormField>
            </div>
          </section>
        </div>

        <p v-if="errorMessage" class="form-error">{{ errorMessage }}</p>

        <div class="modal-actions settings-modal-actions">
          <div class="settings-modal-actions-start">
            <UButton
              v-if="mode === 'settings'"
              icon="i-lucide-rotate-ccw"
              label="恢复默认"
              color="neutral"
              variant="ghost"
              :disabled="saving"
              @click="resetToBaseConfig"
            />
          </div>
          <div class="settings-modal-actions-end">
            <UButton
              icon="i-lucide-x"
              label="取消"
              color="neutral"
              variant="outline"
              :disabled="saving"
              @click="emit('close')"
            />
            <UButton
              :icon="mode === 'settings' ? 'i-lucide-save' : 'i-lucide-clapperboard'"
              :label="mode === 'settings' ? '保存' : '提交'"
              color="primary"
              variant="solid"
              :loading="saving"
              @click="submit"
            />
          </div>
        </div>
      </div>
    </template>
  </UModal>
</template>

<script setup lang="ts">
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import type { AddFfmpegJobPayload, FolderFrameGroup, OutputPathTemplatePreview, PathTemplateKind, RenderJobTranscodeConfig } from '~/types'
import {
  TRANSCODE_PRESET_OPTIONS,
  buildTranscodeOutputPath,
  normalizeTranscodeDirectory,
  sanitizeTranscodeStemPart,
  splitTranscodeOutputPath,
} from '~/composables/useTranscodeConfig'

const props = defineProps<{
  open: boolean
  mode?: 'submit' | 'settings'
  initialConfig?: RenderJobTranscodeConfig | null
  baseConfig?: RenderJobTranscodeConfig | null
  folderPath?: string
  folderInputPath?: string
  folderFrameStart?: number
  folderFrameEnd?: number
  folderName?: string
  blenderJobBlendFile?: string
  blenderJobId?: string
  blenderJobName?: string
  blenderJobFps?: number | null
  blenderJobFrameStart?: number
  blenderJobFrameEnd?: number
  blenderJobOriginalFrameStart?: number
  blenderJobOriginalFrameEnd?: number
  blenderJobOutputPath?: string
}>()

const emit = defineEmits<{
  close: []
  submit: [payload: AddFfmpegJobPayload]
  saveSettings: [payload: {
    transcode_name_override: string | null
    transcode_fps_override: number | null
    transcode_output_path_override: string | null
    transcode_crf_override: number | null
    transcode_preset_override: string | null
  }]
  'update:open': [value: boolean]
}>()

const settingsStore = useSettingsStore()
const { previewOutputPathTemplate, scanFolderFrameGroups } = useTauri()

const mode = computed(() => props.mode ?? 'submit')
const modalTitle = computed(() => {
  if (mode.value === 'settings') return '转码设置'
  return props.blenderJobId ? '提交转码' : '新建转码任务'
})
const presetOptions: string[] = [...TRANSCODE_PRESET_OPTIONS]

type SourceItem = {
  label: string
  value: string
  folderPath: string
  inputPath: string
  frameStart: number
  frameEnd: number
  name: string
}

const form = reactive({
  name: '',
  fps: 30,
  outputDir: '',
  outputStem: '',
  crf: 18,
  preset: 'medium',
})
const saving = ref(false)
const errorMessage = ref('')
const sourceLoading = ref(false)
const selectedSourceFolder = ref('')
const sourceItems = ref<SourceItem[]>([])
const selectedSourceInputPath = ref('')
const selectedFrameStart = ref(1)
const selectedFrameEnd = ref(1)
const outputPathPreview = ref<OutputPathTemplatePreview | null>(null)
const resolvedDefaultOutputPath = ref('')
let lastDerivedConfig: RenderJobTranscodeConfig | null = null
let outputPathPreviewTimer: ReturnType<typeof setTimeout> | null = null

const showManualFolderSourceSection = computed(() =>
  mode.value === 'submit' && !props.blenderJobId && !props.folderInputPath && !props.folderPath,
)
const showBlenderFrameRangeSection = computed(() =>
  mode.value === 'submit' && Boolean(props.blenderJobId),
)

const selectedSourceOption = computed(() =>
  sourceItems.value.find(item => item.value === selectedSourceInputPath.value) ?? null,
)
const templateKind = computed<PathTemplateKind>(() =>
  props.blenderJobId ? 'blender-ffmpeg' : 'standalone-ffmpeg',
)
const blenderFrameRangeMin = computed(() => props.blenderJobOriginalFrameStart ?? props.blenderJobFrameStart ?? 1)
const blenderFrameRangeMax = computed(() => props.blenderJobOriginalFrameEnd ?? props.blenderJobFrameEnd ?? blenderFrameRangeMin.value)
const currentFrameStart = computed(() => {
  if (props.blenderJobId) return selectedFrameStart.value
  return currentFolderSource()?.frameStart ?? props.blenderJobFrameStart ?? 1
})
const currentFrameEnd = computed(() => {
  if (props.blenderJobId) return selectedFrameEnd.value
  return currentFolderSource()?.frameEnd ?? props.blenderJobFrameEnd ?? 1
})
const hasTemplateErrors = computed(() => Boolean(outputPathPreview.value?.errors.length))

function buildSourceItems(folderPath: string, groups: FolderFrameGroup[]): SourceItem[] {
  return groups.map(group => ({
    label: `${group.name} · ${group.frameStart}–${group.frameEnd}`,
    value: group.inputPath,
    folderPath,
    inputPath: group.inputPath,
    frameStart: group.frameStart,
    frameEnd: group.frameEnd,
    name: group.name,
  }))
}

function buildSingleSourceItem(): SourceItem | null {
  if (!props.folderInputPath && !props.folderPath) return null
  return {
    label: `${props.folderName ?? 'render'} · ${props.folderFrameStart ?? 1}–${props.folderFrameEnd ?? 1}`,
    value: props.folderInputPath ?? props.folderPath ?? '',
    folderPath: props.folderPath ?? '',
    inputPath: props.folderInputPath ?? props.folderPath ?? '',
    frameStart: props.folderFrameStart ?? 1,
    frameEnd: props.folderFrameEnd ?? 1,
    name: props.folderName ?? 'render',
  }
}

function currentFolderSource() {
  return selectedSourceOption.value ?? buildSingleSourceItem()
}

function deriveOutputDirectory() {
  const folderSource = currentFolderSource()
  if (folderSource) {
    return normalizeTranscodeDirectory(folderSource.folderPath || folderSource.inputPath)
  }

  if (props.blenderJobOutputPath) {
    const normalized = props.blenderJobOutputPath.replace(/\\/g, '/')
    if (normalized.includes('#') || normalized.includes('{frame}')) {
      return normalized.slice(0, normalized.lastIndexOf('/'))
    }
    return normalizeTranscodeDirectory(props.blenderJobOutputPath)
  }

  return normalizeTranscodeDirectory(props.folderPath)
}

async function refreshResolvedDefaultOutputPath() {
  const template = props.blenderJobId
    ? settingsStore.settings.blenderTranscodeOutputPathTemplate
    : settingsStore.settings.standaloneTranscodeOutputPathTemplate

  try {
    const preview = await previewOutputPathTemplate({
      kind: templateKind.value,
      template,
      blend_file: props.blenderJobBlendFile ?? null,
      source_path: currentFolderSource()?.folderPath ?? props.folderPath ?? null,
      frame_start: currentFrameStart.value,
      frame_end: currentFrameEnd.value,
    })
    resolvedDefaultOutputPath.value = preview.resolvedPath || ''
  } catch {
    resolvedDefaultOutputPath.value = ''
  }
}

function buildDerivedConfig(): RenderJobTranscodeConfig {
  const folderSource = currentFolderSource()
  const name = sanitizeTranscodeStemPart(folderSource?.name ?? props.blenderJobName)
  const fallbackOutputPath = buildTranscodeOutputPath(deriveOutputDirectory(), name)
  const outputPath = resolvedDefaultOutputPath.value || fallbackOutputPath
  const { outputDir, outputStem } = splitTranscodeOutputPath(outputPath)

  return {
    name,
    fps: Math.max(1, Math.round(props.blenderJobFps && props.blenderJobFps > 0 ? props.blenderJobFps : 30)),
    outputPath,
    outputDir,
    outputStem,
    crf: settingsStore.settings.transcodeCrf,
    preset: settingsStore.settings.transcodePreset,
  }
}

function applyConfig(config: RenderJobTranscodeConfig) {
  form.name = config.name
  form.fps = config.fps
  form.outputDir = config.outputDir
  form.outputStem = config.outputStem
  form.crf = config.crf
  form.preset = config.preset
}

function syncDerivedOutputFields(previousConfig: RenderJobTranscodeConfig | null, nextConfig: RenderJobTranscodeConfig) {
  if (!previousConfig) return

  const currentOutputPath = buildTranscodeOutputPath(form.outputDir, form.outputStem)
  if (currentOutputPath === previousConfig.outputPath) {
    form.outputDir = nextConfig.outputDir
    form.outputStem = nextConfig.outputStem
  }
}

async function syncForm() {
  const singleSourceItem = buildSingleSourceItem()
  selectedSourceFolder.value = singleSourceItem?.folderPath ?? ''
  sourceItems.value = singleSourceItem ? [singleSourceItem] : []
  selectedSourceInputPath.value = singleSourceItem?.value ?? ''
  selectedFrameStart.value = props.blenderJobFrameStart ?? blenderFrameRangeMin.value
  selectedFrameEnd.value = props.blenderJobFrameEnd ?? blenderFrameRangeMax.value
  await refreshResolvedDefaultOutputPath()
  const derivedConfig = buildDerivedConfig()
  lastDerivedConfig = derivedConfig
  applyConfig(props.initialConfig ?? derivedConfig)
  errorMessage.value = ''
}

watch(
  () => [
    props.open,
    props.folderPath,
    props.folderInputPath,
    props.folderFrameStart,
    props.folderFrameEnd,
    props.folderName,
    props.blenderJobId,
    props.blenderJobName,
    props.blenderJobFps,
    props.blenderJobFrameStart,
    props.blenderJobFrameEnd,
    props.blenderJobOriginalFrameStart,
    props.blenderJobOriginalFrameEnd,
    props.blenderJobOutputPath,
    props.initialConfig,
    props.baseConfig,
    mode.value,
    settingsStore.settings.transcodeCrf,
    settingsStore.settings.transcodePreset,
  ] as const,
  ([open]) => {
    if (!open) return
    void syncForm()
  },
  { immediate: true },
)

function handleOpenChange(value: boolean) {
  emit('update:open', value)
  if (!value) {
    emit('close')
  }
}

async function loadSourceFolder(folderPath: string) {
  sourceLoading.value = true
  errorMessage.value = ''

  try {
    const result = await scanFolderFrameGroups(folderPath)
    selectedSourceFolder.value = folderPath

    if (result.groups.length === 0) {
      sourceItems.value = []
      selectedSourceInputPath.value = ''
      resolvedDefaultOutputPath.value = ''
      applyConfig(buildDerivedConfig())
      errorMessage.value = '这个目录里没有检测到可转码的序列帧。'
      return
    }

    sourceItems.value = buildSourceItems(folderPath, result.groups)
    selectedSourceInputPath.value = sourceItems.value[0]?.value ?? ''
    await refreshResolvedDefaultOutputPath()
    applyConfig(buildDerivedConfig())
  } catch (error) {
    sourceItems.value = []
    selectedSourceInputPath.value = ''
    resolvedDefaultOutputPath.value = ''
    errorMessage.value = error instanceof Error ? error.message : String(error)
  } finally {
    sourceLoading.value = false
  }
}

async function browseSourceFolder() {
  if (saving.value || sourceLoading.value) return
  const selected = await openDialog({
    directory: true,
    multiple: false,
    title: '选择序列帧目录',
    defaultPath: selectedSourceFolder.value || undefined,
  })
  if (typeof selected === 'string' && selected) {
    await loadSourceFolder(selected)
  }
}

async function browseOutputDirectory() {
  if (saving.value) return
  const selected = await openDialog({
    directory: true,
    multiple: false,
    title: '选择输出目录',
    defaultPath: form.outputDir || undefined,
  })
  if (typeof selected === 'string' && selected) {
    form.outputDir = selected
  }
}

function resetToBaseConfig() {
  if (!props.baseConfig) return
  applyConfig(props.baseConfig)
  errorMessage.value = ''
}

function currentConfig(): RenderJobTranscodeConfig {
  const outputPath = buildTranscodeOutputPath(form.outputDir, form.outputStem)
  const split = splitTranscodeOutputPath(outputPath)
  return {
    name: form.name.trim(),
    fps: Math.max(1, Math.round(Number(form.fps))),
    outputPath,
    outputDir: split.outputDir,
    outputStem: split.outputStem,
    crf: Math.min(51, Math.max(0, Math.round(form.crf))),
    preset: form.preset,
  }
}

async function refreshOutputPathPreview() {
  const outputPath = buildTranscodeOutputPath(form.outputDir, form.outputStem)
  if (!outputPath.trim()) {
    outputPathPreview.value = null
    return
  }

  try {
    outputPathPreview.value = await previewOutputPathTemplate({
      kind: templateKind.value,
      template: outputPath,
      blend_file: props.blenderJobBlendFile ?? null,
      source_path: currentFolderSource()?.folderPath ?? props.folderPath ?? null,
      frame_start: currentFrameStart.value,
      frame_end: currentFrameEnd.value,
    })
  } catch (error) {
    outputPathPreview.value = {
      resolvedPath: null,
      errors: [error instanceof Error ? error.message : String(error)],
      notes: [],
    }
  }
}

watch(
  () => selectedSourceInputPath.value,
  async (value, previousValue) => {
    if (!showManualFolderSourceSection.value || !value || value === previousValue) return
    await refreshResolvedDefaultOutputPath()
    const derivedConfig = buildDerivedConfig()
    lastDerivedConfig = derivedConfig
    applyConfig(derivedConfig)
    errorMessage.value = ''
  },
)

watch(
  () => [props.open, currentFrameStart.value, currentFrameEnd.value] as const,
  async ([open, frameStart, frameEnd], [, previousFrameStart, previousFrameEnd]) => {
    if (!open || !showBlenderFrameRangeSection.value) return
    if (frameStart === previousFrameStart && frameEnd === previousFrameEnd) return

    const previousDerivedConfig = lastDerivedConfig
    await refreshResolvedDefaultOutputPath()
    const nextDerivedConfig = buildDerivedConfig()
    syncDerivedOutputFields(previousDerivedConfig, nextDerivedConfig)
    lastDerivedConfig = nextDerivedConfig
  },
)

watch(
  () => [
    settingsStore.settings.blenderTranscodeOutputPathTemplate,
    settingsStore.settings.standaloneTranscodeOutputPathTemplate,
    form.outputDir,
    form.outputStem,
    selectedSourceInputPath.value,
    selectedSourceFolder.value,
    currentFrameStart.value,
    currentFrameEnd.value,
    props.folderPath,
    props.folderName,
    props.blenderJobBlendFile,
    templateKind.value,
  ] as const,
  () => {
    void refreshResolvedDefaultOutputPath()
    if (outputPathPreviewTimer) clearTimeout(outputPathPreviewTimer)
    outputPathPreviewTimer = setTimeout(() => void refreshOutputPathPreview(), 220)
  },
  { immediate: true },
)

onUnmounted(() => {
  if (outputPathPreviewTimer) clearTimeout(outputPathPreviewTimer)
})

async function submit() {
  if (saving.value) return

  const name = form.name.trim()
  const outputDir = form.outputDir.trim()
  const outputStem = form.outputStem.trim()
  const fps = Number(form.fps)
  const folderSource = currentFolderSource()

  if (!name) {
    errorMessage.value = '任务名不能为空。'
    return
  }
  if (showBlenderFrameRangeSection.value) {
    if (selectedFrameStart.value > selectedFrameEnd.value) {
      errorMessage.value = '转码起始帧不能大于结束帧。'
      return
    }
    if (selectedFrameStart.value < blenderFrameRangeMin.value || selectedFrameEnd.value > blenderFrameRangeMax.value) {
      errorMessage.value = `转码帧范围必须在项目范围 ${blenderFrameRangeMin.value}–${blenderFrameRangeMax.value} 内。`
      return
    }
  }
  if (!outputDir) {
    errorMessage.value = '输出目录不能为空。'
    return
  }
  if (!outputStem) {
    errorMessage.value = '输出文件名不能为空。'
    return
  }
  if (hasTemplateErrors.value) {
    errorMessage.value = outputPathPreview.value?.errors[0] || '输出路径模板无效。'
    return
  }
  if (!Number.isFinite(fps) || fps <= 0) {
    errorMessage.value = '输出 FPS 必须大于 0。'
    return
  }

  saving.value = true
  errorMessage.value = ''

  try {
    const config = currentConfig()
    if (mode.value === 'settings') {
      const baseConfig = props.baseConfig ?? buildDerivedConfig()
      emit('saveSettings', {
        transcode_name_override: config.name === baseConfig.name ? null : config.name,
        transcode_fps_override: config.fps === baseConfig.fps ? null : config.fps,
        transcode_output_path_override: config.outputPath === baseConfig.outputPath ? null : config.outputPath,
        transcode_crf_override: config.crf === baseConfig.crf ? null : config.crf,
        transcode_preset_override: config.preset === baseConfig.preset ? null : config.preset,
      })
      emit('update:open', false)
      emit('close')
      return
    }

    if (!props.blenderJobId && !folderSource) {
      errorMessage.value = '请先选择序列帧目录。'
      return
    }

    emit('submit', {
      name,
      source_type: props.blenderJobId ? 'blender_job' : 'folder',
      source_blender_job_id: props.blenderJobId ?? null,
      input_path: folderSource?.inputPath ?? props.blenderJobOutputPath ?? '',
      frame_start: props.blenderJobId ? selectedFrameStart.value : (folderSource?.frameStart ?? props.blenderJobFrameStart ?? 1),
      frame_end: props.blenderJobId ? selectedFrameEnd.value : (folderSource?.frameEnd ?? props.blenderJobFrameEnd ?? 1),
      fps,
      output_path: config.outputPath,
      crf: config.crf,
      preset: config.preset,
    })
  } finally {
    saving.value = false
  }
}
</script>
