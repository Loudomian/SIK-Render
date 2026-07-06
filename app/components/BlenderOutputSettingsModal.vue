<template>
  <UModal
    :open="open"
    :close="false"
    :title="t('settingsModals.blenderOutput.title')"
    :ui="{ content: 'job-modal-content settings-modal-content' }"
    @update:open="handleOpenChange"
  >
    <template #body>
      <div class="modal-stack">
        <div class="settings-form-stack">
          <section class="surface-panel settings-field-panel">
            <div class="settings-field-copy">
              <p class="settings-field-title">{{ t('settingsModals.blenderOutput.pngTitle') }}</p>
              <p class="hint-text">{{ t('settingsModals.blenderOutput.pngNote') }}</p>
            </div>
            <div class="settings-inline-fields settings-inline-fields-png">
              <UFormField :label="t('settingsModals.blenderOutput.color')">
                <USelect
                  v-model="draft.pngColorMode"
                  :items="pngColorModeOptions"
                  :disabled="saving"
                  trailing-icon="i-lucide-chevron-down"
                  :ui="{ base: 'w-full' }"
                />
              </UFormField>
              <UFormField :label="t('settingsModals.blenderOutput.colorDepth')">
                <USelect
                  v-model="draft.pngColorDepth"
                  :items="pngColorDepthOptions"
                  :disabled="saving"
                  trailing-icon="i-lucide-chevron-down"
                  :ui="{ base: 'w-full' }"
                />
              </UFormField>
              <UFormField :label="t('settingsModals.blenderOutput.compression')">
                <UInputNumber
                  v-model="draft.pngCompression"
                  :min="0"
                  :max="100"
                  :step="1"
                  :disabled="saving"
                  orientation="horizontal"
                  decrement-icon="i-lucide-minus"
                  increment-icon="i-lucide-plus"
                  :ui="{ root: 'w-full' }"
                />
              </UFormField>
            </div>
          </section>

          <section class="surface-panel settings-field-panel">
            <div class="settings-field-copy">
              <p class="settings-field-title">{{ t('settingsModals.blenderOutput.exrTitle') }}</p>
              <p class="hint-text">{{ t('settingsModals.blenderOutput.exrNote') }}</p>
            </div>
            <div class="settings-inline-fields settings-inline-fields-exr">
              <UFormField :label="t('settingsModals.blenderOutput.color')">
                <USelect
                  v-model="draft.exrColorMode"
                  :items="exrColorModeOptions"
                  :disabled="saving"
                  trailing-icon="i-lucide-chevron-down"
                  :ui="{ base: 'w-full' }"
                />
              </UFormField>
              <UFormField :label="t('settingsModals.blenderOutput.colorDepth')">
                <USelect
                  v-model="draft.exrColorDepth"
                  :items="exrColorDepthOptions"
                  :disabled="saving"
                  trailing-icon="i-lucide-chevron-down"
                  :ui="{ base: 'w-full' }"
                />
              </UFormField>
              <UFormField label="Codec">
                <USelect
                  v-model="draft.exrCodec"
                  :items="exrCodecOptions"
                  :disabled="saving"
                  trailing-icon="i-lucide-chevron-down"
                  :ui="{ base: 'w-full' }"
                />
              </UFormField>
              <UFormField :label="t('settingsModals.blenderOutput.quality')">
                <UInputNumber
                  v-model="draft.exrQuality"
                  :min="0"
                  :max="100"
                  :step="1"
                  :disabled="saving"
                  orientation="horizontal"
                  decrement-icon="i-lucide-minus"
                  increment-icon="i-lucide-plus"
                  :ui="{ root: 'w-full' }"
                />
              </UFormField>
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
              @click="saveSettingsDraft"
            />
          </div>
        </div>
      </div>
    </template>
  </UModal>
</template>

<script setup lang="ts">
type ColorModeOption = 'BW' | 'RGB' | 'RGBA'
type ExrCodecOption = 'NONE' | 'ZIP' | 'PIZ' | 'DWAA' | 'DWAB' | 'ZIPS' | 'RLE' | 'PXR24' | 'B44' | 'B44A'

const pngColorModeOptions: ColorModeOption[] = ['BW', 'RGB', 'RGBA']
const pngColorDepthOptions: Array<{ label: string, value: number }> = [
  { label: '8-bit', value: 8 },
  { label: '16-bit', value: 16 },
]
const exrColorModeOptions: ColorModeOption[] = ['BW', 'RGB', 'RGBA']
const exrColorDepthOptions: Array<{ label: string, value: number }> = [
  { label: '16-bit', value: 16 },
  { label: '32-bit', value: 32 },
]
const exrCodecOptions: ExrCodecOption[] = ['NONE', 'ZIP', 'PIZ', 'DWAA', 'DWAB', 'ZIPS', 'RLE', 'PXR24', 'B44', 'B44A']
const defaultOutputSettings = {
  pngColorMode: 'RGB' as const,
  pngColorDepth: 8,
  pngCompression: 15,
  exrColorMode: 'RGB' as const,
  exrColorDepth: 16,
  exrCodec: 'DWAA' as const,
  exrQuality: 98,
}

const props = defineProps<{
  open: boolean
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
}>()

const settingsStore = useSettingsStore()
const { t } = useI18n()

const draft = reactive({
  pngColorMode: defaultOutputSettings.pngColorMode as ColorModeOption,
  pngColorDepth: defaultOutputSettings.pngColorDepth,
  pngCompression: defaultOutputSettings.pngCompression,
  exrColorMode: defaultOutputSettings.exrColorMode as ColorModeOption,
  exrColorDepth: defaultOutputSettings.exrColorDepth,
  exrCodec: defaultOutputSettings.exrCodec as ExrCodecOption,
  exrQuality: defaultOutputSettings.exrQuality,
})
const saving = ref(false)
const errorMessage = ref('')

function syncDraft() {
  draft.pngColorMode = settingsStore.settings.pngColorMode
  draft.pngColorDepth = settingsStore.settings.pngColorDepth
  draft.pngCompression = settingsStore.settings.pngCompression
  draft.exrColorMode = settingsStore.settings.exrColorMode
  draft.exrColorDepth = settingsStore.settings.exrColorDepth
  draft.exrCodec = settingsStore.settings.exrCodec
  draft.exrQuality = settingsStore.settings.exrQuality
  errorMessage.value = ''
}

watch(
  () => [
    props.open,
    settingsStore.settings.pngColorMode,
    settingsStore.settings.pngColorDepth,
    settingsStore.settings.pngCompression,
    settingsStore.settings.exrColorMode,
    settingsStore.settings.exrColorDepth,
    settingsStore.settings.exrCodec,
    settingsStore.settings.exrQuality,
  ] as const,
  ([open]) => {
    if (!open) return
    syncDraft()
  },
  { immediate: true },
)

function handleOpenChange(value: boolean) {
  emit('update:open', value)
}

function resetToDefaults() {
  draft.pngColorMode = defaultOutputSettings.pngColorMode
  draft.pngColorDepth = defaultOutputSettings.pngColorDepth
  draft.pngCompression = defaultOutputSettings.pngCompression
  draft.exrColorMode = defaultOutputSettings.exrColorMode
  draft.exrColorDepth = defaultOutputSettings.exrColorDepth
  draft.exrCodec = defaultOutputSettings.exrCodec
  draft.exrQuality = defaultOutputSettings.exrQuality
  errorMessage.value = ''
}

async function saveSettingsDraft() {
  if (saving.value) return

  saving.value = true
  errorMessage.value = ''

  try {
    settingsStore.settings.pngColorMode = draft.pngColorMode
    settingsStore.settings.pngColorDepth = draft.pngColorDepth
    settingsStore.settings.pngCompression = draft.pngCompression
    settingsStore.settings.exrColorMode = draft.exrColorMode
    settingsStore.settings.exrColorDepth = draft.exrColorDepth
    settingsStore.settings.exrCodec = draft.exrCodec
    settingsStore.settings.exrQuality = draft.exrQuality
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
.settings-inline-fields {
  display: grid;
  gap: 0.8rem;
  align-items: end;
}

.settings-inline-fields-png {
  grid-template-columns: repeat(3, minmax(0, 1fr));
}

.settings-inline-fields-exr {
  grid-template-columns: repeat(4, minmax(0, 1fr));
}

@media (max-width: 900px) {
  .settings-inline-fields-png,
  .settings-inline-fields-exr {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 640px) {
  .settings-inline-fields-png,
  .settings-inline-fields-exr {
    grid-template-columns: 1fr;
  }
}
</style>
