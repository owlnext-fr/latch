import { describe, expect, it } from 'vitest'
import { initialPickState, pickReducer } from './pick-machine'
import type { AnchorDescriptor } from '../anchor/descriptor'

const anchor = {
  v: 1,
  selector: 'button',
  fingerprint: { tag: 'button', text: 'x', role: null, ordinal: 0 },
  textQuote: null,
  offset: { x: 0.5, y: 0.5 },
  fallbackPoint: { x: 0, y: 0 },
} satisfies AnchorDescriptor
const rect = { x: 1, y: 2, width: 3, height: 4 }

describe('pickReducer', () => {
  it('starts idle', () => {
    expect(initialPickState).toEqual({ mode: 'idle' })
  })

  it('ENTER_PICK moves idle -> pick', () => {
    expect(pickReducer(initialPickState, { type: 'ENTER_PICK' })).toEqual({ mode: 'pick' })
  })

  it('CAPTURE moves pick -> compose with the anchor and rect', () => {
    const next = pickReducer({ mode: 'pick' }, { type: 'CAPTURE', anchor, rect })
    expect(next).toEqual({ mode: 'compose', anchor, rect })
  })

  it('SUBMITTED returns to idle', () => {
    expect(pickReducer({ mode: 'compose', anchor, rect }, { type: 'SUBMITTED' })).toEqual({
      mode: 'idle',
    })
  })

  it('CANCEL from any mode returns to idle', () => {
    expect(pickReducer({ mode: 'pick' }, { type: 'CANCEL' })).toEqual({ mode: 'idle' })
    expect(pickReducer({ mode: 'compose', anchor, rect }, { type: 'CANCEL' })).toEqual({
      mode: 'idle',
    })
  })

  it('ignores CAPTURE when not in pick mode', () => {
    expect(pickReducer({ mode: 'idle' }, { type: 'CAPTURE', anchor, rect })).toEqual({
      mode: 'idle',
    })
  })
})
