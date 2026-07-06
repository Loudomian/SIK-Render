<template>
  <div class="settings-page">
    <section class="page-hero settings-header">
      <div class="page-hero-copy">
        <h1>{{ t('settingsPage.title') }}</h1>
        <p class="page-note">{{ t('settingsPage.description') }}</p>
      </div>
    </section>

    <section class="settings-content">
      <section class="settings-section">
        <div class="settings-section-heading">
          <h2 class="settings-section-title">{{ t('settingsPage.pathManagement') }}</h2>
          <p class="hint-text">{{ t('settingsPage.pathManagementNote') }}</p>
        </div>

        <UCard variant="subtle" class="settings-card" :ui="{ body: 'settings-card-body' }">
          <div class="settings-card-header">
            <div>
              <h3 class="settings-card-title">{{ t('settingsPage.toolPaths') }}</h3>
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
                <UButton icon="i-lucide-folder-open" :label="t('settingsPage.managePaths')" color="neutral" variant="outline" size="sm" @click="blenderPathModalOpen = true" />
              </div>
            </section>

            <section class="surface-panel settings-path-panel">
              <div class="settings-path-copy">
                <p class="settings-field-title">FFmpeg</p>
                <p class="hint-text">{{ ffmpegPathNote }}</p>
              </div>
              <div class="settings-card-actions">
                <UButton icon="i-lucide-folder-open" :label="t('settingsPage.managePaths')" color="neutral" variant="outline" size="sm" @click="ffmpegPathModalOpen = true" />
              </div>
            </section>
          </div>
        </UCard>
      </section>

      <section class="settings-section">
      <div class="settings-section-heading">
        <h2 class="settings-section-title">{{ t('settingsPage.parameterManagement') }}</h2>
        <p class="hint-text">{{ t('settingsPage.parameterManagementNote') }}</p>
      </div>

      <div class="settings-grid-2">
        <UCard variant="subtle" class="settings-card" :ui="{ body: 'settings-card-body' }">
          <div class="settings-card-header">
            <div>
              <h3 class="settings-card-title">{{ t('settingsPage.blenderRender') }}</h3>
              <p class="hint-text">{{ t('settingsPage.blenderRenderNote') }}</p>
            </div>
          </div>

          <div class="settings-card-actions">
            <UButton icon="i-lucide-sliders" :label="t('settingsPage.edit')" color="neutral" variant="outline" size="sm" @click="blenderModalOpen = true" />
          </div>
        </UCard>

        <UCard variant="subtle" class="settings-card" :ui="{ body: 'settings-card-body' }">
          <div class="settings-card-header">
            <div>
              <h3 class="settings-card-title">{{ t('settingsPage.blenderOutput') }}</h3>
              <p class="hint-text">{{ t('settingsPage.blenderOutputNote') }}</p>
            </div>
          </div>

          <div class="settings-card-actions">
            <UButton icon="i-lucide-image-up" :label="t('settingsPage.edit')" color="neutral" variant="outline" size="sm" @click="blenderOutputModalOpen = true" />
          </div>
        </UCard>

        <UCard variant="subtle" class="settings-card" :ui="{ body: 'settings-card-body' }">
          <div class="settings-card-header">
            <div>
              <h3 class="settings-card-title">{{ t('settingsPage.ffmpegTranscode') }}</h3>
              <p class="hint-text">{{ t('settingsPage.ffmpegTranscodeNote') }}</p>
            </div>
          </div>

          <div class="settings-card-actions">
            <UButton icon="i-lucide-sliders" :label="t('settingsPage.edit')" color="neutral" variant="outline" size="sm" @click="ffmpegModalOpen = true" />
          </div>
        </UCard>

        <UCard variant="subtle" class="settings-card" :ui="{ body: 'settings-card-body' }">
          <div class="settings-card-header">
            <div>
              <h3 class="settings-card-title">{{ t('settingsPage.outputTemplates') }}</h3>
              <p class="hint-text">{{ outputTemplateSummary }}</p>
            </div>
          </div>

          <div class="settings-card-actions">
            <UButton icon="i-lucide-braces" :label="t('settingsPage.edit')" color="neutral" variant="outline" size="sm" @click="outputPathTemplateModalOpen = true" />
          </div>
        </UCard>

        <UCard variant="subtle" class="settings-card" :ui="{ body: 'settings-card-body' }">
          <div class="settings-card-header">
            <div>
              <h3 class="settings-card-title">{{ t('settingsPage.network') }}</h3>
              <p class="hint-text">{{ t('settingsPage.networkNote') }}</p>
            </div>
          </div>

          <div class="settings-card-actions">
            <UButton icon="i-lucide-network" :label="t('settingsPage.edit')" color="neutral" variant="outline" size="sm" @click="networkModalOpen = true" />
          </div>
        </UCard>

        <UCard variant="subtle" class="settings-card" :ui="{ body: 'settings-card-body' }">
          <div class="settings-card-header">
            <div>
              <h3 class="settings-card-title">{{ t('settingsPage.appearance') }}</h3>
              <p class="hint-text">{{ t('settingsPage.appearanceNote') }}</p>
            </div>
          </div>

          <div class="settings-theme-switcher">
            <UButton
              icon="i-lucide-sun-medium"
              :label="t('theme.light')"
              :color="settingsStore.settings.theme === 'light' ? 'primary' : 'neutral'"
              :variant="settingsStore.settings.theme === 'light' ? 'solid' : 'outline'"
              size="sm"
              @click="setTheme('light')"
            />
            <UButton
              icon="i-lucide-moon-star"
              :label="t('theme.dark')"
              :color="settingsStore.settings.theme === 'dark' ? 'primary' : 'neutral'"
              :variant="settingsStore.settings.theme === 'dark' ? 'solid' : 'outline'"
              size="sm"
              @click="setTheme('dark')"
            />
            <UButton
              icon="i-lucide-computer"
              :label="t('theme.system')"
              :color="settingsStore.settings.theme === 'system' ? 'primary' : 'neutral'"
              :variant="settingsStore.settings.theme === 'system' ? 'solid' : 'outline'"
              size="sm"
              @click="setTheme('system')"
            />
          </div>
        </UCard>

        <UCard variant="subtle" class="settings-card" :ui="{ body: 'settings-card-body' }">
          <div class="settings-card-header">
            <div>
              <h3 class="settings-card-title">{{ t('language.title') }}</h3>
              <p class="hint-text">{{ t('language.note') }}</p>
            </div>
          </div>

          <div class="settings-theme-switcher">
            <UButton
              icon="i-lucide-languages"
              :label="t('language.zhCN')"
              :color="settingsStore.settings.locale === 'zh-CN' ? 'primary' : 'neutral'"
              :variant="settingsStore.settings.locale === 'zh-CN' ? 'solid' : 'outline'"
              size="sm"
              @click="setLocale('zh-CN')"
            />
            <UButton
              icon="i-lucide-languages"
              :label="t('language.enUS')"
              :color="settingsStore.settings.locale === 'en-US' ? 'primary' : 'neutral'"
              :variant="settingsStore.settings.locale === 'en-US' ? 'solid' : 'outline'"
              size="sm"
              @click="setLocale('en-US')"
            />
          </div>
        </UCard>
      </div>
      </section>

      <section class="settings-section">
      <div class="settings-section-heading">
        <h2 class="settings-section-title">{{ t('settingsPage.about') }}</h2>
      </div>

      <div class="settings-form-stack">
        <section class="surface-panel settings-field-panel">
          <div class="settings-field-copy">
            <p class="settings-field-title">{{ t('settingsPage.copyright') }}</p>
            <p class="settings-brand-line">SIKFILM · 罐罐小狗</p>
            <p class="hint-text">Made with ❤️ by Loudomian and 空气小怪.</p>
          </div>
        </section>

        <section class="surface-panel settings-field-panel">
          <div class="settings-field-copy">
            <p class="settings-field-title">{{ t('settingsPage.currentVersion') }}</p>
            <p class="settings-version-value">v{{ appVersion }}<span v-if="commitHash" class="settings-commit-hash"> ({{ commitHash }})</span></p>
          </div>
          <div class="settings-card-actions">
            <UButton
              icon="i-lucide-refresh-cw"
              :label="t('settingsPage.checkUpdates')"
              color="neutral"
              variant="outline"
              size="sm"
              :loading="checkingUpdate"
              @click="checkForUpdates"
            />
            <UButton
              icon="i-lucide-trash-2"
              :label="t('settingsPage.reset')"
              color="error"
              variant="outline"
              size="sm"
              :disabled="runtimeResetting"
              @click="runtimeResetModalOpen = true"
            />
          </div>
          <div v-if="runtimeResetResult" class="settings-reset-result">
            <UAlert
              :color="runtimeResetResult.failedPaths.length ? 'warning' : 'success'"
              variant="subtle"
              :title="runtimeResetResult.failedPaths.length ? t('settingsPage.resetResultPartial') : t('settingsPage.resetResultDone')"
              :description="runtimeResetSummary"
            />
            <p class="settings-reset-path">{{ runtimeResetResult.rootPath }}</p>
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

    <UModal
      :open="runtimeResetModalOpen"
      :close="false"
      :title="t('settingsPage.resetModalTitle')"
      :ui="{ content: 'job-modal-content settings-modal-content' }"
      @update:open="handleRuntimeResetOpenChange"
    >
      <template #body>
        <div class="modal-stack">
          <UAlert
            color="error"
            variant="subtle"
            :title="t('settingsPage.resetAlertTitle')"
            :description="t('settingsPage.resetAlertDescription')"
          />

          <section class="surface-panel settings-field-panel">
            <div class="settings-field-copy">
              <p class="settings-field-title">{{ t('settingsPage.resetConfirmTitle') }}</p>
              <p class="hint-text">{{ t('settingsPage.resetConfirmNote') }}</p>
            </div>
            <UInput
              v-model="runtimeResetConfirmText"
              placeholder="RESET"
              :disabled="runtimeResetting"
              autocomplete="off"
            />
          </section>

          <UAlert
            v-if="runtimeResetError"
            color="error"
            variant="subtle"
            :title="t('settingsPage.resetFailed')"
            :description="runtimeResetError"
          />

          <div class="modal-actions settings-modal-actions">
            <div class="settings-modal-actions-start" />
            <div class="settings-modal-actions-end">
              <UButton
                icon="i-lucide-x"
                :label="t('common.cancel')"
                color="neutral"
                variant="outline"
                :disabled="runtimeResetting"
                @click="runtimeResetModalOpen = false"
              />
              <UButton
                icon="i-lucide-trash-2"
                :label="t('settingsPage.resetConfirm')"
                color="error"
                variant="solid"
                :loading="runtimeResetting"
                :disabled="runtimeResetConfirmText !== 'RESET'"
                @click="confirmRuntimeReset"
              />
            </div>
          </div>
        </div>
      </template>
    </UModal>
  </div>
</template>

<script setup lang="ts">
import { getVersion } from '@tauri-apps/api/app'
import { invoke } from '@tauri-apps/api/core'
import { check } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import type { RuntimeResetResult } from '~/types'
import type { AppLocale } from '~/stores/settings'

const settingsStore = useSettingsStore()
const toast = useToast()
const { t, setLocale: setI18nLocale } = useI18n()
const { resetAppRuntimeData } = useTauri()
const updaterState = useUpdaterState()
const runtimeConfig = useRuntimeConfig()
const blenderPathModalOpen = ref(false)
const ffmpegPathModalOpen = ref(false)
const blenderModalOpen = ref(false)
const blenderOutputModalOpen = ref(false)
const ffmpegModalOpen = ref(false)
const outputPathTemplateModalOpen = ref(false)
const networkModalOpen = ref(false)
const runtimeResetModalOpen = ref(false)
const runtimeResetting = ref(false)
const runtimeResetConfirmText = ref('')
const runtimeResetError = ref('')
const runtimeResetResult = ref<RuntimeResetResult | null>(null)
const checkingUpdate = ref(false)
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
  if (!settingsStore.blenderVersions.length) return t('settingsPage.noBlender')
  return settingsStore.settings.defaultBlender || t('settingsPage.noDefaultBlender')
})

const ffmpegPathNote = computed(() =>
  settingsStore.settings.ffmpegExecutable || t('settingsPage.noFfmpeg'),
)

const outputTemplateSummary = computed(() => t('settingsPage.outputTemplatesNote'))

const runtimeResetSummary = computed(() => {
  if (!runtimeResetResult.value) return ''
  const removedCount = runtimeResetResult.value.removedPaths.length
  const failedCount = runtimeResetResult.value.failedPaths.length
  if (!failedCount) return t('settingsPage.resetSummaryDone', { removed: removedCount })
  return t('settingsPage.resetSummaryPartial', { removed: removedCount, failed: failedCount })
})

async function setTheme(theme: 'dark' | 'light' | 'system') {
  await settingsStore.setTheme(theme)
}

async function setLocale(locale: AppLocale) {
  await setI18nLocale(locale)
  await settingsStore.setLocale(locale)
}

function handleRuntimeResetOpenChange(value: boolean) {
  if (runtimeResetting.value) return
  runtimeResetModalOpen.value = value
  if (!value) {
    runtimeResetConfirmText.value = ''
    runtimeResetError.value = ''
  }
}

async function confirmRuntimeReset() {
  if (runtimeResetting.value || runtimeResetConfirmText.value !== 'RESET') return

  runtimeResetting.value = true
  runtimeResetError.value = ''

  try {
    runtimeResetResult.value = await resetAppRuntimeData()
    runtimeResetModalOpen.value = false
    runtimeResetConfirmText.value = ''
    toast.add({
      title: runtimeResetResult.value.failedPaths.length ? t('settingsPage.resetToastPartial') : t('settingsPage.resetToastDone'),
      description: t('settingsPage.resetToastDescription'),
      color: runtimeResetResult.value.failedPaths.length ? 'warning' : 'success',
    })
    window.setTimeout(() => {
      void relaunch()
    }, 800)
  } catch (error) {
    runtimeResetError.value = error instanceof Error ? error.message : String(error)
  } finally {
    runtimeResetting.value = false
  }
}

async function checkForUpdates() {
  if (checkingUpdate.value) return

  checkingUpdate.value = true
  try {
    const update = shouldUseMockUpdate() ? createMockUpdate(appVersion.value) : await check()
    updaterState.setUpdate(update)
    if (!update) {
      toast.add({
        title: t('updater.latestTitle'),
        description: t('updater.latestDescription', { version: appVersion.value }),
        color: 'success',
      })
      return
    }

    updaterState.modalOpen.value = true
  } catch (error) {
    toast.add({
      title: t('updater.checkFailed'),
      description: error instanceof Error ? error.message : String(error),
      color: 'error',
    })
  } finally {
    checkingUpdate.value = false
  }
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
