<template>
  <div class="nodes-page">
    <section class="nodes-header">
      <section class="page-hero nodes-hero">
        <div class="page-hero-copy">
          <h1>{{ t('nodesPage.title') }}</h1>
          <p class="page-note">{{ t('nodesPage.summary', { connected: nodesStore.connectedCount, total: nodesStore.peerList.length }) }}</p>
        </div>
      </section>
    </section>

    <section class="nodes-content">
      <section v-if="nodesStore.localNode" class="node-section">
        <div class="settings-section-heading">
          <h2 class="settings-section-title">{{ t('nodesPage.local') }}</h2>
        </div>
        <NodeCard
          :node="nodesStore.localNode"
          :jobs="jobsStore.jobs"
          :queue-paused="jobsStore.queuePaused"
          is-local
        />
      </section>

      <section class="node-section">
        <div class="settings-section-heading">
          <h2 class="settings-section-title">{{ t('nodesPage.lan') }}</h2>
          <p class="hint-text">{{ t('nodesPage.lanNote') }}</p>
        </div>

        <div v-if="nodesStore.peerList.length" class="node-list">
          <NodeCard
            v-for="peer in nodesStore.peerList"
            :key="peer.node.id"
            :node="peer.node"
            :jobs="peer.jobs"
            :queue-paused="peer.queuePaused"
            :connected="peer.connected"
            :last-seen-at="peer.lastSeenAt"
          />
        </div>

        <UCard v-else variant="subtle" class="empty-state node-empty-state" :ui="{ body: 'empty-state-body' }">
          <div class="empty-state-icon">
            <UIcon name="i-lucide-share-2" />
          </div>
          <div class="empty-state-title">{{ t('nodesPage.emptyTitle') }}</div>
          <div class="empty-state-note">{{ t('nodesPage.emptyNote', { port: localPort }) }}</div>
        </UCard>
      </section>
    </section>
  </div>
</template>

<script setup lang="ts">
const nodesStore = useNodesStore()
const jobsStore = useJobsStore()
const { t } = useI18n()
const localPort = computed(() => nodesStore.localNode?.port ?? 47878)

onMounted(() => {
  void nodesStore.init().catch((error) => {
    console.warn('Failed to initialize render node listeners:', error)
  })
})
</script>
