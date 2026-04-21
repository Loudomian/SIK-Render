<template>
  <UApp>
    <div class="app-shell">
      <aside class="app-sidebar">
        <div class="app-sidebar-inner">
          <div class="app-brand">
            <img :src="appIconUrl" width="40" height="40" alt="" class="logo-icon" />
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
import appIconUrl from '~/assets/app-icon.png'

const jobsStore = useJobsStore()
const settingsStore = useSettingsStore()
const { onProgress, onJobUpdated, onLog } = useRenderEvents()

const navItems = computed(() => [
  {
    label: '渲染队列',
    icon: 'i-lucide-layers',
    to: '/',
    badge: jobsStore.runningJobs.length || undefined,
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

if (import.meta.client) {
  watch(
    () => settingsStore.settings.theme,
    (theme) => {
      const root = document.documentElement
      root.classList.toggle('dark', theme === 'dark')
      root.classList.toggle('light', theme !== 'dark')
    },
    { immediate: true },
  )
}

onMounted(async () => {
  await Promise.all([
    jobsStore.fetchJobs(),
    jobsStore.fetchQueueState(),
    settingsStore.load(),
  ])
  unlisteners.push(await onProgress(jobsStore.applyProgress))
  unlisteners.push(await onJobUpdated(jobsStore.applyJobUpdate))
  unlisteners.push(await onLog(jobsStore.applyLog))
})

onUnmounted(() => {
  for (const unlisten of unlisteners) {
    unlisten()
  }
})
</script>
