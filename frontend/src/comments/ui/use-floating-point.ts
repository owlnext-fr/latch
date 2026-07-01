import { useCallback, useLayoutEffect, useRef, useState, type CSSProperties, type RefCallback } from 'react'
import { computePosition, flip, limitShift, offset, shift, size, type Middleware } from '@floating-ui/dom'

/** Rayon du pin (PinBadge = `size-7` → 28px de diamètre). */
export const PIN_RADIUS = 14
/** Écart visible entre le bord du pin et le popup. */
export const GAP = 8
/** Distance floating-ui (perpendiculaire au placement) qui dégage le pin → pin visible à côté. */
export const POPUP_OFFSET = PIN_RADIUS + GAP

/**
 * Pipeline de positionnement : garde le popup DANS le viewport, y compris quand la
 * référence est près d'un bord (`shift` avec `crossAxis`+`limitShift` pour l'axe
 * horizontal, `size` borne la hauteur des longs threads). `offset(POPUP_OFFSET)`
 * dégage le pin quel que soit le côté après `flip`.
 */
export function floatingMiddleware(): Middleware[] {
  return [
    offset(POPUP_OFFSET),
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

/**
 * Positionne un élément flottant contre un POINT de l'espace shell (viewport) via un
 * VirtualElement de taille nulle. Le popup s'ouvre donc collé au pin (Figma-like),
 * indépendamment de la taille de l'élément ancré.
 */
export function useFloatingPoint(point: { x: number; y: number } | null): {
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
    if (!floating || !point) return
    const reference = {
      getBoundingClientRect: () =>
        ({
          x: point.x,
          y: point.y,
          width: 0,
          height: 0,
          top: point.y,
          left: point.x,
          right: point.x,
          bottom: point.y,
        }) as DOMRect,
    }
    void computePosition(reference, floating, {
      placement: 'right-start',
      middleware: floatingMiddleware(),
    }).then(({ x, y }) => {
      setStyle({ position: 'fixed', left: `${x}px`, top: `${y}px`, pointerEvents: 'auto' })
    })
  }, [point])

  const ref = useCallback<RefCallback<HTMLElement>>((node) => {
    elRef.current = node
  }, [])
  return { ref, style }
}
