<template>
  <UModal
    :open="open"
    :close="false"
    :title="t('settingsModals.ffmpegPath.title')"
    :ui="{ content: 'job-modal-content settings-modal-content' }"
    @update:open="handleOpenChange"
  >
    <template #body>
      <div class="modal-stack">
        <div class="settings-card-header settings-compact-header">
          <div class="settings-field-copy">
            <p class="settings-field-title">{{ t('settingsModals.ffmpegPath.currentTitle') }}</p>
            <p class="hint-text">{{ t('settingsModals.ffmpegPath.currentNote') }}</p>
          </div>
          <div class="settings-card-actions">
            <UButton
              icon="i-lucide-folder-open"
              :label="t('settingsModals.ffmpegPath.choose')"
              color="neutral"
              variant="outline"
              size="sm"
              :loading="picking"
              @click="browseFfmpeg"
            />
          </div>
        </div>

        <UAlert v-if="errorMessage" color="error" variant="subtle" :description="errorMessage" class="surface-alert" />

        <div v-if="settingsStore.settings.ffmpegExecutable" class="surface-panel ffmpeg-config-item">
          <div class="blender-version-info">
            <div class="blender-version-name">FFmpeg</div>
            <div class="blender-version-path" :title="settingsStore.settings.ffmpegExecutable">
              {{ settingsStore.settings.ffmpegExecutable }}
            </div>
          </div>
          <div class="blender-version-actions">
            <UTooltip :text="t('settingsModals.ffmpegPath.removeTooltip')" :content="{ side: 'left', sideOffset: 6 }">
              <UButton
                icon="i-lucide-x"
                color="error"
                variant="outline"
                size="xs"
                square
                :loading="clearing"
                @click="clearFfmpeg"
              />
            </UTooltip>
          </div>
        </div>
        <p v-else class="hint-text">{{ t('settingsModals.ffmpegPath.empty') }}</p>

        <div class="modal-actions settings-modal-actions">
          <div class="settings-modal-actions-start" />
          <div class="settings-modal-actions-end">
            <UButton
              icon="i-lucide-check"
              :label="t('common.done')"
              color="primary"
              variant="solid"
              @click="emit('update:open', false)"
            />
          </div>
        </div>
      </div>
    </template>
  </UModal>
</template>

<script setup lang="ts">
const props = defineProps<{
  open: boolean
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
}>()

const settingsStore = useSettingsStore()
const { t } = useI18n()
const errorMessage = ref('')
const picking = ref(false)
const clearing = ref(false)

watch(
  () => props.open,
  async (open) => {
    if (!open) return
    errorMessage.value = ''
    await settingsStore.load()
  },
  { immediate: true },
)

function handleOpenChange(value: boolean) {
  emit('update:open', value)
}

async function browseFfmpeg() {
  if (picking.value) return
  picking.value = true
  errorMessage.value = ''
  try {
    await settingsStore.browseAndSetFfmpeg()
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  } finally {
    picking.value = false
  }
}

async function clearFfmpeg() {
  if (clearing.value) return
  clearing.value = true
  errorMessage.value = ''
  try {
    await settingsStore.clearFfmpeg()
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  } finally {
    clearing.value = false
  }
}
</script>
