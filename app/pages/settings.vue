<template>
  <div class="settings-page">
    <section class="page-hero settings-header">
      <div class="page-hero-copy">
        <UBadge label="Settings" color="neutral" variant="subtle" class="page-eyebrow" />
        <h1>设置</h1>
        <p class="page-note">按路径和参数分组管理工具配置，所有细项都通过弹窗调整。</p>
      </div>
    </section>

    <section class="settings-content">
      <section class="settings-section">
        <div class="settings-section-heading">
          <h2 class="settings-section-title">路径管理</h2>
          <p class="hint-text">集中管理 Blender 与 FFmpeg 的可执行文件和默认版本。</p>
        </div>

        <UCard variant="subtle" class="settings-card" :ui="{ body: 'settings-card-body' }">
          <div class="settings-card-header">
            <div>
              <h3 class="settings-card-title">工具路径</h3>
            </div>
          </div>

          <div class="settings-path-grid">
            <section class="surface-panel settings-path-panel">
              <div class="settings-path-copy">
                <p class="settings-field-title">
                  Blender
                  <span v-if="blenderVersionItems.length" class="settings-inline-versions">
                    <template v-for="(item, index) in blenderVersionItems" :key="item.executable">
                      <span class="job-meta-divider settings-inline-separator" v-if="index > 0" aria-hidden="true" />
                      <span
                        class="settings-inline-version"
                        :class="{ 'settings-inline-version-muted': !item.isDefault }"
                      >
                        {{ item.version }}
                      </span>
                    </template>
                  </span>
                </p>
                <p class="hint-text">{{ blenderPathNote }}</p>
              </div>
              <div class="settings-card-actions">
                <UButton icon="i-lucide-folder-open" label="管理路径" color="neutral" variant="outline" size="sm" @click="blenderPathModalOpen = true" />
              </div>
            </section>

            <section class="surface-panel settings-path-panel">
              <div class="settings-path-copy">
                <p class="settings-field-title">FFmpeg</p>
                <p class="hint-text">{{ ffmpegPathNote }}</p>
              </div>
              <div class="settings-card-actions">
                <UButton icon="i-lucide-folder-open" label="管理路径" color="neutral" variant="outline" size="sm" @click="ffmpegPathModalOpen = true" />
              </div>
            </section>
          </div>
        </UCard>
      </section>

      <section class="settings-section">
      <div class="settings-section-heading">
        <h2 class="settings-section-title">参数管理</h2>
        <p class="hint-text">按功能查看当前摘要，需要修改时进入对应弹窗。</p>
      </div>

      <div class="settings-grid-2">
        <UCard variant="subtle" class="settings-card" :ui="{ body: 'settings-card-body' }">
          <div class="settings-card-header">
            <div>
              <h3 class="settings-card-title">Blender 渲染参数</h3>
              <p class="hint-text">读取工程超时与崩溃自动重试。</p>
            </div>
          </div>

          <div class="settings-card-actions">
            <UButton icon="i-lucide-sliders" label="编辑" color="neutral" variant="outline" size="sm" @click="blenderModalOpen = true" />
          </div>
        </UCard>

        <UCard variant="subtle" class="settings-card" :ui="{ body: 'settings-card-body' }">
          <div class="settings-card-header">
            <div>
              <h3 class="settings-card-title">Blender 输出参数</h3>
              <p class="hint-text">PNG 与 OpenEXR 默认输出格式。</p>
            </div>
          </div>

          <div class="settings-card-actions">
            <UButton icon="i-lucide-image-up" label="编辑" color="neutral" variant="outline" size="sm" @click="blenderOutputModalOpen = true" />
          </div>
        </UCard>

        <UCard variant="subtle" class="settings-card" :ui="{ body: 'settings-card-body' }">
          <div class="settings-card-header">
            <div>
              <h3 class="settings-card-title">FFmpeg 转码参数</h3>
              <p class="hint-text">默认质量、预设与并发数。</p>
            </div>
          </div>

          <div class="settings-card-actions">
            <UButton icon="i-lucide-sliders" label="编辑" color="neutral" variant="outline" size="sm" @click="ffmpegModalOpen = true" />
          </div>
        </UCard>

        <UCard variant="subtle" class="settings-card" :ui="{ body: 'settings-card-body' }">
          <div class="settings-card-header">
            <div>
              <h3 class="settings-card-title">输出路径模板</h3>
              <p class="hint-text">{{ outputTemplateSummary }}</p>
            </div>
          </div>

          <div class="settings-card-actions">
            <UButton icon="i-lucide-braces" label="编辑" color="neutral" variant="outline" size="sm" @click="outputPathTemplateModalOpen = true" />
          </div>
        </UCard>

        <UCard variant="subtle" class="settings-card" :ui="{ body: 'settings-card-body' }">
          <div class="settings-card-header">
            <div>
              <h3 class="settings-card-title">节点网络参数</h3>
              <p class="hint-text">端口、网段参考与设备备注。</p>
            </div>
          </div>

          <div class="settings-card-actions">
            <UButton icon="i-lucide-network" label="编辑" color="neutral" variant="outline" size="sm" @click="networkModalOpen = true" />
          </div>
        </UCard>

        <UCard variant="subtle" class="settings-card" :ui="{ body: 'settings-card-body' }">
          <div class="settings-card-header">
            <div>
              <h3 class="settings-card-title">界面与外观</h3>
              <p class="hint-text">直接切换浅色、深色或跟随系统。</p>
            </div>
          </div>

          <div class="settings-theme-switcher">
            <UButton
              icon="i-lucide-sun-medium"
              label="浅色"
              :color="settingsStore.settings.theme === 'light' ? 'primary' : 'neutral'"
              :variant="settingsStore.settings.theme === 'light' ? 'solid' : 'outline'"
              size="sm"
              @click="setTheme('light')"
            />
            <UButton
              icon="i-lucide-moon-star"
              label="深色"
              :color="settingsStore.settings.theme === 'dark' ? 'primary' : 'neutral'"
              :variant="settingsStore.settings.theme === 'dark' ? 'solid' : 'outline'"
              size="sm"
              @click="setTheme('dark')"
            />
            <UButton
              icon="i-lucide-computer"
              label="系统"
              :color="settingsStore.settings.theme === 'system' ? 'primary' : 'neutral'"
              :variant="settingsStore.settings.theme === 'system' ? 'solid' : 'outline'"
              size="sm"
              @click="setTheme('system')"
            />
          </div>
        </UCard>
      </div>
      </section>

      <section class="settings-section">
      <div class="settings-section-heading">
        <h2 class="settings-section-title">关于</h2>
      </div>

      <div class="settings-form-stack">
        <section class="surface-panel settings-field-panel">
          <div class="settings-field-copy">
            <p class="settings-field-title">版权信息</p>
            <p class="settings-brand-line">SIKFILM · 灌灌小狗</p>
            <p class="hint-text">Made with ❤️ by Loudomian and 空气小怪.</p>
          </div>
        </section>

        <section class="surface-panel settings-field-panel">
          <div class="settings-field-copy">
            <p class="settings-field-title">当前版本</p>
            <p class="settings-version-value">v{{ appVersion }}<span v-if="commitHash" class="settings-commit-hash"> ({{ commitHash }})</span></p>
          </div>
        </section>
      </div>
      </section>
    </section>

    <BlenderPathSettingsModal v-model:open="blenderPathModalOpen" />
    <FfmpegPathSettingsModal v-model:open="ffmpegPathModalOpen" />
    <BlenderSettingsModal v-model:open="blenderModalOpen" />
    <BlenderOutputSettingsModal v-model:open="blenderOutputModalOpen" />
    <FfmpegSettingsModal v-model:open="ffmpegModalOpen" />
    <OutputPathTemplateSettingsModal v-model:open="outputPathTemplateModalOpen" />
    <NetworkSettingsModal v-model:open="networkModalOpen" />
  </div>
</template>

<script setup lang="ts">
import { getVersion } from '@tauri-apps/api/app'
import { invoke } from '@tauri-apps/api/core'

const settingsStore = useSettingsStore()
const runtimeConfig = useRuntimeConfig()
const blenderPathModalOpen = ref(false)
const ffmpegPathModalOpen = ref(false)
const blenderModalOpen = ref(false)
const blenderOutputModalOpen = ref(false)
const ffmpegModalOpen = ref(false)
const outputPathTemplateModalOpen = ref(false)
const networkModalOpen = ref(false)
const appVersion = ref(String(runtimeConfig.public.appVersion ?? '0.0.0'))
const commitHash = ref('')

const blenderVersionItems = computed(() => {
  const defaultPath = settingsStore.settings.defaultBlender
  return settingsStore.blenderVersions.map(item => ({
    executable: item.executable,
    version: item.version,
    isDefault: item.executable === defaultPath,
  }))
})

const blenderPathNote = computed(() => {
  if (!settingsStore.blenderVersions.length) return '当前没有可用的 Blender 可执行文件。'
  return settingsStore.settings.defaultBlender || '尚未设置默认 Blender 版本。'
})

const ffmpegPathNote = computed(() =>
  settingsStore.settings.ffmpegExecutable || '配置后才可以提交和执行转码任务。',
)

const outputTemplateSummary = computed(() => '集中管理渲染序列、Blender 转码和独立转码的默认输出模板。')

async function setTheme(theme: 'dark' | 'light' | 'system') {
  await settingsStore.setTheme(theme)
}

onMounted(async () => {
  await settingsStore.load()
  try {
    appVersion.value = await getVersion()
  } catch {
    // Browser dev mode falls back to the package version from runtime config.
  }
  try {
    const info = await invoke<{ version: string; commit: string }>('get_app_version_info')
    commitHash.value = info.commit
  } catch {
    // Browser dev mode — no commit info available.
  }
})
</script>
