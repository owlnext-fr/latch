import { describe, expect, it, beforeEach } from 'vitest'
import { describe as describeAnchor, normalizeText } from './describe'

beforeEach(() => {
  document.body.innerHTML = `
    <main>
      <section><button id="a">First</button></section>
      <section>
        <div class="card"><button id="b">En savoir   plus</button></div>
      </section>
    </main>`
})

describe('normalizeText', () => {
  it('trims and collapses whitespace', () => {
    expect(normalizeText('  En savoir   plus\n')).toBe('En savoir plus')
  })
})

describe('describe()', () => {
  it('captures a selector that re-finds the same element', () => {
    const el = document.getElementById('b')!
    const anchor = describeAnchor(el, { x: 5, y: 5 })
    expect(document.querySelector(anchor.selector)).toBe(el)
  })

  it('captures a fingerprint with tag, normalized text and ordinal', () => {
    const el = document.getElementById('b')!
    const anchor = describeAnchor(el, { x: 5, y: 5 })
    expect(anchor.fingerprint.tag).toBe('button')
    expect(anchor.fingerprint.text).toBe('En savoir plus')
    expect(anchor.fingerprint.ordinal).toBe(0) // seul button dans son parent
  })

  it('encodes the click point as an offset fraction of the element box', () => {
    const el = document.getElementById('b')!
    // jsdom renvoie un rect 0x0 ; on stub getBoundingClientRect pour ce test
    el.getBoundingClientRect = () =>
      ({ left: 0, top: 0, width: 100, height: 50 }) as DOMRect
    const anchor = describeAnchor(el, { x: 42, y: 30 })
    expect(anchor.offset.x).toBeCloseTo(0.42, 2)
    expect(anchor.offset.y).toBeCloseTo(0.6, 2)
  })

  it('sets format version 1', () => {
    const anchor = describeAnchor(document.getElementById('a')!, { x: 0, y: 0 })
    expect(anchor.v).toBe(1)
  })
})
