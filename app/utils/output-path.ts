export function resolveOutputDirectory(path: string | null | undefined) {
  const rawPath = (path ?? '').trim()
  if (!rawPath) return ''

  const normalized = rawPath.replace(/\\/g, '/')
  const slashIndex = normalized.lastIndexOf('/')
  if (slashIndex < 0) return rawPath

  const tail = normalized.slice(slashIndex + 1)
  const looksLikeFile = /#+/.test(tail) || /\.[A-Za-z0-9]{2,5}$/.test(tail)

  return looksLikeFile ? rawPath.slice(0, slashIndex) : rawPath
}

export function resolvePathBaseName(path: string | null | undefined) {
  const rawPath = (path ?? '').trim()
  if (!rawPath) return ''

  const normalized = rawPath.replace(/\\/g, '/')
  const slashIndex = normalized.lastIndexOf('/')
  return slashIndex < 0 ? rawPath : normalized.slice(slashIndex + 1)
}
