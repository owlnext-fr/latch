# Popups de commentaires ancrés au pin — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Faire s'ouvrir les popups de commentaires (`ThreadPopup` + `ComposePopup`) collés au **point du pin** au lieu du côté du bounding box de l'élément cible, façon Figma.

**Architecture:** Frontend-only. On factorise le calcul du point d'ancrage (déjà fait inline dans `PinBadge`) dans un helper pur `anchorPoint(rect, offset)`, puis on transforme le hook de positionnement pour qu'il pointe sur un `VirtualElement` de **taille nulle** à ce point (au lieu du rect complet). Tout le pipeline `@floating-ui/dom` existant (`offset → flip → shift → size`) est conservé ; seul le point de référence et la valeur d'`offset` (dégager le pin) changent. Backend et modèle d'ancrage inchangés.

**Tech Stack:** React 19 + TypeScript (Vite, pnpm), `@floating-ui/dom` `^1.7.6`, Vitest + Testing Library (jsdom).

## Global Constraints

- **Frontend-only** : aucun endpoint, aucun changement backend, aucun changement du modèle/descripteur d'ancrage.
- **Composants partagés client ⇄ admin** : toute modif de `ui/*` profite aux deux surfaces (seul l'`adapter` diffère). Ne pas casser l'une en corrigeant l'autre.
- **Gate « terminé »** : `pnpm lint` ET `pnpm typecheck` ET `pnpm test` verts (depuis `frontend/`). `eslint-plugin-react-hooks` v7 est strict ; `tsconfig` a `erasableSyntaxOnly` (pas de *parameter properties*). Cf. `docs/QUIRKS.md`.
- **Commits** : conventionnels + gitmoji `<gitmoji> <type>: <desc>`. Terminer chaque message par les trailers :
  ```
  Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01MoL7PcQp9Pp49xKcmRqS7q
  ```
- **jsdom** : `computePosition` de floating-ui retombe en (0,0) sans layout réel → les tests de **valeur** de position ne sont pas significatifs ; on teste les fonctions pures et la structure du pipeline, la vérif visuelle finale se fait au navigateur.
- **CWD** : toutes les commandes `pnpm` se lancent depuis `frontend/`.

## File Structure

- **Create** `frontend/src/comments/ui/anchor-point.ts` — helper pur `anchorPoint(rect, offset) → {x,y}`. Source unique du calcul de position du pin.
- **Create** `frontend/src/comments/ui/anchor-point.test.ts` — tests unitaires du helper.
- **Rename+Modify** `frontend/src/comments/ui/use-floating-rect.ts` → `use-floating-point.ts` — hook point-based (`VirtualElement` zéro-size) + constantes `PIN_RADIUS`/`GAP`/`POPUP_OFFSET`.
- **Rename+Modify** `frontend/src/comments/ui/use-floating-rect.test.ts` → `use-floating-point.test.ts` — pipeline + valeur de l'offset.
- **Modify** `frontend/src/comments/ui/pin-badge.tsx` — consomme `anchorPoint` (dédup).
- **Modify** `frontend/src/comments/ui/thread-popup.tsx` — `useFloatingPoint(anchorPoint(position.rect, position.offset))`.
- **Modify** `frontend/src/comments/ui/compose-popup.tsx` — prop `rect` → `point`, `useFloatingPoint(point)`.
- **Modify** `frontend/src/comments/ui/compose-popup.test.tsx` — prop `point`.
- **Modify** `frontend/src/comments/comments-app.tsx` — calcule le point et le passe à `ComposePopup`.

---

### Task 1: Helper pur `anchorPoint`

**Files:**
- Create: `frontend/src/comments/ui/anchor-point.ts`
- Test: `frontend/src/comments/ui/anchor-point.test.ts`

**Interfaces:**
- Consumes: `ShellRect` (de `../picker/picker`), `Point` (de `../anchor/descriptor`).
- Produces: `anchorPoint(rect: ShellRect, offset: Point): { x: number; y: number }` — point absolu (espace shell/viewport) du pin, `x = rect.x + offset.x*rect.width`, `y = rect.y + offset.y*rect.height`.

- [ ] **Step 1: Écrire le test qui échoue**

Créer `frontend/src/comments/ui/anchor-point.test.ts` :

```ts
import { describe, expect, it } from 'vitest'
import { anchorPoint } from './anchor-point'

describe('anchorPoint', () => {
  const rect = { x: 100, y: 50, width: 80, height: 40 }

  it('coin haut-gauche pour offset {0,0}', () => {
    expect(anchorPoint(rect, { x: 0, y: 0 })).toEqual({ x: 100, y: 50 })
  })

  it('centre pour offset {0.5,0.5}', () => {
    expect(anchorPoint(rect, { x: 0.5, y: 0.5 })).toEqual({ x: 140, y: 70 })
  })

  it('coin bas-droit pour offset {1,1}', () => {
    expect(anchorPoint(rect, { x: 1, y: 1 })).toEqual({ x: 180, y: 90 })
  })
})
```

- [ ] **Step 2: Lancer le test pour vérifier qu'il échoue**

Run : `pnpm test -- --run anchor-point`
Expected : FAIL — `Failed to resolve import "./anchor-point"` (le module n'existe pas).

- [ ] **Step 3: Écrire l'implémentation minimale**

Créer `frontend/src/comments/ui/anchor-point.ts` :

```ts
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
```

- [ ] **Step 4: Lancer le test pour vérifier qu'il passe**

Run : `pnpm test -- --run anchor-point`
Expected : PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add frontend/src/comments/ui/anchor-point.ts frontend/src/comments/ui/anchor-point.test.ts
git commit -m "$(cat <<'EOF'
✨ feat(comments): helper pur anchorPoint (point du pin)

Source unique du calcul de position du pin (rect + offset normalisé),
préalable à l'ancrage des popups au point du pin.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01MoL7PcQp9Pp49xKcmRqS7q
EOF
)"
```

---

### Task 2: `PinBadge` consomme `anchorPoint` (dédup)

**Files:**
- Modify: `frontend/src/comments/ui/pin-badge.tsx`
- Test (existant, guard) : `frontend/src/comments/ui/pin-badge.test.tsx`

**Interfaces:**
- Consumes: `anchorPoint` (Task 1).
- Produces: rien de nouveau — refactor à comportement identique (le pin reste à `140px/70px` pour le fixture du test).

- [ ] **Step 1: Vérifier que les tests existants passent (baseline verte)**

Run : `pnpm test -- --run pin-badge`
Expected : PASS (les tests existants asservissent `left: 140px`, `top: 70px`).

- [ ] **Step 2: Refactorer `PinBadge` pour utiliser `anchorPoint`**

Dans `frontend/src/comments/ui/pin-badge.tsx`, ajouter l'import et remplacer le calcul inline :

```tsx
import { cn } from '@/lib/utils'
import type { PinPosition } from '../follow/controller'
import { COMMENT_FLUO } from './colors'
import { anchorPoint } from './anchor-point'
```

Remplacer les deux lignes de calcul :

```tsx
// AVANT
const { rect, offset, status } = position
const left = rect.x + offset.x * rect.width
const top = rect.y + offset.y * rect.height

// APRÈS
const { rect, offset, status } = position
const { x: left, y: top } = anchorPoint(rect, offset)
```

(Le reste du composant — `style={{ left: `${left}px`, top: `${top}px`, ... }}` — est inchangé.)

- [ ] **Step 3: Lancer les tests pour vérifier qu'ils passent toujours**

Run : `pnpm test -- --run pin-badge`
Expected : PASS (comportement identique, `140px`/`70px`).

- [ ] **Step 4: Commit**

```bash
git add frontend/src/comments/ui/pin-badge.tsx
git commit -m "$(cat <<'EOF'
♻️ refactor(comments): PinBadge utilise anchorPoint (dédup)

Supprime le calcul inline dupliqué du point du pin ; comportement
identique (couvert par pin-badge.test).

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01MoL7PcQp9Pp49xKcmRqS7q
EOF
)"
```

---

### Task 3: Hook `useFloatingPoint` (point-based) + câblage des deux popups

**Files:**
- Rename+Modify: `frontend/src/comments/ui/use-floating-rect.ts` → `frontend/src/comments/ui/use-floating-point.ts`
- Rename+Modify: `frontend/src/comments/ui/use-floating-rect.test.ts` → `frontend/src/comments/ui/use-floating-point.test.ts`
- Modify: `frontend/src/comments/ui/thread-popup.tsx`
- Modify: `frontend/src/comments/ui/compose-popup.tsx`
- Modify: `frontend/src/comments/ui/compose-popup.test.tsx`
- Modify: `frontend/src/comments/comments-app.tsx`

**Interfaces:**
- Consumes: `anchorPoint` (Task 1) ; `PinPosition` (`follow/controller`), `AnchorDescriptor`+`ShellRect` (via `pick` state).
- Produces:
  - `floatingMiddleware(): Middleware[]` — pipeline inchangé, `offset(POPUP_OFFSET)`.
  - `useFloatingPoint(point: { x: number; y: number } | null): { ref: RefCallback<HTMLElement>; style: CSSProperties }` — positionne un flottant contre un point (VirtualElement zéro-size).
  - Constantes exportées : `PIN_RADIUS = 14`, `GAP = 8`, `POPUP_OFFSET = PIN_RADIUS + GAP` (= 22).
  - `ComposePopup` prend désormais `point: { x: number; y: number }` (au lieu de `rect: ShellRect`).

- [ ] **Step 1: Renommer les fichiers du hook et de son test**

```bash
cd frontend
git mv src/comments/ui/use-floating-rect.ts src/comments/ui/use-floating-point.ts
git mv src/comments/ui/use-floating-rect.test.ts src/comments/ui/use-floating-point.test.ts
```

- [ ] **Step 2: Écrire les tests (échouent) — pipeline + valeur d'offset**

Remplacer le contenu de `frontend/src/comments/ui/use-floating-point.test.ts` par :

```ts
import { describe, expect, it } from 'vitest'
import { floatingMiddleware, PIN_RADIUS, GAP, POPUP_OFFSET } from './use-floating-point'

describe('floatingMiddleware', () => {
  it('compose un pipeline conscient du débordement (borne le viewport)', () => {
    const names = floatingMiddleware().map((m) => m.name)
    expect(names).toEqual(['offset', 'flip', 'shift', 'size'])
  })

  it("borne aussi l'axe horizontal (crossAxis) avec un padding viewport", () => {
    const shift = floatingMiddleware().find((m) => m.name === 'shift')!
    expect(shift.options).toMatchObject({ crossAxis: true, padding: 8 })
  })
})

describe("offset d'ancrage au pin", () => {
  it('dégage le rayon du pin plus un gap', () => {
    expect(PIN_RADIUS).toBe(14)
    expect(GAP).toBe(8)
    expect(POPUP_OFFSET).toBe(PIN_RADIUS + GAP)
    expect(POPUP_OFFSET).toBe(22)
  })
})
```

- [ ] **Step 3: Lancer le test pour vérifier qu'il échoue**

Run : `pnpm test -- --run use-floating-point`
Expected : FAIL — `PIN_RADIUS`/`GAP`/`POPUP_OFFSET` non exportés (le fichier exporte encore `useFloatingRect`).

- [ ] **Step 4: Réécrire le hook en point-based**

Remplacer le contenu de `frontend/src/comments/ui/use-floating-point.ts` par :

```ts
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
```

- [ ] **Step 5: Lancer le test du hook pour vérifier qu'il passe**

Run : `pnpm test -- --run use-floating-point`
Expected : PASS (3 tests).

- [ ] **Step 6: Câbler `ThreadPopup` sur le point du pin**

Dans `frontend/src/comments/ui/thread-popup.tsx` :

Remplacer l'import :

```tsx
// AVANT
import { useFloatingRect } from './use-floating-rect'
// APRÈS
import { anchorPoint } from './anchor-point'
import { useFloatingPoint } from './use-floating-point'
```

Remplacer l'appel dans le composant :

```tsx
// AVANT
const { ref, style } = useFloatingRect(position.rect)
// APRÈS
const { ref, style } = useFloatingPoint(anchorPoint(position.rect, position.offset))
```

- [ ] **Step 7: Câbler `ComposePopup` sur le point (prop `rect` → `point`)**

Dans `frontend/src/comments/ui/compose-popup.tsx` :

Remplacer les imports concernés :

```tsx
// AVANT
import type { ShellRect } from '../picker/picker'
import { getStoredName, setStoredName } from './name-prompt'
import { useFloatingRect } from './use-floating-rect'
// APRÈS
import { getStoredName, setStoredName } from './name-prompt'
import { useFloatingPoint } from './use-floating-point'
```

Remplacer la prop et l'appel du hook :

```tsx
// AVANT
interface ComposePopupProps {
  rect: ShellRect
  submitting: boolean
  onSubmit: (v: { author_name: string; body: string }) => void
  onCancel: () => void
}

export function ComposePopup({ rect, submitting, onSubmit, onCancel }: Readonly<ComposePopupProps>) {
  const { t } = useTranslation()
  const { ref, style } = useFloatingRect(rect)
// APRÈS
interface ComposePopupProps {
  point: { x: number; y: number }
  submitting: boolean
  onSubmit: (v: { author_name: string; body: string }) => void
  onCancel: () => void
}

export function ComposePopup({ point, submitting, onSubmit, onCancel }: Readonly<ComposePopupProps>) {
  const { t } = useTranslation()
  const { ref, style } = useFloatingPoint(point)
```

- [ ] **Step 8: Passer le point à `ComposePopup` depuis `comments-app`**

Dans `frontend/src/comments/comments-app.tsx` :

Ajouter l'import (à côté des autres imports `./ui/*`) :

```tsx
import { anchorPoint } from './ui/anchor-point'
```

Remplacer la prop passée à `ComposePopup` :

```tsx
// AVANT
<ComposePopup
  rect={pick.rect}
  submitting={createPin.isPending}
  onSubmit={submitNewComment}
  onCancel={() => dispatch({ type: 'CANCEL' })}
/>
// APRÈS
<ComposePopup
  point={anchorPoint(pick.rect, pick.anchor.offset)}
  submitting={createPin.isPending}
  onSubmit={submitNewComment}
  onCancel={() => dispatch({ type: 'CANCEL' })}
/>
```

- [ ] **Step 9: Mettre à jour le test de `ComposePopup` (prop `point`)**

Dans `frontend/src/comments/ui/compose-popup.test.tsx`, remplacer le fixture `rect` et son usage :

```tsx
// AVANT
const rect = { x: 10, y: 10, width: 20, height: 20 }

function renderPopup(props: Partial<Parameters<typeof ComposePopup>[0]> = {}) {
  return render(
    <I18nextProvider i18n={i18n}>
      <ComposePopup
        rect={rect}
        submitting={false}
// APRÈS
const point = { x: 20, y: 20 }

function renderPopup(props: Partial<Parameters<typeof ComposePopup>[0]> = {}) {
  return render(
    <I18nextProvider i18n={i18n}>
      <ComposePopup
        point={point}
        submitting={false}
```

(Le reste du fichier de test est inchangé.)

- [ ] **Step 10: Vérifier typecheck + lint + tests de la couche commentaires**

Run : `pnpm typecheck && pnpm lint`
Expected : 0 erreur (aucune référence résiduelle à `useFloatingRect`/`use-floating-rect` ni à la prop `rect` de `ComposePopup`).

Run : `pnpm test -- --run comments`
Expected : PASS — `anchor-point`, `use-floating-point`, `pin-badge`, `thread-popup`, `compose-popup`, `overlay-layer`, `comments-drawer`, `comments-app` tous verts.

- [ ] **Step 11: Commit**

```bash
git add frontend/src/comments/
git commit -m "$(cat <<'EOF'
✨ feat(comments): popups ancrés au pin (Figma-like)

useFloatingRect → useFloatingPoint : ancrage sur un VirtualElement de
taille nulle au point du pin, au lieu du bounding box de l'élément.
ThreadPopup + ComposePopup s'ouvrent collés au pin (offset = rayon du
pin + gap → pin visible à côté). Pipeline flip/shift/size conservé.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01MoL7PcQp9Pp49xKcmRqS7q
EOF
)"
```

---

### Task 4: Gate finale + vérification navigateur + mémoire

**Files:**
- Modify: `docs/INDEX.md`, `docs/HANDOFF.md`
- Modify (si pertinent): `docs/QUIRKS.md`, `docs/CONVENTIONS.md`

**Interfaces:**
- Consumes: le code livré aux Tasks 1-3.
- Produces: rien de code — clôture + mémoire.

- [ ] **Step 1: Gate frontend complète**

Run (depuis `frontend/`) : `pnpm lint && pnpm typecheck && pnpm test`
Expected : 0 erreur lint, 0 erreur typecheck, Vitest tout vert (les nouveaux tests `anchor-point` + `use-floating-point` inclus).

- [ ] **Step 2: e2e Playwright (suite commentaires inchangée)**

Run (depuis `frontend/`) : `pnpm exec playwright test`
Expected : vert (8 passed / 2 skipped attendus, comme les sessions précédentes ; le parcours pin ancré `comments*.spec.ts` reste valide).

> Si des failures rate-limit `/api/login` apparaissent en dev local, c'est la contrainte pré-existante (cf. QUIRKS « rate-limit /api/login ») — vérifier que `LATCH_LOGIN_RL_BURST` est bien posé par le webServer, ne pas réintroduire de retry.

- [ ] **Step 3: Vérification au navigateur (non automatisable en jsdom)**

Lancer la stack dev (backend `:5150` + `pnpm dev` `:5173`, cf. `docs/ENVIRONMENT.md`), ouvrir un proto avec un **gros conteneur**, et vérifier :
1. Cliquer pour créer un commentaire sur un grand élément → le `ComposePopup` s'ouvre **collé au point de clic** (pas au bord du conteneur).
2. Cliquer un pin existant → le `ThreadPopup` s'ouvre **collé au pin**, avec le pin **pleinement visible à côté**.
3. Pin près d'un bord de viewport → le popup se **rabat** (flip/shift) sans sortir de l'écran, pin toujours dégagé.
4. Côté **admin** (page Review) : même comportement (composants partagés).

- [ ] **Step 4: Mettre à jour `docs/INDEX.md`**

Ajouter sous la section Phase 10 (commentaires), une ligne :

```markdown
- [x] **Popups ancrés au pin (Figma-like)** — `ThreadPopup`+`ComposePopup` s'ouvrent collés au point du pin via `useFloatingPoint` (VirtualElement zéro-size) au lieu du bounding box ; helper pur `anchorPoint` (dédup PinBadge) ; offset = rayon pin + gap (pin visible) — frontend-only — spec `docs/superpowers/specs/2026-07-01-comments-popup-anchor-design.md`, plan `docs/superpowers/plans/2026-07-01-comments-popup-anchor.md` — 2026-07-01
```

- [ ] **Step 5: Ajouter une entrée `docs/HANDOFF.md`**

En haut (sous le H1), ajouter une entrée datée `## 2026-07-01 — Popups commentaires ancrés au pin` avec : `Dernière chose faite` (résumé + gate), `Trucs en suspens` (branche `feat/prototype-comments` toujours non mergée, décision humaine), `Prochaine chose à creuser`, `Notes pour future Claude` (pattern `VirtualElement` zéro-size ; `POPUP_OFFSET = PIN_RADIUS + GAP` load-bearing pour « pin visible »).

- [ ] **Step 6: Ajouter le piège dans `docs/QUIRKS.md`**

Ajouter une entrée :

```markdown
## Popup de commentaire = ancré au POINT du pin, pas au rect de l'élément (2026-07-01)

`ThreadPopup`/`ComposePopup` se positionnent via `useFloatingPoint(point)` contre un
`VirtualElement` de **taille nulle** au point du pin (`anchorPoint(rect, offset)`), PAS
contre le bounding box de l'élément. Ancrer au rect complet (ancien `useFloatingRect`)
faisait ouvrir le popup au bord d'un gros conteneur, loin du pin. L'`offset(POPUP_OFFSET)`
(= `PIN_RADIUS 14 + GAP 8`) est **load-bearing** : il dégage le rayon du pin pour qu'il
reste visible à côté du fil (choix Figma). Baisser l'offset sous 14 recouvre le pin.
```

- [ ] **Step 7: Commit mémoire**

```bash
git add docs/
git commit -m "$(cat <<'EOF'
📝 docs(comments): popups ancrés au pin livré (INDEX/HANDOFF/QUIRKS)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01MoL7PcQp9Pp49xKcmRqS7q
EOF
)"
```

---

## Self-Review

**Spec coverage :**
- Helper `anchorPoint` partagé (dédup) → Task 1 + Task 2. ✓
- `useFloatingRect` → point-based (VirtualElement zéro-size) → Task 3 Steps 1-5. ✓
- Offset dégageant le pin (`PIN_RADIUS + GAP`, pin visible) → Task 3 Step 4 + test Step 2. ✓
- Câblage `ThreadPopup` → Task 3 Step 6. ✓
- Câblage `ComposePopup` (prop `point`) + `comments-app` → Task 3 Steps 7-9. ✓
- Placement `right-start` + flip/shift/size conservés → Task 3 Step 4 (inchangé). ✓
- Tests (`anchorPoint`, hook, régression) → Tasks 1/3 + gate Task 4. ✓
- Vérif navigateur + jsdom caveat → Task 4 Step 3. ✓
- Mémoire (INDEX/HANDOFF/QUIRKS) → Task 4 Steps 4-7. ✓
- Hors périmètre (backend, OverlayLayer, descripteur) : aucune tâche n'y touche. ✓

**Placeholder scan :** aucun TODO/TBD ; tout le code est explicite. ✓

**Type consistency :** `anchorPoint(rect, offset): {x,y}` cohérent (Tasks 1,2,3) ; `useFloatingPoint(point: {x,y}|null)` cohérent ; `ComposePopup` prop `point: {x,y}` cohérente (compose-popup.tsx + comments-app + compose-popup.test). Nom du hook `useFloatingPoint` et fichier `use-floating-point` cohérents partout. ✓
