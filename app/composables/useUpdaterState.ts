import type { Update } from '@tauri-apps/plugin-updater'

export type AppUpdate = Pick<Update, 'currentVersion' | 'version' | 'date' | 'body'>
export type InstallableUpdate = AppUpdate & Pick<Update, 'downloadAndInstall'>

let pendingUpdate: InstallableUpdate | null = null

export function useUpdaterState() {
  const available = useState<boolean>('updater:available', () => false)
  const version = useState<string>('updater:version', () => '')
  const latestUpdate = useState<AppUpdate | null>('updater:update', () => null)
  const modalOpen = useState<boolean>('updater:modal-open', () => false)

  function setUpdate(update: InstallableUpdate | null) {
    pendingUpdate = update
    latestUpdate.value = update
      ? {
          currentVersion: update.currentVersion,
          version: update.version,
          date: update.date,
          body: update.body,
        }
      : null
    available.value = Boolean(update)
    version.value = update?.version ?? ''
  }

  async function downloadAndInstallUpdate() {
    if (!pendingUpdate) {
      throw new Error('The pending update is no longer available. Check for updates again.')
    }

    await pendingUpdate.downloadAndInstall()
  }

  return {
    available,
    version,
    latestUpdate,
    modalOpen,
    setUpdate,
    downloadAndInstallUpdate,
  }
}

export function shouldUseMockUpdate() {
  if (!import.meta.dev || !import.meta.client) return false
  return window.localStorage.getItem('sik-render-mock-update') === '1'
}

export function createMockUpdate(currentVersion: string): InstallableUpdate {
  return {
    currentVersion,
    version: '9.9.9-dev',
    date: new Date().toISOString(),
    body: [
      `### Changes since v${currentVersion}`,
      '',
      '- feat: add updater badge and modal (local mock)',
      '- fix: verify release notes rendering in the app',
    ].join('\n'),
    async downloadAndInstall() {
      await new Promise(resolve => window.setTimeout(resolve, 600))
      throw new Error('Local mock updates do not download installers. Disable the mock and test installation with a real release.')
    },
  }
}
