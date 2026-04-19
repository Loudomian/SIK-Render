<template>
  <div class="settings-page">
    <h1>Settings</h1>

    <label>
      Default Blender
      <select v-model="settingsStore.settings.defaultBlender">
        <option v-for="b in settingsStore.blenderVersions" :key="b.executable" :value="b.executable">
          Blender {{ b.version }}
        </option>
      </select>
    </label>

    <label>
      Max Concurrent Jobs
      <input v-model.number="settingsStore.settings.maxConcurrentJobs" type="number" min="1" max="8" />
    </label>

    <button class="btn-primary" @click="settingsStore.save()">Save</button>
  </div>
</template>

<script setup lang="ts">
const settingsStore = useSettingsStore()
onMounted(() => settingsStore.load())
</script>
