import { useState, type MouseEvent } from 'react'
import type { AnchorDescriptor } from '../anchor/descriptor'
import type { Picker, ShellRect } from '../picker/picker'
import type { PinPosition } from '../follow/controller'
import { PinBadge } from './pin-badge'
import { COMMENT_FLUO } from './colors'

const GLOW_CAP = 30

/** Halo intérieur capé (non proportionnel) : petit composant → petit glow, grand → glow borné. */
export function glowShadow(width: number, height: number): string {
  const depth = Math.min(GLOW_CAP, Math.round(0.3 * Math.min(width, height)))
  const spread = Math.round(depth / 6)
  return `inset 0 0 ${depth}px ${spread}px rgba(24, 160, 251, 0.5)`
}

interface OverlayLayerProps {
  picker: Picker
  positions: PinPosition[]
  pickMode: boolean
  onPick: (anchor: AnchorDescriptor, rect: ShellRect) => void
  onPinClick: (pinId: number) => void
  activePinId: number | null
  labelOf: (pinId: number) => string
}

export function OverlayLayer({
  picker,
  positions,
  pickMode,
  onPick,
  onPinClick,
  activePinId,
  labelOf,
}: Readonly<OverlayLayerProps>) {
  const [hover, setHover] = useState<ShellRect | null>(null)

  function onMove(e: MouseEvent) {
    const el = picker.getElementAt(e.clientX, e.clientY)
    setHover(el ? picker.toShellRect(el) : null)
  }

  function onClick(e: MouseEvent) {
    const el = picker.getElementAt(e.clientX, e.clientY)
    if (!el) return
    const shellRect = picker.toShellRect(el)
    if (!shellRect) return
    const clickPoint = {
      x: e.clientX - shellRect.x,
      y: e.clientY - shellRect.y,
    }
    const anchor = picker.describe(el, clickPoint)
    onPick(anchor, shellRect)
  }

  return (
    <div
      className="absolute inset-0 z-50"
      style={{ pointerEvents: pickMode ? 'auto' : 'none' }}
    >
      {pickMode && (
        <div
          data-testid="pick-surface"
          role="none"
          className="absolute inset-0 cursor-crosshair"
          onMouseMove={onMove}
          onClick={onClick}
        />
      )}
      {pickMode && hover && (
        <div
          data-testid="pick-highlight"
          className="pointer-events-none absolute rounded-sm"
          style={{
            left: `${hover.x}px`,
            top: `${hover.y}px`,
            width: `${hover.width}px`,
            height: `${hover.height}px`,
            border: `2px solid ${COMMENT_FLUO}`,
            boxShadow: glowShadow(hover.width, hover.height),
          }}
        />
      )}
      {positions.map((p) => (
        <PinBadge
          key={p.id}
          position={p}
          label={labelOf(p.id)}
          active={p.id === activePinId}
          onClick={() => onPinClick(p.id)}
        />
      ))}
    </div>
  )
}
