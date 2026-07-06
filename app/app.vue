<template>
  <UApp>
    <div class="app-shell">
      <aside class="app-sidebar">
        <div class="app-sidebar-inner">
          <div class="app-brand">
            <button
              type="button"
              class="app-brand-toggle"
              :title="brandToggleTitle"
              :aria-label="brandToggleTitle"
              @click="handleBrandIconClick"
            >
              <span class="logo-icon-shell" aria-hidden="true">
                <img
                  :src="appIconLightUrl"
                  width="52"
                  height="52"
                  alt=""
                  class="logo-icon logo-icon-light"
                  :class="{ 'is-active': resolvedTheme !== 'dark' }"
                />
                <img
                  :src="appIconDarkUrl"
                  width="52"
                  height="52"
                  alt=""
                  class="logo-icon logo-icon-dark"
                  :class="{ 'is-active': resolvedTheme === 'dark' }"
                />
              </span>
            </button>
            <div class="app-brand-copy">
              <span class="app-brand-title-row">
                <span class="app-brand-title">SIK Render</span>
                <button
                  v-if="updaterAvailable"
                  class="app-update-badge"
                  type="button"
                  @click="openUpdateModal"
                >
                  <UBadge label="NEW" color="error" variant="subtle" class="page-eyebrow" />
                </button>
              </span>
            </div>
          </div>

          <UNavigationMenu
            orientation="vertical"
            highlight
            highlight-color="info"
            color="neutral"
            variant="pill"
            :items="navItems"
            class="app-nav"
            :ui="{
              root: 'gap-3.5',
              list: 'grid gap-3.5',
              link: 'px-3.5 py-3 text-[0.95rem] rounded-lg',
              linkLeadingIcon: 'size-5.5',
              linkLabel: 'font-semibold',
              linkTrailingBadgeSize: 'md',
            }"
          />
        </div>
      </aside>

      <main class="app-main">
        <UContainer class="app-container">
          <NuxtPage />
        </UContainer>
      </main>

      <UModal
        :open="updateModalOpen"
        :close="false"
        :title="t('updater.newVersion')"
        :ui="{ content: 'job-modal-content settings-modal-content' }"
        @update:open="handleUpdateModalOpenChange"
      >
        <template #body>
          <div class="modal-stack">
            <section class="surface-panel settings-field-panel settings-update-version-panel">
              <div class="settings-field-copy">
                <div class="settings-update-version-row">
                  <p class="settings-field-title">SIK Render v{{ updateVersion }}</p>
                  <UButton
                    icon="i-lucide-external-link"
                    :label="t('updater.releasePage')"
                    color="neutral"
                    variant="ghost"
                    size="xs"
                    @click="openUpdateReleasePage"
                  />
                </div>
                <p class="hint-text">{{ t('updater.currentVersion', { version: updateCurrentVersion }) }}</p>
                <p v-if="updateDate" class="hint-text">{{ t('updater.releaseDate', { date: updateDate }) }}</p>
              </div>
            </section>

            <section class="surface-panel settings-field-panel">
              <div class="settings-field-copy">
                <p class="settings-field-title">{{ t('updater.notes') }}</p>
                <div class="settings-update-notes">
                  <template v-if="updateNoteBlocks.length">
                    <template v-for="(block, index) in updateNoteBlocks" :key="index">
                      <p v-if="block.type === 'paragraph'">{{ block.text }}</p>
                      <p v-else-if="block.type === 'heading'" class="settings-update-notes-heading">
                        {{ block.text }}
                      </p>
                      <div v-else class="settings-update-notes-list" role="list">
                        <div
                          v-for="(item, itemIndex) in block.items"
                          :key="itemIndex"
                          class="settings-update-notes-list-item"
                          role="listitem"
                        >
                          <span class="settings-update-notes-dot" aria-hidden="true" />
                          <span class="settings-update-notes-list-text">{{ item }}</span>
                        </div>
                      </div>
                    </template>
                  </template>
                  <p v-else>{{ t('updater.emptyNotes') }}</p>
                </div>
              </div>
            </section>

            <UAlert
              v-if="updateError"
              color="error"
              variant="subtle"
              :title="t('updater.failed')"
              :description="updateError"
            />

            <div class="modal-actions settings-modal-actions">
              <div class="settings-modal-actions-start" />
              <div class="settings-modal-actions-end">
                <UButton
                  icon="i-lucide-clock"
                  :label="t('updater.later')"
                  color="neutral"
                  variant="outline"
                  :disabled="installingUpdate"
                  @click="updateModalOpen = false"
                />
                <UButton
                  icon="i-lucide-download"
                  :label="t('updater.install')"
                  color="primary"
                  variant="solid"
                  :loading="installingUpdate"
                  @click="installAvailableUpdate"
                />
              </div>
            </div>
          </div>
        </template>
      </UModal>
    </div>
  </UApp>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { getVersion } from '@tauri-apps/api/app'
import { check } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import { open as openUrl } from '@tauri-apps/plugin-shell'
import appIconLightUrl from '~/assets/app-icon-light.png'
import appIconDarkUrl from '~/assets/app-icon-dark.png'
import { formatDateTime } from '~/utils/date-format'

const jobsStore = useJobsStore()
const settingsStore = useSettingsStore()
const { onProgress, onJobUpdated, onLog, onQueueState } = useRenderEvents()
const shadowRecoveryToast = useShadowRecoveryToast()
const updaterState = useUpdaterState()
const toast = useToast()
const runtimeConfig = useRuntimeConfig()
const { t, locale, setLocale } = useI18n()
const CONTEXT_MENU_ALLOW_SELECTOR = '[data-context-menu]'

const navItems = computed(() => [
  {
    label: t('nav.renderQueue'),
    icon: 'i-lucide-layers',
    to: '/',
    badge: jobsStore.runningJobs.length || undefined,
  },
  {
    label: t('nav.transcodeQueue'),
    icon: 'i-lucide-film',
    to: '/transcode',
  },
  {
    label: t('nav.nodes'),
    icon: 'i-lucide-share-2',
    to: '/nodes',
  },
  {
    label: t('nav.settings'),
    icon: 'i-lucide-sliders',
    to: '/settings',
  },
])

const unlisteners: Array<() => void> = []
let systemThemeMedia: MediaQueryList | null = null
const systemPrefersDark = ref(false)
let startupReadyNotified = false

function resolveTheme(theme: 'dark' | 'light' | 'system') {
  if (theme === 'system') {
    return systemPrefersDark.value ? 'dark' : 'light'
  }
  return theme
}

const resolvedTheme = computed(() => resolveTheme(settingsStore.settings.theme))
const brandToggleTitle = computed(() => resolvedTheme.value === 'dark' ? t('updater.switchLight') : t('updater.switchDark'))
const updaterAvailable = computed(() => updaterState.available.value)
const updateModalOpen = updaterState.modalOpen
const installingUpdate = ref(false)
const updateError = ref('')
const updateVersion = computed(() => updaterState.version.value || t('updater.unknownVersion'))
const updateCurrentVersion = computed(() => updaterState.latestUpdate.value?.currentVersion || t('updater.unknownVersion'))
const updateDate = computed(() => formatUpdateDate(updaterState.latestUpdate.value?.date))
const updateNoteBlocks = computed(() => parseUpdateNotes(updaterState.latestUpdate.value?.body))
const updateReleaseUrl = computed(() => {
  const version = updaterState.latestUpdate.value?.version
  if (!version) return 'https://github.com/Loudomian/SIK-Render/releases'
  const tag = version.startsWith('v') ? version : `v${version}`
  return `https://github.com/Loudomian/SIK-Render/releases/tag/${tag}`
})

type UpdateNoteBlock =
  | { type: 'heading'; text: string }
  | { type: 'paragraph'; text: string }
  | { type: 'list'; items: string[] }

function applyThemeClass(theme: 'dark' | 'light' | 'system') {
  const resolved = resolveTheme(theme)
  const root = document.documentElement
  root.classList.toggle('dark', resolved === 'dark')
  root.classList.toggle('light', resolved !== 'dark')
  root.style.colorScheme = resolved === 'dark' ? 'dark' : 'light'
}

function handleSystemThemeChange() {
  systemPrefersDark.value = systemThemeMedia?.matches ?? false
  if (settingsStore.settings.theme !== 'system') return
  applyThemeClass('system')
}

function handleGlobalContextMenu(event: MouseEvent) {
  const target = event.target as HTMLElement | null
  if (target?.closest(CONTEXT_MENU_ALLOW_SELECTOR)) return
  event.preventDefault()
}

function formatUpdateDate(value?: string) {
  if (!value) return ''

  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value

  return formatDateTime(date, locale.value)
}

function parseUpdateNotes(body?: string): UpdateNoteBlock[] {
  if (!body?.trim()) return []

  const lines = body
    .replaceAll('\r\n', '\n')
    .split('\n')
    .map(line => line.trim())

  while (lines.length && !lines[0]) lines.shift()
  if (/^#{1,6}\s+changes since\b/i.test(lines[0] ?? '')) {
    lines.shift()
  }
  while (lines.length && !lines[0]) lines.shift()

  const blocks: UpdateNoteBlock[] = []
  let paragraph: string[] = []
  let listItems: string[] = []

  const flushParagraph = () => {
    if (!paragraph.length) return
    blocks.push({ type: 'paragraph', text: paragraph.join(' ') })
    paragraph = []
  }

  const flushList = () => {
    if (!listItems.length) return
    blocks.push({ type: 'list', items: listItems })
    listItems = []
  }

  for (const line of lines) {
    if (!line) {
      flushParagraph()
      flushList()
      continue
    }

    const heading = line.match(/^#{1,6}\s+(.+)$/)
    if (heading) {
      flushParagraph()
      flushList()
      blocks.push({ type: 'heading', text: cleanMarkdownInline(heading[1] ?? '') })
      continue
    }

    const listItem = line.match(/^[-*]\s+(.+)$/)
    if (listItem) {
      flushParagraph()
      listItems.push(cleanMarkdownInline(listItem[1] ?? ''))
      continue
    }

    flushList()
    paragraph.push(cleanMarkdownInline(line))
  }

  flushParagraph()
  flushList()

  return blocks
}

function cleanMarkdownInline(value: string) {
  return value
    .replace(/`([^`]+)`/g, '$1')
    .replace(/\*\*([^*]+)\*\*/g, '$1')
    .replace(/\*([^*]+)\*/g, '$1')
    .replace(/\[([^\]]+)]\([^)]+\)/g, '$1')
    .trim()
}

async function handleBrandIconClick() {
  try {
    await settingsStore.toggleTheme()
  } catch (error) {
    console.error('Failed to toggle theme from brand icon:', error)
  }
}

async function notifyAppReadyAfterPaint() {
  if (!import.meta.client || startupReadyNotified) return
  startupReadyNotified = true

  try {
    await nextTick()
    await new Promise<void>(resolve => window.requestAnimationFrame(() => resolve()))
    await new Promise<void>(resolve => window.requestAnimationFrame(() => resolve()))
    await invoke('app_ready')
  } catch (error) {
    console.error('Failed to finalize app startup:', error)
  }
}

async function checkForAvailableUpdate() {
  try {
    if (shouldUseMockUpdate()) {
      updaterState.setUpdate(createMockUpdate(await getCurrentAppVersion()))
      return
    }
    updaterState.setUpdate(await check())
  } catch (error) {
    console.debug('Background update check failed:', error)
  }
}

async function getCurrentAppVersion() {
  try {
    return await getVersion()
  } catch {
    return String(runtimeConfig.public.appVersion ?? '0.0.0')
  }
}

function openUpdateModal() {
  if (!updaterState.latestUpdate.value) return
  updateError.value = ''
  updaterState.modalOpen.value = true
}

function handleUpdateModalOpenChange(value: boolean) {
  if (installingUpdate.value) return
  updaterState.modalOpen.value = value
  if (!value) updateError.value = ''
}

async function openUpdateReleasePage() {
  try {
    await openUrl(updateReleaseUrl.value)
  } catch (error) {
    toast.add({
      title: t('updater.openReleaseFailed'),
      description: error instanceof Error ? error.message : String(error),
      color: 'error',
    })
  }
}

async function installAvailableUpdate() {
  if (installingUpdate.value) return

  const update = updaterState.latestUpdate.value
  if (!update) {
    updateError.value = t('updater.stale')
    return
  }

  installingUpdate.value = true
  updateError.value = ''

  try {
    await update.downloadAndInstall()
    toast.add({
      title: t('updater.installedTitle'),
      description: t('updater.installedDescription'),
      color: 'success',
    })
    await relaunch()
  } catch (error) {
    updateError.value = error instanceof Error ? error.message : String(error)
  } finally {
    installingUpdate.value = false
  }
}

if (import.meta.client) {
  systemThemeMedia = window.matchMedia('(prefers-color-scheme: dark)')
  systemPrefersDark.value = systemThemeMedia.matches

  watch(
    () => settingsStore.settings.theme,
    (theme) => {
      applyThemeClass(theme)
    },
    { immediate: true },
  )

  watch(
    () => settingsStore.settings.locale,
    (locale) => {
      void setLocale(locale)
    },
    { immediate: true },
  )
}

onMounted(async () => {
  systemThemeMedia?.addEventListener('change', handleSystemThemeChange)
  window.addEventListener('contextmenu', handleGlobalContextMenu, true)
  await Promise.all([
    jobsStore.fetchJobs(),
    jobsStore.fetchQueueState(),
    settingsStore.load(),
  ])
  await notifyAppReadyAfterPaint()
  void checkForAvailableUpdate()
  unlisteners.push(await onProgress(jobsStore.applyProgress))
  unlisteners.push(await onJobUpdated(jobsStore.applyJobUpdate))
  unlisteners.push(await onLog((event) => {
    jobsStore.applyLog(event)
    shadowRecoveryToast.handleLogEvent(event)
  }))
  unlisteners.push(await onQueueState(jobsStore.applyQueueState))
})

onUnmounted(() => {
  systemThemeMedia?.removeEventListener('change', handleSystemThemeChange)
  systemThemeMedia = null
  window.removeEventListener('contextmenu', handleGlobalContextMenu, true)
  for (const unlisten of unlisteners) {
    unlisten()
  }
})
</script>
