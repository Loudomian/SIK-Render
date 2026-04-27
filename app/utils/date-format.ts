const shortTimestampFormatter = new Intl.DateTimeFormat('zh-CN', {
  month: '2-digit',
  day: '2-digit',
  hour: '2-digit',
  minute: '2-digit',
  second: '2-digit',
})

export function formatTimestamp(timestamp: number) {
  return new Date(timestamp).toLocaleString()
}

export function formatShortTimestamp(timestamp: number) {
  return shortTimestampFormatter.format(new Date(timestamp))
}
