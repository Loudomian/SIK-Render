import { defineStore } from 'pinia'
import { open } from '@tauri-apps/plugin-dialog'
import type { AppSettings, BlenderInstall } from '~/types'

export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<AppSettings>({
    defaultBlender: '',
    ffmpegExecutable: '',
    defaultOutputDir: '',
    maxConcurrentJobs: 1,
    theme: 'dark',
    extraBlenderPaths: [],
    excludedBlenderPaths: [],
  })
  const blenderVersions = ref<BlenderInstall[]>([])
  const { getSettings, saveSettings, addBlenderByPath } = useTauri()

  function mergeInstalls(extra: BlenderInstall[]): BlenderInstall[] {
    const seen = new Set<string>()
    return extra.filter((b) => {
      if (seen.has(b.executable)) return false
      seen.add(b.executable)
      return true
    })
  }

  async function load() {
    const loaded = await getSettings()
    settings.value = { extraBlenderPaths: [], excludedBlenderPaths: [], ...loaded }

    const extraResults = await Promise.allSettled(
      settings.value.extraBlenderPaths.map((p) => addBlenderByPath(p)),
    )
    const extra = extraResults.flatMap((r) => (r.status === 'fulfilled' ? [r.value] : []))
    blenderVersions.value = mergeInstalls(extra)
  }

  async function refreshBlenderVersions() {
    const extraResults = await Promise.allSettled(
      settings.value.extraBlenderPaths.map((p) => addBlenderByPath(p)),
    )
    const extra = extraResults.flatMap((r) => (r.status === 'fulfilled' ? [r.value] : []))
    blenderVersions.value = mergeInstalls(extra)
    if (!settings.value.defaultBlender && blenderVersions.value.length > 0) {
      settings.value.defaultBlender = blenderVersions.value[0].executable
    }
  }

  async function browseAndAddBlender() {
    const selected = await open({
      title: 'Select blender.exe',
      filters: [{ name: 'Blender Executable', extensions: ['exe'] }],
    })
    if (!selected) return null

    const path = typeof selected === 'string' ? selected : (selected as string[])[0]
    if (!path) return null

    const install = await addBlenderByPath(path)

    if (!blenderVersions.value.some((b) => b.executable === install.executable)) {
      blenderVersions.value.push(install)
      blenderVersions.value.sort((a, b) => b.version.localeCompare(a.version))
    }
    if (!settings.value.defaultBlender) {
      settings.value.defaultBlender = install.executable
    }
    if (!settings.value.extraBlenderPaths.includes(install.executable)) {
      settings.value.extraBlenderPaths.push(install.executable)
    }
    await saveSettings(settings.value)
    return install
  }

  async function browseAndSetFfmpeg() {
    const selected = await open({
      title: 'Select ffmpeg executable',
      filters: [{ name: 'FFmpeg Executable', extensions: ['exe'] }],
    })
    if (!selected) return null

    const path = typeof selected === 'string' ? selected : (selected as string[])[0]
    if (!path) return null

    settings.value.ffmpegExecutable = path
    await saveSettings(settings.value)
    return path
  }

  async function clearFfmpeg() {
    settings.value.ffmpegExecutable = ''
    await saveSettings(settings.value)
  }

  async function removeBlenderVersion(executable: string) {
    blenderVersions.value = blenderVersions.value.filter((b) => b.executable !== executable)
    settings.value.extraBlenderPaths = settings.value.extraBlenderPaths.filter((p) => p !== executable)
    if (settings.value.defaultBlender === executable) {
      settings.value.defaultBlender = blenderVersions.value[0]?.executable ?? ''
    }
    await saveSettings(settings.value)
  }

  async function save() {
    await saveSettings(settings.value)
  }

  return {
    settings,
    blenderVersions,
    load,
    refreshBlenderVersions,
    browseAndAddBlender,
    browseAndSetFfmpeg,
    clearFfmpeg,
    removeBlenderVersion,
    save,
  }
})
