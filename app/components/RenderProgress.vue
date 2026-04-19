<template>
  <div class="render-progress">
    <div class="progress-bar">
      <div class="progress-fill" :style="{ width: `${percent}%` }" />
    </div>
    <span class="progress-label">Frame {{ frame }} / {{ totalFrames }} ({{ percent }}%)</span>
  </div>
</template>

<script setup lang="ts">
const props = defineProps<{ frame: number; totalFrames: number }>()
const percent = computed(() =>
  props.totalFrames > 0 ? Math.round((props.frame / props.totalFrames) * 100) : 0,
)
</script>
