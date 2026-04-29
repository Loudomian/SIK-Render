import { defineNuxtPlugin } from '#app'
import { attachConsole } from '@tauri-apps/plugin-log'

export default defineNuxtPlugin(async () => {
  if (!import.meta.dev) return

  try {
    await attachConsole()
  } catch {
    // Browser-only Nuxt sessions do not have the Tauri log plugin available.
  }
})
