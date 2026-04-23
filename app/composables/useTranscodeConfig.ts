import type { AppSettings, RenderJob, RenderJobTranscodeConfig } from '~/types'

export const TRANSCODE_PRESET_OPTIONS = [
  'ultrafast',
  'superfast',
  'veryfast',
  'faster',
  'fast',
  'medium',
  'slow',
  'slower',
  'veryslow',
] as const

export function sanitizeTranscodeStemPart(value: string | null | undefined) {
  const normalized = (value ?? '').trim().replace(/[<>:"/\\|?*]+/g, '_')
  return normalized || 'render'
}

export function normalizeTranscodeDirectory(value: string | null | undefined) {
  return (value ?? '').trim().replace(/[\\/]+$/, '')
}

export function buildTranscodeOutputPath(directory: string, stem: string) {
  const normalizedDirectory = normalizeTranscodeDirectory(directory)
  const safeStem = sanitizeTranscodeStemPart(stem)
  if (!normalizedDirectory) return `${safeStem}.mp4`

  const separator = normalizedDirectory.includes('\\') && !normalizedDirectory.includes('/') ? '\\' : '/'
  return `${normalizedDirectory}${separator}${safeStem}.mp4`
}

export function splitTranscodeOutputPath(outputPath: string | null | undefined) {
  const rawPath = (outputPath ?? '').trim()
  if (!rawPath) {
    return {
      outputDir: '',
      outputStem: 'render',
    }
  }

  const normalized = rawPath.replace(/\\/g, '/')
  const slashIndex = normalized.lastIndexOf('/')
  const outputDir = slashIndex >= 0 ? rawPath.slice(0, slashIndex).replace(/[\\/]+$/, '') : ''
  const filename = slashIndex >= 0 ? rawPath.slice(slashIndex + 1) : rawPath
  const outputStem = filename.replace(/\.mp4$/i, '') || 'render'

  return {
    outputDir,
    outputStem,
  }
}

function deriveRenderSequenceDirectory(outputPath: string) {
  const normalized = outputPath.replace(/\\/g, '/')
  if (normalized.includes('#')) {
    const slashIndex = normalized.lastIndexOf('/')
    return slashIndex >= 0 ? outputPath.slice(0, slashIndex) : outputPath
  }

  const slashIndex = normalized.lastIndexOf('/')
  return slashIndex >= 0 ? outputPath.slice(0, slashIndex) : outputPath
}

export function defaultTranscodeOutputPathForRenderJob(job: RenderJob) {
  const outputDir = normalizeTranscodeDirectory(deriveRenderSequenceDirectory(job.outputPath))
  return buildTranscodeOutputPath(outputDir, job.name)
}

export function resolveBaseRenderJobTranscodeConfig(
  job: RenderJob,
  settings: AppSettings,
): RenderJobTranscodeConfig {
  const outputPath = defaultTranscodeOutputPathForRenderJob(job)
  const { outputDir, outputStem } = splitTranscodeOutputPath(outputPath)

  return {
    name: job.name,
    fps: Math.max(1, Math.round(job.fps && job.fps > 0 ? job.fps : 30)),
    outputPath,
    outputDir,
    outputStem,
    crf: settings.transcodeCrf,
    preset: settings.transcodePreset,
  }
}

export function resolveEffectiveRenderJobTranscodeConfig(
  job: RenderJob,
  settings: AppSettings,
): RenderJobTranscodeConfig {
  const base = resolveBaseRenderJobTranscodeConfig(job, settings)
  const outputPath = job.transcodeOutputPathOverride || base.outputPath
  const { outputDir, outputStem } = splitTranscodeOutputPath(outputPath)

  return {
    name: job.transcodeNameOverride || base.name,
    fps: Math.max(1, Math.round(job.transcodeFpsOverride && job.transcodeFpsOverride > 0 ? job.transcodeFpsOverride : base.fps)),
    outputPath,
    outputDir,
    outputStem,
    crf: job.transcodeCrfOverride ?? base.crf,
    preset: job.transcodePresetOverride || base.preset,
  }
}
