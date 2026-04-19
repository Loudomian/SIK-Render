<template>
  <ul class="node-list">
    <li v-for="node in nodes" :key="node.id" class="node-item">
      <span class="node-name">{{ node.name }}</span>
      <span class="node-address">{{ node.address }}</span>
      <span class="node-status" :class="`status-${node.status.toLowerCase()}`">
        {{ node.status }}
      </span>
    </li>
  </ul>
</template>

<script setup lang="ts">
// Stub: local node only until P3 LAN discovery
const nodes = ref([
  { id: 'local', name: 'This Machine', address: '127.0.0.1', status: 'Online' },
])
</script>
