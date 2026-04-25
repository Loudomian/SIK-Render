<template>
  <UModal
    :open="open"
    :close="false"
    title="Blender 渲染参数"
    :ui="{ content: 'job-modal-content settings-modal-content' }"
    @update:open="handleOpenChange"
  >
    <template #body>
      <div class="modal-stack">
        <div class="settings-form-stack">
          <section class="surface-panel settings-field-panel">
            <div class="settings-field-copy">
              <p class="settings-field-title">读取工程超时</p>
              <p class="hint-text">用于读取 `.blend` 工程参数和 FPS，默认 300 秒，范围 30 到 800 秒。</p>
            </div>
            <UFormField>
              <UInputNumber
                v-model="draft.blendInspectTimeoutSeconds"
                :min="30"
                :max="800"
                :step="10"
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
              <p class="settings-field-title">崩溃自动重试次数</p>
              <p class="hint-text">Blender 崩溃后自动续帧重试的最多次数，0 表示不重试，最多 10 次，默认 3 次。</p>
            </div>
            <UFormField>
              <UInputNumber
                v-model="draft.maxCrashRetries"
                :min="0"
                :max="10"
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
const props = defineProps<{
  open: boolean
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
}>()

const settingsStore = useSettingsStore()
const defaultBlenderSettings = {
  blendInspectTimeoutSeconds: 300,
  maxCrashRetries: 3,
}

const draft = reactive({
  blendInspectTimeoutSeconds: defaultBlenderSettings.blendInspectTimeoutSeconds,
  maxCrashRetries: defaultBlenderSettings.maxCrashRetries,
})
const saving = ref(false)
const errorMessage = ref('')

function syncDraft() {
  draft.blendInspectTimeoutSeconds = settingsStore.settings.blendInspectTimeoutSeconds
  draft.maxCrashRetries = settingsStore.settings.maxCrashRetries
  errorMessage.value = ''
}

watch(
  () => [
    props.open,
    settingsStore.settings.blendInspectTimeoutSeconds,
    settingsStore.settings.maxCrashRetries,
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
  draft.blendInspectTimeoutSeconds = defaultBlenderSettings.blendInspectTimeoutSeconds
  draft.maxCrashRetries = defaultBlenderSettings.maxCrashRetries
  errorMessage.value = ''
}

async function saveSettingsDraft() {
  if (saving.value) return

  saving.value = true
  errorMessage.value = ''

  try {
    settingsStore.settings.blendInspectTimeoutSeconds = draft.blendInspectTimeoutSeconds
    settingsStore.settings.maxCrashRetries = draft.maxCrashRetries
    await settingsStore.save()
    emit('update:open', false)
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  } finally {
    saving.value = false
  }
}
</script>
