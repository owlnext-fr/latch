export interface Point {
  x: number
  y: number
}

export interface Fingerprint {
  /** nom de balise en minuscules, ex. "button" */
  tag: string
  /** texte normalisé (trim, espaces compactés), tronqué à 120 chars */
  text: string
  /** rôle ARIA explicite ou implicite, sinon null */
  role: string | null
  /** index parmi les frères même-balise (0-based), pour désambiguïser */
  ordinal: number
}

export interface TextQuote {
  exact: string
  prefix: string
  suffix: string
}

/** Descripteur d'ancrage versionné — format de contrat (spec §5.4). */
export interface AnchorDescriptor {
  v: 1
  /** sélecteur CSS (rung 1, lib finder) */
  selector: string
  /** empreinte (rung 2 : désambiguïsation + base du scorer) */
  fingerprint: Fingerprint
  /** citation texte W3C (rung 3), null si l'élément n'a pas de texte stable */
  textQuote: TextQuote | null
  /** point du clic en % de la boîte de l'élément (placement du pin) */
  offset: Point
  /** coordonnée page normalisée (dernier recours, orphaned/approximate) */
  fallbackPoint: Point
}

export function serializeAnchor(a: AnchorDescriptor): string {
  return JSON.stringify(a)
}

export function parseAnchor(raw: string): AnchorDescriptor | null {
  let parsed: unknown
  try {
    parsed = JSON.parse(raw)
  } catch {
    return null
  }
  if (typeof parsed !== 'object' || parsed === null) return null
  const a = parsed as Record<string, unknown>
  if (a.v !== 1) return null
  return a as unknown as AnchorDescriptor
}
