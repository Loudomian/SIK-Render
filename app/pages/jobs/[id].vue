<template>
  <div class="job-detail">
    <h1>{{ job?.name ?? 'Job Not Found' }}</h1>
    <pre v-if="job">{{ JSON.stringify(job, null, 2) }}</pre>
  </div>
</template>

<script setup lang="ts">
const route = useRoute()
const jobsStore = useJobsStore()
const job = computed(() => jobsStore.jobs.find((j) => j.id === route.params.id))
</script>
