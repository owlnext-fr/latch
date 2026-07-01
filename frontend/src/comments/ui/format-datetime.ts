/**
 * Date + heure absolues, localisées et lisibles (mois en lettres, jour/heure zéro-paddés).
 * Ex. « 05 mars 2026 à 08:03 » (fr) / « March 05, 2026 at 08:03 AM » (en). Vide si date invalide.
 */
export function formatDateTime(iso: string, locale: string): string {
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) return ''
  return d.toLocaleString(locale, {
    day: '2-digit',
    month: 'long',
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  })
}
