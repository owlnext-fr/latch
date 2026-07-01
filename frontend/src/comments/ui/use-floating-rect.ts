import { useCallback, useLayoutEffect, useRef, useState, type CSSProperties, type RefCallback } from 'react'
import { computePosition, flip, limitShift, offset, shift, size, type Middleware } from '@floating-ui/dom'
import type { ShellRect } from '../picker/picker'

/**
 * Pipeline de positionnement : garde le popup DANS le viewport, y compris quand la
 * référence est près d'un bord (le `shift` par défaut ne borne que l'axe d'alignement
 * en `right-start` ; on active `crossAxis` + `limitShift` pour l'axe horizontal, et
 * `size` borne la hauteur des longs threads).
 */
export function floatingMiddleware(): Middleware[] {
  return [
    offset(8),
    flip({ fallbackAxisSideDirection: 'end' }),
    shift({ crossAxis: true, padding: 8, limiter: limitShift() }),
    size({
      padding: 8,
      apply({ availableHeight, elements }) {
        Object.assign(elements.floating.style, {
          maxHeight: `${Math.max(160, availableHeight)}px`,
          overflowY: 'auto',
        })
      },
    }),
  ]
}

/** Positionne un élément flottant contre un rect de l'espace shell (VirtualElement). */
export function useFloatingRect(rect: ShellRect | null): {
  ref: RefCallback<HTMLElement>
  style: CSSProperties
} {
  const [style, setStyle] = useState<CSSProperties>({
    position: 'fixed',
    top: 0,
    left: 0,
    pointerEvents: 'auto',
  })
  const elRef = useRef<HTMLElement | null>(null)

  useLayoutEffect(() => {
    const floating = elRef.current
    if (!floating || !rect) return
    const reference = {
      getBoundingClientRect: () =>
        ({
          x: rect.x,
          y: rect.y,
          width: rect.width,
          height: rect.height,
          top: rect.y,
          left: rect.x,
          right: rect.x + rect.width,
          bottom: rect.y + rect.height,
        }) as DOMRect,
    }
    void computePosition(reference, floating, {
      placement: 'right-start',
      middleware: floatingMiddleware(),
    }).then(({ x, y }) => {
      setStyle({ position: 'fixed', left: `${x}px`, top: `${y}px`, pointerEvents: 'auto' })
    })
  }, [rect])

  const ref = useCallback<RefCallback<HTMLElement>>((node) => {
    elRef.current = node
  }, [])
  return { ref, style }
}
