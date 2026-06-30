import { normalizeText } from './describe'
import { score } from './similarity'
import type { AnchorDescriptor } from './descriptor'

export type AnchorStatus = 'anchored' | 'approximate' | 'orphaned'

export interface ResolveResult {
  element: Element | null
  status: AnchorStatus
}

const STRONG = 0.9
const WEAK = 0.6

function safeQueryAll(doc: Document, selector: string): Element[] {
  try {
    return Array.from(doc.querySelectorAll(selector))
  } catch {
    return [] // sélecteur invalide après évolution du DOM
  }
}

function bestByFingerprint(
  candidates: Element[],
  anchor: AnchorDescriptor,
): { el: Element; s: number } | null {
  let best: { el: Element; s: number } | null = null
  for (const el of candidates) {
    const s = score(el, anchor.fingerprint)
    if (!best || s > best.s) best = { el, s }
  }
  return best
}

function byTextQuote(doc: Document, anchor: AnchorDescriptor): Element | null {
  const exact = anchor.textQuote?.exact
  if (!exact) return null
  const walker = doc.createTreeWalker(doc.body, NodeFilter.SHOW_ELEMENT)
  let node = walker.nextNode() as Element | null
  while (node) {
    if (normalizeText(node.textContent ?? '') === exact) return node
    node = walker.nextNode() as Element | null
  }
  return null
}

export function resolve(doc: Document, anchor: AnchorDescriptor): ResolveResult {
  const direct = safeQueryAll(doc, anchor.selector)

  if (direct.length === 1) return { element: direct[0], status: 'anchored' }

  if (direct.length > 1) {
    const best = bestByFingerprint(direct, anchor)
    if (best) {
      return { element: best.el, status: best.s >= STRONG ? 'anchored' : 'approximate' }
    }
  }

  // 0 match sélecteur : scorer global sur tout le document
  const all = Array.from(doc.body.querySelectorAll('*'))
  const best = bestByFingerprint(all, anchor)
  if (best && best.s >= WEAK) return { element: best.el, status: 'approximate' }

  // dernier recours texte
  const byText = byTextQuote(doc, anchor)
  if (byText) return { element: byText, status: 'approximate' }

  return { element: null, status: 'orphaned' }
}
