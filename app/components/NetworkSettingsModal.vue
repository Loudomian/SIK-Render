<template>
  <UModal
    :open="open"
    :close="false"
    title="节点网络参数"
    :ui="{ content: 'job-modal-content settings-modal-content' }"
    @update:open="handleOpenChange"
  >
    <template #body>
      <div class="modal-stack">
        <div class="settings-form-stack">
          <section class="surface-panel settings-field-panel">
            <div class="settings-field-copy">
              <p class="settings-field-title">设备备注</p>
              <p class="hint-text">显示在渲染节点卡片上，用于区分机器用途或位置，最多 80 个字符。</p>
            </div>
            <UFormField>
              <UInput
                v-model="draft.nodeNote"
                placeholder="例如：主力渲染机 / 二楼工作站"
                :maxlength="80"
                :disabled="saving"
                class="settings-network-input"
              />
            </UFormField>
          </section>

          <section class="surface-panel settings-field-panel">
            <div class="settings-field-copy">
              <p class="settings-field-title">节点端口</p>
              <p class="hint-text">HTTP 与 WebSocket 监听端口，默认 47878。保存后重启应用生效。</p>
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
              <p class="settings-field-title">网卡网段</p>
              <p class="hint-text">用于多网卡机器选择对外公布的 IPv4，可填写网关或同网段参考地址。</p>
            </div>
            <UFormField>
              <UInput
                v-model="draft.nodeInterfaceAddress"
                placeholder="例如：192.168.1.1"
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
