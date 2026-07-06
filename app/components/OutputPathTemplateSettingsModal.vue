<template>
  <UModal
    :open="open"
    :close="false"
    :title="t('settingsModals.outputTemplates.title')"
    :ui="{ content: 'job-modal-content settings-modal-content output-template-modal' }"
    @update:open="handleOpenChange"
  >
    <template #body>
      <div class="modal-stack">
        <section class="surface-panel settings-field-panel settings-field-panel-stack output-template-modal-copy">
          <div class="settings-field-copy">
            <p class="settings-field-title">{{ t('settingsModals.outputTemplates.introTitle') }}</p>
            <p class="hint-text">{{ t('settingsModals.outputTemplates.introNote') }}</p>
            <p class="hint-text">{{ t('settingsModals.outputTemplates.sampleProject', { path: sampleBlendFile }) }}</p>
            <p class="hint-text">{{ t('settingsModals.outputTemplates.sampleFrameRange', { start: sampleFrameStart, end: sampleFrameEnd }) }}</p>
          </div>
        </section>

        <div class="settings-form-stack">
          <section
            v-for="section in sections"
            :key="section.key"
            class="surface-panel settings-field-panel settings-field-panel-stack template-section"
          >
            <div class="template-section-head">
              <div class="settings-field-copy">
                <p class="settings-field-title">{{ section.title }}</p>
                <p class="hint-text">{{ section.description }}</p>
              </div>
            </div>

            <PathTemplateInput
              v-model="draft[section.key]"
              :kind="section.kind"
              :placeholder="section.placeholder"
              :disabled="saving"
              variable-panel-mode="hidden"
              :blend-file-name="section.sampleBlendFileName"
              :folder-name="section.sampleFolderName"
              :frame-start="sampleFrameStart"
              :frame-end="sampleFrameEnd"
            />

            <div class="template-preview-panel">
              <p class="template-subtitle">{{ t('settingsModals.outputTemplates.previewTitle') }}</p>
              <div class="template-preview-grid">
                <section class="surface-panel template-preview-card">
                  <p class="template-preview-label">{{ t('settingsModals.outputTemplates.pathResult') }}</p>
                  <p class="template-preview-value">{{ previewDetails[section.key].resolvedPath || t('settingsModals.outputTemplates.unresolved') }}</p>
                </section>

                <section v-if="section.kind === 'blender'" class="surface-panel template-preview-card">
                  <p class="template-preview-label">{{ t('settingsModals.outputTemplates.frameNameRange') }}</p>
                  <p class="template-preview-value">{{ previewDetails[section.key].frameNameRange || '—' }}</p>
                </section>
              </div>

              <p v-for="message in previews[section.key]?.notes ?? []" :key="`${section.key}-note-${message}`" class="hint-text">
                {{ message }}
              </p>
              <p v-for="message in previews[section.key]?.errors ?? []" :key="`${section.key}-error-${message}`" class="form-error">
                {{ message }}
              </p>
            </div>
          </section>
        </div>

        <p v-if="errorMessage" class="form-error">{{ errorMessage }}</p>

        <div class="modal-actions settings-modal-actions">
          <div class="settings-modal-actions-start">
            <UButton
              icon="i-lucide-rotate-ccw"
              :label="t('common.resetDefaults')"
              color="neutral"
              variant="ghost"
              :disabled="saving"
              @click="resetToDefaults"
            />
          </div>
          <div class="settings-modal-actions-end">
            <UButton
              icon="i-lucide-x"
              :label="t('common.cancel')"
              color="neutral"
              variant="outline"
              :disabled="saving"
              @click="emit('update:open', false)"
            />
            <UButton
              icon="i-lucide-save"
              :label="t('common.save')"
              color="primary"
              variant="solid"
              :loading="saving"
              :disabled="hasErrors"
              @click="saveSettingsDraft"
            />
          </div>
        </div>
      </div>
    </template>
  </UModal>
</template>

<script setup lang="ts">
import type { OutputPathTemplatePreview, PathTemplateKind } from '~/types'

type TemplateKey =
  | 'renderOutputPathTemplate'
  | 'blenderTranscodeOutputPathTemplate'
  | 'standaloneTranscodeOutputPathTemplate'

type TemplatePreviewDetails = {
  resolvedPath: string | null
  frameNameRange: string | null
}

const props = defineProps<{
  open: boolean
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
}>()

const settingsStore = useSettingsStore()
const { previewOutputPathTemplate } = useTauri()
const { t } = useI18n()

const sampleFrameStart = 1
const sampleFrameEnd = 250
const sampleBlendFile = 'F:\\项目\\测试项目.blend'
const sampleBlendFileName = '测试项目'
const sampleFolderPath = 'F:\\项目\\测试项目\\测试项目_000001.png'
const sampleFolderName = '测试项目'

const defaultTemplateSettings: Record<TemplateKey, string> = {
  renderOutputPathTemplate: './{blendFileName}_{frameStart}-{frameEnd}/{blendFileName}_{frame}',
  blenderTranscodeOutputPathTemplate: './transcode/{blendFileName}_{frameStart}-{frameEnd}.mp4',
  standaloneTranscodeOutputPathTemplate: '../transcode/{folderName}_{frameStart}-{frameEnd}.mp4',
}

const draft = reactive<Record<TemplateKey, string>>({
  renderOutputPathTemplate: defaultTemplateSettings.renderOutputPathTemplate,
  blenderTranscodeOutputPathTemplate: defaultTemplateSettings.blenderTranscodeOutputPathTemplate,
  standaloneTranscodeOutputPathTemplate: defaultTemplateSettings.standaloneTranscodeOutputPathTemplate,
})

const previews = reactive<Record<TemplateKey, OutputPathTemplatePreview | null>>({
  renderOutputPathTemplate: null,
  blenderTranscodeOutputPathTemplate: null,
  standaloneTranscodeOutputPathTemplate: null,
})

const saving = ref(false)
const errorMessage = ref('')
let previewTimer: ReturnType<typeof setTimeout> | null = null

const sections = computed(() => [
  {
    key: 'renderOutputPathTemplate' as const,
    title: t('settingsModals.outputTemplates.renderTitle'),
    description: t('settingsModals.outputTemplates.renderDescription'),
    kind: 'blender' as const,
    placeholder: './{blendFileName}_{frameStart}-{frameEnd}/{blendFileName}_{frame}',
    sampleBlendFileName,
    sampleFolderName: null,
  },
  {
    key: 'blenderTranscodeOutputPathTemplate' as const,
    title: t('settingsModals.outputTemplates.blenderTranscodeTitle'),
    description: t('settingsModals.outputTemplates.blenderTranscodeDescription'),
    kind: 'blender-ffmpeg' as const,
    placeholder: './transcode/{blendFileName}_{frameStart}-{frameEnd}.mp4',
    sampleBlendFileName,
    sampleFolderName: null,
  },
  {
    key: 'standaloneTranscodeOutputPathTemplate' as const,
    title: t('settingsModals.outputTemplates.standaloneTranscodeTitle'),
    description: t('settingsModals.outputTemplates.standaloneTranscodeDescription'),
    kind: 'standalone-ffmpeg' as const,
    placeholder: '../transcode/{folderName}_{frameStart}-{frameEnd}.mp4',
    sampleBlendFileName: null,
    sampleFolderName,
  },
])

const hasErrors = computed(() =>
  Object.values(previews).some(preview => Boolean(preview?.errors.length)),
)

const previewDetails = computed<Record<TemplateKey, TemplatePreviewDetails>>(() => ({
  renderOutputPathTemplate: buildPreviewDetails('blender', previews.renderOutputPathTemplate),
  blenderTranscodeOutputPathTemplate: buildPreviewDetails('blender-ffmpeg', previews.blenderTranscodeOutputPathTemplate),
  standaloneTranscodeOutputPathTemplate: buildPreviewDetails('standalone-ffmpeg', previews.standaloneTranscodeOutputPathTemplate),
}))

function padFrame(frame: number) {
  return String(frame).padStart(6, '0')
}

function fileNameFromPath(path: string | null) {
  if (!path) return null
  return path.split(/[/\\]/).pop() || null
}

function replaceAllFramePlaceholders(path: string, frame: number) {
  return path.replace(/#+/g, (placeholder) => String(frame).padStart(placeholder.length, '0'))
}

function buildPreviewDetails(kind: PathTemplateKind, preview: OutputPathTemplatePreview | null): TemplatePreviewDetails {
  const resolvedPath = preview?.resolvedPath ?? null
  if (!resolvedPath) {
    return {
      resolvedPath: null,
      frameNameRange: null,
    }
  }

  if (kind === 'blender') {
    const firstFramePath = replaceAllFramePlaceholders(resolvedPath, sampleFrameStart)
    const lastFramePath = replaceAllFramePlaceholders(resolvedPath, sampleFrameEnd)
    return {
      resolvedPath,
      frameNameRange: [fileNameFromPath(firstFramePath), fileNameFromPath(lastFramePath)]
        .filter(Boolean)
        .join(' / ') || null,
    }
  }

  return {
    resolvedPath,
    frameNameRange: null,
  }
}

function buildPreviewPayload(kind: PathTemplateKind, template: string) {
  if (kind === 'standalone-ffmpeg') {
    return {
      kind,
      template,
      source_path: sampleFolderPath,
      frame_start: sampleFrameStart,
      frame_end: sampleFrameEnd,
    } as const
  }

  return {
    kind,
    template,
    blend_file: sampleBlendFile,
    frame_start: sampleFrameStart,
    frame_end: sampleFrameEnd,
  } as const
}

function syncDraft() {
  draft.renderOutputPathTemplate = settingsStore.settings.renderOutputPathTemplate
  draft.blenderTranscodeOutputPathTemplate = settingsStore.settings.blenderTranscodeOutputPathTemplate
  draft.standaloneTranscodeOutputPathTemplate = settingsStore.settings.standaloneTranscodeOutputPathTemplate
  errorMessage.value = ''
}

async function refreshPreviews() {
  const [render, blenderTranscode, standaloneTranscode] = await Promise.all([
    previewOutputPathTemplate(buildPreviewPayload('blender', draft.renderOutputPathTemplate)),
    previewOutputPathTemplate(buildPreviewPayload('blender-ffmpeg', draft.blenderTranscodeOutputPathTemplate)),
    previewOutputPathTemplate(buildPreviewPayload('standalone-ffmpeg', draft.standaloneTranscodeOutputPathTemplate)),
  ])

  previews.renderOutputPathTemplate = render
  previews.blenderTranscodeOutputPathTemplate = blenderTranscode
  previews.standaloneTranscodeOutputPathTemplate = standaloneTranscode
}

watch(
  () => [
    props.open,
    settingsStore.settings.renderOutputPathTemplate,
    settingsStore.settings.blenderTranscodeOutputPathTemplate,
    settingsStore.settings.standaloneTranscodeOutputPathTemplate,
  ] as const,
  ([open]) => {
    if (!open) return
    syncDraft()
  },
  { immediate: true },
)

watch(
  () => [
    props.open,
    draft.renderOutputPathTemplate,
    draft.blenderTranscodeOutputPathTemplate,
    draft.standaloneTranscodeOutputPathTemplate,
  ] as const,
  ([open]) => {
    if (!open) return
    if (previewTimer) clearTimeout(previewTimer)
    previewTimer = setTimeout(() => {
      void refreshPreviews().catch((error) => {
        errorMessage.value = error instanceof Error ? error.message : String(error)
      })
    }, 120)
  },
  { immediate: true },
)

onUnmounted(() => {
  if (previewTimer) clearTimeout(previewTimer)
})

function handleOpenChange(value: boolean) {
  emit('update:open', value)
}

function resetToDefaults() {
  draft.renderOutputPathTemplate = defaultTemplateSettings.renderOutputPathTemplate
  draft.blenderTranscodeOutputPathTemplate = defaultTemplateSettings.blenderTranscodeOutputPathTemplate
  draft.standaloneTranscodeOutputPathTemplate = defaultTemplateSettings.standaloneTranscodeOutputPathTemplate
  errorMessage.value = ''
}

async function saveSettingsDraft() {
  if (saving.value || hasErrors.value) return

  saving.value = true
  errorMessage.value = ''

  try {
    settingsStore.settings.renderOutputPathTemplate = draft.renderOutputPathTemplate.trim()
    settingsStore.settings.blenderTranscodeOutputPathTemplate = draft.blenderTranscodeOutputPathTemplate.trim()
    settingsStore.settings.standaloneTranscodeOutputPathTemplate = draft.standaloneTranscodeOutputPathTemplate.trim()
    await settingsStore.save()
    emit('update:open', false)
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  } finally {
    saving.value = false
  }
}
</script>

<style scoped>
.output-template-modal {
  max-width: 78rem;
}

.output-template-modal-copy {
  display: grid;
  gap: 0.4rem;
}

.template-section {
  align-items: stretch;
  gap: 1rem;
}

.template-section-head {
  display: flex;
  align-items: flex-start;
}

.template-preview-panel {
  display: grid;
  gap: 0.55rem;
}

.template-subtitle {
  margin: 0;
  font-size: 0.78rem;
  font-weight: 700;
  color: var(--ui-text-highlighted);
}

.template-preview-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(16rem, 1fr));
  gap: 0.75rem;
}

.template-preview-card {
  display: grid;
  gap: 0.45rem;
  padding: 0.85rem 0.95rem;
}

.template-preview-label {
  margin: 0;
  color: var(--ui-text-muted);
  font-size: 0.76rem;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.template-preview-value {
  margin: 0;
  color: var(--ui-text-highlighted);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 0.84rem;
  line-height: 1.45;
  word-break: break-all;
}

@media (max-width: 900px) {
  .output-template-modal-copy,
  .template-section-head {
    grid-template-columns: 1fr;
    display: grid;
  }
}
</style>
