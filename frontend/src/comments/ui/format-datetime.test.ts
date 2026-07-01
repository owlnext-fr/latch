import { describe, expect, it } from 'vitest'
import { formatDateTime } from './format-datetime'

describe('formatDateTime', () => {
  it('rend une date absolue AVEC heure (pas seulement le jour)', () => {
    const out = formatDateTime('2026-07-01T14:32:00Z', 'en-US')
    expect(out).not.toBe('')
    // porte une heure au format h:mm
    expect(out).toMatch(/\d{1,2}:\d{2}/)
  })

  it('rend une chaîne vide pour une date invalide', () => {
    expect(formatDateTime('', 'en')).toBe('')
    expect(formatDateTime('not-a-date', 'en')).toBe('')
  })
})
