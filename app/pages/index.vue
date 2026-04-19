<template>
  <div class="queue-page">
    <header class="page-header">
      <h1>Render Queue</h1>
      <button class="btn-primary" @click="showAddJob = true">+ Add Job</button>
    </header>

    <div v-if="jobsStore.loading" class="loading">Loading...</div>

    <div v-else-if="jobsStore.jobs.length === 0" class="empty">
      No jobs yet. Add a .blend file to get started.
    </div>

    <div v-else class="job-list">
      <JobCard
        v-for="job in jobsStore.jobs"
        :key="job.id"
        :job="job"
        @cancel="jobsStore.stopJob(job.id)"
        @remove="jobsStore.deleteJob(job.id)"
      />
    </div>

    <!-- Add job modal (stub) -->
    <div v-if="showAddJob" class="modal-backdrop" @click.self="showAddJob = false">
      <div class="modal">
        <h2>New Render Job</h2>
        <p>TODO: form fields</p>
        <button @click="showAddJob = false">Close</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
const jobsStore = useJobsStore()
const showAddJob = ref(false)
</script>
