import type { Update } from '@tauri-apps/plugin-updater'

export type AppUpdate = Pick<Update, 'currentVersion' | 'version' | 'date' | 'body' | 'downloadAndInstall'>

export function useUpdaterState() {
  const available = useState<boolean>('updater:available', () => false)
  const version = useState<string>('updater:version', () => '')
  const latestUpdate = useState<AppUpdate | null>('updater:update', () => null)
  const modalOpen = useState<boolean>('updater:modal-open', () => false)

  function setUpdate(update: AppUpdate | null) {
    latestUpdate.value = update
    available.value = Boolean(update)
    version.value = update?.version ?? ''
  }

  return {
    available,
    version,
    latestUpdate,
    modalOpen,
    setUpdate,
  }
}

export function shouldUseMockUpdate() {
  if (!import.meta.dev || !import.meta.client) return false
  return window.localStorage.getItem('sik-render-mock-update') === '1'
}

export function createMockUpdate(currentVersion: string): AppUpdate {
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
      throw new Error('本地 mock 更新不会下载安装包。关闭 mock 后使用真实 Release 测试安装。')
    },
  }
}
