import { finder } from '@medv/finder'
import type { AnchorDescriptor, Fingerprint, Point, TextQuote } from './descriptor'

/** Trim + compactage des espaces ; tronque à 120 chars (taille d'empreinte). */
export function normalizeText(s: string): string {
  return s.replace(/\s+/g, ' ').trim().slice(0, 120)
}

/** Rôle ARIA explicite, sinon rôle implicite minimal, sinon null. */
function roleOf(el: Element): string | null {
  const explicit = el.getAttribute('role')
  if (explicit) return explicit
  const implicit: Record<string, string> = {
    BUTTON: 'button',
    A: 'link',
    NAV: 'navigation',
    MAIN: 'main',
    HEADER: 'banner',
  }
  return implicit[el.tagName] ?? null
}

/** Ordinal parmi les frères de même balise (0-based). */
function ordinalAmongSiblings(el: Element): number {
  const parent = el.parentElement
  if (!parent) return 0
  let n = 0
  for (const sib of Array.from(parent.children)) {
    if (sib === el) return n
    if (sib.tagName === el.tagName) n++
  }
  return n
}

function fingerprintOf(el: Element): Fingerprint {
  return {
    tag: el.tagName.toLowerCase(),
    text: normalizeText(el.textContent ?? ''),
    role: roleOf(el),
    ordinal: ordinalAmongSiblings(el),
  }
}

/** Citation texte W3C : exact + voisinage (jusqu'à 32 chars de chaque côté). */
function textQuoteOf(el: Element): TextQuote | null {
  const exact = normalizeText(el.textContent ?? '')
  if (!exact) return null
  const root = el.ownerDocument?.body
  const full = normalizeText(root?.textContent ?? '')
  const idx = full.indexOf(exact)
  const prefix = idx > 0 ? full.slice(Math.max(0, idx - 32), idx) : ''
  const suffix =
    idx >= 0 ? full.slice(idx + exact.length, idx + exact.length + 32) : ''
  return { exact, prefix, suffix }
}

/** Sélecteur stable : finder en excluant les classes manifestement volatiles. */
function selectorOf(el: Element, root: Document): string {
  try {
    return finder(el, {
      root: root.body,
      className: (name) => !/^(is-|has-|css-|sc-)/.test(name) && !/\d{4,}/.test(name),
    })
  } catch {
    return el.tagName.toLowerCase()
  }
}

/**
 * Capture un descripteur d'ancrage pour `el`.
 * `clickPoint` est en px relatifs au coin haut-gauche de `el` (espace client de l'élément).
 */
export function describe(
  el: Element,
  clickPoint: Point,
  root: Document = el.ownerDocument,
): AnchorDescriptor {
  const rect = el.getBoundingClientRect()
  const offset: Point = {
    x: rect.width > 0 ? clamp01(clickPoint.x / rect.width) : 0.5,
    y: rect.height > 0 ? clamp01(clickPoint.y / rect.height) : 0.5,
  }
  const docEl = root.documentElement
  const fallbackPoint: Point = {
    x: docEl.scrollWidth > 0 ? clamp01((rect.left + clickPoint.x) / docEl.scrollWidth) : 0,
    y: docEl.scrollHeight > 0 ? clamp01((rect.top + clickPoint.y) / docEl.scrollHeight) : 0,
  }
  return {
    v: 1,
    selector: selectorOf(el, root),
    fingerprint: fingerprintOf(el),
    textQuote: textQuoteOf(el),
    offset,
    fallbackPoint,
  }
}

function clamp01(n: number): number {
  if (Number.isNaN(n)) return 0
  return Math.min(1, Math.max(0, n))
}
