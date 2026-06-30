import { useState, type MouseEvent } from 'react'
import type { AnchorDescriptor } from '../anchor/descriptor'
import type { Picker, ShellRect } from '../picker/picker'
import type { PinPosition } from '../follow/controller'
import { PinBadge } from './pin-badge'

interface OverlayLayerProps {
  picker: Picker
  positions: PinPosition[]
  pickMode: boolean
  onPick: (anchor: AnchorDescriptor, rect: ShellRect) => void
  onPinClick: (pinId: number) => void
  activePinId: number | null
  countOf?: (pinId: number) => number
}

export function OverlayLayer({
  picker,
  positions,
  pickMode,
  onPick,
  onPinClick,
  activePinId,
  countOf,
}: Readonly<OverlayLayerProps>) {
  const [hover, setHover] = useState<ShellRect | null>(null)

  function onMove(e: MouseEvent) {
    if (!pickMode) return
    const el = picker.getElementAt(e.clientX, e.clientY)
    setHover(el ? picker.toShellRect(el) : null)
  }

  function onClick(e: MouseEvent) {
    if (!pickMode) return
    const el = picker.getElementAt(e.clientX, e.clientY)
    if (!el) return
    const shellRect = picker.toShellRect(el)
    if (!shellRect) return
    const clickPoint = { x: e.clientX - shellRect.x, y: e.clientY - shellRect.y }
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
          className="border-primary pointer-events-none absolute rounded-sm border-2"
          style={{
            left: `${hover.x}px`,
            top: `${hover.y}px`,
            width: `${hover.width}px`,
            height: `${hover.height}px`,
          }}
        />
      )}
      {positions.map((p) => (
        <PinBadge
          key={p.id}
          position={p}
          count={countOf ? countOf(p.id) : 1}
          active={p.id === activePinId}
          onClick={() => onPinClick(p.id)}
        />
      ))}
    </div>
  )
}
