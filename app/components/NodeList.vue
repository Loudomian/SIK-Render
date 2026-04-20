<template>
  <div class="node-list">
    <UCard v-for="node in nodes" :key="node.id" variant="subtle" class="node-card" :ui="{ body: 'node-card-body' }">
      <div class="node-item">
        <div class="node-info">
          <span class="node-name">{{ node.name }}</span>
          <span class="node-address">{{ node.address }}</span>
        </div>
        <UBadge
          :label="node.status"
          :color="node.status === 'Online' ? 'success' : 'neutral'"
          variant="subtle"
        />
      </div>
    </UCard>
  </div>
</template>

<script setup lang="ts">
// Stub: local node only until P3 LAN discovery
const nodes = ref([
  { id: 'local', name: 'This Machine', address: '127.0.0.1', status: 'Online' },
])
</script>
