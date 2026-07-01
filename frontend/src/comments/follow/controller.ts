import type { Point } from '../anchor/descriptor'
import type { AnchorDescriptor } from '../anchor/descriptor'
import type { AnchorStatus } from '../anchor/resolve'
import type { Picker, ShellRect } from '../picker/picker'

export interface PinInput {
  id: number
  anchor: AnchorDescriptor
}

export interface PinPosition {
  id: number
  status: AnchorStatus
  rect: ShellRect
  offset: Point
  /** Élément résolu mais non affiché (rect à aire nulle → scène `display:none` du proto). */
  hidden?: boolean
}

type FrameFn = (cb: () => void) => void

const defaultRequestFrame: FrameFn = (cb) =>
  typeof requestAnimationFrame === 'function' ? void requestAnimationFrame(cb) : void cb()

export class FollowController {
  private readonly picker: Picker
  private pins: PinInput[] = []
  private listeners = new Set<(p: PinPosition[]) => void>()
  private unsubscribe: (() => void) | null = null
  private dirty = false
  private frameScheduled = false
  private readonly requestFrame: FrameFn

  constructor(
    picker: Picker,
    opts?: { requestFrame?: FrameFn },
  ) {
    this.picker = picker
    this.requestFrame = opts?.requestFrame ?? defaultRequestFrame
  }

  setPins(pins: PinInput[]): void {
    this.pins = pins
    this.markDirty()
  }

  onUpdate(cb: (positions: PinPosition[]) => void): () => void {
    this.listeners.add(cb)
    return () => this.listeners.delete(cb)
  }

  start(): void {
    this.unsubscribe?.()
    this.unsubscribe = this.picker.subscribe(() => this.markDirty())
    this.markDirty()
  }

  stop(): void {
    this.unsubscribe?.()
    this.unsubscribe = null
  }

  markDirty(): void {
    this.dirty = true
    if (this.frameScheduled) return
    this.frameScheduled = true
    this.requestFrame(() => {
      this.frameScheduled = false
      if (this.dirty) this.measure()
    })
  }

  /** Phase de lecture puis d'émission (un seul passage par frame). */
  private measure(): void {
    this.dirty = false
    const positions: PinPosition[] = this.pins.map((pin) => {
      const res = this.picker.resolve(pin.anchor)
      const found = res.element ? this.picker.toShellRect(res.element) : null
      // Élément résolu mais rect à aire nulle = vue/scène masquée (display:none) du proto :
      // on le signale `hidden` pour ne pas coller le pin en (0,0) ni ouvrir un fil fantôme.
      const hidden = found != null && found.width === 0 && found.height === 0
      const rect = found ?? this.picker.fallbackRect(pin.anchor)
      return { id: pin.id, status: res.status, rect, offset: pin.anchor.offset, hidden }
    })
    for (const cb of this.listeners) cb(positions)
  }
}
