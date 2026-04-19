<template>
  <div class="job-card" :class="`status-${job.status}`">
    <div class="job-info">
      <span class="job-name">{{ job.name }}</span>
      <span class="job-status">{{ job.status }}</span>
    </div>

    <RenderProgress
      v-if="job.status === 'running'"
      :frame="(job as any)._frame ?? 0"
      :total-frames="(job as any)._totalFrames ?? job.frameEnd - job.frameStart + 1"
    />

    <div class="job-meta">
      <span>Frames {{ job.frameStart }}–{{ job.frameEnd }}</span>
      <span>{{ job.outputFormat }}</span>
    </div>

    <div class="job-actions">
      <button v-if="job.status === 'running'" class="btn-warn" @click="$emit('cancel')">Cancel</button>
      <button v-if="['done', 'failed', 'cancelled'].includes(job.status)" class="btn-danger" @click="$emit('remove')">Remove</button>
      <NuxtLink :to="`/jobs/${job.id}`" class="btn-ghost">Details</NuxtLink>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { RenderJob } from '~/types'

defineProps<{ job: RenderJob }>()
defineEmits(['cancel', 'remove'])
</script>
