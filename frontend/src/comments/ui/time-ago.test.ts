import { describe, expect, it } from 'vitest'
import { timeAgo } from './time-ago'

const now = new Date('2026-07-01T12:00:00Z').getTime()

describe('timeAgo', () => {
  it('formate en heures', () => {
    expect(timeAgo('2026-07-01T10:00:00Z', now, 'en')).toContain('2')
  })
  it('formate en jours', () => {
    expect(timeAgo('2026-06-28T12:00:00Z', now, 'en')).toContain('3')
  })
  it('borne à 0 pour un futur proche', () => {
    expect(timeAgo('2026-07-01T12:00:05Z', now, 'en')).toMatch(/now|0/)
  })
})
