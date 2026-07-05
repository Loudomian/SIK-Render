<template>
  <div class="nodes-page">
    <section class="nodes-header">
      <section class="page-hero nodes-hero">
        <div class="page-hero-copy">
          <h1>渲染节点</h1>
          <p class="page-note">当前在线 {{ nodesStore.connectedCount }} 个节点，已记录 {{ nodesStore.peerList.length }} 个节点。</p>
        </div>
      </section>
    </section>

    <section class="nodes-content">
      <section v-if="nodesStore.localNode" class="node-section">
        <div class="settings-section-heading">
          <h2 class="settings-section-title">本机</h2>
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
          <h2 class="settings-section-title">局域网节点</h2>
          <p class="hint-text">在线节点实时同步；离线节点保留最后一次记录。</p>
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
          <div class="empty-state-title">未发现其他节点</div>
          <div class="empty-state-note">确认同网段，并放行端口 {{ localPort }} 与 mDNS。</div>
        </UCard>
      </section>
    </section>
  </div>
</template>

<script setup lang="ts">
const nodesStore = useNodesStore()
const jobsStore = useJobsStore()
const localPort = computed(() => nodesStore.localNode?.port ?? 47878)

onMounted(async () => {
  await nodesStore.init()
})

onUnmounted(() => {
  nodesStore.dispose()
})
</script>
