<template>
  <UModal
    :open="open"
    :close="false"
    title="Blender 路径管理"
    :ui="{ content: 'job-modal-content settings-modal-content' }"
    @update:open="handleOpenChange"
  >
    <template #body>
      <div class="modal-stack">
        <div class="settings-card-header settings-compact-header">
          <div class="settings-field-copy">
            <p class="settings-field-title">已安装版本</p>
            <p class="hint-text">管理 Blender 可执行文件、默认版本和备用路径。</p>
          </div>
          <div class="settings-card-actions">
            <UButton
              icon="i-lucide-plus"
              label="添加…"
              color="neutral"
              variant="outline"
              size="sm"
              :loading="adding"
              @click="browseBlender"
            />
          </div>
        </div>

        <UAlert v-if="errorMessage" color="error" variant="subtle" :description="errorMessage" class="surface-alert" />

        <ul v-if="settingsStore.blenderVersions.length" class="settings-blender-list settings-blender-list-compact">
          <li v-for="b in settingsStore.blenderVersions" :key="b.executable" class="surface-panel blender-version-item">
            <div class="blender-version-info">
              <div class="blender-version-name">Blender {{ b.version }}</div>
              <div class="blender-version-path" :title="b.executable">{{ b.executable }}</div>
            </div>
            <div class="blender-version-actions">
              <UButton
                icon="i-lucide-check"
                color="success"
                :variant="settingsStore.settings.defaultBlender === b.executable ? 'subtle' : 'outline'"
                size="xs"
                :label="settingsStore.settings.defaultBlender === b.executable ? '已设为默认' : '设为默认'"
                @click="setDefaultBlender(b.executable)"
              />
              <UTooltip text="移除此版本" :content="{ side: 'left', sideOffset: 6 }">
                <UButton icon="i-lucide-x" color="error" variant="outline" size="xs" square @click="removeBlenderVersion(b.executable)" />
              </UTooltip>
            </div>
          </li>
        </ul>
        <p v-else class="hint-text">还没有 Blender 路径，点击右上角“添加”手动指定。</p>

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
const adding = ref(false)

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

async function browseBlender() {
  if (adding.value) return
  adding.value = true
  errorMessage.value = ''
  try {
    await settingsStore.browseAndAddBlender()
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  } finally {
    adding.value = false
  }
}

async function setDefaultBlender(executable: string) {
  errorMessage.value = ''
  try {
    await settingsStore.setDefaultBlender(executable)
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  }
}

async function removeBlenderVersion(executable: string) {
  errorMessage.value = ''
  try {
    await settingsStore.removeBlenderVersion(executable)
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  }
}
</script>
