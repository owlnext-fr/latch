import { describe, expect, it, beforeEach } from 'vitest'
import i18n from './i18n'

describe('unlock i18n', () => {
  beforeEach(async () => {
    await i18n.changeLanguage('en')
  })

  it('interpolates the brand placeholder', () => {
    expect(i18n.t('unlock.title_brand', { brand: 'ACME' })).toBe('Prototype prepared for ACME')
  })

  it('switches to French', async () => {
    await i18n.changeLanguage('fr')
    expect(i18n.t('unlock.submit')).toBe('Déverrouiller')
  })

  it('does not expose _meta as a translation key', () => {
    expect(i18n.t('_meta')).toBe('_meta')
  })
})
