<template>
  <UModal
    :open="open"
    :close="false"
    :title="t('settingsModals.network.title')"
    :ui="{ content: 'job-modal-content settings-modal-content' }"
    @update:open="handleOpenChange"
  >
    <template #body>
      <div class="modal-stack">
        <div class="settings-form-stack">
          <section class="surface-panel settings-field-panel">
            <div class="settings-field-copy">
              <p class="settings-field-title">{{ t('settingsModals.network.nodeNoteTitle') }}</p>
              <p class="hint-text">{{ t('settingsModals.network.nodeNoteNote') }}</p>
            </div>
            <UFormField>
              <UInput
                v-model="draft.nodeNote"
                :placeholder="t('settingsModals.network.nodeNotePlaceholder')"
                :maxlength="80"
                :disabled="saving"
                class="settings-network-input"
              />
            </UFormField>
          </section>

          <section class="surface-panel settings-field-panel">
            <div class="settings-field-copy">
              <p class="settings-field-title">{{ t('settingsModals.network.nodePortTitle') }}</p>
              <p class="hint-text">{{ t('settingsModals.network.nodePortNote') }}</p>
            </div>
            <UFormField>
              <UInputNumber
                v-model="draft.nodePort"
                :min="1"
                :max="65535"
                :step="1"
                :disabled="saving"
                orientation="horizontal"
                decrement-icon="i-lucide-minus"
                increment-icon="i-lucide-plus"
                :ui="{ root: 'w-36' }"
              />
            </UFormField>
          </section>

          <section class="surface-panel settings-field-panel">
            <div class="settings-field-copy">
              <p class="settings-field-title">{{ t('settingsModals.network.interfaceTitle') }}</p>
              <p class="hint-text">{{ t('settingsModals.network.interfaceNote') }}</p>
            </div>
            <UFormField>
              <UInput
                v-model="draft.nodeInterfaceAddress"
                :placeholder="t('settingsModals.network.interfacePlaceholder')"
                :disabled="saving"
                class="settings-network-input"
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
const props = defineProps<{
  open: boolean
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
}>()

const settingsStore = useSettingsStore()
const { t } = useI18n()
const defaults = {
  nodePort: 47878,
  nodeInterfaceAddress: '192.168.1.1',
  nodeNote: '',
}

const draft = reactive({ ...defaults })
const saving = ref(false)
const errorMessage = ref('')

function syncDraft() {
  draft.nodePort = settingsStore.settings.nodePort
  draft.nodeInterfaceAddress = settingsStore.settings.nodeInterfaceAddress
  draft.nodeNote = settingsStore.settings.nodeNote
  errorMessage.value = ''
}

watch(
  () => [
    props.open,
    settingsStore.settings.nodePort,
    settingsStore.settings.nodeInterfaceAddress,
    settingsStore.settings.nodeNote,
  ] as const,
  ([open]) => {
    if (open) syncDraft()
  },
  { immediate: true },
)

function handleOpenChange(value: boolean) {
  emit('update:open', value)
}

function resetToDefaults() {
  draft.nodePort = defaults.nodePort
  draft.nodeInterfaceAddress = defaults.nodeInterfaceAddress
  draft.nodeNote = defaults.nodeNote
  errorMessage.value = ''
}

async function saveSettingsDraft() {
  if (saving.value) return

  saving.value = true
  errorMessage.value = ''

  try {
    settingsStore.settings.nodePort = draft.nodePort
    settingsStore.settings.nodeInterfaceAddress = draft.nodeInterfaceAddress.trim()
    settingsStore.settings.nodeNote = draft.nodeNote.trim()
    await settingsStore.save()
    emit('update:open', false)
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  } finally {
    saving.value = false
  }
}
</script>
