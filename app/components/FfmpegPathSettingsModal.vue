<template>
  <UModal
    :open="open"
    :close="false"
    title="FFmpeg 路径管理"
    :ui="{ content: 'job-modal-content settings-modal-content' }"
    @update:open="handleOpenChange"
  >
    <template #body>
      <div class="modal-stack">
        <div class="settings-card-header settings-compact-header">
          <div class="settings-field-copy">
            <p class="settings-field-title">当前可执行文件</p>
            <p class="hint-text">用于提交和执行转码任务，支持随时替换或清空。</p>
          </div>
          <div class="settings-card-actions">
            <UButton
              icon="i-lucide-folder-open"
              label="选择…"
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
            <UTooltip text="移除 FFmpeg" :content="{ side: 'left', sideOffset: 6 }">
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
        <p v-else class="hint-text">未指定 FFmpeg，点击右上角“选择”手动指定可执行文件。</p>

        <div class="modal-actions settings-modal-actions">
          <div class="settings-modal-actions-start" />
          <div class="settings-modal-actions-end">
            <UButton
              icon="i-lucide-check"
              label="完成"
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
