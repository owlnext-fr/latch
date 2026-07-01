import { cn } from '@/lib/utils'
import type { PinPosition } from '../follow/controller'
import { COMMENT_FLUO } from './colors'

interface PinBadgeProps {
  position: PinPosition
  label: string
  active: boolean
  onClick: () => void
}

export function PinBadge({ position, label, active, onClick }: Readonly<PinBadgeProps>) {
  const { rect, offset, status } = position
  const left = rect.x + offset.x * rect.width
  const top = rect.y + offset.y * rect.height
  const anchored = status === 'anchored'
  return (
    <button
      type="button"
      data-testid="pin-badge"
      data-status={status}
      onClick={onClick}
      style={{
        left: `${left}px`,
        top: `${top}px`,
        pointerEvents: 'auto',
        background: anchored ? COMMENT_FLUO : undefined,
      }}
      className={cn(
        'absolute flex size-7 -translate-x-1/2 -translate-y-1/2 items-center justify-center rounded-full border-2 border-white text-xs font-semibold text-white shadow-md',
        !anchored && 'bg-amber-500',
        active && 'ring-2 ring-black/25',
      )}
    >
      {label}
    </button>
  )
}
