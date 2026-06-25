import { describe, expect, it, vi } from 'vitest'
import { parseLocales } from './available-locales'

const fakeGlob = {
  './locales/admin/fr.json': {
    default: { _meta: { name: 'Français', flag: 'FR' }, 'login.title': 'latch — admin' },
  },
  './locales/admin/en.json': {
    default: { _meta: { name: 'English', flag: 'GB' }, 'login.title': 'latch — admin' },
  },
}

describe('parseLocales', () => {
  it('derives the language code from the filename', () => {
    const { locales } = parseLocales(fakeGlob)
    expect(locales.map((l) => l.code).sort()).toEqual(['en', 'fr'])
  })

  it('sorts locales by code (stable order)', () => {
    const { locales } = parseLocales(fakeGlob)
    expect(locales[0].code).toBe('en')
    expect(locales[1].code).toBe('fr')
  })

  it('exposes name and flag from _meta', () => {
    const { locales } = parseLocales(fakeGlob)
    expect(locales).toEqual([
      { code: 'en', name: 'English', flag: 'GB' },
      { code: 'fr', name: 'Français', flag: 'FR' },
    ])
  })

  it('strips _meta from translation resources', () => {
    const { resources } = parseLocales(fakeGlob)
    expect(resources.en.translation).toEqual({ 'login.title': 'latch — admin' })
    expect('_meta' in resources.en.translation).toBe(false)
  })

  it('falls back to CODE/CODE and warns when _meta is missing', () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {})
    const { locales } = parseLocales({
      './locales/admin/de.json': { default: { 'login.title': 'x' } },
    })
    expect(locales[0]).toEqual({ code: 'de', name: 'DE', flag: 'DE' })
    expect(warn).toHaveBeenCalledOnce()
    warn.mockRestore()
  })

  it('falls back per-field when _meta is partial', () => {
    const { locales } = parseLocales({
      './locales/admin/es.json': { default: { _meta: { name: 'Español' } } },
    })
    expect(locales[0]).toEqual({ code: 'es', name: 'Español', flag: 'ES' })
  })
})
