import { describe, expect, it, vi } from 'vitest'
import { FollowController } from './controller'
import type { Picker, ShellRect } from '../picker/picker'
import type { AnchorDescriptor } from '../anchor/descriptor'

function anchor(id: number): AnchorDescriptor {
  return {
    v: 1,
    selector: `#p${id}`,
    fingerprint: { tag: 'div', text: '', role: null, ordinal: 0 },
    textQuote: null,
    offset: { x: 0.5, y: 0.5 },
    fallbackPoint: { x: 0.1, y: 0.1 },
  }
}

/** Picker factice : résout #p1 vers un élément, #p2 vers orphaned. */
function fakePicker(): Picker {
  let onChange: (() => void) | null = null
  const el = { tag: 'el' } as unknown as Element
  return {
    doc: document,
    getElementAt: () => null,
    describe: () => anchor(0),
    resolve: (a: AnchorDescriptor) =>
      a.selector === '#p1'
        ? { element: el, status: 'anchored' as const }
        : { element: null, status: 'orphaned' as const },
    toShellRect: (): ShellRect => ({ x: 1, y: 2, width: 3, height: 4 }),
    fallbackRect: (): ShellRect => ({ x: 9, y: 9, width: 0, height: 0 }),
    subscribe: (cb: () => void) => {
      onChange = cb
      return () => {
        onChange = null
      }
    },
    // helper exposé au test pour simuler un scroll
    ...({ fire: () => onChange?.() } as object),
  } as unknown as Picker & { fire: () => void }
}

describe('FollowController', () => {
  it('emits a position per pin on the next frame', () => {
    const frames: Array<() => void> = []
    const picker = fakePicker()
    const ctrl = new FollowController(picker, { requestFrame: (cb) => frames.push(cb) })
    const updates: unknown[] = []
    ctrl.onUpdate((p) => updates.push(p))
    ctrl.setPins([{ id: 1, anchor: anchor(1) }, { id: 2, anchor: anchor(2) }])
    ctrl.start()
    expect(frames).toHaveLength(1) // 1 frame schedulée, pas N
    frames[0]() // exécuter la frame
    expect(updates).toHaveLength(1)
    const positions = updates[0] as Array<{ id: number; status: string; rect: ShellRect }>
    expect(positions).toHaveLength(2)
    expect(positions[0]).toMatchObject({ id: 1, status: 'anchored', rect: { x: 1, y: 2 } })
    expect(positions[1]).toMatchObject({ id: 2, status: 'orphaned', rect: { x: 9, y: 9 } })
  })

  it('coalesces multiple markDirty into a single frame', () => {
    const frames: Array<() => void> = []
    const ctrl = new FollowController(fakePicker(), { requestFrame: (cb) => frames.push(cb) })
    ctrl.onUpdate(() => {})
    ctrl.setPins([{ id: 1, anchor: anchor(1) }])
    ctrl.start()
    ctrl.markDirty()
    ctrl.markDirty()
    expect(frames).toHaveLength(1) // coalescé : une seule frame en vol
  })

  it('stop() unsubscribes from the picker', () => {
    const picker = fakePicker()
    const off = vi.fn()
    picker.subscribe = () => off
    const ctrl = new FollowController(picker, { requestFrame: (cb) => cb() })
    ctrl.onUpdate(() => {})
    ctrl.start()
    ctrl.stop()
    expect(off).toHaveBeenCalled()
  })
})
