<template>
  <UModal
    :open="open"
    :close="false"
    :title="t('settingsModals.appearance.title')"
    :ui="{ content: 'job-modal-content settings-modal-content' }"
    @update:open="handleOpenChange"
  >
    <template #body>
      <div class="modal-stack">
        <div class="settings-form-stack">
          <section class="surface-panel settings-field-panel">
            <div class="settings-field-copy">
              <p class="settings-field-title">{{ t('settingsModals.appearance.themeTitle') }}</p>
              <p class="hint-text">{{ t('settingsModals.appearance.themeNote') }}</p>
            </div>
            <UFormField>
              <ColorModeSelect v-model="draftTheme" />
            </UFormField>
          </section>
        </div>

        <div class="modal-actions settings-modal-actions">
          <div class="settings-modal-actions-start" />
          <div class="settings-modal-actions-end">
            <UButton
              icon="i-lucide-x"
              :label="t('common.cancel')"
              color="neutral"
              variant="outline"
              @click="emit('update:open', false)"
            />
            <UButton
              icon="i-lucide-save"
              :label="t('common.save')"
              color="primary"
              variant="solid"
              @click="saveAppearance"
            />
          </div>
        </div>
      </div>
    </template>
  </UModal>
</template>

<script setup lang="ts">
import type { AppSettings } from '~/types'

const props = defineProps<{
  open: boolean
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
}>()

const settingsStore = useSettingsStore()
const { t } = useI18n()
const draftTheme = ref<AppSettings['theme']>('dark')

watch(
  () => props.open,
  (open) => {
    if (!open) return
    draftTheme.value = settingsStore.settings.theme
  },
  { immediate: true },
)

function handleOpenChange(value: boolean) {
  emit('update:open', value)
}

async function saveAppearance() {
  settingsStore.settings.theme = draftTheme.value
  await settingsStore.save()
  emit('update:open', false)
}
</script>
