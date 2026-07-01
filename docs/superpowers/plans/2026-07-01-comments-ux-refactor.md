# Refactor UX commentaires — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Corriger et moderniser l'UX de la couche commentaires (drawer de liste, label/couleur des pins, positionnement des popups, ciblage DOM, décalage des pins admin), 100 % frontend.

**Architecture :** Tous les changements vivent dans `frontend/src/comments/` (composants partagés client ⇄ admin) sauf l'i18n et les e2e. Aucun endpoint ni schéma modifié. Le point admin (décalage) se règle en passant l'overlay des pins en `position: fixed` (espace viewport), cohérent avec `toShellRect` et les popups déjà en `fixed`.

**Tech Stack :** React 19, TypeScript, Vite, `@floating-ui/dom` v1.7.6, `@medv/finder`, TanStack Query, react-i18next (FR/EN), Vitest + Testing-Library, Playwright.

## Global Constraints

- **Frontend « terminé »** = `pnpm lint` **et** `pnpm typecheck` **et** `pnpm test` (Vitest) verts, plus Playwright. `eslint-plugin-react-hooks` v7 est strict et passe à travers Vitest.
- **On ne touche pas `vite.config.ts`** → suite `pnpm test:vite` non impactée ; la suite Playwright par défaut (`:5150`) suffit + on la lance.
- **Pluriels i18next = CLDR `_one`/`_other`** (jamais `_plural`). Fournir la clé de base + `_one` + `_other`, comme `comment.bar.count` existant.
- **Couleur fluo commentaires = `#18A0FB`, fixe** (jamais via `--primary` ni variable de thème).
- **Confidentialité** : aucun nom de client réel. Placeholders uniquement (`ACME`, `Léa`, `Max`, `Mon Projet`).
- **Sonar new-coverage ≥ 80 %** sur le code neuf (gate CI bloquante).
- **Chaque commit** : préfixer `rtk`, et terminer le message par le trailer standard :
  ```
  Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01MoL7PcQp9Pp49xKcmRqS7q
  ```

---

## File Structure

**Créés :**
- `frontend/src/comments/ui/colors.ts` — constante `COMMENT_FLUO`.
- `frontend/src/comments/ui/pin-label.ts` — helper `firstLetter`.
- `frontend/src/comments/ui/pin-label.test.ts`.
- `frontend/src/comments/ui/use-floating-rect.test.ts`.
- `frontend/src/comments/ui/time-ago.ts` — helper `timeAgo`.
- `frontend/src/comments/ui/time-ago.test.ts`.
- `frontend/src/comments/ui/comments-drawer.tsx` — panneau liste + `sortPins`.
- `frontend/src/comments/ui/comments-drawer.test.tsx`.

**Modifiés :**
- `frontend/src/comments/ui/pin-badge.tsx` (+ `.test.tsx`) — prop `count`→`label`, couleur fluo.
- `frontend/src/comments/ui/overlay-layer.tsx` (+ `.test.tsx`) — `labelOf`, ciblage inset glow, conteneur `fixed`.
- `frontend/src/comments/ui/use-floating-rect.ts` — pipeline middleware anti-débordement.
- `frontend/src/comments/comments-app.tsx` (+ `.test.tsx`) — label, drawer, focus-depuis-liste.
- `frontend/src/i18n/locales/comments/en.json` + `fr.json` — clés `comment.drawer.*`.
- `frontend/e2e/comments.spec.ts` — parcours drawer visiteur.
- `frontend/e2e/comments-admin.spec.ts` — assertion d'alignement des pins (régression topbar).
- `docs/INDEX.md`, `docs/HANDOFF.md`, `docs/QUIRKS.md`, `docs/CONVENTIONS.md`.

> Toutes les commandes frontend se lancent **depuis `frontend/`**.

---

### Task 1 : Couleur fluo + label du pin (1ʳᵉ lettre de l'auteur)

**Files:**
- Create: `frontend/src/comments/ui/colors.ts`
- Create: `frontend/src/comments/ui/pin-label.ts`
- Create: `frontend/src/comments/ui/pin-label.test.ts`
- Modify: `frontend/src/comments/ui/pin-badge.tsx`
- Modify: `frontend/src/comments/ui/pin-badge.test.tsx`
- Modify: `frontend/src/comments/ui/overlay-layer.tsx`
- Modify: `frontend/src/comments/ui/overlay-layer.test.tsx`
- Modify: `frontend/src/comments/comments-app.tsx`

**Interfaces:**
- Produces:
  - `COMMENT_FLUO: string` (`'#18A0FB'`) depuis `./colors`.
  - `firstLetter(name: string): string` depuis `./pin-label`.
  - `PinBadge` prop `label: string` (remplace `count: number`).
  - `OverlayLayer` prop `labelOf: (pinId: number) => string` (remplace `countOf?`).

- [ ] **Step 1 : Test du helper `firstLetter`**

Create `frontend/src/comments/ui/pin-label.test.ts` :
```ts
import { describe, expect, it } from 'vitest'
import { firstLetter } from './pin-label'

describe('firstLetter', () => {
  it('renvoie la première lettre en majuscule', () => {
    expect(firstLetter('alice')).toBe('A')
    expect(firstLetter('  léa ')).toBe('L')
  })
  it('retombe sur • pour un nom vide', () => {
    expect(firstLetter('')).toBe('•')
    expect(firstLetter('   ')).toBe('•')
  })
})
```

- [ ] **Step 2 : Lancer le test → échec (module absent)**

Run: `pnpm test -- pin-label`
Expected: FAIL — `Cannot find module './pin-label'`.

- [ ] **Step 3 : Créer les deux modules**

Create `frontend/src/comments/ui/colors.ts` :
```ts
/** Bleu fluo fixe des commentaires — indépendant du thème (rendu sur un proto arbitraire). */
export const COMMENT_FLUO = '#18A0FB'
```

Create `frontend/src/comments/ui/pin-label.ts` :
```ts
/** 1ʳᵉ lettre (majuscule) d'un nom d'auteur ; `•` si vide. */
export function firstLetter(name: string): string {
  const trimmed = name.trim()
  return trimmed ? trimmed[0].toUpperCase() : '•'
}
```

- [ ] **Step 4 : Lancer le test → succès**

Run: `pnpm test -- pin-label`
Expected: PASS.

- [ ] **Step 5 : Mettre à jour le test de `PinBadge`**

Dans `frontend/src/comments/ui/pin-badge.test.tsx`, remplacer les 3 `it(...)` par :
```tsx
import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { PinBadge } from './pin-badge'
import type { PinPosition } from '../follow/controller'

const pos: PinPosition = {
  id: 1,
  status: 'anchored',
  rect: { x: 100, y: 50, width: 80, height: 40 },
  offset: { x: 0.5, y: 0.5 },
}

describe('PinBadge', () => {
  it('affiche le label et se positionne', () => {
    render(<PinBadge position={pos} label="A" active={false} onClick={() => {}} />)
    const btn = screen.getByRole('button')
    expect(btn).toHaveTextContent('A')
    expect(btn.style.left).toBe('140px') // 100 + 0.5*80
    expect(btn.style.top).toBe('70px') // 50 + 0.5*40
  })

  it('un pin ancré n’utilise pas la couleur d’avertissement (ambre)', () => {
    render(<PinBadge position={pos} label="A" active={false} onClick={() => {}} />)
    expect(screen.getByRole('button').className).not.toContain('amber')
  })

  it('un pin orphelin passe en ambre', () => {
    render(<PinBadge position={{ ...pos, status: 'orphaned' }} label="J" active={false} onClick={() => {}} />)
    expect(screen.getByRole('button').className).toContain('amber')
  })

  it('appelle onClick', async () => {
    const onClick = vi.fn()
    render(<PinBadge position={pos} label="A" active={false} onClick={onClick} />)
    await userEvent.click(screen.getByRole('button'))
    expect(onClick).toHaveBeenCalledOnce()
  })
})
```

- [ ] **Step 6 : Lancer → échec (PinBadge attend encore `count`)**

Run: `pnpm test -- pin-badge`
Expected: FAIL (type/prop `label` inconnu, `count` requis).

- [ ] **Step 7 : Réécrire `PinBadge`**

Remplacer intégralement `frontend/src/comments/ui/pin-badge.tsx` :
```tsx
import { cn } from '@/lib/utils'
import type { PinPosition } from '../follow/controller'
import { COMMENT_FLUO } from './colors'

interface PinBadgeProps {
  position: PinPosition
  label: string
  active: boolean
  onClick: () => void
}

export function PinBadge({ position, label, active, onClick }: Readonly<PinBadgeProps>) {
  const { rect, offset, status } = position
  const left = rect.x + offset.x * rect.width
  const top = rect.y + offset.y * rect.height
  const anchored = status === 'anchored'
  return (
    <button
      type="button"
      data-testid="pin-badge"
      data-status={status}
      onClick={onClick}
      style={{
        left: `${left}px`,
        top: `${top}px`,
        pointerEvents: 'auto',
        background: anchored ? COMMENT_FLUO : undefined,
      }}
      className={cn(
        'absolute flex size-7 -translate-x-1/2 -translate-y-1/2 items-center justify-center rounded-full border-2 border-white text-xs font-semibold text-white shadow-md',
        !anchored && 'bg-amber-500',
        active && 'ring-2 ring-black/25',
      )}
    >
      {label}
    </button>
  )
}
```

- [ ] **Step 8 : Lancer → succès**

Run: `pnpm test -- pin-badge`
Expected: PASS.

- [ ] **Step 9 : Adapter `OverlayLayer` (prop `labelOf`)**

Dans `frontend/src/comments/ui/overlay-layer.tsx` :
- Remplacer dans l'interface `countOf?: (pinId: number) => number` par `labelOf: (pinId: number) => string`.
- Dans la signature déstructurée, remplacer `countOf` par `labelOf`.
- Dans le `.map`, remplacer `count={countOf ? countOf(p.id) : 1}` par `label={labelOf(p.id)}`.

- [ ] **Step 10 : Adapter les rendus dans `overlay-layer.test.tsx`**

Ajouter `labelOf={() => 'A'}` à **chacun** des 3 `<OverlayLayer .../>` du fichier (props requises).

- [ ] **Step 11 : Câbler le label dans `comments-app.tsx`**

- Ajouter l'import : `import { firstLetter } from './ui/pin-label'`.
- Remplacer la prop de l'`OverlayLayer` :
  `countOf={(id) => pins.find((p) => p.id === id)?.messages.length ?? 0}`
  par
  `labelOf={(id) => firstLetter(pins.find((p) => p.id === id)?.messages[0]?.author_name ?? '')}`.

- [ ] **Step 12 : Suite complète + qualité**

Run: `pnpm test -- pin-badge pin-label overlay-layer comments-app && pnpm lint && pnpm typecheck`
Expected: PASS partout.

- [ ] **Step 13 : Commit**

```bash
rtk git add frontend/src/comments/ui/colors.ts frontend/src/comments/ui/pin-label.ts \
  frontend/src/comments/ui/pin-label.test.ts frontend/src/comments/ui/pin-badge.tsx \
  frontend/src/comments/ui/pin-badge.test.tsx frontend/src/comments/ui/overlay-layer.tsx \
  frontend/src/comments/ui/overlay-layer.test.tsx frontend/src/comments/comments-app.tsx
rtk git commit -m "✨ feat(comments): pin fluo + label 1re lettre auteur"
```

---

### Task 2 : Ciblage DOM — inset edge glow capé

**Files:**
- Modify: `frontend/src/comments/ui/overlay-layer.tsx`
- Modify: `frontend/src/comments/ui/overlay-layer.test.tsx`

**Interfaces:**
- Consumes: `COMMENT_FLUO` (Task 1).
- Produces: `glowShadow(width: number, height: number): string` exporté depuis `overlay-layer.tsx`.

- [ ] **Step 1 : Tests du glow (unité + rendu)**

Ajouter dans `frontend/src/comments/ui/overlay-layer.test.tsx` :
- l'import : `import { OverlayLayer, glowShadow } from './overlay-layer'` (remplacer l'import existant `{ OverlayLayer }`).
- ce bloc de tests :
```tsx
describe('ciblage DOM (glow)', () => {
  it('cape la profondeur du glow (non proportionnelle)', () => {
    expect(glowShadow(20, 20)).toContain('6px') // 0.3*20 = 6
    expect(glowShadow(1000, 1000)).toContain('30px') // capé à 30
    expect(glowShadow(20, 20)).toContain('inset')
  })

  it('rend un highlight fluo au survol en pick mode', () => {
    const { container } = render(
      <OverlayLayer
        picker={fakePicker()}
        positions={[]}
        pickMode
        onPick={vi.fn()}
        onPinClick={vi.fn()}
        activePinId={null}
        labelOf={() => 'A'}
      />,
    )
    const surface = container.querySelector('[data-testid="pick-surface"]')!
    fireEvent.mouseMove(surface, { clientX: 5, clientY: 6 })
    const hl = container.querySelector('[data-testid="pick-highlight"]') as HTMLElement
    expect(hl).not.toBeNull()
    expect(hl.style.boxShadow).toContain('inset')
    expect(hl.style.borderColor.toLowerCase()).toContain('18a0fb')
  })
})
```
> Note : `fakePicker().toShellRect` renvoie `{x:1,y:2,width:3,height:4}` → le highlight se rend après `mouseMove`.
> `borderColor` peut être renvoyé en hex par jsdom ; si le test échoue sur la casse, comparer `hl.style.border` qui contient `#18A0FB`.

- [ ] **Step 2 : Lancer → échec**

Run: `pnpm test -- overlay-layer`
Expected: FAIL — `glowShadow` non exporté / pas de `pick-highlight`.

- [ ] **Step 3 : Implémenter le glow**

Dans `frontend/src/comments/ui/overlay-layer.tsx` :
- Ajouter l'import : `import { COMMENT_FLUO } from './colors'`.
- Ajouter, avant le composant :
```tsx
const GLOW_CAP = 30

/** Halo intérieur capé (non proportionnel) : petit composant → petit glow, grand → glow borné. */
export function glowShadow(width: number, height: number): string {
  const depth = Math.min(GLOW_CAP, Math.round(0.3 * Math.min(width, height)))
  const spread = Math.round(depth / 6)
  return `inset 0 0 ${depth}px ${spread}px rgba(24, 160, 251, 0.5)`
}
```
- Remplacer le bloc `{pickMode && hover && (...)}` par :
```tsx
{pickMode && hover && (
  <div
    data-testid="pick-highlight"
    className="pointer-events-none absolute rounded-sm"
    style={{
      left: `${hover.x}px`,
      top: `${hover.y}px`,
      width: `${hover.width}px`,
      height: `${hover.height}px`,
      border: `2px solid ${COMMENT_FLUO}`,
      boxShadow: glowShadow(hover.width, hover.height),
    }}
  />
)}
```

- [ ] **Step 4 : Lancer → succès**

Run: `pnpm test -- overlay-layer`
Expected: PASS.

- [ ] **Step 5 : Qualité + commit**

```bash
pnpm lint && pnpm typecheck
rtk git add frontend/src/comments/ui/overlay-layer.tsx frontend/src/comments/ui/overlay-layer.test.tsx
rtk git commit -m "✨ feat(comments): ciblage DOM en bleu fluo + glow intérieur capé"
```

---

### Task 3 : Moteur de positionnement des popups (anti-débordement)

**Files:**
- Modify: `frontend/src/comments/ui/use-floating-rect.ts`
- Create: `frontend/src/comments/ui/use-floating-rect.test.ts`

**Interfaces:**
- Produces: `floatingMiddleware(): Middleware[]` exporté depuis `use-floating-rect.ts`.

- [ ] **Step 1 : Test de composition du pipeline**

Create `frontend/src/comments/ui/use-floating-rect.test.ts` :
```ts
import { describe, expect, it } from 'vitest'
import { floatingMiddleware } from './use-floating-rect'

describe('floatingMiddleware', () => {
  it('compose un pipeline conscient du débordement (borne le viewport)', () => {
    const names = floatingMiddleware().map((m) => m.name)
    expect(names).toEqual(['offset', 'flip', 'shift', 'size'])
  })
})
```

- [ ] **Step 2 : Lancer → échec**

Run: `pnpm test -- use-floating-rect`
Expected: FAIL — `floatingMiddleware` non exporté.

- [ ] **Step 3 : Réécrire le hook**

Remplacer intégralement `frontend/src/comments/ui/use-floating-rect.ts` :
```ts
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
```

- [ ] **Step 4 : Lancer → succès (+ non-régression popups)**

Run: `pnpm test -- use-floating-rect thread-popup compose-popup`
Expected: PASS.

- [ ] **Step 5 : Qualité + commit**

```bash
pnpm lint && pnpm typecheck
rtk git add frontend/src/comments/ui/use-floating-rect.ts frontend/src/comments/ui/use-floating-rect.test.ts
rtk git commit -m "🐛 fix(comments): popups de thread bornés au viewport (shift crossAxis + size)"
```

---

### Task 4 : Décalage des pins admin — overlay `fixed`

**Files:**
- Modify: `frontend/src/comments/ui/overlay-layer.tsx`
- Modify: `frontend/src/comments/ui/overlay-layer.test.tsx`

**Interfaces:** aucune nouvelle. Change la classe du conteneur racine d'`OverlayLayer`.

- [ ] **Step 1 : Test d'ancrage viewport**

Ajouter dans `frontend/src/comments/ui/overlay-layer.test.tsx` :
```tsx
it('ancre l’overlay au viewport (fixed) pour ignorer l’offset du conteneur (topbar admin)', () => {
  const { container } = render(
    <OverlayLayer
      picker={fakePicker()}
      positions={[]}
      pickMode={false}
      onPick={vi.fn()}
      onPinClick={vi.fn()}
      activePinId={null}
      labelOf={() => 'A'}
    />,
  )
  const root = container.firstElementChild as HTMLElement
  expect(root.className).toContain('fixed')
  expect(root.className).not.toContain('absolute')
})
```

- [ ] **Step 2 : Lancer → échec (root est `absolute`)**

Run: `pnpm test -- overlay-layer`
Expected: FAIL — root contient `absolute`.

- [ ] **Step 3 : Passer le conteneur en `fixed`**

Dans `frontend/src/comments/ui/overlay-layer.tsx`, le `<div>` racine retourné : remplacer `className="absolute inset-0 z-50"` par `className="fixed inset-0 z-50"`.

- [ ] **Step 4 : Lancer → succès**

Run: `pnpm test -- overlay-layer`
Expected: PASS.

- [ ] **Step 5 : Qualité + commit**

```bash
pnpm lint && pnpm typecheck
rtk git add frontend/src/comments/ui/overlay-layer.tsx frontend/src/comments/ui/overlay-layer.test.tsx
rtk git commit -m "🐛 fix(comments): pins admin alignés — overlay en espace viewport (fixed)"
```

---

### Task 5 : Composant `CommentsDrawer` (+ `timeAgo` + i18n)

**Files:**
- Create: `frontend/src/comments/ui/time-ago.ts`
- Create: `frontend/src/comments/ui/time-ago.test.ts`
- Create: `frontend/src/comments/ui/comments-drawer.tsx`
- Create: `frontend/src/comments/ui/comments-drawer.test.tsx`
- Modify: `frontend/src/i18n/locales/comments/en.json`
- Modify: `frontend/src/i18n/locales/comments/fr.json`

**Interfaces:**
- Consumes: `COMMENT_FLUO` (Task 1), `firstLetter` (Task 1), `AnchorStatus` (`../anchor/resolve`), `CommentPin` (`../data/adapter`).
- Produces :
  - `timeAgo(iso: string, now: number, locale: string): string`.
  - `sortPins(pins: CommentPin[], statusOf: (id: number) => AnchorStatus | undefined): CommentPin[]`.
  - `CommentsDrawer` avec props `{ open: boolean; pins: CommentPin[]; statusOf: (pinId: number) => AnchorStatus | undefined; onClose: () => void; onSelect: (pinId: number) => void }`.

- [ ] **Step 1 : Test de `timeAgo`**

Create `frontend/src/comments/ui/time-ago.test.ts` :
```ts
import { describe, expect, it } from 'vitest'
import { timeAgo } from './time-ago'

const now = new Date('2026-07-01T12:00:00Z').getTime()

describe('timeAgo', () => {
  it('formate en heures', () => {
    expect(timeAgo('2026-07-01T10:00:00Z', now, 'en')).toContain('2')
  })
  it('formate en jours', () => {
    expect(timeAgo('2026-06-28T12:00:00Z', now, 'en')).toContain('3')
  })
  it('borne à 0 pour un futur proche', () => {
    expect(timeAgo('2026-07-01T12:00:05Z', now, 'en')).toMatch(/now|0/)
  })
})
```

- [ ] **Step 2 : Lancer → échec**

Run: `pnpm test -- time-ago`
Expected: FAIL — module absent.

- [ ] **Step 3 : Implémenter `timeAgo`**

Create `frontend/src/comments/ui/time-ago.ts` :
```ts
/** Âge relatif compact et localisé (ex. « 2h », « 3d »), borné à la plus grande unité. */
export function timeAgo(iso: string, now: number, locale: string): string {
  const diffSec = Math.max(0, Math.round((now - new Date(iso).getTime()) / 1000))
  const rtf = new Intl.RelativeTimeFormat(locale, { numeric: 'auto', style: 'narrow' })
  if (diffSec < 60) return rtf.format(-diffSec, 'second')
  const diffMin = Math.round(diffSec / 60)
  if (diffMin < 60) return rtf.format(-diffMin, 'minute')
  const diffHour = Math.round(diffMin / 60)
  if (diffHour < 24) return rtf.format(-diffHour, 'hour')
  const diffDay = Math.round(diffHour / 24)
  if (diffDay < 7) return rtf.format(-diffDay, 'day')
  return rtf.format(-Math.round(diffDay / 7), 'week')
}
```

- [ ] **Step 4 : Lancer → succès**

Run: `pnpm test -- time-ago`
Expected: PASS.

- [ ] **Step 5 : Ajouter les clés i18n**

Dans `frontend/src/i18n/locales/comments/en.json`, ajouter (avant l'accolade fermante, virgule sur la ligne précédente) :
```json
  "comment.drawer.title_one": "{{count}} comment",
  "comment.drawer.title_other": "{{count}} comments",
  "comment.drawer.close": "Close comments list",
  "comment.drawer.empty": "No comments yet",
  "comment.drawer.orphaned": "orphaned",
  "comment.drawer.replies_one": "{{count}} reply",
  "comment.drawer.replies_other": "{{count}} replies"
```
Dans `frontend/src/i18n/locales/comments/fr.json`, ajouter de même :
```json
  "comment.drawer.title_one": "{{count}} commentaire",
  "comment.drawer.title_other": "{{count}} commentaires",
  "comment.drawer.close": "Fermer la liste des commentaires",
  "comment.drawer.empty": "Aucun commentaire",
  "comment.drawer.orphaned": "orphelin",
  "comment.drawer.replies_one": "{{count}} réponse",
  "comment.drawer.replies_other": "{{count}} réponses"
```

- [ ] **Step 6 : Test du drawer (rendu, tri, clic, vide)**

Create `frontend/src/comments/ui/comments-drawer.test.tsx` :
```tsx
import { describe, expect, it, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { I18nextProvider } from 'react-i18next'
import i18n from '@/shell/i18n'
import { CommentsDrawer, sortPins } from './comments-drawer'
import type { CommentPin } from '../data/adapter'
import type { AnchorStatus } from '../anchor/resolve'

function pin(id: number, author: string, created: string): CommentPin {
  return {
    id,
    anchor: '{}',
    created_at: created,
    messages: [
      { id: id * 10, author_name: author, body: `Body ${id}`, created_at: created, updated_at: created, editable: false },
    ],
  }
}

const pins = [pin(1, 'Alice', '2026-07-01T09:00:00Z'), pin(2, 'Max', '2026-07-01T11:00:00Z'), pin(3, 'Jo', '2026-07-01T08:00:00Z')]
const statusOf = (id: number): AnchorStatus | undefined => (id === 3 ? 'orphaned' : 'anchored')

function renderDrawer(over: Partial<Parameters<typeof CommentsDrawer>[0]> = {}) {
  return render(
    <I18nextProvider i18n={i18n}>
      <CommentsDrawer open pins={pins} statusOf={statusOf} onClose={vi.fn()} onSelect={vi.fn()} {...over} />
    </I18nextProvider>,
  )
}

beforeEach(() => i18n.changeLanguage('en'))

describe('sortPins', () => {
  it('met les orphelins en bas, sains par récence desc', () => {
    const ids = sortPins(pins, statusOf).map((p) => p.id)
    expect(ids).toEqual([2, 1, 3]) // 2 (11h) > 1 (9h) sains ; 3 orphelin en dernier
  })
})

describe('CommentsDrawer', () => {
  it('rend une ligne par pin', () => {
    renderDrawer()
    expect(screen.getAllByTestId('drawer-row')).toHaveLength(3)
    expect(screen.getByText('orphaned')).toBeInTheDocument()
  })

  it('appelle onSelect avec l’id au clic', async () => {
    const onSelect = vi.fn()
    renderDrawer({ onSelect })
    await userEvent.click(screen.getAllByTestId('drawer-row')[0])
    expect(onSelect).toHaveBeenCalledWith(2) // première ligne = plus récente
  })

  it('affiche l’état vide', () => {
    renderDrawer({ pins: [] })
    expect(screen.getByText('No comments yet')).toBeInTheDocument()
    expect(screen.queryByTestId('drawer-row')).toBeNull()
  })

  it('ne rend rien si fermé', () => {
    const { container } = renderDrawer({ open: false })
    expect(container.querySelector('[data-testid="comments-drawer"]')).toBeNull()
  })
})
```

- [ ] **Step 7 : Lancer → échec**

Run: `pnpm test -- comments-drawer`
Expected: FAIL — module absent.

- [ ] **Step 8 : Implémenter `CommentsDrawer`**

Create `frontend/src/comments/ui/comments-drawer.tsx` :
```tsx
import { useTranslation } from 'react-i18next'
import { X } from 'lucide-react'
import { Button } from '@/components/ui/button'
import type { AnchorStatus } from '../anchor/resolve'
import type { CommentPin } from '../data/adapter'
import { COMMENT_FLUO } from './colors'
import { firstLetter } from './pin-label'
import { timeAgo } from './time-ago'

interface CommentsDrawerProps {
  open: boolean
  pins: CommentPin[]
  statusOf: (pinId: number) => AnchorStatus | undefined
  onClose: () => void
  onSelect: (pinId: number) => void
}

/** Tri : threads sains d'abord (récence desc), orphelins en bas. */
export function sortPins(
  pins: CommentPin[],
  statusOf: (id: number) => AnchorStatus | undefined,
): CommentPin[] {
  const isOrphan = (p: CommentPin) => (statusOf(p.id) === 'orphaned' ? 1 : 0)
  return [...pins].sort((a, b) => {
    const delta = isOrphan(a) - isOrphan(b)
    return delta !== 0 ? delta : b.created_at.localeCompare(a.created_at)
  })
}

export function CommentsDrawer({ open, pins, statusOf, onClose, onSelect }: Readonly<CommentsDrawerProps>) {
  const { t, i18n } = useTranslation()
  if (!open) return null
  const now = Date.now()
  const ordered = sortPins(pins, statusOf)
  return (
    <aside
      data-testid="comments-drawer"
      className="bg-background fixed inset-y-0 right-0 z-[60] flex w-80 flex-col border-l shadow-xl"
    >
      <header className="flex items-center justify-between border-b px-4 py-3">
        <h2 className="text-sm font-semibold">{t('comment.drawer.title', { count: pins.length })}</h2>
        <Button variant="ghost" size="sm" aria-label={t('comment.drawer.close')} onClick={onClose}>
          <X className="size-4" />
        </Button>
      </header>
      {ordered.length === 0 ? (
        <p className="text-muted-foreground p-4 text-sm">{t('comment.drawer.empty')}</p>
      ) : (
        <ul className="flex-1 overflow-y-auto">
          {ordered.map((pin) => {
            const author = pin.messages[0]?.author_name ?? ''
            const orphaned = statusOf(pin.id) === 'orphaned'
            const replies = Math.max(0, pin.messages.length - 1)
            return (
              <li key={pin.id}>
                <button
                  type="button"
                  data-testid="drawer-row"
                  onClick={() => onSelect(pin.id)}
                  className="hover:bg-muted flex w-full gap-3 border-b px-4 py-3 text-left"
                >
                  <span
                    className="flex size-7 shrink-0 items-center justify-center rounded-full border-2 border-white text-xs font-semibold text-white shadow-sm"
                    style={{ background: orphaned ? '#f59e0b' : COMMENT_FLUO }}
                  >
                    {firstLetter(author)}
                  </span>
                  <span className="min-w-0 flex-1">
                    <span className="flex items-center gap-2">
                      <span className="truncate text-xs font-semibold">{author}</span>
                      <span className="text-muted-foreground text-[10px]">
                        {timeAgo(pin.messages[0]?.created_at ?? pin.created_at, now, i18n.language)}
                      </span>
                      {orphaned && (
                        <span className="rounded-full bg-amber-100 px-1.5 py-0.5 text-[9px] text-amber-700">
                          {t('comment.drawer.orphaned')}
                        </span>
                      )}
                    </span>
                    <span className="text-muted-foreground block truncate text-xs">{pin.messages[0]?.body ?? ''}</span>
                    <span className="text-muted-foreground mt-0.5 block text-[10px]">
                      {t('comment.drawer.replies', { count: replies })}
                    </span>
                  </span>
                </button>
              </li>
            )
          })}
        </ul>
      )}
    </aside>
  )
}
```

- [ ] **Step 9 : Lancer → succès**

Run: `pnpm test -- comments-drawer time-ago`
Expected: PASS.

- [ ] **Step 10 : Qualité + commit**

```bash
pnpm lint && pnpm typecheck
rtk git add frontend/src/comments/ui/time-ago.ts frontend/src/comments/ui/time-ago.test.ts \
  frontend/src/comments/ui/comments-drawer.tsx frontend/src/comments/ui/comments-drawer.test.tsx \
  frontend/src/i18n/locales/comments/en.json frontend/src/i18n/locales/comments/fr.json
rtk git commit -m "✨ feat(comments): composant CommentsDrawer (liste des threads)"
```

---

### Task 6 : Câbler le drawer dans `CommentsApp` (ouverture + focus depuis la liste)

**Files:**
- Modify: `frontend/src/comments/comments-app.tsx`
- Modify: `frontend/src/comments/comments-app.test.tsx`

**Interfaces:**
- Consumes: `CommentsDrawer` (Task 5), `parseAnchor` (déjà importé), `picker.resolve` (déjà dispo).

- [ ] **Step 1 : Test d'ouverture du drawer**

Dans `frontend/src/comments/comments-app.test.tsx` :
- Ajouter les imports : `import userEvent from '@testing-library/user-event'` et `import { it as vitestIt } from 'vitest'` n'est pas nécessaire (garder `it`).
- Ajouter ce test à la fin :
```tsx
it('ouvre le drawer via le bouton « My comments »', async () => {
  render(
    <I18nextProvider i18n={i18n}>
      <CommentsApp cacheKey="demo" frame={fakeFrame()} adapter={fakeAdapter} />
    </I18nextProvider>,
  )
  const listBtn = await screen.findByRole('button', { name: 'My comments' })
  await userEvent.click(listBtn)
  expect(await screen.findByTestId('comments-drawer')).toBeInTheDocument()
})
```

- [ ] **Step 2 : Lancer → échec (pas de drawer monté)**

Run: `pnpm test -- comments-app`
Expected: FAIL — `comments-drawer` introuvable après clic.

- [ ] **Step 3 : Câbler le drawer**

Dans `frontend/src/comments/comments-app.tsx` :
- Ajouter l'import : `import { CommentsDrawer } from './ui/comments-drawer'`.
- Ajouter l'état, à côté de `activePinId` : `const [drawerOpen, setDrawerOpen] = useState(false)`.
- Ajouter le handler de focus (après `submitNewComment`) :
```tsx
function focusPinFromList(pinId: number) {
  const pin = pins.find((p) => p.id === pinId)
  const anchor = pin ? parseAnchor(pin.anchor) : null
  const el = anchor ? picker.resolve(anchor).element : null
  el?.scrollIntoView({ block: 'center', behavior: 'smooth' })
  setActivePinId(pinId)
  setDrawerOpen(false)
}
```
- Remplacer `onOpenList={() => setPinsVisible(true)}` par `onOpenList={() => setDrawerOpen((o) => !o)}`.
- Juste avant `<ActionBar ... />`, monter le drawer :
```tsx
<CommentsDrawer
  open={drawerOpen}
  pins={pins}
  statusOf={(id) => positions.find((p) => p.id === id)?.status}
  onClose={() => setDrawerOpen(false)}
  onSelect={focusPinFromList}
/>
```

- [ ] **Step 4 : Lancer → succès**

Run: `pnpm test -- comments-app`
Expected: PASS.

- [ ] **Step 5 : Suite frontend complète + qualité**

Run: `pnpm lint && pnpm typecheck && pnpm test`
Expected: PASS (toute la suite Vitest verte).

- [ ] **Step 6 : Commit**

```bash
rtk git add frontend/src/comments/comments-app.tsx frontend/src/comments/comments-app.test.tsx
rtk git commit -m "✨ feat(comments): drawer branché — ouverture + focus/scroll vers le pin"
```

---

### Task 7 : e2e — drawer visiteur + alignement pins admin

**Files:**
- Modify: `frontend/e2e/comments.spec.ts`
- Modify: `frontend/e2e/comments-admin.spec.ts`

**Interfaces:** réutilise les helpers existants (`apiLogin`, `createProject`, `deploy`, `seedComment`, `pageLogin`).

- [ ] **Step 1 : Parcours drawer visiteur**

Dans `frontend/e2e/comments.spec.ts`, **juste avant** le `await page.reload()` final (une fois le pin `[data-status="anchored"]` visible), insérer :
```ts
  // Ouvrir le drawer de liste et focus le thread depuis une ligne.
  await page.getByRole('button', { name: /My comments|Mes commentaires/ }).click()
  await expect(page.getByTestId('comments-drawer')).toBeVisible()
  await page.getByTestId('drawer-row').first().click()
  // Le clic ferme le drawer et ouvre le thread (bouton Répondre visible).
  await expect(page.getByRole('button', { name: /^(Reply|Répondre)$/ })).toBeVisible()
```

- [ ] **Step 2 : Assertion d'alignement des pins admin (régression topbar)**

Dans `frontend/e2e/comments-admin.spec.ts`, dans le 1ᵉʳ test, **après** l'étape 6 (`await expect(pinBadge).toBeVisible(...)`) et **avant** l'étape 7 (`await pinBadge.click()`), insérer :
```ts
  // Régression : le pin doit s'aligner verticalement sur #cta (offset 0.5,0.5 → centre),
  // et NON être décalé vers le bas de la hauteur de la topbar (bug corrigé par l'overlay fixed).
  const ctaBox = await page
    .frameLocator('iframe[title="Prototype preview"]')
    .locator('#cta')
    .boundingBox()
  const pinBox = await pinBadge.boundingBox()
  const ctaCenterY = ctaBox!.y + ctaBox!.height / 2
  const pinCenterY = pinBox!.y + pinBox!.height / 2
  expect(Math.abs(pinCenterY - ctaCenterY)).toBeLessThan(20)
```

- [ ] **Step 3 : Lancer les e2e commentaires**

Run: `pnpm exec playwright test comments`
Expected: PASS (les 2 fichiers `comments*.spec.ts`).
> Si le webServer n'est pas déjà lancé, Playwright le démarre via sa config (`webServer`).

- [ ] **Step 4 : Commit**

```bash
rtk git add frontend/e2e/comments.spec.ts frontend/e2e/comments-admin.spec.ts
rtk git commit -m "✅ test(e2e): drawer visiteur + alignement pins admin (régression topbar)"
```

---

### Task 8 : Vérification finale + mise à jour mémoire

**Files:**
- Modify: `docs/INDEX.md`, `docs/HANDOFF.md`, `docs/QUIRKS.md`, `docs/CONVENTIONS.md`

- [ ] **Step 1 : Gate frontend complète**

Run (depuis `frontend/`): `pnpm lint && pnpm typecheck && pnpm test && pnpm exec playwright test`
Expected: tout vert. (Backend inchangé → `cargo nextest` non requis, mais le lancer ne coûte rien : `cd .. && cargo nextest run` doit rester vert.)

- [ ] **Step 2 : `docs/INDEX.md`**

Ajouter une ligne dans la table des features : « Refactor UX commentaires — drawer de liste, pins fluo + label auteur, ciblage inset-glow, positionnement popups borné, fix décalage pins admin » avec liens vers `specs/2026-07-01-comments-ux-refactor-design.md` et `plans/2026-07-01-comments-ux-refactor.md`.

- [ ] **Step 3 : `docs/QUIRKS.md`**

Ajouter l'entrée : « **Overlay commentaires = espace viewport.** `SameOriginPicker.toShellRect` renvoie des coordonnées viewport. L'`OverlayLayer` (pins + ciblage) DOIT être `position: fixed` inset-0 ; en `absolute` dans un conteneur décalé (topbar admin `h-14`=56px), les pins se décalent vers le bas de la hauteur du décalage (double-comptage). Les popups (`useFloatingRect`, `fixed`) sont déjà correctes. »

- [ ] **Step 4 : `docs/CONVENTIONS.md`**

Ajouter : « **Couleur fluo commentaires** : constante `COMMENT_FLUO = '#18A0FB'` dans `comments/ui/colors.ts`. Fixe, jamais via `--primary` (l'overlay est rendu sur un proto au thème arbitraire). L'ambre `#f59e0b` reste réservé aux pins orphaned/moved. »

- [ ] **Step 5 : `docs/HANDOFF.md`**

Ajouter une entrée datée en haut (sous le H1) avec : `Dernière chose faite` (refactor UX commentaires livré), `Trucs en suspens`, `Prochaine chose à creuser`, `Notes pour future Claude`.

- [ ] **Step 6 : Commit mémoire**

```bash
rtk git add docs/INDEX.md docs/HANDOFF.md docs/QUIRKS.md docs/CONVENTIONS.md
rtk git commit -m "📝 docs(memory): refactor UX commentaires livré (drawer + pins + positionnement)"
```

---

## Self-Review (auteur du plan)

- **Couverture spec** : (1) drawer → Task 5+6 ; (2) label pin → Task 1 ; (3) couleur fluo → Task 1 ; (4) positionnement popups → Task 3 ; (5) ciblage inset-glow → Task 2 ; (6) propagation admin → automatique (composants partagés) + validée par Task 7 e2e admin ; (7) décalage pins admin → Task 4. ✅ Tous les points couverts.
- **Placeholders** : aucun « TBD/TODO » ; code complet à chaque step.
- **Cohérence des types** : `label` (string) cohérent entre `PinBadge` (Task 1) et `labelOf` d'`OverlayLayer` (Task 1) et `comments-app` ; `glowShadow` (Task 2), `floatingMiddleware` (Task 3), `firstLetter`/`COMMENT_FLUO` (Task 1) réutilisés en Task 5 ; `statusOf` d'`OverlayLayer`/`CommentsDrawer` alimenté par `positions[].status` en Task 6. ✅
