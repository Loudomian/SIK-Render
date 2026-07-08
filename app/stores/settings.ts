import { defineStore } from 'pinia'
import { open } from '@tauri-apps/plugin-dialog'
import type { AppSettings, BlenderInstall } from '~/types'

const THEME_STORAGE_KEY = 'sik-render-theme'
const LOCALE_STORAGE_KEY = 'sik-render-locale'
export type AppLocale = AppSettings['locale']

export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<AppSettings>({
    defaultBlender: '',
    ffmpegExecutable: '',
    blendInspectTimeoutSeconds: 300,
    transcodeCrf: 18,
    transcodePreset: 'medium',
    transcodeEncoder: 'auto',
    ffmpegMaxConcurrent: 2,
    renderOutputPathTemplate: './{blendFileName}_{frameStart}-{frameEnd}/{blendFileName}_{frame}',
    blenderTranscodeOutputPathTemplate: './transcode/{blendFileName}_{frameStart}-{frameEnd}.mp4',
    standaloneTranscodeOutputPathTemplate: '../transcode/{folderName}_{frameStart}-{frameEnd}.mp4',
    pngColorMode: 'RGB',
    pngColorDepth: 8,
    pngCompression: 15,
    exrColorMode: 'RGB',
    exrColorDepth: 16,
    exrCodec: 'DWAA',
    exrQuality: 98,
    theme: 'dark',
    locale: 'zh-CN',
    extraBlenderPaths: [],
    excludedBlenderPaths: [],
    maxCrashRetries: 3,
    nodePort: 47878,
    nodeInterfaceAddress: '192.168.1.1',
    nodeNote: '',
  })
  const blenderVersions = ref<BlenderInstall[]>([])
  const { getSettings, saveSettings, addBlenderByPath, getBlenderVersions } = useTauri()

  function persistThemePreference(theme: AppSettings['theme']) {
    if (!import.meta.client) return
    try {
      window.localStorage.setItem(THEME_STORAGE_KEY, theme)
    } catch {
      // Ignore storage failures and fall back to settings file on next launch.
    }
  }

  function persistLocalePreference(locale: AppLocale) {
    if (!import.meta.client) return
    try {
      window.localStorage.setItem(LOCALE_STORAGE_KEY, locale)
    } catch {
      // Ignore storage failures and fall back to settings file on next launch.
    }
  }

  function normalizeLocale(locale: string | undefined): AppLocale {
    return locale === 'en-US' ? 'en-US' : 'zh-CN'
  }

  function applyStoredLocalePreference() {
    if (!import.meta.client) return
    try {
      const storedLocale = window.localStorage.getItem(LOCALE_STORAGE_KEY)
      if (storedLocale) {
        settings.value.locale = normalizeLocale(storedLocale)
      }
    } catch {
      // Ignore storage failures and fall back to the bundled default.
    }
  }

  applyStoredLocalePreference()

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
    settings.value = {
      ...settings.value,
      ...loaded,
      extraBlenderPaths: loaded.extraBlenderPaths ?? [],
      excludedBlenderPaths: loaded.excludedBlenderPaths ?? [],
      nodePort: loaded.nodePort ?? 47878,
      nodeInterfaceAddress: loaded.nodeInterfaceAddress || '192.168.1.1',
      nodeNote: loaded.nodeNote ?? '',
      transcodeEncoder: loaded.transcodeEncoder ?? 'auto',
      locale: normalizeLocale(loaded.locale),
    }
    persistThemePreference(settings.value.theme)
    persistLocalePreference(settings.value.locale)
    blenderVersions.value = mergeInstalls(await getBlenderVersions())
  }

  async function refreshBlenderVersions() {
    blenderVersions.value = mergeInstalls(await getBlenderVersions())
    const firstBlender = blenderVersions.value[0]
    if (!settings.value.defaultBlender && firstBlender) {
      settings.value.defaultBlender = firstBlender.executable
    }
  }

  async function browseAndAddBlender() {
    const filters = navigator.userAgent.includes('Windows')
      ? [{ name: 'Blender Executable', extensions: ['exe'] }]
      : []
    const selected = await open({
      title: 'Select Blender executable',
      filters,
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
    const filters = navigator.userAgent.includes('Windows')
      ? [{ name: 'FFmpeg Executable', extensions: ['exe'] }]
      : []
    const selected = await open({
      title: 'Select ffmpeg executable',
      filters,
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

  async function setDefaultBlender(executable: string) {
    if (settings.value.defaultBlender === executable) return
    settings.value.defaultBlender = executable
    await saveSettings(settings.value)
  }

  async function setTheme(theme: AppSettings['theme']) {
    if (settings.value.theme === theme) return
    settings.value.theme = theme
    persistThemePreference(theme)
    await saveSettings(settings.value)
  }

  async function setLocale(locale: AppLocale) {
    const normalized = normalizeLocale(locale)
    if (settings.value.locale === normalized) return
    settings.value.locale = normalized
    persistLocalePreference(normalized)
    await saveSettings(settings.value)
  }

  async function toggleTheme() {
    const nextTheme: AppSettings['theme'] = settings.value.theme === 'dark' ? 'light' : 'dark'
    settings.value.theme = nextTheme
    persistThemePreference(nextTheme)
    await saveSettings(settings.value)
    return nextTheme
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
    setDefaultBlender,
    setTheme,
    setLocale,
    toggleTheme,
    save,
  }
})
