<template>
  <div class="app-layout">
    <nav class="sidebar">
      <div class="logo">SIK Render</div>
      <NuxtLink to="/" class="nav-item">Queue</NuxtLink>
      <NuxtLink to="/nodes" class="nav-item">Nodes</NuxtLink>
      <NuxtLink to="/settings" class="nav-item">Settings</NuxtLink>
    </nav>
    <main class="content">
      <NuxtPage />
    </main>
  </div>
</template>

<script setup lang="ts">
const jobsStore = useJobsStore()
const { onProgress } = useRenderEvents()

let unlisten: (() => void) | null = null

onMounted(async () => {
  await jobsStore.fetchJobs()
  unlisten = await onProgress(jobsStore.applyProgress)
})

onUnmounted(() => {
  unlisten?.()
})
</script>
