<template>
  <UModal
    :open="open"
    :close="false"
    title="界面与外观"
    :ui="{ content: 'job-modal-content settings-modal-content' }"
    @update:open="handleOpenChange"
  >
    <template #body>
      <div class="modal-stack">
        <div class="settings-form-stack">
          <section class="surface-panel settings-field-panel">
            <div class="settings-field-copy">
              <p class="settings-field-title">界面主题</p>
              <p class="hint-text">切换浅色和深色主题，应用会立即生效。</p>
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
              label="取消"
              color="neutral"
              variant="outline"
              @click="emit('update:open', false)"
            />
            <UButton
              icon="i-lucide-save"
              label="保存"
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
