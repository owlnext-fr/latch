import { describe as describeAnchor } from '../anchor/describe'
import { resolve as resolveAnchor, type ResolveResult } from '../anchor/resolve'
import type { AnchorDescriptor, Point } from '../anchor/descriptor'
import type { FrameRef, Picker, ShellRect } from './picker'

export class SameOriginPicker implements Picker {
  constructor(private readonly frame: FrameRef) {}

  get doc(): Document | null {
    return this.frame.contentDocument
  }

  getElementAt(shellX: number, shellY: number): Element | null {
    const doc = this.frame.contentDocument
    if (!doc) return null
    const f = this.frame.getBoundingClientRect()
    return doc.elementFromPoint(shellX - f.left, shellY - f.top)
  }

  describe(el: Element, clickPoint: Point): AnchorDescriptor {
    const doc = this.frame.contentDocument ?? el.ownerDocument
    return describeAnchor(el, clickPoint, doc)
  }

  resolve(anchor: AnchorDescriptor): ResolveResult {
    const doc = this.frame.contentDocument
    if (!doc) return { element: null, status: 'orphaned' }
    return resolveAnchor(doc, anchor)
  }

  toShellRect(el: Element): ShellRect | null {
    const f = this.frame.getBoundingClientRect()
    const r = el.getBoundingClientRect()
    return { x: f.left + r.left, y: f.top + r.top, width: r.width, height: r.height }
  }

  fallbackRect(anchor: AnchorDescriptor): ShellRect {
    const f = this.frame.getBoundingClientRect()
    return {
      x: f.left + anchor.fallbackPoint.x * f.width,
      y: f.top + anchor.fallbackPoint.y * f.height,
      width: 0,
      height: 0,
    }
  }

  subscribe(cb: () => void): () => void {
    const win = this.frame.contentWindow
    const doc = this.frame.contentDocument
    if (!win) return () => {}
    win.addEventListener('scroll', cb, { passive: true, capture: true })
    win.addEventListener('resize', cb)
    let mo: MutationObserver | null = null
    if (doc && typeof MutationObserver !== 'undefined') {
      mo = new MutationObserver(cb)
      mo.observe(doc.body, { childList: true, subtree: true, attributes: true })
    }
    return () => {
      win.removeEventListener('scroll', cb, { capture: true } as EventListenerOptions)
      win.removeEventListener('resize', cb)
      mo?.disconnect()
    }
  }
}
