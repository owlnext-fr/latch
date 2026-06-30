import { cn } from '@/lib/utils'
import type { PinPosition } from '../follow/controller'

interface PinBadgeProps {
  position: PinPosition
  count: number
  active: boolean
  onClick: () => void
}

export function PinBadge({ position, count, active, onClick }: Readonly<PinBadgeProps>) {
  const { rect, offset, status } = position
  const left = rect.x + offset.x * rect.width
  const top = rect.y + offset.y * rect.height
  return (
    <button
      type="button"
      data-status={status}
      onClick={onClick}
      style={{ left: `${left}px`, top: `${top}px`, pointerEvents: 'auto' }}
      className={cn(
        'absolute flex size-7 -translate-x-1/2 -translate-y-1/2 items-center justify-center rounded-full border-2 border-white text-xs font-semibold text-white shadow-md',
        status === 'anchored' ? 'bg-primary' : 'bg-amber-500',
        active && 'ring-primary/40 ring-2',
      )}
    >
      {count}
    </button>
  )
}
