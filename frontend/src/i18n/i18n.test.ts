import { describe, expect, it, beforeEach } from 'vitest'
import i18n, { locales } from '@/i18n'

describe('admin i18n', () => {
  beforeEach(async () => {
    await i18n.changeLanguage('en')
  })

  it('resolves flat keys in English', () => {
    expect(i18n.t('common.cancel')).toBe('Cancel')
  })

  it('switches to French', async () => {
    await i18n.changeLanguage('fr')
    expect(i18n.t('common.cancel')).toBe('Annuler')
  })

  it('exposes discovered locales with _meta', () => {
    expect(locales).toEqual([
      { code: 'en', name: 'English', flag: 'GB' },
      { code: 'fr', name: 'Français', flag: 'FR' },
    ])
  })

  it('derives supportedLngs from discovered locales', () => {
    expect(i18n.options.supportedLngs).toContain('en')
    expect(i18n.options.supportedLngs).toContain('fr')
  })

  it('does not expose _meta as a translation key', () => {
    expect(i18n.t('_meta')).toBe('_meta')
  })

  it('resolves comment.thread.delete from merged comments fragment', () => {
    expect(i18n.t('comment.thread.delete')).toBe('Delete')
  })

  it('resolves comment.thread.delete in French from merged comments fragment', async () => {
    await i18n.changeLanguage('fr')
    expect(i18n.t('comment.thread.delete')).toBe('Supprimer')
  })
})
