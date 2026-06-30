import { describe, expect, it, beforeEach } from 'vitest'
import { score } from './similarity'
import type { Fingerprint } from './descriptor'

const fp: Fingerprint = { tag: 'button', text: 'En savoir plus', role: 'button', ordinal: 2 }

beforeEach(() => {
  document.body.innerHTML = `
    <button id="exact">En savoir plus</button>
    <button id="othertext">Acheter</button>
    <div id="wrongtag">En savoir plus</div>`
})

describe('score()', () => {
  it('gives 1 (or near) to an exact tag+text+role match', () => {
    expect(score(document.getElementById('exact')!, fp)).toBeGreaterThan(0.8)
  })

  it('penalises a wrong tag', () => {
    const right = score(document.getElementById('exact')!, fp)
    const wrong = score(document.getElementById('wrongtag')!, fp)
    expect(wrong).toBeLessThan(right)
  })

  it('penalises different text', () => {
    expect(score(document.getElementById('othertext')!, fp)).toBeLessThan(
      score(document.getElementById('exact')!, fp),
    )
  })

  it('returns a value in [0, 1]', () => {
    const s = score(document.getElementById('othertext')!, fp)
    expect(s).toBeGreaterThanOrEqual(0)
    expect(s).toBeLessThanOrEqual(1)
  })
})
