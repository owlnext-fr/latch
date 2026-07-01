/** Âge relatif compact et localisé (ex. « 2h », « 3d »), borné à la plus grande unité. */
export function timeAgo(iso: string, now: number, locale: string): string {
  const diffSec = Math.max(0, Math.round((now - new Date(iso).getTime()) / 1000))
  const rtf = new Intl.RelativeTimeFormat(locale, { numeric: 'auto', style: 'narrow' })
  if (diffSec < 60) return rtf.format(-diffSec, 'second')
  const diffMin = Math.round(diffSec / 60)
  if (diffMin < 60) return rtf.format(-diffMin, 'minute')
  const diffHour = Math.round(diffMin / 60)
  if (diffHour < 24) return rtf.format(-diffHour, 'hour')
  const diffDay = Math.round(diffHour / 24)
  if (diffDay < 7) return rtf.format(-diffDay, 'day')
  return rtf.format(-Math.round(diffDay / 7), 'week')
}
