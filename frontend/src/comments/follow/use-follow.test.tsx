import { describe, expect, it, vi } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useFollow } from './use-follow'
import type { Picker, ShellRect } from '../picker/picker'
import type { AnchorDescriptor } from '../anchor/descriptor'

function anchor(): AnchorDescriptor {
  return {
    v: 1,
    selector: '#x',
    fingerprint: { tag: 'div', text: '', role: null, ordinal: 0 },
    textQuote: null,
    offset: { x: 0.5, y: 0.5 },
    fallbackPoint: { x: 0, y: 0 },
  }
}

function fakePicker(): Picker {
  const el = {} as Element
  return {
    doc: document,
    getElementAt: () => null,
    describe: anchor,
    resolve: () => ({ element: el, status: 'anchored' }),
    toShellRect: (): ShellRect => ({ x: 5, y: 6, width: 7, height: 8 }),
    fallbackRect: (): ShellRect => ({ x: 0, y: 0, width: 0, height: 0 }),
    subscribe: () => () => {},
  }
}

describe('useFollow', () => {
  it('returns a position per pin after the synchronous frame', () => {
    // rAF n'existe pas forcément en jsdom ; on le rend synchrone le temps du test.
    const raf = vi
      .spyOn(globalThis, 'requestAnimationFrame')
      .mockImplementation((cb: FrameRequestCallback) => {
        cb(0)
        return 0
      })
    let positions: unknown[] = []
    renderHook(() => {
      positions = useFollow(fakePicker(), [{ id: 1, anchor: anchor() }])
      return null
    })
    act(() => {})
    expect(positions).toHaveLength(1)
    expect(positions[0]).toMatchObject({ id: 1, status: 'anchored', rect: { x: 5 } })
    raf.mockRestore()
  })

  it('returns empty array when picker is null', () => {
    let positions: unknown[] = [{ x: 1 }]
    renderHook(() => {
      positions = useFollow(null, [{ id: 1, anchor: anchor() }])
      return null
    })
    expect(positions).toEqual([])
  })
})
