import type { Point } from '../anchor/descriptor'
import type { ShellRect } from '../picker/picker'

/**
 * Point absolu (espace shell/viewport) où poser le pin / ancrer le popup.
 * `offset` est le point de clic normalisé (0..1) porté par l'AnchorDescriptor.
 * Source unique : partagé par PinBadge (rendu du pin) et les popups (ancrage floating-ui).
 */
export function anchorPoint(rect: ShellRect, offset: Point): { x: number; y: number } {
  return {
    x: rect.x + offset.x * rect.width,
    y: rect.y + offset.y * rect.height,
  }
}
