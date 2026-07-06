function normalizeDateLocale(locale?: string) {
  return locale === 'en-US' ? 'en-US' : 'zh-CN'
}

export function formatDateTime(date: Date, locale?: string) {
  if (locale === 'en-US') {
    return new Intl.DateTimeFormat('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
      hour: 'numeric',
      minute: '2-digit',
    }).format(date)
  }

  return new Intl.DateTimeFormat(normalizeDateLocale(locale), {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  }).format(date)
}

export function formatTimestamp(timestamp: number, locale?: string) {
  const date = new Date(timestamp)
  if (locale === 'en-US') {
    return new Intl.DateTimeFormat('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
      hour: 'numeric',
      minute: '2-digit',
      second: '2-digit',
    }).format(date)
  }

  return date.toLocaleString(normalizeDateLocale(locale))
}

export function formatShortTimestamp(timestamp: number, locale?: string) {
  if (locale === 'en-US') {
    return new Intl.DateTimeFormat('en-US', {
      month: 'short',
      day: 'numeric',
      hour: 'numeric',
      minute: '2-digit',
      second: '2-digit',
    }).format(new Date(timestamp))
  }

  return new Intl.DateTimeFormat(normalizeDateLocale(locale), {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  }).format(new Date(timestamp))
}

export function formatDuration(ms: number): string {
  const s = Math.round(ms / 1000)
  if (s < 60) return `${s}s`
  const m = Math.floor(s / 60)
  if (m < 60) return `${m}m ${s % 60}s`
  return `${Math.floor(m / 60)}h ${m % 60}m`
}

export function useDateFormatters() {
  const { locale } = useI18n()

  return {
    formatTimestamp: (timestamp: number) => formatTimestamp(timestamp, locale.value),
    formatShortTimestamp: (timestamp: number) => formatShortTimestamp(timestamp, locale.value),
    formatDateTime: (date: Date) => formatDateTime(date, locale.value),
  }
}
