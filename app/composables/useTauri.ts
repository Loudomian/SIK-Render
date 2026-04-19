import { invoke } from '@tauri-apps/api/core'
import type { AddJobPayload, AppSettings, BlenderInstall, RenderJob } from '~/types'

// Typed wrappers around Tauri IPC commands

export const useTauri = () => ({
  listJobs: () =>
    invoke<RenderJob[]>('list_jobs'),

  addJob: (payload: AddJobPayload) =>
    invoke<RenderJob>('add_job', { payload }),

  removeJob: (id: string) =>
    invoke<void>('remove_job', { id }),

  cancelJob: (id: string) =>
    invoke<void>('cancel_job', { id }),

  reorderJob: (id: string, priority: number) =>
    invoke<void>('reorder_job', { id, priority }),

  getBlenderVersions: () =>
    invoke<BlenderInstall[]>('get_blender_versions'),

  validateBlendFile: (path: string) =>
    invoke<boolean>('validate_blend_file', { path }),

  getSettings: () =>
    invoke<AppSettings>('get_settings'),

  saveSettings: (settings: AppSettings) =>
    invoke<void>('save_settings', { settings }),
})
