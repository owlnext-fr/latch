import { describe, expect, it } from 'vitest'
import { resolve } from './resolve'
import type { AnchorDescriptor } from './descriptor'

function anchorFor(selector: string, text: string): AnchorDescriptor {
  return {
    v: 1,
    selector,
    fingerprint: { tag: 'button', text, role: 'button', ordinal: 0 },
    textQuote: { exact: text, prefix: '', suffix: '' },
    offset: { x: 0.5, y: 0.5 },
    fallbackPoint: { x: 0.5, y: 0.5 },
  }
}

function docWith(html: string): Document {
  const doc = document.implementation.createHTMLDocument('t')
  doc.body.innerHTML = html
  return doc
}

describe('resolve()', () => {
  it('returns anchored on a unique selector match', () => {
    const doc = docWith('<button class="cta">Buy</button>')
    const res = resolve(doc, anchorFor('button.cta', 'Buy'))
    expect(res.status).toBe('anchored')
    expect(res.element?.textContent).toBe('Buy')
  })

  it('falls back to fingerprint scoring when the selector misses', () => {
    const doc = docWith('<section><button class="renamed">Buy</button></section>')
    const res = resolve(doc, anchorFor('button.cta', 'Buy'))
    expect(res.status).toBe('approximate')
    expect(res.element?.textContent).toBe('Buy')
  })

  it('uses textQuote when fingerprint scoring is weak', () => {
    const doc = docWith('<p>Some unique sentence here</p>')
    const anchor = anchorFor('button.gone', 'Some unique sentence here')
    const res = resolve(doc, anchor)
    expect(res.status).toBe('approximate')
    expect(res.element?.tagName).toBe('P')
  })

  it('returns orphaned when nothing matches', () => {
    const doc = docWith('<div>totally different</div>')
    const res = resolve(doc, anchorFor('button.cta', 'Vanished label'))
    expect(res.status).toBe('orphaned')
    expect(res.element).toBeNull()
  })
})
