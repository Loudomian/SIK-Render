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
          <section class="surface-panel transcode-submit-section">
            <div class="transcode-submit-head">
              <div>
                <p class="choice-card-mode">基础项</p>
                <h2 class="choice-card-title">输出配置</h2>
              </div>
            </div>

            <div class="transcode-submit-fields">
              <UFormField label="任务名">
                <UInput
                  v-model.trim="form.name"
                  :disabled="saving"
                  class="w-full"
                  placeholder="shot_001"
                />
              </UFormField>

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
                  :ui="{ root: 'w-32' }"
                />
              </UFormField>

              <UFormField label="输出目录">
                <div class="transcode-submit-path-row">
                  <UInput
                    v-model.trim="form.outputDir"
                    :disabled="saving"
                    class="w-full"
                    placeholder="C:\\Renders\\shot_001"
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
                  <UInput
                    v-model.trim="form.outputStem"
                    :disabled="saving"
                    class="w-full"
                    placeholder="shot_001"
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
              <UButton
                :icon="showAdvanced ? 'i-lucide-chevron-up' : 'i-lucide-chevron-down'"
                :label="showAdvanced ? '收起' : '展开'"
                color="neutral"
                variant="ghost"
                size="sm"
                @click="showAdvanced = !showAdvanced"
              />
            </div>

            <div v-if="showAdvanced" class="transcode-submit-fields">
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
                  :ui="{ root: 'w-32' }"
                />
              </UFormField>

              <UFormField label="Preset">
                <USelect
                  v-model="form.preset"
                  :items="presetOptions"
                  :disabled="saving"
                  trailing-icon="i-lucide-chevron-down"
                  :ui="{ base: 'w-40' }"
                />
              </UFormField>
            </div>
          </section>
        </div>

        <p v-if="errorMessage" class="form-error">{{ errorMessage }}</p>

        <div class="modal-actions settings-modal-actions">
          <UButton
            v-if="mode === 'settings'"
            icon="i-lucide-rotate-ccw"
            label="恢复默认"
            color="neutral"
            variant="ghost"
            :disabled="saving"
            @click="resetToBaseConfig"
          />
          <div class="settings-modal-spacer" />
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
    </template>
  </UModal>
</template>

<script setup lang="ts">
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import type { AddFfmpegJobPayload, RenderJobTranscodeConfig } from '~/types'
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
  blenderJobId?: string
  blenderJobName?: string
  blenderJobFps?: number | null
  blenderJobFrameStart?: number
  blenderJobFrameEnd?: number
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

const mode = computed(() => props.mode ?? 'submit')
const modalTitle = computed(() => mode.value === 'settings' ? '转码设置' : '提交转码')
const presetOptions: string[] = [...TRANSCODE_PRESET_OPTIONS]

const form = reactive({
  name: '',
  fps: 30,
  outputDir: '',
  outputStem: '',
  crf: 18,
  preset: 'medium',
})
const saving = ref(false)
const showAdvanced = ref(false)
const errorMessage = ref('')

function deriveOutputDirectory() {
  if (props.blenderJobOutputPath) {
    const normalized = props.blenderJobOutputPath.replace(/\\/g, '/')
    if (normalized.includes('#')) {
      return normalized.slice(0, normalized.lastIndexOf('/'))
    }
    return normalizeTranscodeDirectory(props.blenderJobOutputPath)
  }

  return normalizeTranscodeDirectory(props.folderPath)
}

function buildDerivedConfig(): RenderJobTranscodeConfig {
  const name = sanitizeTranscodeStemPart(props.folderName ?? props.blenderJobName)
  const outputPath = buildTranscodeOutputPath(deriveOutputDirectory(), name)
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

function syncForm() {
  applyConfig(props.initialConfig ?? buildDerivedConfig())
  showAdvanced.value = mode.value === 'settings'
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
    props.blenderJobOutputPath,
    props.initialConfig,
    props.baseConfig,
    mode.value,
    settingsStore.settings.transcodeCrf,
    settingsStore.settings.transcodePreset,
  ] as const,
  ([open]) => {
    if (!open) return
    syncForm()
  },
  { immediate: true },
)

function handleOpenChange(value: boolean) {
  emit('update:open', value)
  if (!value) {
    emit('close')
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

async function submit() {
  if (saving.value) return

  const name = form.name.trim()
  const outputDir = form.outputDir.trim()
  const outputStem = form.outputStem.trim()
  const fps = Number(form.fps)

  if (!name) {
    errorMessage.value = '任务名不能为空。'
    return
  }
  if (!outputDir) {
    errorMessage.value = '输出目录不能为空。'
    return
  }
  if (!outputStem) {
    errorMessage.value = '输出文件名不能为空。'
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

    emit('submit', {
      name,
      source_type: props.blenderJobId ? 'blender_job' : 'folder',
      source_blender_job_id: props.blenderJobId ?? null,
      input_path: props.folderInputPath ?? props.folderPath ?? props.blenderJobOutputPath ?? '',
      frame_start: props.folderFrameStart ?? props.blenderJobFrameStart ?? 1,
      frame_end: props.folderFrameEnd ?? props.blenderJobFrameEnd ?? 1,
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
