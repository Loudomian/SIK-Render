export function findAppendedLogLines(
  lines: string[],
  previousLines: string[],
  lastKnownLine?: string | null,
) {
  if (!lines.length) return []
  if (!previousLines.length) return lines

  if (lines.length > previousLines.length && linesStartWith(lines, previousLines)) {
    return lines.slice(previousLines.length)
  }

  if (
    lines.length === previousLines.length
    && lines.length > 1
    && linesStartWith(lines, previousLines.slice(1), lines.length - 1)
  ) {
    return [lines[lines.length - 1] as string]
  }

  const lastLine = lines.at(-1)
  return lastLine && lastKnownLine !== lastLine ? [lastLine] : []
}

function linesStartWith(lines: string[], prefix: string[], length = prefix.length) {
  if (length > lines.length || length > prefix.length) return false
  for (let index = 0; index < length; index++) {
    if (lines[index] !== prefix[index]) return false
  }
  return true
}
