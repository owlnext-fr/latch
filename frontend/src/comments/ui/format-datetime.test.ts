import { describe, expect, it } from 'vitest'
import { formatDateTime } from './format-datetime'

describe('formatDateTime', () => {
  it('rend une date lisible : mois en lettres + heure zéro-paddée', () => {
    const out = formatDateTime('2026-03-05T09:03:00Z', 'en-US')
    expect(out).not.toBe('')
    // mois écrit en lettres (pas seulement des chiffres)
    expect(out).toMatch(/[A-Za-z]/)
    // heure zéro-paddée h:mm (2 chiffres)
    expect(out).toMatch(/\d{2}:\d{2}/)
  })

  it('rend une chaîne vide pour une date invalide', () => {
    expect(formatDateTime('', 'en')).toBe('')
    expect(formatDateTime('not-a-date', 'en')).toBe('')
  })
})
