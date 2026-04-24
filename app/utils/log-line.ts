export interface ParsedLogLine {
  timestamp: string | null
  content: string
}

const LOG_TIMESTAMP_PATTERN = /^\[([0-9]{4}-[0-9]{2}-[0-9]{2} [0-9:.]+)\]\s?(.*)$/s

export function parseLogLine(line: string | null | undefined): ParsedLogLine {
  const normalized = line ?? ''
  const match = normalized.match(LOG_TIMESTAMP_PATTERN)
  if (!match) {
    return {
      timestamp: null,
      content: normalized,
    }
  }

  return {
    timestamp: match[1] ?? null,
    content: match[2] ?? '',
  }
}
