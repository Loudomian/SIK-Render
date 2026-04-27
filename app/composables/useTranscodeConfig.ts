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
