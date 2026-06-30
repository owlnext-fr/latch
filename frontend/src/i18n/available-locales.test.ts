import { describe, expect, it, vi } from 'vitest'
import { parseLocales, mergeFragmentGlob } from './available-locales'

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
    expect(locales.map((l) => l.code)).toEqual(['en', 'fr'])
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
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {})
    const { locales } = parseLocales({
      './locales/admin/es.json': { default: { _meta: { name: 'Español' } } },
    })
    expect(locales[0]).toEqual({ code: 'es', name: 'Español', flag: 'ES' })
    expect(warn).not.toHaveBeenCalled()
    warn.mockRestore()
  })
})

describe('mergeFragmentGlob', () => {
  it('merges keys into existing locale resources by language code', () => {
    const resources: Record<string, { translation: Record<string, string> }> = {
      en: { translation: { 'existing.key': 'hello' } },
    }
    mergeFragmentGlob(resources, {
      './locales/comments/en.json': { default: { 'comment.thread.delete': 'Delete' } },
    })
    expect(resources.en.translation).toEqual({
      'existing.key': 'hello',
      'comment.thread.delete': 'Delete',
    })
  })

  it('ignores _meta from fragment files', () => {
    const resources: Record<string, { translation: Record<string, string> }> = {
      en: { translation: {} },
    }
    mergeFragmentGlob(resources, {
      './locales/comments/en.json': {
        default: { _meta: { name: 'English', flag: 'GB' }, 'comment.bar.pick': 'Comment' },
      },
    })
    expect('_meta' in resources.en.translation).toBe(false)
    expect(resources.en.translation['comment.bar.pick']).toBe('Comment')
  })

  it('creates a new locale entry when the code is absent from resources', () => {
    const resources: Record<string, { translation: Record<string, string> }> = {}
    mergeFragmentGlob(resources, {
      './locales/comments/de.json': { default: { 'comment.bar.pick': 'Kommentieren' } },
    })
    expect(resources.de.translation['comment.bar.pick']).toBe('Kommentieren')
  })

  it('merges keys for multiple languages in one glob', () => {
    const resources: Record<string, { translation: Record<string, string> }> = {
      en: { translation: {} },
      fr: { translation: {} },
    }
    mergeFragmentGlob(resources, {
      './locales/comments/en.json': { default: { 'comment.thread.delete': 'Delete' } },
      './locales/comments/fr.json': { default: { 'comment.thread.delete': 'Supprimer' } },
    })
    expect(resources.en.translation['comment.thread.delete']).toBe('Delete')
    expect(resources.fr.translation['comment.thread.delete']).toBe('Supprimer')
  })
})
