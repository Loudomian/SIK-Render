<template>
  <div class="settings-page">
    <section class="page-hero">
      <div class="page-hero-copy">
        <UBadge label="Settings" color="neutral" variant="subtle" class="page-eyebrow" />
        <h1>设置</h1>
        <p class="page-note">集中管理 Blender、FFmpeg 与界面偏好。</p>
      </div>
    </section>

    <section class="settings-section">
      <div class="settings-section-heading">
        <h2 class="settings-section-title">Blender 渲染</h2>
        <p class="hint-text">管理 Blender 版本与渲染相关默认行为。</p>
      </div>

      <UCard variant="subtle" class="settings-card" :ui="{ body: 'settings-card-body' }">
        <div class="settings-card-header">
          <div>
            <h3 class="settings-card-title">Blender 版本</h3>
          </div>
          <div class="settings-card-actions">
            <UButton icon="i-lucide-plus" label="添加…" color="neutral" variant="outline" size="sm" @click="browseBlender" />
          </div>
        </div>

        <UAlert v-if="blenderError" color="error" variant="subtle" :description="blenderError" class="surface-alert" />

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
                @click="setDefaultBlender(b.executable)"
              />
              <UTooltip text="移除此版本" :content="{ side: 'left', sideOffset: 6 }">
                <UButton icon="i-lucide-x" color="error" variant="outline" size="xs" square @click="removeBlenderVersion(b.executable)" />
              </UTooltip>
            </div>
          </li>
        </ul>
        <p v-else class="hint-text">还没有 Blender 路径，点击右上角“添加”手动指定。</p>
      </UCard>

      <div class="settings-grid-2">
        <UCard variant="subtle" class="settings-card" :ui="{ body: 'settings-card-body' }">
          <div class="settings-card-header">
            <div>
              <h3 class="settings-card-title">Blender 渲染参数</h3>
            </div>
            <div class="settings-card-actions">
              <UButton icon="i-lucide-sliders" label="编辑" color="neutral" variant="outline" size="sm" @click="blenderModalOpen = true" />
            </div>
          </div>

          <section class="surface-panel settings-summary-panel">
            <div class="settings-field-copy">
              <p class="settings-field-title">当前摘要</p>
              <p class="settings-summary-text">{{ blenderSettingsSummary }}</p>
            </div>
          </section>
        </UCard>
      </div>
    </section>

    <section class="settings-section">
      <div class="settings-section-heading">
        <h2 class="settings-section-title">FFmpeg 转码</h2>
        <p class="hint-text">管理 FFmpeg 路径与默认转码参数。</p>
      </div>

      <UCard variant="subtle" class="settings-card" :ui="{ body: 'settings-card-body' }">
        <div class="settings-card-header">
          <div>
            <h3 class="settings-card-title">FFmpeg</h3>
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
              <UButton icon="i-lucide-x" color="error" variant="outline" size="xs" square @click="clearFfmpeg" />
            </UTooltip>
          </div>
        </div>
        <p v-else class="hint-text">未指定 FFmpeg，点击右上角“选择”手动指定可执行文件。</p>
      </UCard>

      <div class="settings-grid-2">
        <UCard variant="subtle" class="settings-card" :ui="{ body: 'settings-card-body' }">
          <div class="settings-card-header">
            <div>
              <h3 class="settings-card-title">FFmpeg 转码参数</h3>
            </div>
            <div class="settings-card-actions">
              <UButton icon="i-lucide-sliders" label="编辑" color="neutral" variant="outline" size="sm" @click="ffmpegModalOpen = true" />
            </div>
          </div>

          <section class="surface-panel settings-summary-panel">
            <div class="settings-field-copy">
              <p class="settings-field-title">当前摘要</p>
              <p class="settings-summary-text">{{ ffmpegSettingsSummary }}</p>
            </div>
          </section>
        </UCard>
      </div>
    </section>

    <section class="settings-section">
      <div class="settings-section-heading">
        <h2 class="settings-section-title">外观与关于</h2>
      </div>

      <UCard variant="subtle" class="settings-card" :ui="{ body: 'settings-card-body' }">
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
            <p class="settings-version-value">v{{ appVersion }}</p>
          </section>

          <section class="surface-panel settings-field-panel">
            <div class="settings-field-copy">
              <p class="settings-field-title">版权信息</p>
              <p class="settings-brand-line">SIKFILM · 灌灌小狗</p>
              <p class="hint-text">Made with ❤️ by Loudomian and 空气小怪.</p>
            </div>
          </section>
        </div>
      </UCard>
    </section>

    <BlenderSettingsModal v-model:open="blenderModalOpen" />
    <FfmpegSettingsModal v-model:open="ffmpegModalOpen" />
  </div>
</template>

<script setup lang="ts">
import { getVersion } from '@tauri-apps/api/app'

const settingsStore = useSettingsStore()
const runtimeConfig = useRuntimeConfig()
const blenderError = ref('')
const ffmpegError = ref('')
const blenderModalOpen = ref(false)
const ffmpegModalOpen = ref(false)
const settingsReady = ref(false)
const appVersion = ref(String(runtimeConfig.public.appVersion ?? '0.0.0'))
let autoSaveTimer: ReturnType<typeof setTimeout> | null = null

const blenderSettingsSummary = computed(() =>
  `超时 ${settingsStore.settings.blendInspectTimeoutSeconds}s · 崩溃重试 ${settingsStore.settings.maxCrashRetries} 次`,
)

const ffmpegSettingsSummary = computed(() =>
  `CRF ${settingsStore.settings.transcodeCrf} · ${settingsStore.settings.transcodePreset} · 并发 ${settingsStore.settings.ffmpegMaxConcurrent}`,
)

async function browseBlender() {
  blenderError.value = ''
  try {
    await settingsStore.browseAndAddBlender()
  } catch (error) {
    blenderError.value = error instanceof Error ? error.message : String(error)
  }
}

async function setDefaultBlender(executable: string) {
  blenderError.value = ''
  try {
    await settingsStore.setDefaultBlender(executable)
  } catch (error) {
    blenderError.value = error instanceof Error ? error.message : String(error)
  }
}

async function removeBlenderVersion(executable: string) {
  blenderError.value = ''
  try {
    await settingsStore.removeBlenderVersion(executable)
  } catch (error) {
    blenderError.value = error instanceof Error ? error.message : String(error)
  }
}

async function browseFfmpeg() {
  ffmpegError.value = ''
  try {
    await settingsStore.browseAndSetFfmpeg()
  } catch (error) {
    ffmpegError.value = error instanceof Error ? error.message : String(error)
  }
}

async function clearFfmpeg() {
  ffmpegError.value = ''
  try {
    await settingsStore.clearFfmpeg()
  } catch (error) {
    ffmpegError.value = error instanceof Error ? error.message : String(error)
  }
}

watch(
  () => settingsStore.settings.theme,
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

  try {
    appVersion.value = await getVersion()
  } catch {
    // Browser dev mode falls back to the package version from runtime config.
  }
})

onUnmounted(() => {
  if (autoSaveTimer) clearTimeout(autoSaveTimer)
})
</script>
