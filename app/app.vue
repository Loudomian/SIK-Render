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
              <span class="app-brand-title">SIK Render</span>
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
    </div>
  </UApp>
</template>

<script setup lang="ts">
import appIconLightUrl from '~/assets/app-icon-light.png'
import appIconDarkUrl from '~/assets/app-icon-dark.png'

const jobsStore = useJobsStore()
const settingsStore = useSettingsStore()
const { onProgress, onJobUpdated, onLog, onQueueState } = useRenderEvents()
const CONTEXT_MENU_ALLOW_SELECTOR = '[data-context-menu]'

const navItems = computed(() => [
  {
    label: '渲染队列',
    icon: 'i-lucide-layers',
    to: '/',
    badge: jobsStore.runningJobs.length || undefined,
  },
  {
    label: '转码队列',
    icon: 'i-lucide-film',
    to: '/transcode',
  },
  {
    label: '节点',
    icon: 'i-lucide-share-2',
    to: '/nodes',
  },
  {
    label: '设置',
    icon: 'i-lucide-sliders',
    to: '/settings',
  },
])

const unlisteners: Array<() => void> = []
let systemThemeMedia: MediaQueryList | null = null
const systemPrefersDark = ref(false)

function resolveTheme(theme: 'dark' | 'light' | 'system') {
  if (theme === 'system') {
    return systemPrefersDark.value ? 'dark' : 'light'
  }
  return theme
}

const resolvedTheme = computed(() => resolveTheme(settingsStore.settings.theme))
const brandToggleTitle = computed(() => resolvedTheme.value === 'dark' ? '切换到浅色模式' : '切换到深色模式')

function applyThemeClass(theme: 'dark' | 'light' | 'system') {
  const resolved = resolveTheme(theme)
  const root = document.documentElement
  root.classList.toggle('dark', resolved === 'dark')
  root.classList.toggle('light', resolved !== 'dark')
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

async function handleBrandIconClick() {
  try {
    await settingsStore.toggleTheme()
  } catch (error) {
    console.error('Failed to toggle theme from brand icon:', error)
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
}

onMounted(async () => {
  systemThemeMedia.addEventListener('change', handleSystemThemeChange)
  window.addEventListener('contextmenu', handleGlobalContextMenu, true)
  await Promise.all([
    jobsStore.fetchJobs(),
    jobsStore.fetchQueueState(),
    settingsStore.load(),
  ])
  unlisteners.push(await onProgress(jobsStore.applyProgress))
  unlisteners.push(await onJobUpdated(jobsStore.applyJobUpdate))
  unlisteners.push(await onLog(jobsStore.applyLog))
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
