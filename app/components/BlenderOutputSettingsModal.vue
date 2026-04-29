<template>
  <UModal
    :open="open"
    :close="false"
    title="输出设置"
    :ui="{ content: 'job-modal-content settings-modal-content' }"
    @update:open="handleOpenChange"
  >
    <template #body>
      <div class="modal-stack">
        <div class="settings-form-stack">
          <section class="surface-panel settings-field-panel">
            <div class="settings-field-copy">
              <p class="settings-field-title">PNG 默认输出</p>
              <p class="hint-text">默认使用 RGB，压缩率和色深按 Blender 的常用默认值处理。</p>
            </div>
            <div class="settings-inline-fields settings-inline-fields-png">
              <UFormField label="颜色">
                <USelect
                  v-model="draft.pngColorMode"
                  :items="pngColorModeOptions"
                  :disabled="saving"
                  trailing-icon="i-lucide-chevron-down"
                  :ui="{ base: 'w-full' }"
                />
              </UFormField>
              <UFormField label="色深">
                <USelect
                  v-model="draft.pngColorDepth"
                  :items="pngColorDepthOptions"
                  :disabled="saving"
                  trailing-icon="i-lucide-chevron-down"
                  :ui="{ base: 'w-full' }"
                />
              </UFormField>
              <UFormField label="压缩率">
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
              <p class="settings-field-title">OpenEXR 默认输出</p>
              <p class="hint-text">默认使用 RGB、16-bit、DWAA，质量 98%。选择 OpenEXR 时会禁用转码。</p>
            </div>
            <div class="settings-inline-fields settings-inline-fields-exr">
              <UFormField label="颜色">
                <USelect
                  v-model="draft.exrColorMode"
                  :items="exrColorModeOptions"
                  :disabled="saving"
                  trailing-icon="i-lucide-chevron-down"
                  :ui="{ base: 'w-full' }"
                />
              </UFormField>
              <UFormField label="色深">
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
              <UFormField label="质量">
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
              label="恢复默认"
              color="neutral"
              variant="ghost"
              :disabled="saving"
              @click="resetToDefaults"
            />
          </div>
          <div class="settings-modal-actions-end">
            <UButton
              icon="i-lucide-x"
              label="取消"
              color="neutral"
              variant="outline"
              :disabled="saving"
              @click="emit('update:open', false)"
            />
            <UButton
              icon="i-lucide-save"
              label="保存"
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
