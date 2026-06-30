import { describe, expect, it } from 'vitest'
import {
  parseAnchor,
  serializeAnchor,
  type AnchorDescriptor,
} from './descriptor'

const sample: AnchorDescriptor = {
  v: 1,
  selector: 'main > section .card > button',
  fingerprint: { tag: 'button', text: 'En savoir plus', role: 'button', ordinal: 2 },
  textQuote: { exact: 'En savoir plus', prefix: 'avant ', suffix: ' après' },
  offset: { x: 0.42, y: 0.6 },
  fallbackPoint: { x: 0.31, y: 0.78 },
}

describe('anchor descriptor', () => {
  it('round-trips through serialize/parse', () => {
    expect(parseAnchor(serializeAnchor(sample))).toEqual(sample)
  })

  it('returns null on invalid JSON', () => {
    expect(parseAnchor('{not json')).toBeNull()
  })

  it('returns null when version is not 1', () => {
    const raw = JSON.stringify({ ...sample, v: 2 })
    expect(parseAnchor(raw)).toBeNull()
  })

  it('accepts a null textQuote', () => {
    const noQuote = { ...sample, textQuote: null }
    expect(parseAnchor(serializeAnchor(noQuote))).toEqual(noQuote)
  })
})
