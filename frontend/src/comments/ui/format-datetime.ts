/** Date + heure absolues, localisées (ex. « 01/07/2026 14:32 »). Vide si date invalide. */
export function formatDateTime(iso: string, locale: string): string {
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) return ''
  return d.toLocaleString(locale, { dateStyle: 'short', timeStyle: 'short' })
}
