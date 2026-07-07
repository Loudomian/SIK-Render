<template>
  <UModal
    :open="open"
    :close="false"
    :title="t('settingsModals.ffmpegTranscode.title')"
    :ui="{ content: 'job-modal-content settings-modal-content' }"
    @update:open="handleOpenChange"
  >
    <template #body>
      <div class="modal-stack">
        <div class="settings-form-stack">
          <section class="surface-panel settings-field-panel">
            <div class="settings-field-copy">
              <p class="settings-field-title">{{ t('settingsModals.ffmpegTranscode.crfTitle') }}</p>
              <p class="hint-text">{{ t('settingsModals.ffmpegTranscode.crfNote') }}</p>
            </div>
            <UFormField>
              <UInputNumber
                v-model="draft.transcodeCrf"
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
          </section>

          <section class="surface-panel settings-field-panel">
            <div class="settings-field-copy">
              <p class="settings-field-title">{{ t('settingsModals.ffmpegTranscode.presetTitle') }}</p>
              <p class="hint-text">{{ t('settingsModals.ffmpegTranscode.presetNote') }}</p>
            </div>
            <UFormField>
              <USelect
                v-model="draft.transcodePreset"
                :items="transcodePresetOptions"
                :disabled="saving"
                trailing-icon="i-lucide-chevron-down"
                :ui="{ base: 'w-40' }"
              />
            </UFormField>
          </section>

          <section class="surface-panel settings-field-panel">
            <div class="settings-field-copy">
              <p class="settings-field-title">{{ t('settingsModals.ffmpegTranscode.encoderTitle') }}</p>
              <p
                v-for="line in encoderDetectionLines"
                :key="line"
                class="hint-text"
              >
                {{ line }}
              </p>
            </div>
            <div class="settings-card-actions">
              <UFormField>
                <USelect
                  v-model="draft.transcodeEncoder"
                  :items="transcodeEncoderOptions"
                  :disabled="saving"
                  trailing-icon="i-lucide-chevron-down"
                  :ui="{ base: 'w-48' }"
                />
              </UFormField>
              <UButton
                icon="i-lucide-refresh-cw"
                :label="t('settingsModals.ffmpegTranscode.detectEncoders')"
                color="neutral"
                variant="ghost"
                size="sm"
                :loading="detectingEncoders"
                :disabled="saving"
                @click="detectEncoders(true)"
              />
            </div>
          </section>

          <section class="surface-panel settings-field-panel">
            <div class="settings-field-copy">
              <p class="settings-field-title">{{ t('settingsModals.ffmpegTranscode.concurrentTitle') }}</p>
              <p class="hint-text">{{ t('settingsModals.ffmpegTranscode.concurrentNote') }}</p>
            </div>
            <UFormField>
              <UInputNumber
                v-model="draft.ffmpegMaxConcurrent"
                :min="1"
                :max="8"
                :step="1"
                :disabled="saving"
                orientation="horizontal"
                decrement-icon="i-lucide-minus"
                increment-icon="i-lucide-plus"
                :ui="{ root: 'w-32' }"
              />
            </UFormField>
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
import type { SelectItem } from '@nuxt/ui'
import type { TranscodeEncoder, VideoEncoderDetection } from '~/types'

const props = defineProps<{
  open: boolean
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
}>()

const settingsStore = useSettingsStore()
const { t, locale } = useI18n()
const { detectVideoEncoders } = useTauri()
const defaultFfmpegSettings = {
  transcodeCrf: 18,
  transcodePreset: 'medium',
  transcodeEncoder: 'auto' as TranscodeEncoder,
  ffmpegMaxConcurrent: 2,
}

const transcodePresetOptions = [
  'ultrafast',
  'superfast',
  'veryfast',
  'faster',
  'fast',
  'medium',
  'slow',
  'slower',
  'veryslow',
]
const transcodeEncoderOptions = computed<SelectItem[]>(() =>
  TRANSCODE_ENCODER_OPTIONS.map(value => ({
    label: t(`settingsModals.ffmpegTranscode.encoderOptions.${value}`),
    value,
  })),
)

const draft = reactive({
  transcodeCrf: defaultFfmpegSettings.transcodeCrf,
  transcodePreset: defaultFfmpegSettings.transcodePreset,
  transcodeEncoder: defaultFfmpegSettings.transcodeEncoder,
  ffmpegMaxConcurrent: defaultFfmpegSettings.ffmpegMaxConcurrent,
})
const saving = ref(false)
const detectingEncoders = ref(false)
const encoderDetections = ref<VideoEncoderDetection[]>([])
const errorMessage = ref('')

const encoderDetectionLines = computed(() => {
  if (detectingEncoders.value) return [t('settingsModals.ffmpegTranscode.detectingEncoders')]
  if (!encoderDetections.value.length) return [t('settingsModals.ffmpegTranscode.detectUnknown')]

  const separator = locale.value.startsWith('zh') ? '、' : ', '
  const available = encoderDetections.value.filter(item => item.available).map(item => item.label)
  const unavailable = encoderDetections.value.filter(item => !item.available).map(item => item.label)

  return [
    t('settingsModals.ffmpegTranscode.detectAvailable', {
      available: available.length ? available.join(separator) : t('common.none'),
    }),
    t('settingsModals.ffmpegTranscode.detectUnavailable', {
      unavailable: unavailable.length ? unavailable.join(separator) : t('common.none'),
    }),
  ]
})

function syncDraft() {
  draft.transcodeCrf = settingsStore.settings.transcodeCrf
  draft.transcodePreset = settingsStore.settings.transcodePreset
  draft.transcodeEncoder = settingsStore.settings.transcodeEncoder
  draft.ffmpegMaxConcurrent = settingsStore.settings.ffmpegMaxConcurrent
  errorMessage.value = ''
}

async function detectEncoders(force = false) {
  if (detectingEncoders.value) return
  detectingEncoders.value = true
  try {
    encoderDetections.value = await detectVideoEncoders(force)
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  } finally {
    detectingEncoders.value = false
  }
}

watch(
  () => [
    props.open,
    settingsStore.settings.transcodeCrf,
    settingsStore.settings.transcodePreset,
    settingsStore.settings.transcodeEncoder,
    settingsStore.settings.ffmpegMaxConcurrent,
  ] as const,
  ([open]) => {
    if (!open) return
    syncDraft()
    void detectEncoders(false)
  },
  { immediate: true },
)

function handleOpenChange(value: boolean) {
  emit('update:open', value)
}

function resetToDefaults() {
  draft.transcodeCrf = defaultFfmpegSettings.transcodeCrf
  draft.transcodePreset = defaultFfmpegSettings.transcodePreset
  draft.transcodeEncoder = defaultFfmpegSettings.transcodeEncoder
  draft.ffmpegMaxConcurrent = defaultFfmpegSettings.ffmpegMaxConcurrent
  errorMessage.value = ''
}

async function saveSettingsDraft() {
  if (saving.value) return

  saving.value = true
  errorMessage.value = ''

  try {
    settingsStore.settings.transcodeCrf = draft.transcodeCrf
    settingsStore.settings.transcodePreset = draft.transcodePreset
    settingsStore.settings.transcodeEncoder = draft.transcodeEncoder
    settingsStore.settings.ffmpegMaxConcurrent = draft.ffmpegMaxConcurrent
    await settingsStore.save()
    emit('update:open', false)
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  } finally {
    saving.value = false
  }
}
</script>
