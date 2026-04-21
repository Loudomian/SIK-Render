<template>
  <div class="settings-page">
    <section class="page-hero">
      <div class="page-hero-copy">
        <UBadge label="Settings" color="neutral" variant="subtle" class="page-eyebrow" />
        <h1>设置</h1>
        <p class="page-note">集中管理 Blender、FFmpeg 与界面偏好。</p>
      </div>
    </section>

    <UCard variant="subtle" class="settings-card" :ui="{ body: 'settings-card-body' }">
      <div class="settings-card-header">
        <div>
          <h2 class="settings-card-title">Blender 版本</h2>
        </div>
        <div class="settings-card-actions">
          <UButton icon="i-lucide-plus" label="添加…" color="neutral" variant="outline" size="sm" @click="browse" />
        </div>
      </div>

      <UAlert v-if="browseError" color="error" variant="subtle" :description="browseError" class="surface-alert" />

      <ul v-if="settingsStore.blenderVersions.length" class="settings-blender-list">
        <li v-for="b in settingsStore.blenderVersions" :key="b.executable" class="surface-panel blender-version-item">
          <div class="blender-version-info">
            <div class="blender-version-name">
              Blender {{ b.version }}
            </div>
            <div class="blender-version-path" :title="b.executable">{{ b.executable }}</div>
          </div>
          <div class="blender-version-actions">
            <UButton
              icon="i-lucide-check"
              color="success"
              :variant="settingsStore.settings.defaultBlender === b.executable ? 'subtle' : 'outline'"
              size="xs"
              :label="settingsStore.settings.defaultBlender === b.executable ? '默认' : '设为默认'"
              @click="settingsStore.settings.defaultBlender = b.executable"
            />
            <UTooltip text="移除此版本" :content="{ side: 'left', sideOffset: 6 }">
              <UButton icon="i-lucide-x" color="error" variant="outline" size="xs" square @click="settingsStore.removeBlenderVersion(b.executable)" />
            </UTooltip>
          </div>
        </li>
      </ul>
      <p v-else class="hint-text">还没有 Blender 路径，点击右上角“添加”手动指定。</p>
    </UCard>

    <UCard variant="subtle" class="settings-card" :ui="{ body: 'settings-card-body' }">
      <div class="settings-card-header">
        <div>
          <h2 class="settings-card-title">FFmpeg</h2>
        </div>
        <div class="settings-card-actions">
          <UButton icon="i-lucide-folder-open" label="选择…" color="neutral" variant="outline" size="sm" @click="browseFfmpeg" />
        </div>
      </div>

      <UAlert v-if="ffmpegError" color="error" variant="subtle" :description="ffmpegError" class="surface-alert" />

      <div v-if="settingsStore.settings.ffmpegExecutable" class="surface-panel ffmpeg-config-item">
        <div class="blender-version-info">
          <div class="blender-version-name">FFmpeg</div>
          <div class="blender-version-path" :title="settingsStore.settings.ffmpegExecutable">
            {{ settingsStore.settings.ffmpegExecutable }}
          </div>
        </div>
        <div class="blender-version-actions">
          <UBadge label="已指定" color="success" variant="subtle" />
          <UTooltip text="移除 FFmpeg" :content="{ side: 'left', sideOffset: 6 }">
            <UButton icon="i-lucide-x" color="error" variant="outline" size="xs" square @click="settingsStore.clearFfmpeg()" />
          </UTooltip>
        </div>
      </div>
      <p v-else class="hint-text">未指定 FFmpeg，点击右上角“选择”手动指定可执行文件。</p>
    </UCard>

    <div class="settings-grid-2">
      <UCard variant="subtle" class="settings-card" :ui="{ body: 'settings-card-body' }">
        <div class="settings-card-header">
          <div>
            <h2 class="settings-card-title">工具行为</h2>
          </div>
        </div>

        <div class="settings-form-stack">
          <section class="surface-panel settings-field-panel">
            <div class="settings-field-copy">
              <p class="settings-field-title">读取工程超时</p>
              <p class="hint-text">用于读取 `.blend` 工程参数和 FPS，默认 30 秒。</p>
            </div>
            <UFormField>
              <UInputNumber
                v-model="settingsStore.settings.blendInspectTimeoutSeconds"
                :min="5"
                :max="600"
                :step="5"
                orientation="horizontal"
                decrement-icon="i-lucide-minus"
                increment-icon="i-lucide-plus"
                :ui="{ root: 'w-32' }"
              />
            </UFormField>
          </section>
        </div>
      </UCard>

      <UCard variant="subtle" class="settings-card" :ui="{ body: 'settings-card-body' }">
        <div class="settings-card-header">
          <div>
            <h2 class="settings-card-title">外观与应用信息</h2>
          </div>
        </div>

        <div class="settings-form-stack">
          <section class="surface-panel settings-field-panel">
            <div class="settings-field-copy">
              <p class="settings-field-title">界面主题</p>
            </div>
            <UFormField>
              <ColorModeSelect v-model="settingsStore.settings.theme" />
            </UFormField>
          </section>

          <section class="surface-panel settings-field-panel">
            <div class="settings-field-copy">
              <p class="settings-field-title">当前版本</p>
            </div>
            <p class="settings-version-value">v0.1.0</p>
          </section>
        </div>
      </UCard>
    </div>
  </div>
</template>

<script setup lang="ts">
const settingsStore = useSettingsStore()
const browseError = ref('')
const ffmpegError = ref('')
const settingsReady = ref(false)
let autoSaveTimer: ReturnType<typeof setTimeout> | null = null

async function browse() {
  browseError.value = ''
  try {
    await settingsStore.browseAndAddBlender()
  } catch (e) {
    browseError.value = e instanceof Error ? e.message : String(e)
  }
}

async function browseFfmpeg() {
  ffmpegError.value = ''
  try {
    await settingsStore.browseAndSetFfmpeg()
  } catch (e) {
    ffmpegError.value = e instanceof Error ? e.message : String(e)
  }
}

watch(
  () => [
    settingsStore.settings.defaultBlender,
    settingsStore.settings.blendInspectTimeoutSeconds,
    settingsStore.settings.theme,
  ] as const,
  () => {
    if (!settingsReady.value) return
    if (autoSaveTimer) clearTimeout(autoSaveTimer)
    autoSaveTimer = setTimeout(() => {
      void settingsStore.save()
    }, 240)
  },
)

onMounted(async () => {
  await settingsStore.load()
  settingsReady.value = true
})

onUnmounted(() => {
  if (autoSaveTimer) clearTimeout(autoSaveTimer)
})
</script>
