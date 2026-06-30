import { normalizeText, roleOf } from './describe'
import type { Fingerprint } from './descriptor'

/** Similarité de texte par bag-of-words (Jaccard sur les tokens). */
function textSimilarity(a: string, b: string): number {
  const ta = new Set(a.toLowerCase().split(' ').filter(Boolean))
  const tb = new Set(b.toLowerCase().split(' ').filter(Boolean))
  if (ta.size === 0 && tb.size === 0) return 1
  if (ta.size === 0 || tb.size === 0) return 0
  let inter = 0
  for (const t of ta) if (tb.has(t)) inter++
  const union = ta.size + tb.size - inter
  return union === 0 ? 0 : inter / union
}

/**
 * Score de ressemblance d'un candidat à une empreinte (0..1).
 * Pondération : balise 0.4, texte 0.4, rôle 0.2.
 */
export function score(el: Element, fp: Fingerprint): number {
  const tagScore = el.tagName.toLowerCase() === fp.tag ? 1 : 0
  const textScore = textSimilarity(normalizeText(el.textContent ?? ''), fp.text)
  const elRole = roleOf(el)
  const roleScore = fp.role === null ? 1 : elRole === fp.role ? 1 : 0
  return 0.4 * tagScore + 0.4 * textScore + 0.2 * roleScore
}
