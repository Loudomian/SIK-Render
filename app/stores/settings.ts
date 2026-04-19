import { defineStore } from 'pinia'
import type { AppSettings, BlenderInstall } from '~/types'

export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<AppSettings>({
    defaultBlender: '',
    defaultOutputDir: '',
    maxConcurrentJobs: 1,
    theme: 'dark',
  })
  const blenderVersions = ref<BlenderInstall[]>([])
  const { getSettings, saveSettings, getBlenderVersions } = useTauri()

  async function load() {
    settings.value = await getSettings()
    blenderVersions.value = await getBlenderVersions()
  }

  async function save() {
    await saveSettings(settings.value)
  }

  return { settings, blenderVersions, load, save }
})
