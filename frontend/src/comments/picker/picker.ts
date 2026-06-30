import type { AnchorDescriptor, Point } from '../anchor/descriptor'
import type { ResolveResult } from '../anchor/resolve'

/** Rect en coordonnées de l'espace shell (le viewport du parent). */
export interface ShellRect {
  x: number
  y: number
  width: number
  height: number
}

/** Sous-ensemble d'`HTMLIFrameElement` dont le picker a besoin (testable). */
export interface FrameRef {
  contentDocument: Document | null
  contentWindow: Window | null
  getBoundingClientRect(): DOMRect
}

/**
 * Seam d'accès au proto. Seule impl v1 : SameOriginPicker (lit l'iframe same-origin).
 * Une future impl PostMessagePicker (cross-origin) se brancherait sans toucher au reste.
 */
export interface Picker {
  /** Document du proto, ou null tant que l'iframe n'est pas chargée. */
  readonly doc: Document | null
  /** Élément du proto sous un point exprimé en coordonnées shell. */
  getElementAt(shellX: number, shellY: number): Element | null
  /** Descripteur d'ancrage pour `el` ; `clickPoint` en px relatifs à l'élément. */
  describe(el: Element, clickPoint: Point): AnchorDescriptor
  /** Résout un descripteur dans le DOM courant du proto. */
  resolve(anchor: AnchorDescriptor): ResolveResult
  /** Rect de `el` transposé dans l'espace shell, ou null si indisponible. */
  toShellRect(el: Element): ShellRect | null
  /** Rect de repli (orphaned) calculé depuis `fallbackPoint`. */
  fallbackRect(anchor: AnchorDescriptor): ShellRect
  /** Notifie sur scroll/resize/mutation du proto ; renvoie une fonction de désinscription. */
  subscribe(cb: () => void): () => void
}
