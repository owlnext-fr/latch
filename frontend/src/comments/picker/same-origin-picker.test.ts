import { describe, expect, it, vi } from 'vitest'
import { SameOriginPicker } from './same-origin-picker'
import type { FrameRef } from './picker'

/** Construit un faux iframe pointant sur un document jsdom détaché. */
function fakeFrame(html: string, frameRect: Partial<DOMRect> = {}): {
  frame: FrameRef
  doc: Document
  win: { addEventListener: ReturnType<typeof vi.fn>; removeEventListener: ReturnType<typeof vi.fn> }
} {
  const doc = document.implementation.createHTMLDocument('proto')
  doc.body.innerHTML = html
  const win = { addEventListener: vi.fn(), removeEventListener: vi.fn() }
  const frame: FrameRef = {
    contentDocument: doc,
    contentWindow: win as unknown as Window,
    getBoundingClientRect: () =>
      ({ left: 10, top: 20, width: 800, height: 600, ...frameRect }) as DOMRect,
  }
  return { frame, doc, win }
}

describe('SameOriginPicker', () => {
  it('exposes the content document', () => {
    const { frame, doc } = fakeFrame('<button>Hi</button>')
    expect(new SameOriginPicker(frame).doc).toBe(doc)
  })

  it('getElementAt translates shell coords into iframe coords', () => {
    const { frame, doc } = fakeFrame('<button id="b">Hi</button>')
    const target = doc.getElementById('b')!
    // elementFromPoint is not an own property on detached jsdom documents (vitest v4 requires it)
    Object.defineProperty(doc, 'elementFromPoint', { value: () => null, writable: true, configurable: true })
    const spy = vi.spyOn(doc, 'elementFromPoint').mockReturnValue(target)
    const picker = new SameOriginPicker(frame)
    const el = picker.getElementAt(110, 220) // shell (110,220) - frame (10,20) = (100,200)
    expect(spy).toHaveBeenCalledWith(100, 200)
    expect(el).toBe(target)
  })

  it('toShellRect offsets the element rect by the frame position', () => {
    const { frame, doc } = fakeFrame('<button id="b">Hi</button>')
    const el = doc.getElementById('b')!
    el.getBoundingClientRect = () =>
      ({ left: 5, top: 7, width: 30, height: 12 }) as DOMRect
    const rect = new SameOriginPicker(frame).toShellRect(el)
    expect(rect).toEqual({ x: 15, y: 27, width: 30, height: 12 }) // +frame(10,20)
  })

  it('subscribe attaches scroll/resize listeners and unsubscribe detaches', () => {
    const { frame, win } = fakeFrame('<div>x</div>')
    const cb = vi.fn()
    const off = new SameOriginPicker(frame).subscribe(cb)
    expect(win.addEventListener).toHaveBeenCalledWith('scroll', expect.any(Function), expect.anything())
    expect(win.addEventListener).toHaveBeenCalledWith('resize', expect.any(Function))
    off()
    expect(win.removeEventListener).toHaveBeenCalled()
  })

  it('getElementAt returns null when the document is not ready', () => {
    const frame: FrameRef = {
      contentDocument: null,
      contentWindow: null,
      getBoundingClientRect: () => ({ left: 0, top: 0, width: 0, height: 0 }) as DOMRect,
    }
    expect(new SameOriginPicker(frame).getElementAt(1, 1)).toBeNull()
  })
})
