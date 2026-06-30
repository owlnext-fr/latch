# Commentaires ancrés — Frontend (Plan 2 : module partagé + shell visiteur) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Construire la couche frontend des commentaires ancrés type Figma sur la surface visiteur `/c/<slug>` — moteur d'ancrage, suivi live, overlay/popups, barre d'action — chargée en lazy dans le shell, contre un backend déjà livré.

**Architecture :** Module partagé `src/comments/` à responsabilités isolées derrière le *seam* `Picker` (1 interface, 1 impl `SameOriginPicker` qui lit l'iframe same-origin). Le moteur d'ancrage (descripteur W3C → `describe` → `resolve` en cascade) est en TypeScript pur, testable en jsdom sans React. Le contrôleur de suivi (un seul rAF à dirty-flag + observers) repositionne tous les pins. La couche overlay (React + `@floating-ui/dom`) rend pastilles/popups. Le shell lit `PublicMeta.comments_enabled` et charge le gros module en **lazy** (`React.lazy` + `import()` dynamique — premier du repo) ; React Query est confiné au chunk lazy avec son propre `QueryClient`.

**Tech Stack :** React 19, TypeScript ~6, Vite 8 (multi-entrées, `shell` = `shell.html` → `src/shell/main.tsx`), `@tanstack/react-query` ^5, `openapi-fetch` ^0.17 (client typé depuis `src/api/schema.d.ts`, déjà régénéré pour les commentaires), `react-i18next` ^17 (instance i18n **par bundle**, clés **plates**, `keySeparator:false`), `@medv/finder` (génération de sélecteur — **à ajouter**), `@floating-ui/dom` ^1 (positionnement popups — **à ajouter**), Vitest 4 + jsdom + MSW 2 + Testing Library, Playwright (e2e desktop). Lint : ESLint flat + Prettier (`semi:false`, `singleQuote:true`, `trailingComma:all`).

## Global Constraints

- **Périmètre de CE plan = visiteur uniquement** (spec §8 module partagé + §9 montage shell). Admin Review (§10), toggle `ProjectForm` (§10.1) et docs publiques (§13) sont **hors périmètre → Plan 3**. Ne PAS toucher `src/main.tsx` (admin), `src/router.tsx`, `project-form.tsx`.
- **Backend déjà livré** — ne rien modifier sous `backend/`. Toutes les routes, le gating (`unlock_ok` + `comments_enabled`), le cookie d'identité `latch_comment`, le rate-limit, la garde Origin, le header `X-Comment-Client` et la CSP `frame-ancestors 'self'` sur `/c/{slug}/raw` existent. Le frontend consomme les types **déjà générés** dans `src/api/schema.d.ts` (NE PAS relancer `pnpm gen:api` — `openapi.json` est figé pour ce plan).
- **Invariant build-breaking (contrat §9 inv. 7)** : `owner_token` n'est JAMAIS sérialisé par le backend → le frontend ne le reçoit jamais. Le frontend utilise UNIQUEMENT le booléen `editable` (sur `CommentMessage`) pour décider d'afficher éditer/supprimer. Ne JAMAIS inventer ni stocker de champ `owner_token` côté client.
- **Corps de commentaire = texte brut** (décision produit, spec §3). Échappement JSX par défaut (afficher `{message.body}` dans un nœud texte). NE PAS introduire `react-markdown` ni aucun rendu HTML dans la couche commentaire.
- **Parent-reaches-in same-origin** : le shell lit `iframe.contentDocument` / `contentWindow`. AUCUNE injection de script dans `/raw`. AUCUN `postMessage`. Si `contentDocument` est `null` (pas encore chargé), attendre l'événement `load` de l'iframe.
- **Confidentialité (CLAUDE.md, NON-NÉGOCIABLE)** : aucun nom de client réel nulle part. Placeholders fictifs uniquement (`demo`, `ACME`, `mon-projet`, `Mon Projet`, `Léa`).
- **i18n** : clés **plates** (`comment.xxx`), `_meta` conservé dans chaque fichier de locale, EN + FR, défaut EN. Les clés visiteur vont dans `src/i18n/locales/shell/{en,fr}.json` (bundle shell), PAS dans `admin/`.
- **Style de code** : Prettier `semi:false singleQuote:true trailingComma:all`. Props de composants typées `Readonly<…>` (convention du repo). Imports via alias `@/…`.
- **« Terminé »** (CLAUDE.md) : `pnpm lint` + `pnpm typecheck` + `pnpm test` verts ; gate Sonar `new_coverage ≥ 80 %` sur le code neuf ; e2e Playwright vert ; docs mémoire mises à jour à la livraison (HANDOFF/INDEX, et QUIRKS pour les pièges jsdom/iframe découverts).
- **Commits** : fréquents, un par tâche minimum, en français, style gitmoji du repo (ex. `✨ feat(comments): …`, `✅ test(comments): …`).

---

## File Structure

Module partagé (nouveau dossier), tout en lazy-chunk :

```
frontend/src/comments/
  anchor/
    descriptor.ts          # type AnchorDescriptor (format v1 §5.4) + parse/serialize
    describe.ts            # describe(el, clickPoint) -> AnchorDescriptor
    similarity.ts          # score(candidate, fingerprint) -> number (0..1)
    resolve.ts             # resolve(doc, anchor) -> ResolveResult { element, status } | null
  picker/
    picker.ts              # interface Picker + types (ResolveResult, ShellRect, PickerEvent)
    same-origin-picker.ts  # SameOriginPicker(iframe) : seule impl v1
  follow/
    controller.ts          # FollowController : 1 rAF dirty-flag, observers, transposition iframe->shell
  data/
    adapter.ts             # interface CommentsAdapter + type Capabilities
    visitor-adapter.ts     # createVisitorAdapter(slug) : appels openapi-fetch (+ X-Comment-Client)
    use-comments.ts        # hooks React Query (confinés au module)
  state/
    pick-machine.ts        # réducteur d'état du mode pick (idle|pick|compose)
  ui/
    overlay-layer.tsx      # calque frère de l'iframe : highlight survol + pastilles
    pin-badge.tsx          # pastille d'un pin (+ état approximate/orphaned)
    thread-popup.tsx       # popup floating-ui d'un fil (lecture + reply + edit + delete)
    compose-popup.tsx      # popup nouveau-commentaire (nom lazy + corps)
    action-bar.tsx         # barre flottante bas (3 boutons) + liste « mes commentaires »
    name-prompt.ts         # helpers localStorage du nom pré-rempli
  comments-app.tsx         # composant racine du module : wire picker+follow+overlay+adapter+QueryClient
  index.ts                 # export public du module (point d'entrée du import() dynamique)

frontend/src/components/ui/
  textarea.tsx             # composant shadcn manquant (corps de commentaire)

frontend/src/shell/
  shell-page.tsx           # MODIF : fetch PublicMeta ; monte CommentsMount si comments_enabled
  comments-mount.tsx       # NOUVEAU : React.lazy du module + déclencheurs de chargement

frontend/src/i18n/locales/shell/
  en.json / fr.json        # MODIF : ajout des clés comment.*

frontend/e2e/
  comments.spec.ts         # NOUVEAU : parcours visiteur desktop
```

**Découpage** : `anchor/*` est du TS pur (zéro React, zéro DOM-vivant requis au-delà de jsdom) → testé en isolation et atteint l'essentiel de la couverture. `picker/`, `follow/` dépendent de `anchor/`. `data/` est indépendant (testé via MSW). `ui/` dépend de `picker`+`follow`+`data`. `comments-app.tsx` câble tout. Le shell ne connaît que `comments-mount.tsx`.

---

## Phase 0 — Dépendances, scaffolding, i18n

### Task 0: Dépendances + dossier module + clés i18n shell

**Files:**
- Modify: `frontend/package.json` (dependencies)
- Create: `frontend/src/comments/index.ts` (placeholder d'amorce, remplacé en Task D7)
- Modify: `frontend/src/i18n/locales/shell/en.json`
- Modify: `frontend/src/i18n/locales/shell/fr.json`

**Interfaces:**
- Produces: dépendances `@medv/finder` et `@floating-ui/dom` disponibles ; clés `comment.*` résolues par l'instance i18n du shell.

- [ ] **Step 1: Installer les deux dépendances runtime**

Run (depuis `frontend/`) :
```bash
pnpm add @medv/finder @floating-ui/dom
```
Attendu : `package.json` gagne `@medv/finder` et `@floating-ui/dom` sous `dependencies` ; `pnpm-lock.yaml` mis à jour. Ces deux libs sont tree-shakées dans le chunk lazy (jamais dans le bundle admin).

- [ ] **Step 2: Vérifier l'installation**

Run : `pnpm ls @medv/finder @floating-ui/dom`
Attendu : les deux paquets listés avec une version résolue (pas d'erreur `missing`).

- [ ] **Step 3: Créer le fichier d'amorce du module**

Create `frontend/src/comments/index.ts` :
```ts
// Point d'entrée du module commentaires (chargé en lazy depuis le shell).
// Le composant racine est ajouté en Task D7 ; ce placeholder permet aux tâches
// amont (anchor/picker/follow/data) de committer sans casser le typecheck.
export const COMMENTS_MODULE_VERSION = 1
```

- [ ] **Step 4: Ajouter les clés i18n EN (shell)**

Remplacer le contenu de `frontend/src/i18n/locales/shell/en.json` par :
```json
{
  "_meta": { "name": "English", "flag": "GB" },
  "shell.notes_title": "What's new",
  "shell.dismiss": "Got it",
  "comment.bar.pick": "Comment",
  "comment.bar.toggle_pins": "Show comments",
  "comment.bar.list": "My comments",
  "comment.bar.count": "{{count}} comment",
  "comment.bar.count_plural": "{{count}} comments",
  "comment.compose.name_label": "Your name",
  "comment.compose.name_placeholder": "e.g. Léa",
  "comment.compose.body_label": "Comment",
  "comment.compose.body_placeholder": "Write your comment…",
  "comment.compose.submit": "Post",
  "comment.compose.cancel": "Cancel",
  "comment.thread.reply_placeholder": "Reply…",
  "comment.thread.reply_submit": "Reply",
  "comment.thread.edit": "Edit",
  "comment.thread.delete": "Delete",
  "comment.thread.save": "Save",
  "comment.thread.cancel": "Cancel",
  "comment.thread.delete_confirm": "Delete this comment?",
  "comment.thread.moved": "This element may have moved",
  "comment.thread.orphaned": "Original element not found",
  "comment.list.empty": "You haven't added any comments yet.",
  "comment.list.title": "My comments",
  "comment.error.name_required": "Please enter your name.",
  "comment.error.body_required": "Comment can't be empty.",
  "comment.error.body_too_long": "Comment is too long (max 2000 characters).",
  "comment.error.network": "Something went wrong. Please try again."
}
```

- [ ] **Step 5: Ajouter les clés i18n FR (shell)**

Remplacer le contenu de `frontend/src/i18n/locales/shell/fr.json` par :
```json
{
  "_meta": { "name": "Français", "flag": "FR" },
  "shell.notes_title": "Nouveautés",
  "shell.dismiss": "Compris",
  "comment.bar.pick": "Commenter",
  "comment.bar.toggle_pins": "Afficher les commentaires",
  "comment.bar.list": "Mes commentaires",
  "comment.bar.count": "{{count}} commentaire",
  "comment.bar.count_plural": "{{count}} commentaires",
  "comment.compose.name_label": "Votre nom",
  "comment.compose.name_placeholder": "ex. Léa",
  "comment.compose.body_label": "Commentaire",
  "comment.compose.body_placeholder": "Écrivez votre commentaire…",
  "comment.compose.submit": "Publier",
  "comment.compose.cancel": "Annuler",
  "comment.thread.reply_placeholder": "Répondre…",
  "comment.thread.reply_submit": "Répondre",
  "comment.thread.edit": "Modifier",
  "comment.thread.delete": "Supprimer",
  "comment.thread.save": "Enregistrer",
  "comment.thread.cancel": "Annuler",
  "comment.thread.delete_confirm": "Supprimer ce commentaire ?",
  "comment.thread.moved": "Cet élément a peut-être bougé",
  "comment.thread.orphaned": "Élément d'origine introuvable",
  "comment.list.empty": "Vous n'avez pas encore de commentaire.",
  "comment.list.title": "Mes commentaires",
  "comment.error.name_required": "Veuillez saisir votre nom.",
  "comment.error.body_required": "Le commentaire ne peut pas être vide.",
  "comment.error.body_too_long": "Le commentaire est trop long (2000 caractères max).",
  "comment.error.network": "Une erreur est survenue. Réessayez."
}
```

- [ ] **Step 6: Vérifier le typecheck + lint**

Run : `pnpm typecheck && pnpm lint`
Attendu : PASS (aucune erreur). Le JSON i18n est valide, le module index compile.

- [ ] **Step 7: Commit**

```bash
git add frontend/package.json frontend/pnpm-lock.yaml frontend/src/comments/index.ts frontend/src/i18n/locales/shell/en.json frontend/src/i18n/locales/shell/fr.json
git commit -m "✨ feat(comments): deps (@medv/finder, @floating-ui/dom) + scaffold module + i18n shell"
```

---

## Phase A — Moteur d'ancrage (TypeScript pur, jsdom)

> Tout `anchor/*` est testable sans React ni vraie iframe : on construit un `document`
> jsdom (via `document.body.innerHTML = …` dans les tests Vitest) et on appelle les
> fonctions directement. C'est le cœur de la couverture.

### Task A1: Descripteur d'ancrage — type + (dé)sérialisation

**Files:**
- Create: `frontend/src/comments/anchor/descriptor.ts`
- Test: `frontend/src/comments/anchor/descriptor.test.ts`

**Interfaces:**
- Produces:
  - `interface AnchorDescriptor { v: 1; selector: string; fingerprint: Fingerprint; textQuote: TextQuote | null; offset: Point; fallbackPoint: Point }`
  - `interface Fingerprint { tag: string; text: string; role: string | null; ordinal: number }`
  - `interface TextQuote { exact: string; prefix: string; suffix: string }`
  - `interface Point { x: number; y: number }`
  - `serializeAnchor(a: AnchorDescriptor): string` (JSON stable)
  - `parseAnchor(raw: string): AnchorDescriptor | null` (null si JSON invalide ou `v !== 1`)

- [ ] **Step 1: Écrire le test qui échoue**

Create `frontend/src/comments/anchor/descriptor.test.ts` :
```ts
import { describe, expect, it } from 'vitest'
import {
  parseAnchor,
  serializeAnchor,
  type AnchorDescriptor,
} from './descriptor'

const sample: AnchorDescriptor = {
  v: 1,
  selector: 'main > section .card > button',
  fingerprint: { tag: 'button', text: 'En savoir plus', role: 'button', ordinal: 2 },
  textQuote: { exact: 'En savoir plus', prefix: 'avant ', suffix: ' après' },
  offset: { x: 0.42, y: 0.6 },
  fallbackPoint: { x: 0.31, y: 0.78 },
}

describe('anchor descriptor', () => {
  it('round-trips through serialize/parse', () => {
    expect(parseAnchor(serializeAnchor(sample))).toEqual(sample)
  })

  it('returns null on invalid JSON', () => {
    expect(parseAnchor('{not json')).toBeNull()
  })

  it('returns null when version is not 1', () => {
    const raw = JSON.stringify({ ...sample, v: 2 })
    expect(parseAnchor(raw)).toBeNull()
  })

  it('accepts a null textQuote', () => {
    const noQuote = { ...sample, textQuote: null }
    expect(parseAnchor(serializeAnchor(noQuote))).toEqual(noQuote)
  })
})
```

- [ ] **Step 2: Lancer le test, vérifier l'échec**

Run : `pnpm exec vitest run src/comments/anchor/descriptor.test.ts`
Attendu : FAIL (`Cannot find module './descriptor'`).

- [ ] **Step 3: Implémenter le minimum**

Create `frontend/src/comments/anchor/descriptor.ts` :
```ts
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
```

- [ ] **Step 4: Lancer le test, vérifier le succès**

Run : `pnpm exec vitest run src/comments/anchor/descriptor.test.ts`
Attendu : PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add frontend/src/comments/anchor/descriptor.ts frontend/src/comments/anchor/descriptor.test.ts
git commit -m "✨ feat(comments): descripteur d'ancrage v1 (type + parse/serialize)"
```

### Task A2: `describe()` — capture du descripteur depuis un élément

**Files:**
- Create: `frontend/src/comments/anchor/describe.ts`
- Test: `frontend/src/comments/anchor/describe.test.ts`

**Interfaces:**
- Consumes: `AnchorDescriptor`, `Fingerprint`, `TextQuote`, `Point` (Task A1) ; `finder` depuis `@medv/finder`.
- Produces:
  - `describe(el: Element, clickPoint: Point, root?: Document): AnchorDescriptor`
    où `clickPoint` est en coordonnées **client de l'élément** (px relatifs à `el.getBoundingClientRect()`), converti en offset% interne ; `root` par défaut = `el.ownerDocument`.
  - `normalizeText(s: string): string` (export utilitaire, réutilisé par `similarity.ts` et `resolve.ts`)

- [ ] **Step 1: Écrire le test qui échoue**

Create `frontend/src/comments/anchor/describe.test.ts` :
```ts
import { describe, expect, it, beforeEach } from 'vitest'
import { describe as describeAnchor, normalizeText } from './describe'

beforeEach(() => {
  document.body.innerHTML = `
    <main>
      <section><button id="a">First</button></section>
      <section>
        <div class="card"><button id="b">En savoir   plus</button></div>
      </section>
    </main>`
})

describe('normalizeText', () => {
  it('trims and collapses whitespace', () => {
    expect(normalizeText('  En savoir   plus\n')).toBe('En savoir plus')
  })
})

describe('describe()', () => {
  it('captures a selector that re-finds the same element', () => {
    const el = document.getElementById('b')!
    const anchor = describeAnchor(el, { x: 5, y: 5 })
    expect(document.querySelector(anchor.selector)).toBe(el)
  })

  it('captures a fingerprint with tag, normalized text and ordinal', () => {
    const el = document.getElementById('b')!
    const anchor = describeAnchor(el, { x: 5, y: 5 })
    expect(anchor.fingerprint.tag).toBe('button')
    expect(anchor.fingerprint.text).toBe('En savoir plus')
    expect(anchor.fingerprint.ordinal).toBe(0) // seul button dans son parent
  })

  it('encodes the click point as an offset fraction of the element box', () => {
    const el = document.getElementById('b')!
    // jsdom renvoie un rect 0x0 ; on stub getBoundingClientRect pour ce test
    el.getBoundingClientRect = () =>
      ({ left: 0, top: 0, width: 100, height: 50 }) as DOMRect
    const anchor = describeAnchor(el, { x: 42, y: 30 })
    expect(anchor.offset.x).toBeCloseTo(0.42, 2)
    expect(anchor.offset.y).toBeCloseTo(0.6, 2)
  })

  it('sets format version 1', () => {
    const anchor = describeAnchor(document.getElementById('a')!, { x: 0, y: 0 })
    expect(anchor.v).toBe(1)
  })
})
```

- [ ] **Step 2: Lancer le test, vérifier l'échec**

Run : `pnpm exec vitest run src/comments/anchor/describe.test.ts`
Attendu : FAIL (module introuvable).

- [ ] **Step 3: Implémenter**

Create `frontend/src/comments/anchor/describe.ts` :
```ts
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
```

- [ ] **Step 4: Lancer le test, vérifier le succès**

Run : `pnpm exec vitest run src/comments/anchor/describe.test.ts`
Attendu : PASS (5 tests). Si `finder` échoue en jsdom sur un cas, le `try/catch` retombe sur le tag — le test du sélecteur reste vert car `querySelector('button')` ne renverra pas forcément `#b`. NOTE : le test « re-finds the same element » suppose que finder produit un sélecteur unique ; en jsdom finder fonctionne. Si flaky, durcir le DOM-fixture (ids/classes distincts).

- [ ] **Step 5: Commit**

```bash
git add frontend/src/comments/anchor/describe.ts frontend/src/comments/anchor/describe.test.ts
git commit -m "✨ feat(comments): describe() — capture sélecteur+empreinte+textQuote+offset"
```

### Task A3: Scorer de similarité

**Files:**
- Create: `frontend/src/comments/anchor/similarity.ts`
- Test: `frontend/src/comments/anchor/similarity.test.ts`

**Interfaces:**
- Consumes: `Fingerprint`, `normalizeText` (Task A1/A2).
- Produces: `score(el: Element, fp: Fingerprint): number` → score 0..1 (1 = parfait). Utilisé par `resolve()` pour départager des candidats quand le sélecteur échoue.

- [ ] **Step 1: Écrire le test qui échoue**

Create `frontend/src/comments/anchor/similarity.test.ts` :
```ts
import { describe, expect, it, beforeEach } from 'vitest'
import { score } from './similarity'
import type { Fingerprint } from './descriptor'

const fp: Fingerprint = { tag: 'button', text: 'En savoir plus', role: 'button', ordinal: 2 }

beforeEach(() => {
  document.body.innerHTML = `
    <button id="exact">En savoir plus</button>
    <button id="othertext">Acheter</button>
    <div id="wrongtag">En savoir plus</div>`
})

describe('score()', () => {
  it('gives 1 (or near) to an exact tag+text+role match', () => {
    expect(score(document.getElementById('exact')!, fp)).toBeGreaterThan(0.8)
  })

  it('penalises a wrong tag', () => {
    const right = score(document.getElementById('exact')!, fp)
    const wrong = score(document.getElementById('wrongtag')!, fp)
    expect(wrong).toBeLessThan(right)
  })

  it('penalises different text', () => {
    expect(score(document.getElementById('othertext')!, fp)).toBeLessThan(
      score(document.getElementById('exact')!, fp),
    )
  })

  it('returns a value in [0, 1]', () => {
    const s = score(document.getElementById('othertext')!, fp)
    expect(s).toBeGreaterThanOrEqual(0)
    expect(s).toBeLessThanOrEqual(1)
  })
})
```

- [ ] **Step 2: Lancer le test, vérifier l'échec**

Run : `pnpm exec vitest run src/comments/anchor/similarity.test.ts`
Attendu : FAIL (module introuvable).

- [ ] **Step 3: Implémenter**

Create `frontend/src/comments/anchor/similarity.ts` :
```ts
import { normalizeText } from './describe'
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

function roleOf(el: Element): string | null {
  return el.getAttribute('role')
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
```

- [ ] **Step 4: Lancer le test, vérifier le succès**

Run : `pnpm exec vitest run src/comments/anchor/similarity.test.ts`
Attendu : PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add frontend/src/comments/anchor/similarity.ts frontend/src/comments/anchor/similarity.test.ts
git commit -m "✨ feat(comments): scorer de similarité d'empreinte (tag/texte/rôle)"
```

### Task A4: `resolve()` — cascade de résolution

**Files:**
- Create: `frontend/src/comments/anchor/resolve.ts`
- Test: `frontend/src/comments/anchor/resolve.test.ts`

**Interfaces:**
- Consumes: `AnchorDescriptor`, `score` (A3), `normalizeText` (A2).
- Produces:
  - `type AnchorStatus = 'anchored' | 'approximate' | 'orphaned'`
  - `interface ResolveResult { element: Element | null; status: AnchorStatus }`
  - `resolve(doc: Document, anchor: AnchorDescriptor): ResolveResult`
    - sélecteur → 1 match exact = `anchored`
    - plusieurs matches → empreinte+ordinal → meilleur = `anchored` si score ≥ 0.9 sinon `approximate`
    - 0 match sélecteur → scorer global sur tous les éléments → meilleur ≥ 0.6 = `approximate`
    - sinon `textQuote` exact dans le DOM → `approximate`
    - sinon `{ element: null, status: 'orphaned' }`

- [ ] **Step 1: Écrire le test qui échoue**

Create `frontend/src/comments/anchor/resolve.test.ts` :
```ts
import { describe, expect, it } from 'vitest'
import { resolve } from './resolve'
import type { AnchorDescriptor } from './descriptor'

function anchorFor(selector: string, text: string): AnchorDescriptor {
  return {
    v: 1,
    selector,
    fingerprint: { tag: 'button', text, role: 'button', ordinal: 0 },
    textQuote: { exact: text, prefix: '', suffix: '' },
    offset: { x: 0.5, y: 0.5 },
    fallbackPoint: { x: 0.5, y: 0.5 },
  }
}

function docWith(html: string): Document {
  const doc = document.implementation.createHTMLDocument('t')
  doc.body.innerHTML = html
  return doc
}

describe('resolve()', () => {
  it('returns anchored on a unique selector match', () => {
    const doc = docWith('<button class="cta">Buy</button>')
    const res = resolve(doc, anchorFor('button.cta', 'Buy'))
    expect(res.status).toBe('anchored')
    expect(res.element?.textContent).toBe('Buy')
  })

  it('falls back to fingerprint scoring when the selector misses', () => {
    const doc = docWith('<section><button class="renamed">Buy</button></section>')
    const res = resolve(doc, anchorFor('button.cta', 'Buy'))
    expect(res.status).toBe('approximate')
    expect(res.element?.textContent).toBe('Buy')
  })

  it('uses textQuote when fingerprint scoring is weak', () => {
    const doc = docWith('<p>Some unique sentence here</p>')
    const anchor = anchorFor('button.gone', 'Some unique sentence here')
    const res = resolve(doc, anchor)
    expect(res.status).toBe('approximate')
    expect(res.element?.tagName).toBe('P')
  })

  it('returns orphaned when nothing matches', () => {
    const doc = docWith('<div>totally different</div>')
    const res = resolve(doc, anchorFor('button.cta', 'Vanished label'))
    expect(res.status).toBe('orphaned')
    expect(res.element).toBeNull()
  })
})
```

- [ ] **Step 2: Lancer le test, vérifier l'échec**

Run : `pnpm exec vitest run src/comments/anchor/resolve.test.ts`
Attendu : FAIL (module introuvable).

- [ ] **Step 3: Implémenter**

Create `frontend/src/comments/anchor/resolve.ts` :
```ts
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
```

- [ ] **Step 4: Lancer le test, vérifier le succès**

Run : `pnpm exec vitest run src/comments/anchor/resolve.test.ts`
Attendu : PASS (4 tests).

- [ ] **Step 5: Lancer toute la suite anchor + lint + typecheck**

Run : `pnpm exec vitest run src/comments/anchor && pnpm lint && pnpm typecheck`
Attendu : PASS (4 fichiers de test, lint/typecheck clean).

- [ ] **Step 6: Commit**

```bash
git add frontend/src/comments/anchor/resolve.ts frontend/src/comments/anchor/resolve.test.ts
git commit -m "✨ feat(comments): resolve() — cascade sélecteur→empreinte→textQuote→orphaned"
```

## Phase B — Picker (seam) + contrôleur de suivi

### Task B0: Étendre les shims jsdom (Observers + rAF)

> Le picker et le contrôleur utilisent `IntersectionObserver`, `MutationObserver`,
> `getBoundingClientRect` et `requestAnimationFrame`. jsdom fournit `MutationObserver`
> mais PAS `IntersectionObserver`, et `getBoundingClientRect` renvoie des zéros. On
> ajoute les shims manquants au setup global ; le contrôleur **injecte** son scheduler
> de frame pour rester déterministe en test (pas de dépendance à rAF réel).

**Files:**
- Modify: `frontend/vitest.setup.ts`

**Interfaces:**
- Produces: `globalThis.IntersectionObserver` (stub no-op) disponible dans tous les tests.

- [ ] **Step 1: Ajouter le shim IntersectionObserver**

Dans `frontend/vitest.setup.ts`, après le bloc `ResizeObserver` (autour de la ligne 12), insérer :
```ts
// jsdom ne fournit pas IntersectionObserver (utilisé par le contrôleur de suivi des pins).
if (!('IntersectionObserver' in globalThis)) {
  globalThis.IntersectionObserver = class {
    readonly root = null
    readonly rootMargin = ''
    readonly thresholds = []
    observe() {}
    unobserve() {}
    disconnect() {}
    takeRecords() {
      return []
    }
  } as unknown as typeof IntersectionObserver
}
```

- [ ] **Step 2: Vérifier que la suite existante reste verte**

Run : `pnpm test`
Attendu : PASS (aucune régression ; le shim est gardé par `if (!('IntersectionObserver' in globalThis))`).

- [ ] **Step 3: Commit**

```bash
git add frontend/vitest.setup.ts
git commit -m "✅ test(comments): shim jsdom IntersectionObserver pour le contrôleur de suivi"
```

### Task B1: Interface `Picker` + `SameOriginPicker`

**Files:**
- Create: `frontend/src/comments/picker/picker.ts`
- Create: `frontend/src/comments/picker/same-origin-picker.ts`
- Test: `frontend/src/comments/picker/same-origin-picker.test.ts`

**Interfaces:**
- Consumes: `describe` (A2), `resolve`/`ResolveResult` (A4), `AnchorDescriptor`/`Point` (A1).
- Produces:
  - `interface ShellRect { x: number; y: number; width: number; height: number }`
  - `interface FrameRef { contentDocument: Document | null; contentWindow: Window | null; getBoundingClientRect(): DOMRect }` (satisfait par `HTMLIFrameElement` ; permet un double de test)
  - `interface Picker { getElementAt(x, y): Element | null; describe(el, clickPoint): AnchorDescriptor; resolve(anchor): ResolveResult; toShellRect(el): ShellRect | null; fallbackRect(anchor): ShellRect; subscribe(cb: () => void): () => void; readonly doc: Document | null }`
  - `class SameOriginPicker implements Picker` ; constructeur `(frame: FrameRef)`.

- [ ] **Step 1: Écrire l'interface (pas de logique)**

Create `frontend/src/comments/picker/picker.ts` :
```ts
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
```

- [ ] **Step 2: Écrire le test qui échoue**

Create `frontend/src/comments/picker/same-origin-picker.test.ts` :
```ts
import { describe, expect, it, vi } from 'vitest'
import { SameOriginPicker } from './same-origin-picker'
import type { FrameRef } from './picker'

/** Construit un faux iframe pointant sur un document jsdom détaché. */
function fakeFrame(html: string, frameRect: Partial<DOMRect> = {}): {
  frame: FrameRef
  doc: Document
  win: { addEventListener: ReturnType<typeof vi.fn>; removeEventListener: ReturnType<typeof vi.fn> }
} {
  const doc = document.implementation.createHTMLDocument('proto')
  doc.body.innerHTML = html
  const win = { addEventListener: vi.fn(), removeEventListener: vi.fn() }
  const frame: FrameRef = {
    contentDocument: doc,
    contentWindow: win as unknown as Window,
    getBoundingClientRect: () =>
      ({ left: 10, top: 20, width: 800, height: 600, ...frameRect }) as DOMRect,
  }
  return { frame, doc, win }
}

describe('SameOriginPicker', () => {
  it('exposes the content document', () => {
    const { frame, doc } = fakeFrame('<button>Hi</button>')
    expect(new SameOriginPicker(frame).doc).toBe(doc)
  })

  it('getElementAt translates shell coords into iframe coords', () => {
    const { frame, doc } = fakeFrame('<button id="b">Hi</button>')
    const target = doc.getElementById('b')!
    const spy = vi.spyOn(doc, 'elementFromPoint').mockReturnValue(target)
    const picker = new SameOriginPicker(frame)
    const el = picker.getElementAt(110, 220) // shell (110,220) - frame (10,20) = (100,200)
    expect(spy).toHaveBeenCalledWith(100, 200)
    expect(el).toBe(target)
  })

  it('toShellRect offsets the element rect by the frame position', () => {
    const { frame, doc } = fakeFrame('<button id="b">Hi</button>')
    const el = doc.getElementById('b')!
    el.getBoundingClientRect = () =>
      ({ left: 5, top: 7, width: 30, height: 12 }) as DOMRect
    const rect = new SameOriginPicker(frame).toShellRect(el)
    expect(rect).toEqual({ x: 15, y: 27, width: 30, height: 12 }) // +frame(10,20)
  })

  it('subscribe attaches scroll/resize listeners and unsubscribe detaches', () => {
    const { frame, win } = fakeFrame('<div>x</div>')
    const cb = vi.fn()
    const off = new SameOriginPicker(frame).subscribe(cb)
    expect(win.addEventListener).toHaveBeenCalledWith('scroll', expect.any(Function), expect.anything())
    expect(win.addEventListener).toHaveBeenCalledWith('resize', expect.any(Function))
    off()
    expect(win.removeEventListener).toHaveBeenCalled()
  })

  it('getElementAt returns null when the document is not ready', () => {
    const frame: FrameRef = {
      contentDocument: null,
      contentWindow: null,
      getBoundingClientRect: () => ({ left: 0, top: 0, width: 0, height: 0 }) as DOMRect,
    }
    expect(new SameOriginPicker(frame).getElementAt(1, 1)).toBeNull()
  })
})
```

- [ ] **Step 3: Lancer le test, vérifier l'échec**

Run : `pnpm exec vitest run src/comments/picker/same-origin-picker.test.ts`
Attendu : FAIL (module introuvable).

- [ ] **Step 4: Implémenter**

Create `frontend/src/comments/picker/same-origin-picker.ts` :
```ts
import { describe as describeAnchor } from '../anchor/describe'
import { resolve as resolveAnchor, type ResolveResult } from '../anchor/resolve'
import type { AnchorDescriptor, Point } from '../anchor/descriptor'
import type { FrameRef, Picker, ShellRect } from './picker'

export class SameOriginPicker implements Picker {
  constructor(private readonly frame: FrameRef) {}

  get doc(): Document | null {
    return this.frame.contentDocument
  }

  getElementAt(shellX: number, shellY: number): Element | null {
    const doc = this.frame.contentDocument
    if (!doc) return null
    const f = this.frame.getBoundingClientRect()
    return doc.elementFromPoint(shellX - f.left, shellY - f.top)
  }

  describe(el: Element, clickPoint: Point): AnchorDescriptor {
    const doc = this.frame.contentDocument ?? el.ownerDocument
    return describeAnchor(el, clickPoint, doc)
  }

  resolve(anchor: AnchorDescriptor): ResolveResult {
    const doc = this.frame.contentDocument
    if (!doc) return { element: null, status: 'orphaned' }
    return resolveAnchor(doc, anchor)
  }

  toShellRect(el: Element): ShellRect | null {
    const f = this.frame.getBoundingClientRect()
    const r = el.getBoundingClientRect()
    return { x: f.left + r.left, y: f.top + r.top, width: r.width, height: r.height }
  }

  fallbackRect(anchor: AnchorDescriptor): ShellRect {
    const f = this.frame.getBoundingClientRect()
    return {
      x: f.left + anchor.fallbackPoint.x * f.width,
      y: f.top + anchor.fallbackPoint.y * f.height,
      width: 0,
      height: 0,
    }
  }

  subscribe(cb: () => void): () => void {
    const win = this.frame.contentWindow
    const doc = this.frame.contentDocument
    if (!win) return () => {}
    win.addEventListener('scroll', cb, { passive: true, capture: true })
    win.addEventListener('resize', cb)
    let mo: MutationObserver | null = null
    if (doc && typeof MutationObserver !== 'undefined') {
      mo = new MutationObserver(cb)
      mo.observe(doc.body, { childList: true, subtree: true, attributes: true })
    }
    return () => {
      win.removeEventListener('scroll', cb, { capture: true } as EventListenerOptions)
      win.removeEventListener('resize', cb)
      mo?.disconnect()
    }
  }
}
```

- [ ] **Step 5: Lancer le test, vérifier le succès**

Run : `pnpm exec vitest run src/comments/picker/same-origin-picker.test.ts`
Attendu : PASS (5 tests).

- [ ] **Step 6: Commit**

```bash
git add frontend/src/comments/picker/picker.ts frontend/src/comments/picker/same-origin-picker.ts frontend/src/comments/picker/same-origin-picker.test.ts
git commit -m "✨ feat(comments): seam Picker + SameOriginPicker (hit-test, transposition, subscribe)"
```

### Task B2: Contrôleur de suivi (1 rAF dirty-flag)

**Files:**
- Create: `frontend/src/comments/follow/controller.ts`
- Test: `frontend/src/comments/follow/controller.test.ts`

**Interfaces:**
- Consumes: `Picker`, `ShellRect` (B1) ; `AnchorDescriptor` (A1), `AnchorStatus` (A4).
- Produces:
  - `interface PinInput { id: number; anchor: AnchorDescriptor }`
  - `interface PinPosition { id: number; status: AnchorStatus; rect: ShellRect; offset: Point }`
  - `class FollowController` :
    - `constructor(picker: Picker, opts?: { requestFrame?: (cb: () => void) => void })` (défaut rAF ; injectable pour les tests)
    - `setPins(pins: PinInput[]): void`
    - `onUpdate(cb: (positions: PinPosition[]) => void): () => void`
    - `start(): void` / `stop(): void`
    - `markDirty(): void`

- [ ] **Step 1: Écrire le test qui échoue**

Create `frontend/src/comments/follow/controller.test.ts` :
```ts
import { describe, expect, it, vi } from 'vitest'
import { FollowController } from './controller'
import type { Picker, ShellRect } from '../picker/picker'
import type { AnchorDescriptor } from '../anchor/descriptor'

function anchor(id: number): AnchorDescriptor {
  return {
    v: 1,
    selector: `#p${id}`,
    fingerprint: { tag: 'div', text: '', role: null, ordinal: 0 },
    textQuote: null,
    offset: { x: 0.5, y: 0.5 },
    fallbackPoint: { x: 0.1, y: 0.1 },
  }
}

/** Picker factice : résout #p1 vers un élément, #p2 vers orphaned. */
function fakePicker(): Picker {
  let onChange: (() => void) | null = null
  const el = { tag: 'el' } as unknown as Element
  return {
    doc: document,
    getElementAt: () => null,
    describe: () => anchor(0),
    resolve: (a) =>
      a.selector === '#p1'
        ? { element: el, status: 'anchored' }
        : { element: null, status: 'orphaned' },
    toShellRect: (): ShellRect => ({ x: 1, y: 2, width: 3, height: 4 }),
    fallbackRect: (): ShellRect => ({ x: 9, y: 9, width: 0, height: 0 }),
    subscribe: (cb) => {
      onChange = cb
      return () => {
        onChange = null
      }
    },
    // helper exposé au test pour simuler un scroll
    ...({ fire: () => onChange?.() } as object),
  } as Picker & { fire: () => void }
}

describe('FollowController', () => {
  it('emits a position per pin on the next frame', () => {
    const frames: Array<() => void> = []
    const picker = fakePicker()
    const ctrl = new FollowController(picker, { requestFrame: (cb) => frames.push(cb) })
    const updates: unknown[] = []
    ctrl.onUpdate((p) => updates.push(p))
    ctrl.setPins([{ id: 1, anchor: anchor(1) }, { id: 2, anchor: anchor(2) }])
    ctrl.start()
    expect(frames).toHaveLength(1) // 1 frame schedulée, pas N
    frames[0]() // exécuter la frame
    expect(updates).toHaveLength(1)
    const positions = updates[0] as Array<{ id: number; status: string; rect: ShellRect }>
    expect(positions).toHaveLength(2)
    expect(positions[0]).toMatchObject({ id: 1, status: 'anchored', rect: { x: 1, y: 2 } })
    expect(positions[1]).toMatchObject({ id: 2, status: 'orphaned', rect: { x: 9, y: 9 } })
  })

  it('coalesces multiple markDirty into a single frame', () => {
    const frames: Array<() => void> = []
    const ctrl = new FollowController(fakePicker(), { requestFrame: (cb) => frames.push(cb) })
    ctrl.onUpdate(() => {})
    ctrl.setPins([{ id: 1, anchor: anchor(1) }])
    ctrl.start()
    ctrl.markDirty()
    ctrl.markDirty()
    expect(frames).toHaveLength(1) // coalescé : une seule frame en vol
  })

  it('stop() unsubscribes from the picker', () => {
    const picker = fakePicker()
    const off = vi.fn()
    picker.subscribe = () => off
    const ctrl = new FollowController(picker, { requestFrame: (cb) => cb() })
    ctrl.onUpdate(() => {})
    ctrl.start()
    ctrl.stop()
    expect(off).toHaveBeenCalled()
  })
})
```

- [ ] **Step 2: Lancer le test, vérifier l'échec**

Run : `pnpm exec vitest run src/comments/follow/controller.test.ts`
Attendu : FAIL (module introuvable).

- [ ] **Step 3: Implémenter**

Create `frontend/src/comments/follow/controller.ts` :
```ts
import type { Point } from '../anchor/descriptor'
import type { AnchorDescriptor } from '../anchor/descriptor'
import type { AnchorStatus } from '../anchor/resolve'
import type { Picker, ShellRect } from '../picker/picker'

export interface PinInput {
  id: number
  anchor: AnchorDescriptor
}

export interface PinPosition {
  id: number
  status: AnchorStatus
  rect: ShellRect
  offset: Point
}

type FrameFn = (cb: () => void) => void

const defaultRequestFrame: FrameFn = (cb) =>
  typeof requestAnimationFrame === 'function' ? void requestAnimationFrame(cb) : void cb()

export class FollowController {
  private pins: PinInput[] = []
  private listeners = new Set<(p: PinPosition[]) => void>()
  private unsubscribe: (() => void) | null = null
  private dirty = false
  private frameScheduled = false
  private readonly requestFrame: FrameFn

  constructor(
    private readonly picker: Picker,
    opts?: { requestFrame?: FrameFn },
  ) {
    this.requestFrame = opts?.requestFrame ?? defaultRequestFrame
  }

  setPins(pins: PinInput[]): void {
    this.pins = pins
    this.markDirty()
  }

  onUpdate(cb: (positions: PinPosition[]) => void): () => void {
    this.listeners.add(cb)
    return () => this.listeners.delete(cb)
  }

  start(): void {
    this.unsubscribe?.()
    this.unsubscribe = this.picker.subscribe(() => this.markDirty())
    this.markDirty()
  }

  stop(): void {
    this.unsubscribe?.()
    this.unsubscribe = null
  }

  markDirty(): void {
    this.dirty = true
    if (this.frameScheduled) return
    this.frameScheduled = true
    this.requestFrame(() => {
      this.frameScheduled = false
      if (this.dirty) this.measure()
    })
  }

  /** Phase de lecture puis d'émission (un seul passage par frame). */
  private measure(): void {
    this.dirty = false
    const positions: PinPosition[] = this.pins.map((pin) => {
      const res = this.picker.resolve(pin.anchor)
      const rect =
        (res.element ? this.picker.toShellRect(res.element) : null) ??
        this.picker.fallbackRect(pin.anchor)
      return { id: pin.id, status: res.status, rect, offset: pin.anchor.offset }
    })
    for (const cb of this.listeners) cb(positions)
  }
}
```

- [ ] **Step 4: Lancer le test, vérifier le succès**

Run : `pnpm exec vitest run src/comments/follow/controller.test.ts`
Attendu : PASS (3 tests).

- [ ] **Step 5: Suite + lint + typecheck**

Run : `pnpm exec vitest run src/comments && pnpm lint && pnpm typecheck`
Attendu : PASS.

- [ ] **Step 6: Commit**

```bash
git add frontend/src/comments/follow/controller.ts frontend/src/comments/follow/controller.test.ts
git commit -m "✨ feat(comments): contrôleur de suivi (rAF dirty-flag, lecture groupée des rects)"
```

---

## Phase C — Couche données (adaptateur + hooks React Query)

### Task C1: Adaptateur de données + capabilities + adaptateur visiteur

**Files:**
- Create: `frontend/src/comments/data/adapter.ts`
- Create: `frontend/src/comments/data/visitor-adapter.ts`
- Test: `frontend/src/comments/data/visitor-adapter.test.ts`

**Interfaces:**
- Consumes: client `api` (`@/api/client`), types `components` (`@/api/schema`).
- Produces:
  - `interface Capabilities { canAuthor: boolean; canEditOwn: boolean; canModerate: boolean }`
  - types ré-exportés : `CommentList`, `CommentPin`, `CommentMessage` (alias depuis `components['schemas']`)
  - `interface CommentsAdapter { capabilities: Capabilities; list(): Promise<CommentList>; createPin(i: { anchor: string; author_name: string; body: string }): Promise<CommentPin>; addReply(pinId: number, i: { author_name: string; body: string }): Promise<CommentMessage>; editMessage(id: number, body: string): Promise<CommentMessage>; deleteMessage(id: number): Promise<void>; deletePin(id: number): Promise<void> }`
  - `createVisitorAdapter(slug: string): CommentsAdapter` (writes portent l'en-tête `X-Comment-Client: 1`).

- [ ] **Step 1: Écrire le test qui échoue**

Create `frontend/src/comments/data/visitor-adapter.test.ts` :
```ts
import { describe, expect, it } from 'vitest'
import { http, HttpResponse } from 'msw'
import { server } from '@/test/msw'
import { createVisitorAdapter } from './visitor-adapter'

const ORIGIN = globalThis.location.origin
const SLUG = 'mon-projet-aB3dEf9z'

describe('visitor adapter', () => {
  it('list() fetches the visitor comment list', async () => {
    server.use(
      http.get(`${ORIGIN}/c/${SLUG}/comments`, () =>
        HttpResponse.json({ version: 3, pins: [] }, { status: 200 }),
      ),
    )
    const adapter = createVisitorAdapter(SLUG)
    const list = await adapter.list()
    expect(list.version).toBe(3)
    expect(list.pins).toEqual([])
  })

  it('createPin() POSTs with the X-Comment-Client header', async () => {
    let seenHeader: string | null = null
    server.use(
      http.post(`${ORIGIN}/c/${SLUG}/comments`, ({ request }) => {
        seenHeader = request.headers.get('X-Comment-Client')
        return HttpResponse.json(
          { id: 12, anchor: '{}', created_at: 'now', messages: [] },
          { status: 200 },
        )
      }),
    )
    const adapter = createVisitorAdapter(SLUG)
    const pin = await adapter.createPin({ anchor: '{}', author_name: 'Léa', body: 'Hi' })
    expect(pin.id).toBe(12)
    expect(seenHeader).toBe('1')
  })

  it('deleteMessage() resolves on ok response', async () => {
    server.use(
      http.delete(`${ORIGIN}/c/${SLUG}/comments/messages/31`, () =>
        HttpResponse.json({ ok: true }, { status: 200 }),
      ),
    )
    await expect(createVisitorAdapter(SLUG).deleteMessage(31)).resolves.toBeUndefined()
  })

  it('list() rejects on a 403 (locked project)', async () => {
    server.use(
      http.get(`${ORIGIN}/c/${SLUG}/comments`, () => new HttpResponse(null, { status: 403 })),
    )
    await expect(createVisitorAdapter(SLUG).list()).rejects.toThrow()
  })

  it('exposes visitor capabilities', () => {
    expect(createVisitorAdapter(SLUG).capabilities).toEqual({
      canAuthor: true,
      canEditOwn: true,
      canModerate: false,
    })
  })
})
```

- [ ] **Step 2: Lancer le test, vérifier l'échec**

Run : `pnpm exec vitest run src/comments/data/visitor-adapter.test.ts`
Attendu : FAIL (module introuvable).

- [ ] **Step 3: Écrire l'interface de l'adaptateur**

Create `frontend/src/comments/data/adapter.ts` :
```ts
import type { components } from '@/api/schema'

export type CommentList = components['schemas']['CommentList']
export type CommentPin = components['schemas']['CommentPin']
export type CommentMessage = components['schemas']['CommentMessage']

/** Capacités de l'appelant — pilotent l'UI (l'autorisation réelle vit au backend). */
export interface Capabilities {
  canAuthor: boolean
  canEditOwn: boolean
  canModerate: boolean
}

/** Façade de données partagée par le visiteur (et plus tard l'admin, Plan 3). */
export interface CommentsAdapter {
  readonly capabilities: Capabilities
  list(): Promise<CommentList>
  createPin(input: { anchor: string; author_name: string; body: string }): Promise<CommentPin>
  addReply(pinId: number, input: { author_name: string; body: string }): Promise<CommentMessage>
  editMessage(messageId: number, body: string): Promise<CommentMessage>
  deleteMessage(messageId: number): Promise<void>
  deletePin(pinId: number): Promise<void>
}
```

- [ ] **Step 4: Implémenter l'adaptateur visiteur**

Create `frontend/src/comments/data/visitor-adapter.ts` :
```ts
import { api } from '@/api/client'
import type {
  Capabilities,
  CommentList,
  CommentMessage,
  CommentPin,
  CommentsAdapter,
} from './adapter'

/** En-tête anti-CSRF exigé par le backend sur tous les writes commentaires. */
const WRITE_HEADERS = { 'X-Comment-Client': '1' }

const VISITOR_CAPS: Capabilities = {
  canAuthor: true,
  canEditOwn: true,
  canModerate: false,
}

export function createVisitorAdapter(slug: string): CommentsAdapter {
  return {
    capabilities: VISITOR_CAPS,

    async list(): Promise<CommentList> {
      const { data, error } = await api.GET('/c/{slug}/comments', {
        params: { path: { slug } },
      })
      if (error || !data) throw new Error('comments:list')
      return data
    },

    async createPin(input): Promise<CommentPin> {
      const { data, error } = await api.POST('/c/{slug}/comments', {
        params: { path: { slug } },
        body: input,
        headers: WRITE_HEADERS,
      })
      if (error || !data) throw new Error('comments:createPin')
      return data
    },

    async addReply(pinId, input): Promise<CommentMessage> {
      const { data, error } = await api.POST('/c/{slug}/comments/pins/{pin}/replies', {
        params: { path: { slug, pin: pinId } },
        body: input,
        headers: WRITE_HEADERS,
      })
      if (error || !data) throw new Error('comments:addReply')
      return data
    },

    async editMessage(messageId, body): Promise<CommentMessage> {
      const { data, error } = await api.PUT('/c/{slug}/comments/messages/{id}', {
        params: { path: { slug, id: messageId } },
        body: { body },
        headers: WRITE_HEADERS,
      })
      if (error || !data) throw new Error('comments:editMessage')
      return data
    },

    async deleteMessage(messageId): Promise<void> {
      const { error } = await api.DELETE('/c/{slug}/comments/messages/{id}', {
        params: { path: { slug, id: messageId } },
        headers: WRITE_HEADERS,
      })
      if (error) throw new Error('comments:deleteMessage')
    },

    async deletePin(pinId): Promise<void> {
      const { error } = await api.DELETE('/c/{slug}/comments/pins/{pin}', {
        params: { path: { slug, pin: pinId } },
        headers: WRITE_HEADERS,
      })
      if (error) throw new Error('comments:deletePin')
    },
  }
}
```

- [ ] **Step 5: Lancer le test, vérifier le succès**

Run : `pnpm exec vitest run src/comments/data/visitor-adapter.test.ts`
Attendu : PASS (5 tests). NOTE : si openapi-fetch type `headers` strictement et refuse `X-Comment-Client` (header hors-spec), passer l'en-tête via `headers: { ...WRITE_HEADERS } as Record<string, string>` — openapi-fetch accepte les en-têtes additionnels au runtime.

- [ ] **Step 6: Commit**

```bash
git add frontend/src/comments/data/adapter.ts frontend/src/comments/data/visitor-adapter.ts frontend/src/comments/data/visitor-adapter.test.ts
git commit -m "✨ feat(comments): adaptateur de données + impl visiteur (X-Comment-Client)"
```

### Task C2: Hooks React Query (confinés au module)

**Files:**
- Create: `frontend/src/comments/data/use-comments.ts`
- Test: `frontend/src/comments/data/use-comments.test.tsx`

**Interfaces:**
- Consumes: `CommentsAdapter`, `CommentList` (C1).
- Produces (hooks acceptant un `adapter` en paramètre — pas de couplage au transport) :
  - `commentsKey(slug: string): unknown[]` → `['comments', slug]`
  - `useCommentList(slug, adapter): UseQueryResult<CommentList>`
  - `useCreatePin(slug, adapter): UseMutationResult<…>` (invalide la liste au succès)
  - `useAddReply(slug, adapter)`, `useEditMessage(slug, adapter)`, `useDeleteMessage(slug, adapter)`, `useDeletePin(slug, adapter)` (tous invalident la liste au succès)

- [ ] **Step 1: Écrire le test qui échoue**

Create `frontend/src/comments/data/use-comments.test.tsx` :
```tsx
import { describe, expect, it, vi } from 'vitest'
import { type ReactNode } from 'react'
import { renderHook, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { useCommentList, useCreatePin, commentsKey } from './use-comments'
import type { CommentsAdapter } from './adapter'

function fakeAdapter(over: Partial<CommentsAdapter> = {}): CommentsAdapter {
  return {
    capabilities: { canAuthor: true, canEditOwn: true, canModerate: false },
    list: vi.fn().mockResolvedValue({ version: 1, pins: [] }),
    createPin: vi.fn().mockResolvedValue({ id: 1, anchor: '{}', created_at: 'n', messages: [] }),
    addReply: vi.fn(),
    editMessage: vi.fn(),
    deleteMessage: vi.fn(),
    deletePin: vi.fn(),
    ...over,
  }
}

function makeWrapper() {
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  })
  const invalidate = vi.spyOn(qc, 'invalidateQueries')
  function Wrapper({ children }: Readonly<{ children: ReactNode }>) {
    return <QueryClientProvider client={qc}>{children}</QueryClientProvider>
  }
  return { Wrapper, invalidate }
}

describe('use-comments hooks', () => {
  it('useCommentList loads via the adapter', async () => {
    const adapter = fakeAdapter()
    const { Wrapper } = makeWrapper()
    const { result } = renderHook(() => useCommentList('demo', adapter), { wrapper: Wrapper })
    await waitFor(() => expect(result.current.isSuccess).toBe(true))
    expect(result.current.data?.version).toBe(1)
  })

  it('useCreatePin invalidates the comment list on success', async () => {
    const adapter = fakeAdapter()
    const { Wrapper, invalidate } = makeWrapper()
    const { result } = renderHook(() => useCreatePin('demo', adapter), { wrapper: Wrapper })
    result.current.mutate({ anchor: '{}', author_name: 'Léa', body: 'Hi' })
    await waitFor(() => expect(result.current.isSuccess).toBe(true))
    expect(invalidate).toHaveBeenCalledWith({ queryKey: commentsKey('demo') })
  })
})
```

- [ ] **Step 2: Lancer le test, vérifier l'échec**

Run : `pnpm exec vitest run src/comments/data/use-comments.test.tsx`
Attendu : FAIL (module introuvable).

- [ ] **Step 3: Implémenter**

Create `frontend/src/comments/data/use-comments.ts` :
```ts
import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseQueryResult,
} from '@tanstack/react-query'
import type { CommentList, CommentsAdapter } from './adapter'

export function commentsKey(slug: string): unknown[] {
  return ['comments', slug]
}

export function useCommentList(
  slug: string,
  adapter: CommentsAdapter,
): UseQueryResult<CommentList> {
  return useQuery({
    queryKey: commentsKey(slug),
    queryFn: () => adapter.list(),
  })
}

/** Fabrique un hook de mutation qui invalide la liste au succès (DRY). */
function makeMutation<TArgs extends unknown[], TResult>(
  run: (adapter: CommentsAdapter, ...args: TArgs) => Promise<TResult>,
) {
  return (slug: string, adapter: CommentsAdapter) => {
    const qc = useQueryClient()
    return useMutation({
      mutationFn: (args: TArgs) => run(adapter, ...args),
      onSuccess: () => qc.invalidateQueries({ queryKey: commentsKey(slug) }),
    })
  }
}

export const useCreatePin = (slug: string, adapter: CommentsAdapter) =>
  makeMutationCreatePin(slug, adapter)

// Hooks concrets (signatures d'argument explicites pour le typage des appelants).
const makeMutationCreatePin = (slug: string, adapter: CommentsAdapter) => {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (input: { anchor: string; author_name: string; body: string }) =>
      adapter.createPin(input),
    onSuccess: () => qc.invalidateQueries({ queryKey: commentsKey(slug) }),
  })
}

export const useAddReply = (slug: string, adapter: CommentsAdapter) => {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (v: { pinId: number; author_name: string; body: string }) =>
      adapter.addReply(v.pinId, { author_name: v.author_name, body: v.body }),
    onSuccess: () => qc.invalidateQueries({ queryKey: commentsKey(slug) }),
  })
}

export const useEditMessage = (slug: string, adapter: CommentsAdapter) => {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (v: { messageId: number; body: string }) =>
      adapter.editMessage(v.messageId, v.body),
    onSuccess: () => qc.invalidateQueries({ queryKey: commentsKey(slug) }),
  })
}

export const useDeleteMessage = (slug: string, adapter: CommentsAdapter) => {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (messageId: number) => adapter.deleteMessage(messageId),
    onSuccess: () => qc.invalidateQueries({ queryKey: commentsKey(slug) }),
  })
}

export const useDeletePin = (slug: string, adapter: CommentsAdapter) => {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (pinId: number) => adapter.deletePin(pinId),
    onSuccess: () => qc.invalidateQueries({ queryKey: commentsKey(slug) }),
  })
}
```

> NOTE d'implémentation : la fabrique `makeMutation` ci-dessus est illustrative mais
> appelle des hooks hors d'un composant si mal utilisée ; les hooks **concrets**
> (`useCreatePin`, `useAddReply`, …) sont la forme à conserver. Lors de l'implémentation,
> SUPPRIMER `makeMutation` et l'alias `useCreatePin = makeMutationCreatePin`, et garder
> uniquement les hooks concrets nommés (un `useMutation` chacun). Le test ne référence
> que `useCommentList`, `useCreatePin`, `commentsKey`.

- [ ] **Step 4: Nettoyer selon la note (forme finale)**

Éditer `use-comments.ts` pour ne garder que : `commentsKey`, `useCommentList`, puis les 5 hooks concrets `useCreatePin`/`useAddReply`/`useEditMessage`/`useDeleteMessage`/`useDeletePin`, chacun un `useQueryClient()` + `useMutation({ mutationFn, onSuccess: invalidate })`. Retirer `makeMutation` et `makeMutationCreatePin`.

- [ ] **Step 5: Lancer le test, vérifier le succès**

Run : `pnpm exec vitest run src/comments/data/use-comments.test.tsx`
Attendu : PASS (2 tests).

- [ ] **Step 6: Suite data + lint + typecheck**

Run : `pnpm exec vitest run src/comments/data && pnpm lint && pnpm typecheck`
Attendu : PASS.

- [ ] **Step 7: Commit**

```bash
git add frontend/src/comments/data/use-comments.ts frontend/src/comments/data/use-comments.test.tsx
git commit -m "✨ feat(comments): hooks React Query (list + mutations, invalidation liste)"
```

---

## Phase D — UI / overlay

### Task D0: Composant shadcn `Textarea` (manquant)

**Files:**
- Create: `frontend/src/components/ui/textarea.tsx`

**Interfaces:**
- Produces: `Textarea` (forwardRef sur `<textarea>`, classes shadcn stone) importable via `@/components/ui/textarea`.

- [ ] **Step 1: Créer le composant (calqué sur `input.tsx`)**

Create `frontend/src/components/ui/textarea.tsx` :
```tsx
import * as React from 'react'
import { cn } from '@/lib/utils'

function Textarea({ className, ...props }: React.ComponentProps<'textarea'>) {
  return (
    <textarea
      data-slot="textarea"
      className={cn(
        'border-input placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-ring/50 aria-invalid:border-destructive flex min-h-16 w-full rounded-md border bg-transparent px-3 py-2 text-sm shadow-xs outline-none focus-visible:ring-[3px] disabled:cursor-not-allowed disabled:opacity-50',
        className,
      )}
      {...props}
    />
  )
}

export { Textarea }
```

> Vérifier la signature de `cn` : `frontend/src/lib/utils.ts` exporte `cn(...inputs)`
> (clsx + tailwind-merge). Si `input.tsx` n'utilise PAS `data-slot`, aligner le style sur
> `input.tsx` réel (lire le fichier avant d'écrire).

- [ ] **Step 2: Typecheck**

Run : `pnpm typecheck`
Attendu : PASS.

- [ ] **Step 3: Commit**

```bash
git add frontend/src/components/ui/textarea.tsx
git commit -m "✨ feat(ui): composant Textarea (shadcn) pour le corps des commentaires"
```

### Task D1: Machine à états du mode pick (réducteur pur)

**Files:**
- Create: `frontend/src/comments/state/pick-machine.ts`
- Test: `frontend/src/comments/state/pick-machine.test.ts`

**Interfaces:**
- Consumes: `AnchorDescriptor` (A1), `ShellRect` (B1).
- Produces:
  - `type PickState = { mode: 'idle' } | { mode: 'pick' } | { mode: 'compose'; anchor: AnchorDescriptor; rect: ShellRect }`
  - `type PickEvent = { type: 'ENTER_PICK' } | { type: 'CANCEL' } | { type: 'CAPTURE'; anchor: AnchorDescriptor; rect: ShellRect } | { type: 'SUBMITTED' }`
  - `pickReducer(state: PickState, event: PickEvent): PickState`
  - `initialPickState: PickState`

- [ ] **Step 1: Écrire le test qui échoue**

Create `frontend/src/comments/state/pick-machine.test.ts` :
```ts
import { describe, expect, it } from 'vitest'
import { initialPickState, pickReducer } from './pick-machine'
import type { AnchorDescriptor } from '../anchor/descriptor'

const anchor = {
  v: 1,
  selector: 'button',
  fingerprint: { tag: 'button', text: 'x', role: null, ordinal: 0 },
  textQuote: null,
  offset: { x: 0.5, y: 0.5 },
  fallbackPoint: { x: 0, y: 0 },
} satisfies AnchorDescriptor
const rect = { x: 1, y: 2, width: 3, height: 4 }

describe('pickReducer', () => {
  it('starts idle', () => {
    expect(initialPickState).toEqual({ mode: 'idle' })
  })

  it('ENTER_PICK moves idle -> pick', () => {
    expect(pickReducer(initialPickState, { type: 'ENTER_PICK' })).toEqual({ mode: 'pick' })
  })

  it('CAPTURE moves pick -> compose with the anchor and rect', () => {
    const next = pickReducer({ mode: 'pick' }, { type: 'CAPTURE', anchor, rect })
    expect(next).toEqual({ mode: 'compose', anchor, rect })
  })

  it('SUBMITTED returns to idle', () => {
    expect(pickReducer({ mode: 'compose', anchor, rect }, { type: 'SUBMITTED' })).toEqual({
      mode: 'idle',
    })
  })

  it('CANCEL from any mode returns to idle', () => {
    expect(pickReducer({ mode: 'pick' }, { type: 'CANCEL' })).toEqual({ mode: 'idle' })
    expect(pickReducer({ mode: 'compose', anchor, rect }, { type: 'CANCEL' })).toEqual({
      mode: 'idle',
    })
  })

  it('ignores CAPTURE when not in pick mode', () => {
    expect(pickReducer({ mode: 'idle' }, { type: 'CAPTURE', anchor, rect })).toEqual({
      mode: 'idle',
    })
  })
})
```

- [ ] **Step 2: Lancer le test, vérifier l'échec**

Run : `pnpm exec vitest run src/comments/state/pick-machine.test.ts`
Attendu : FAIL (module introuvable).

- [ ] **Step 3: Implémenter**

Create `frontend/src/comments/state/pick-machine.ts` :
```ts
import type { AnchorDescriptor } from '../anchor/descriptor'
import type { ShellRect } from '../picker/picker'

export type PickState =
  | { mode: 'idle' }
  | { mode: 'pick' }
  | { mode: 'compose'; anchor: AnchorDescriptor; rect: ShellRect }

export type PickEvent =
  | { type: 'ENTER_PICK' }
  | { type: 'CANCEL' }
  | { type: 'CAPTURE'; anchor: AnchorDescriptor; rect: ShellRect }
  | { type: 'SUBMITTED' }

export const initialPickState: PickState = { mode: 'idle' }

export function pickReducer(state: PickState, event: PickEvent): PickState {
  switch (event.type) {
    case 'ENTER_PICK':
      return { mode: 'pick' }
    case 'CANCEL':
    case 'SUBMITTED':
      return { mode: 'idle' }
    case 'CAPTURE':
      if (state.mode !== 'pick') return state
      return { mode: 'compose', anchor: event.anchor, rect: event.rect }
    default:
      return state
  }
}
```

- [ ] **Step 4: Lancer le test, vérifier le succès**

Run : `pnpm exec vitest run src/comments/state/pick-machine.test.ts`
Attendu : PASS (6 tests).

- [ ] **Step 5: Commit**

```bash
git add frontend/src/comments/state/pick-machine.ts frontend/src/comments/state/pick-machine.test.ts
git commit -m "✨ feat(comments): machine à états du mode pick (idle/pick/compose)"
```

### Task D2: Helpers de nom pré-rempli (localStorage)

**Files:**
- Create: `frontend/src/comments/ui/name-prompt.ts`
- Test: `frontend/src/comments/ui/name-prompt.test.ts`

**Interfaces:**
- Produces: `getStoredName(): string` (jamais throw, '' si absent ou storage indispo), `setStoredName(name: string): void`. Clé : `latch:comment-name`.

- [ ] **Step 1: Écrire le test qui échoue**

Create `frontend/src/comments/ui/name-prompt.test.ts` :
```ts
import { describe, expect, it, beforeEach } from 'vitest'
import { getStoredName, setStoredName } from './name-prompt'

beforeEach(() => localStorage.clear())

describe('name prompt storage', () => {
  it('returns empty string when nothing stored', () => {
    expect(getStoredName()).toBe('')
  })

  it('persists and reads back a name', () => {
    setStoredName('Léa')
    expect(getStoredName()).toBe('Léa')
  })

  it('trims on write', () => {
    setStoredName('  Léa  ')
    expect(getStoredName()).toBe('Léa')
  })
})
```

- [ ] **Step 2: Lancer le test, vérifier l'échec**

Run : `pnpm exec vitest run src/comments/ui/name-prompt.test.ts`
Attendu : FAIL (module introuvable).

- [ ] **Step 3: Implémenter**

Create `frontend/src/comments/ui/name-prompt.ts` :
```ts
const KEY = 'latch:comment-name'

export function getStoredName(): string {
  try {
    return localStorage.getItem(KEY) ?? ''
  } catch {
    return ''
  }
}

export function setStoredName(name: string): void {
  try {
    localStorage.setItem(KEY, name.trim())
  } catch {
    /* storage indisponible (mode privé) : on ignore, le nom reste en mémoire */
  }
}
```

- [ ] **Step 4: Lancer le test, vérifier le succès**

Run : `pnpm exec vitest run src/comments/ui/name-prompt.test.ts`
Attendu : PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add frontend/src/comments/ui/name-prompt.ts frontend/src/comments/ui/name-prompt.test.ts
git commit -m "✨ feat(comments): nom pré-rempli (localStorage, fail-safe)"
```

### Task D3: Hook `useFollow` (contrôleur → état React)

**Files:**
- Create: `frontend/src/comments/follow/use-follow.ts`
- Test: `frontend/src/comments/follow/use-follow.test.tsx`

**Interfaces:**
- Consumes: `FollowController`, `PinInput`, `PinPosition` (B2) ; `Picker` (B1).
- Produces: `useFollow(picker: Picker | null, pins: PinInput[]): PinPosition[]` — instancie un `FollowController`, `setPins` à chaque changement de `pins`, `start`/`stop` au montage/démontage, renvoie les dernières positions.

- [ ] **Step 1: Écrire le test qui échoue**

Create `frontend/src/comments/follow/use-follow.test.tsx` :
```tsx
import { describe, expect, it, vi } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useFollow } from './use-follow'
import type { Picker, ShellRect } from '../picker/picker'
import type { AnchorDescriptor } from '../anchor/descriptor'

function anchor(): AnchorDescriptor {
  return {
    v: 1,
    selector: '#x',
    fingerprint: { tag: 'div', text: '', role: null, ordinal: 0 },
    textQuote: null,
    offset: { x: 0.5, y: 0.5 },
    fallbackPoint: { x: 0, y: 0 },
  }
}

function fakePicker(): Picker {
  const el = {} as Element
  return {
    doc: document,
    getElementAt: () => null,
    describe: anchor,
    resolve: () => ({ element: el, status: 'anchored' }),
    toShellRect: (): ShellRect => ({ x: 5, y: 6, width: 7, height: 8 }),
    fallbackRect: (): ShellRect => ({ x: 0, y: 0, width: 0, height: 0 }),
    subscribe: () => () => {},
  }
}

describe('useFollow', () => {
  it('returns a position per pin after the synchronous frame', () => {
    // rAF n'existe pas forcément en jsdom ; on le rend synchrone le temps du test.
    const raf = vi
      .spyOn(globalThis, 'requestAnimationFrame')
      .mockImplementation((cb: FrameRequestCallback) => {
        cb(0)
        return 0
      })
    let positions: unknown[] = []
    renderHook(() => {
      positions = useFollow(fakePicker(), [{ id: 1, anchor: anchor() }])
      return null
    })
    act(() => {})
    expect(positions).toHaveLength(1)
    expect(positions[0]).toMatchObject({ id: 1, status: 'anchored', rect: { x: 5 } })
    raf.mockRestore()
  })

  it('returns empty array when picker is null', () => {
    let positions: unknown[] = [{ x: 1 }]
    renderHook(() => {
      positions = useFollow(null, [{ id: 1, anchor: anchor() }])
      return null
    })
    expect(positions).toEqual([])
  })
})
```

- [ ] **Step 2: Lancer le test, vérifier l'échec**

Run : `pnpm exec vitest run src/comments/follow/use-follow.test.tsx`
Attendu : FAIL (module introuvable).

- [ ] **Step 3: Implémenter**

Create `frontend/src/comments/follow/use-follow.ts` :
```ts
import { useEffect, useRef, useState } from 'react'
import { FollowController, type PinInput, type PinPosition } from './controller'
import type { Picker } from '../picker/picker'

export function useFollow(picker: Picker | null, pins: PinInput[]): PinPosition[] {
  const [positions, setPositions] = useState<PinPosition[]>([])
  const ctrlRef = useRef<FollowController | null>(null)

  useEffect(() => {
    if (!picker) {
      setPositions([])
      return
    }
    const ctrl = new FollowController(picker)
    ctrlRef.current = ctrl
    const off = ctrl.onUpdate(setPositions)
    ctrl.start()
    return () => {
      off()
      ctrl.stop()
      ctrlRef.current = null
    }
  }, [picker])

  useEffect(() => {
    ctrlRef.current?.setPins(pins)
  }, [pins])

  return positions
}
```

- [ ] **Step 4: Lancer le test, vérifier le succès**

Run : `pnpm exec vitest run src/comments/follow/use-follow.test.tsx`
Attendu : PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add frontend/src/comments/follow/use-follow.ts frontend/src/comments/follow/use-follow.test.tsx
git commit -m "✨ feat(comments): hook useFollow (FollowController → état React)"
```

### Task D4: `PinBadge` (pastille positionnée)

**Files:**
- Create: `frontend/src/comments/ui/pin-badge.tsx`
- Test: `frontend/src/comments/ui/pin-badge.test.tsx`

**Interfaces:**
- Consumes: `PinPosition` (B2).
- Produces: `PinBadge({ position, count, active, onClick }: { position: PinPosition; count: number; active: boolean; onClick: () => void })` — bouton absolu placé à `rect.x + offset.x*width`, `rect.y + offset.y*height` ; classe d'alerte si `status !== 'anchored'`.

- [ ] **Step 1: Écrire le test qui échoue**

Create `frontend/src/comments/ui/pin-badge.test.tsx` :
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
  it('renders the message count and positions itself', () => {
    render(<PinBadge position={pos} count={3} active={false} onClick={() => {}} />)
    const btn = screen.getByRole('button')
    expect(btn).toHaveTextContent('3')
    // 100 + 0.5*80 = 140 ; 50 + 0.5*40 = 70
    expect(btn.style.left).toBe('140px')
    expect(btn.style.top).toBe('70px')
  })

  it('calls onClick', async () => {
    const onClick = vi.fn()
    render(<PinBadge position={pos} count={1} active={false} onClick={onClick} />)
    await userEvent.click(screen.getByRole('button'))
    expect(onClick).toHaveBeenCalledOnce()
  })

  it('marks a moved (approximate) pin via data-status', () => {
    render(
      <PinBadge position={{ ...pos, status: 'approximate' }} count={1} active={false} onClick={() => {}} />,
    )
    expect(screen.getByRole('button')).toHaveAttribute('data-status', 'approximate')
  })
})
```

- [ ] **Step 2: Lancer le test, vérifier l'échec**

Run : `pnpm exec vitest run src/comments/ui/pin-badge.test.tsx`
Attendu : FAIL (module introuvable).

- [ ] **Step 3: Implémenter**

Create `frontend/src/comments/ui/pin-badge.tsx` :
```tsx
import { cn } from '@/lib/utils'
import type { PinPosition } from '../follow/controller'

interface PinBadgeProps {
  position: PinPosition
  count: number
  active: boolean
  onClick: () => void
}

export function PinBadge({ position, count, active, onClick }: Readonly<PinBadgeProps>) {
  const { rect, offset, status } = position
  const left = rect.x + offset.x * rect.width
  const top = rect.y + offset.y * rect.height
  return (
    <button
      type="button"
      data-status={status}
      onClick={onClick}
      style={{ left: `${left}px`, top: `${top}px`, pointerEvents: 'auto' }}
      className={cn(
        'absolute flex size-7 -translate-x-1/2 -translate-y-1/2 items-center justify-center rounded-full border-2 border-white text-xs font-semibold text-white shadow-md',
        status === 'anchored' ? 'bg-primary' : 'bg-amber-500',
        active && 'ring-primary/40 ring-2',
      )}
    >
      {count}
    </button>
  )
}
```

- [ ] **Step 4: Lancer le test, vérifier le succès**

Run : `pnpm exec vitest run src/comments/ui/pin-badge.test.tsx`
Attendu : PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add frontend/src/comments/ui/pin-badge.tsx frontend/src/comments/ui/pin-badge.test.tsx
git commit -m "✨ feat(comments): PinBadge (pastille positionnée, état approximate/orphaned)"
```

### Task D5: `useFloating` helper + `ComposePopup` (nouveau commentaire)

**Files:**
- Create: `frontend/src/comments/ui/use-floating-rect.ts`
- Create: `frontend/src/comments/ui/compose-popup.tsx`
- Test: `frontend/src/comments/ui/compose-popup.test.tsx`

**Interfaces:**
- Consumes: `ShellRect` (B1), `@floating-ui/dom` (`computePosition`, `offset`, `flip`, `shift`), `Textarea` (D0), `Button` (`@/components/ui/button`), `getStoredName`/`setStoredName` (D2), i18n.
- Produces:
  - `useFloatingRect(rect: ShellRect | null): { ref: RefCallback<HTMLElement>; style: CSSProperties }` — positionne un élément flottant via floating-ui contre un `VirtualElement` dérivé de `rect`.
  - `ComposePopup({ rect, onSubmit, onCancel, submitting }: { rect: ShellRect; onSubmit: (v: { author_name: string; body: string }) => void; onCancel: () => void; submitting: boolean })`.

- [ ] **Step 1: Écrire le helper floating (sans test dédié — couvert via ComposePopup)**

Create `frontend/src/comments/ui/use-floating-rect.ts` :
```ts
import { useLayoutEffect, useRef, useState, type CSSProperties, type RefCallback } from 'react'
import { computePosition, flip, offset, shift } from '@floating-ui/dom'
import type { ShellRect } from '../picker/picker'

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
      middleware: [offset(8), flip(), shift({ padding: 8 })],
    }).then(({ x, y }) => {
      setStyle({ position: 'fixed', left: `${x}px`, top: `${y}px`, pointerEvents: 'auto' })
    })
  }, [rect])

  const ref: RefCallback<HTMLElement> = (node) => {
    elRef.current = node
  }
  return { ref, style }
}
```

- [ ] **Step 2: Écrire le test qui échoue (ComposePopup)**

Create `frontend/src/comments/ui/compose-popup.test.tsx` :
```tsx
import { describe, expect, it, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { I18nextProvider } from 'react-i18next'
import i18n from '@/shell/i18n'
import { ComposePopup } from './compose-popup'

const rect = { x: 10, y: 10, width: 20, height: 20 }

function renderPopup(props: Partial<Parameters<typeof ComposePopup>[0]> = {}) {
  return render(
    <I18nextProvider i18n={i18n}>
      <ComposePopup
        rect={rect}
        submitting={false}
        onSubmit={vi.fn()}
        onCancel={vi.fn()}
        {...props}
      />
    </I18nextProvider>,
  )
}

beforeEach(() => {
  localStorage.clear()
  return i18n.changeLanguage('en')
})

describe('ComposePopup', () => {
  it('blocks submit when name or body is empty', async () => {
    const onSubmit = vi.fn()
    renderPopup({ onSubmit })
    await userEvent.click(screen.getByRole('button', { name: 'Post' }))
    expect(onSubmit).not.toHaveBeenCalled()
    expect(screen.getByText('Please enter your name.')).toBeInTheDocument()
  })

  it('submits name + body and stores the name', async () => {
    const onSubmit = vi.fn()
    renderPopup({ onSubmit })
    await userEvent.type(screen.getByLabelText('Your name'), 'Léa')
    await userEvent.type(screen.getByLabelText('Comment'), 'Looks good')
    await userEvent.click(screen.getByRole('button', { name: 'Post' }))
    expect(onSubmit).toHaveBeenCalledWith({ author_name: 'Léa', body: 'Looks good' })
    expect(localStorage.getItem('latch:comment-name')).toBe('Léa')
  })

  it('pre-fills the name from localStorage', () => {
    localStorage.setItem('latch:comment-name', 'Léa')
    renderPopup()
    expect(screen.getByLabelText('Your name')).toHaveValue('Léa')
  })

  it('calls onCancel', async () => {
    const onCancel = vi.fn()
    renderPopup({ onCancel })
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }))
    expect(onCancel).toHaveBeenCalledOnce()
  })
})
```

- [ ] **Step 3: Lancer le test, vérifier l'échec**

Run : `pnpm exec vitest run src/comments/ui/compose-popup.test.tsx`
Attendu : FAIL (module introuvable).

- [ ] **Step 4: Implémenter ComposePopup**

Create `frontend/src/comments/ui/compose-popup.tsx` :
```tsx
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import type { ShellRect } from '../picker/picker'
import { getStoredName, setStoredName } from './name-prompt'
import { useFloatingRect } from './use-floating-rect'

const MAX_BODY = 2000

interface ComposePopupProps {
  rect: ShellRect
  submitting: boolean
  onSubmit: (v: { author_name: string; body: string }) => void
  onCancel: () => void
}

export function ComposePopup({ rect, submitting, onSubmit, onCancel }: Readonly<ComposePopupProps>) {
  const { t } = useTranslation()
  const { ref, style } = useFloatingRect(rect)
  const [name, setName] = useState(getStoredName())
  const [body, setBody] = useState('')
  const [error, setError] = useState<string | null>(null)

  function submit() {
    const trimmedName = name.trim()
    const trimmedBody = body.trim()
    if (!trimmedName) return setError(t('comment.error.name_required'))
    if (!trimmedBody) return setError(t('comment.error.body_required'))
    if (trimmedBody.length > MAX_BODY) return setError(t('comment.error.body_too_long'))
    setStoredName(trimmedName)
    onSubmit({ author_name: trimmedName, body: trimmedBody })
  }

  return (
    <div
      ref={ref}
      style={style}
      className="bg-background z-[60] w-72 rounded-lg border p-3 shadow-xl"
    >
      <div className="flex flex-col gap-2">
        <Label htmlFor="comment-name">{t('comment.compose.name_label')}</Label>
        <Input
          id="comment-name"
          value={name}
          placeholder={t('comment.compose.name_placeholder')}
          onChange={(e) => setName(e.target.value)}
        />
        <Label htmlFor="comment-body">{t('comment.compose.body_label')}</Label>
        <Textarea
          id="comment-body"
          value={body}
          placeholder={t('comment.compose.body_placeholder')}
          onChange={(e) => setBody(e.target.value)}
        />
        {error && <p className="text-destructive text-xs">{error}</p>}
        <div className="flex justify-end gap-2">
          <Button type="button" variant="ghost" onClick={onCancel}>
            {t('comment.compose.cancel')}
          </Button>
          <Button type="button" loading={submitting} onClick={submit}>
            {t('comment.compose.submit')}
          </Button>
        </div>
      </div>
    </div>
  )
}
```

> VÉRIFIER : `Button` du repo accepte-t-il une prop `loading` ? (le recon a vu
> `<Button loading={…}>` dans `project-form.tsx` → oui). Sinon, désactiver via `disabled={submitting}`.

- [ ] **Step 5: Lancer le test, vérifier le succès**

Run : `pnpm exec vitest run src/comments/ui/compose-popup.test.tsx`
Attendu : PASS (4 tests). NOTE : floating-ui `computePosition` est async ; jsdom n'a pas de layout donc le positionnement retombe sur (0,0) — sans incidence sur les assertions (qui portent sur le formulaire, pas la position). Pas de shim requis.

- [ ] **Step 6: Commit**

```bash
git add frontend/src/comments/ui/use-floating-rect.ts frontend/src/comments/ui/compose-popup.tsx frontend/src/comments/ui/compose-popup.test.tsx
git commit -m "✨ feat(comments): ComposePopup (nouveau commentaire, nom lazy, floating-ui)"
```

### Task D6: `ThreadPopup` (fil : lecture + reply + edit + delete)

**Files:**
- Create: `frontend/src/comments/ui/thread-popup.tsx`
- Test: `frontend/src/comments/ui/thread-popup.test.tsx`

**Interfaces:**
- Consumes: `CommentPin`/`CommentMessage` (C1), `PinPosition` (B2), `useFloatingRect` (D5), `Capabilities` (C1), i18n, `Button`/`Textarea`.
- Produces:
  - `ThreadPopup({ pin, position, capabilities, busy, onReply, onEdit, onDelete, onDeletePin, onClose }: {...})` :
    - `pin: CommentPin`, `position: PinPosition`
    - `capabilities: Capabilities`
    - `onReply(body: string)`, `onEdit(messageId: number, body: string)`, `onDelete(messageId: number)`, `onDeletePin()`, `onClose()`
    - affiche `data-status` warning si `position.status !== 'anchored'`
    - boutons éditer/supprimer rendus **uniquement si `message.editable`** (jamais d'`owner_token`).

- [ ] **Step 1: Écrire le test qui échoue**

Create `frontend/src/comments/ui/thread-popup.test.tsx` :
```tsx
import { describe, expect, it, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { I18nextProvider } from 'react-i18next'
import i18n from '@/shell/i18n'
import { ThreadPopup } from './thread-popup'
import type { CommentPin } from '../data/adapter'
import type { PinPosition } from '../follow/controller'

const pin: CommentPin = {
  id: 7,
  anchor: '{}',
  created_at: 'now',
  messages: [
    { id: 1, author_name: 'Léa', body: 'First', created_at: 'n', updated_at: 'n', editable: true },
    { id: 2, author_name: 'Max', body: 'Reply', created_at: 'n', updated_at: 'n', editable: false },
  ],
}
const position: PinPosition = {
  id: 7,
  status: 'anchored',
  rect: { x: 0, y: 0, width: 10, height: 10 },
  offset: { x: 0.5, y: 0.5 },
}
const caps = { canAuthor: true, canEditOwn: true, canModerate: false }

function renderThread(over: Partial<Parameters<typeof ThreadPopup>[0]> = {}) {
  return render(
    <I18nextProvider i18n={i18n}>
      <ThreadPopup
        pin={pin}
        position={position}
        capabilities={caps}
        busy={false}
        onReply={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onDeletePin={vi.fn()}
        onClose={vi.fn()}
        {...over}
      />
    </I18nextProvider>,
  )
}

beforeEach(() => i18n.changeLanguage('en'))

describe('ThreadPopup', () => {
  it('renders every message body as plain text', () => {
    renderThread()
    expect(screen.getByText('First')).toBeInTheDocument()
    expect(screen.getByText('Reply')).toBeInTheDocument()
    expect(screen.getByText('Léa')).toBeInTheDocument()
  })

  it('shows edit/delete only on editable messages', () => {
    renderThread()
    // 1 message editable -> 1 bouton Edit, 1 bouton Delete (hors delete-pin)
    expect(screen.getAllByRole('button', { name: 'Edit' })).toHaveLength(1)
  })

  it('submits a reply', async () => {
    const onReply = vi.fn()
    renderThread({ onReply })
    await userEvent.type(screen.getByPlaceholderText('Reply…'), 'Nice')
    await userEvent.click(screen.getByRole('button', { name: 'Reply' }))
    expect(onReply).toHaveBeenCalledWith('Nice')
  })

  it('flags a moved pin', () => {
    renderThread({ position: { ...position, status: 'approximate' } })
    expect(screen.getByText('This element may have moved')).toBeInTheDocument()
  })
})
```

- [ ] **Step 2: Lancer le test, vérifier l'échec**

Run : `pnpm exec vitest run src/comments/ui/thread-popup.test.tsx`
Attendu : FAIL (module introuvable).

- [ ] **Step 3: Implémenter**

Create `frontend/src/comments/ui/thread-popup.tsx` :
```tsx
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'
import type { Capabilities, CommentMessage, CommentPin } from '../data/adapter'
import type { PinPosition } from '../follow/controller'
import { useFloatingRect } from './use-floating-rect'

interface ThreadPopupProps {
  pin: CommentPin
  position: PinPosition
  capabilities: Capabilities
  busy: boolean
  onReply: (body: string) => void
  onEdit: (messageId: number, body: string) => void
  onDelete: (messageId: number) => void
  onDeletePin: () => void
  onClose: () => void
}

export function ThreadPopup(props: Readonly<ThreadPopupProps>) {
  const { pin, position, capabilities, busy, onReply, onEdit, onDelete } = props
  const { t } = useTranslation()
  const { ref, style } = useFloatingRect(position.rect)
  const [reply, setReply] = useState('')
  const [editingId, setEditingId] = useState<number | null>(null)
  const [editBody, setEditBody] = useState('')

  function startEdit(m: CommentMessage) {
    setEditingId(m.id)
    setEditBody(m.body)
  }
  function commitEdit() {
    if (editingId !== null && editBody.trim()) onEdit(editingId, editBody.trim())
    setEditingId(null)
  }

  return (
    <div
      ref={ref}
      style={style}
      data-status={position.status}
      className="bg-background z-[60] flex w-80 flex-col gap-3 rounded-lg border p-3 shadow-xl"
    >
      {position.status !== 'anchored' && (
        <p className="text-xs text-amber-600">
          {position.status === 'orphaned'
            ? t('comment.thread.orphaned')
            : t('comment.thread.moved')}
        </p>
      )}
      <ul className="flex max-h-64 flex-col gap-3 overflow-y-auto">
        {pin.messages.map((m) => (
          <li key={m.id} className="flex flex-col gap-1">
            <span className="text-xs font-semibold">{m.author_name}</span>
            {editingId === m.id ? (
              <div className="flex flex-col gap-1">
                <Textarea value={editBody} onChange={(e) => setEditBody(e.target.value)} />
                <div className="flex justify-end gap-2">
                  <Button type="button" variant="ghost" onClick={() => setEditingId(null)}>
                    {t('comment.thread.cancel')}
                  </Button>
                  <Button type="button" loading={busy} onClick={commitEdit}>
                    {t('comment.thread.save')}
                  </Button>
                </div>
              </div>
            ) : (
              <>
                <p className="text-sm whitespace-pre-wrap">{m.body}</p>
                {m.editable && capabilities.canEditOwn && (
                  <div className="flex gap-2">
                    <Button type="button" variant="ghost" size="sm" onClick={() => startEdit(m)}>
                      {t('comment.thread.edit')}
                    </Button>
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      onClick={() => onDelete(m.id)}
                    >
                      {t('comment.thread.delete')}
                    </Button>
                  </div>
                )}
              </>
            )}
          </li>
        ))}
      </ul>
      {capabilities.canAuthor && (
        <div className="flex flex-col gap-1">
          <Textarea
            value={reply}
            placeholder={t('comment.thread.reply_placeholder')}
            onChange={(e) => setReply(e.target.value)}
          />
          <div className="flex justify-end">
            <Button
              type="button"
              loading={busy}
              onClick={() => {
                if (reply.trim()) {
                  onReply(reply.trim())
                  setReply('')
                }
              }}
            >
              {t('comment.thread.reply_submit')}
            </Button>
          </div>
        </div>
      )}
    </div>
  )
}
```

- [ ] **Step 4: Lancer le test, vérifier le succès**

Run : `pnpm exec vitest run src/comments/ui/thread-popup.test.tsx`
Attendu : PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add frontend/src/comments/ui/thread-popup.tsx frontend/src/comments/ui/thread-popup.test.tsx
git commit -m "✨ feat(comments): ThreadPopup (fil, reply/edit/delete gardés par editable)"
```

### Task D7: `OverlayLayer` (surlignage pick + rendu pastilles)

**Files:**
- Create: `frontend/src/comments/ui/overlay-layer.tsx`
- Test: `frontend/src/comments/ui/overlay-layer.test.tsx`

**Interfaces:**
- Consumes: `Picker` (B1), `PinPosition` (B2), `PinBadge` (D4), `pickReducer`/`PickState` (D1).
- Produces:
  - `OverlayLayer({ picker, positions, pickMode, onPick, onPinClick, activePinId }: { picker: Picker; positions: PinPosition[]; pickMode: boolean; onPick: (anchor, rect) => void; onPinClick: (pinId: number) => void; activePinId: number | null })`
  - en `pickMode` : `pointer-events:auto`, `mousemove` → `picker.getElementAt` → surligne le rect (`picker.toShellRect`), `click` → `picker.describe` + `onPick(anchor, rect)`.
  - hors `pickMode` : conteneur `pointer-events:none` ; chaque `PinBadge` réactive `pointer-events:auto`.

- [ ] **Step 1: Écrire le test qui échoue**

Create `frontend/src/comments/ui/overlay-layer.test.tsx` :
```tsx
import { describe, expect, it, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { OverlayLayer } from './overlay-layer'
import type { Picker, ShellRect } from '../picker/picker'
import type { PinPosition } from '../follow/controller'

const anchor = {
  v: 1 as const,
  selector: '#x',
  fingerprint: { tag: 'div', text: '', role: null, ordinal: 0 },
  textQuote: null,
  offset: { x: 0.5, y: 0.5 },
  fallbackPoint: { x: 0, y: 0 },
}

function fakePicker(over: Partial<Picker> = {}): Picker {
  const el = {} as Element
  return {
    doc: document,
    getElementAt: () => el,
    describe: () => anchor,
    resolve: () => ({ element: el, status: 'anchored' }),
    toShellRect: (): ShellRect => ({ x: 1, y: 2, width: 3, height: 4 }),
    fallbackRect: (): ShellRect => ({ x: 0, y: 0, width: 0, height: 0 }),
    subscribe: () => () => {},
    ...over,
  }
}

const positions: PinPosition[] = [
  { id: 5, status: 'anchored', rect: { x: 10, y: 10, width: 20, height: 20 }, offset: { x: 0.5, y: 0.5 } },
]

describe('OverlayLayer', () => {
  it('renders a badge per position', () => {
    render(
      <OverlayLayer
        picker={fakePicker()}
        positions={positions}
        pickMode={false}
        onPick={vi.fn()}
        onPinClick={vi.fn()}
        activePinId={null}
      />,
    )
    expect(screen.getByRole('button')).toBeInTheDocument()
  })

  it('captures an anchor on click in pick mode', () => {
    const onPick = vi.fn()
    const { container } = render(
      <OverlayLayer
        picker={fakePicker()}
        positions={[]}
        pickMode
        onPick={onPick}
        onPinClick={vi.fn()}
        activePinId={null}
      />,
    )
    const surface = container.querySelector('[data-testid="pick-surface"]')!
    fireEvent.click(surface, { clientX: 50, clientY: 60 })
    expect(onPick).toHaveBeenCalledWith(anchor, { x: 1, y: 2, width: 3, height: 4 })
  })

  it('forwards pin clicks', async () => {
    const onPinClick = vi.fn()
    render(
      <OverlayLayer
        picker={fakePicker()}
        positions={positions}
        pickMode={false}
        onPick={vi.fn()}
        onPinClick={onPinClick}
        activePinId={null}
      />,
    )
    fireEvent.click(screen.getByRole('button'))
    expect(onPinClick).toHaveBeenCalledWith(5)
  })
})
```

- [ ] **Step 2: Lancer le test, vérifier l'échec**

Run : `pnpm exec vitest run src/comments/ui/overlay-layer.test.tsx`
Attendu : FAIL (module introuvable).

- [ ] **Step 3: Implémenter**

Create `frontend/src/comments/ui/overlay-layer.tsx` :
```tsx
import { useState, type MouseEvent } from 'react'
import type { AnchorDescriptor } from '../anchor/descriptor'
import type { Picker, ShellRect } from '../picker/picker'
import type { PinPosition } from '../follow/controller'
import { PinBadge } from './pin-badge'

interface OverlayLayerProps {
  picker: Picker
  positions: PinPosition[]
  pickMode: boolean
  onPick: (anchor: AnchorDescriptor, rect: ShellRect) => void
  onPinClick: (pinId: number) => void
  activePinId: number | null
}

export function OverlayLayer({
  picker,
  positions,
  pickMode,
  onPick,
  onPinClick,
  activePinId,
}: Readonly<OverlayLayerProps>) {
  const [hover, setHover] = useState<ShellRect | null>(null)

  function onMove(e: MouseEvent) {
    if (!pickMode) return
    const el = picker.getElementAt(e.clientX, e.clientY)
    setHover(el ? picker.toShellRect(el) : null)
  }

  function onClick(e: MouseEvent) {
    if (!pickMode) return
    const el = picker.getElementAt(e.clientX, e.clientY)
    if (!el) return
    const rect = el.getBoundingClientRect()
    const shellRect = picker.toShellRect(el)
    if (!shellRect) return
    const anchor = picker.describe(el, { x: e.clientX - rect.left, y: e.clientY - rect.top })
    onPick(anchor, shellRect)
  }

  return (
    <div
      className="absolute inset-0 z-50"
      style={{ pointerEvents: pickMode ? 'auto' : 'none' }}
    >
      {pickMode && (
        <div
          data-testid="pick-surface"
          className="absolute inset-0 cursor-crosshair"
          onMouseMove={onMove}
          onClick={onClick}
        />
      )}
      {pickMode && hover && (
        <div
          className="border-primary pointer-events-none absolute rounded-sm border-2"
          style={{
            left: `${hover.x}px`,
            top: `${hover.y}px`,
            width: `${hover.width}px`,
            height: `${hover.height}px`,
          }}
        />
      )}
      {positions.map((p) => (
        <PinBadge
          key={p.id}
          position={p}
          count={1}
          active={p.id === activePinId}
          onClick={() => onPinClick(p.id)}
        />
      ))}
    </div>
  )
}
```

> NOTE : `count={1}` est un placeholder de comptage ; le vrai nombre de messages est
> injecté par `comments-app` (Task D9) qui connaît les pins. Voir D9 : OverlayLayer y
> reçoit `positions` enrichies OU `comments-app` mappe `position.id` → `pin.messages.length`.
> Pour rester simple, D9 passera le compte via une variante : remplacer `count={1}` par
> une prop `countOf: (pinId: number) => number` ajoutée à OverlayLayer. AJOUTER cette prop
> maintenant : `countOf?: (pinId: number) => number` (défaut `() => 1`), et utiliser
> `count={(countOf ?? (() => 1))(p.id)}`.

- [ ] **Step 4: Intégrer la prop `countOf`**

Éditer `overlay-layer.tsx` : ajouter `countOf?: (pinId: number) => number` à `OverlayLayerProps`, et rendre `<PinBadge … count={countOf ? countOf(p.id) : 1} … />`. (Le test existant ne passe pas `countOf` → défaut 1, reste vert.)

- [ ] **Step 5: Lancer le test, vérifier le succès**

Run : `pnpm exec vitest run src/comments/ui/overlay-layer.test.tsx`
Attendu : PASS (3 tests).

- [ ] **Step 6: Commit**

```bash
git add frontend/src/comments/ui/overlay-layer.tsx frontend/src/comments/ui/overlay-layer.test.tsx
git commit -m "✨ feat(comments): OverlayLayer (surlignage pick, rendu pastilles, clics)"
```

### Task D8: `ActionBar` (barre flottante 3 boutons)

**Files:**
- Create: `frontend/src/comments/ui/action-bar.tsx`
- Test: `frontend/src/comments/ui/action-bar.test.tsx`

**Interfaces:**
- Consumes: `Button`, i18n, `Capabilities` (C1).
- Produces: `ActionBar({ capabilities, pinCount, pickActive, pinsVisible, onTogglePick, onToggleVisible, onOpenList }: {...})` — 3 boutons : ✏️ (masqué si `!canAuthor`), 👁️ + compteur, 💬 liste.

- [ ] **Step 1: Écrire le test qui échoue**

Create `frontend/src/comments/ui/action-bar.test.tsx` :
```tsx
import { describe, expect, it, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { I18nextProvider } from 'react-i18next'
import i18n from '@/shell/i18n'
import { ActionBar } from './action-bar'

const caps = { canAuthor: true, canEditOwn: true, canModerate: false }

function renderBar(over: Partial<Parameters<typeof ActionBar>[0]> = {}) {
  return render(
    <I18nextProvider i18n={i18n}>
      <ActionBar
        capabilities={caps}
        pinCount={2}
        pickActive={false}
        pinsVisible
        onTogglePick={vi.fn()}
        onToggleVisible={vi.fn()}
        onOpenList={vi.fn()}
        {...over}
      />
    </I18nextProvider>,
  )
}

beforeEach(() => i18n.changeLanguage('en'))

describe('ActionBar', () => {
  it('shows the comment count', () => {
    renderBar()
    expect(screen.getByText('2 comments')).toBeInTheDocument()
  })

  it('triggers pick mode toggle', async () => {
    const onTogglePick = vi.fn()
    renderBar({ onTogglePick })
    await userEvent.click(screen.getByRole('button', { name: 'Comment' }))
    expect(onTogglePick).toHaveBeenCalledOnce()
  })

  it('hides the pick button when canAuthor is false', () => {
    renderBar({ capabilities: { ...caps, canAuthor: false } })
    expect(screen.queryByRole('button', { name: 'Comment' })).not.toBeInTheDocument()
  })
})
```

- [ ] **Step 2: Lancer le test, vérifier l'échec**

Run : `pnpm exec vitest run src/comments/ui/action-bar.test.tsx`
Attendu : FAIL (module introuvable).

- [ ] **Step 3: Implémenter**

Create `frontend/src/comments/ui/action-bar.tsx` :
```tsx
import { useTranslation } from 'react-i18next'
import { MessageSquarePlus, Eye, List } from 'lucide-react'
import { Button } from '@/components/ui/button'
import type { Capabilities } from '../data/adapter'

interface ActionBarProps {
  capabilities: Capabilities
  pinCount: number
  pickActive: boolean
  pinsVisible: boolean
  onTogglePick: () => void
  onToggleVisible: () => void
  onOpenList: () => void
}

export function ActionBar({
  capabilities,
  pinCount,
  pickActive,
  pinsVisible,
  onTogglePick,
  onToggleVisible,
  onOpenList,
}: Readonly<ActionBarProps>) {
  const { t } = useTranslation()
  return (
    <div className="bg-background fixed bottom-4 left-1/2 z-[55] flex -translate-x-1/2 items-center gap-1 rounded-full border p-1 shadow-lg">
      {capabilities.canAuthor && (
        <Button
          type="button"
          variant={pickActive ? 'default' : 'ghost'}
          size="sm"
          onClick={onTogglePick}
        >
          <MessageSquarePlus className="size-4" />
          {t('comment.bar.pick')}
        </Button>
      )}
      <Button
        type="button"
        variant={pinsVisible ? 'default' : 'ghost'}
        size="sm"
        onClick={onToggleVisible}
      >
        <Eye className="size-4" />
        {t('comment.bar.count', { count: pinCount })}
      </Button>
      <Button type="button" variant="ghost" size="sm" onClick={onOpenList}>
        <List className="size-4" />
        {t('comment.bar.list')}
      </Button>
    </div>
  )
}
```

> VÉRIFIER les noms d'icônes `lucide-react` (`MessageSquarePlus`, `Eye`, `List`) — la
> version épinglée est `lucide-react ^1.21.0` (inhabituelle ; lire un import existant dans
> `src/` pour confirmer le style d'import et qu'aucune icône n'a été renommée).

- [ ] **Step 4: Lancer le test, vérifier le succès**

Run : `pnpm exec vitest run src/comments/ui/action-bar.test.tsx`
Attendu : PASS (3 tests). `comment.bar.count` avec `count:2` doit résoudre le pluriel EN → `comment.bar.count_plural` = "2 comments".

- [ ] **Step 5: Commit**

```bash
git add frontend/src/comments/ui/action-bar.tsx frontend/src/comments/ui/action-bar.test.tsx
git commit -m "✨ feat(comments): ActionBar (3 boutons, compteur, gated par canAuthor)"
```

### Task D9: `comments-app` (racine du module) + export

**Files:**
- Create: `frontend/src/comments/comments-app.tsx`
- Modify: `frontend/src/comments/index.ts`
- Test: `frontend/src/comments/comments-app.test.tsx`

**Interfaces:**
- Consumes: tout le module (`SameOriginPicker`, `useFollow`, `useCommentList` + mutations, `pickReducer`, `OverlayLayer`, `ComposePopup`, `ThreadPopup`, `ActionBar`, `createVisitorAdapter`).
- Produces:
  - `CommentsApp({ slug, frame }: { slug: string; frame: FrameRef })` — composant racine, crée son `QueryClient`, monte tout l'UX. Exporté **par défaut** depuis `index.ts` pour le `React.lazy` du shell.
  - `index.ts` ré-exporte `export { CommentsApp as default } from './comments-app'`.

- [ ] **Step 1: Écrire le test qui échoue (rendu + chargement liste)**

Create `frontend/src/comments/comments-app.test.tsx` :
```tsx
import { describe, expect, it, beforeEach } from 'vitest'
import { http, HttpResponse } from 'msw'
import { render, screen, waitFor } from '@testing-library/react'
import { I18nextProvider } from 'react-i18next'
import i18n from '@/shell/i18n'
import { server } from '@/test/msw'
import { CommentsApp } from './comments-app'
import type { FrameRef } from './picker/picker'

const ORIGIN = globalThis.location.origin
const SLUG = 'demo-aB3dEf9z'

function fakeFrame(): FrameRef {
  const doc = document.implementation.createHTMLDocument('proto')
  doc.body.innerHTML = '<button id="b">Hi</button>'
  return {
    contentDocument: doc,
    contentWindow: { addEventListener() {}, removeEventListener() {} } as unknown as Window,
    getBoundingClientRect: () => ({ left: 0, top: 0, width: 800, height: 600 }) as DOMRect,
  }
}

beforeEach(() => i18n.changeLanguage('en'))

describe('CommentsApp', () => {
  it('renders the action bar with the loaded pin count', async () => {
    server.use(
      http.get(`${ORIGIN}/c/${SLUG}/comments`, () =>
        HttpResponse.json(
          {
            version: 1,
            pins: [
              {
                id: 1,
                anchor: JSON.stringify({
                  v: 1,
                  selector: '#b',
                  fingerprint: { tag: 'button', text: 'Hi', role: 'button', ordinal: 0 },
                  textQuote: null,
                  offset: { x: 0.5, y: 0.5 },
                  fallbackPoint: { x: 0, y: 0 },
                }),
                created_at: 'n',
                messages: [
                  { id: 9, author_name: 'Léa', body: 'Hi', created_at: 'n', updated_at: 'n', editable: true },
                ],
              },
            ],
          },
          { status: 200 },
        ),
      ),
    )
    render(
      <I18nextProvider i18n={i18n}>
        <CommentsApp slug={SLUG} frame={fakeFrame()} />
      </I18nextProvider>,
    )
    expect(await screen.findByText('1 comment')).toBeInTheDocument()
  })
})
```

- [ ] **Step 2: Lancer le test, vérifier l'échec**

Run : `pnpm exec vitest run src/comments/comments-app.test.tsx`
Attendu : FAIL (module introuvable).

- [ ] **Step 3: Implémenter `comments-app.tsx`**

Create `frontend/src/comments/comments-app.tsx` :
```tsx
import { useMemo, useReducer, useState } from 'react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { SameOriginPicker } from './picker/same-origin-picker'
import type { FrameRef } from './picker/picker'
import type { AnchorDescriptor } from './anchor/descriptor'
import { parseAnchor } from './anchor/descriptor'
import type { ShellRect } from './picker/picker'
import { useFollow } from './follow/use-follow'
import type { PinInput } from './follow/controller'
import { createVisitorAdapter } from './data/visitor-adapter'
import {
  useCommentList,
  useCreatePin,
  useAddReply,
  useEditMessage,
  useDeleteMessage,
  useDeletePin,
} from './data/use-comments'
import { initialPickState, pickReducer } from './state/pick-machine'
import { OverlayLayer } from './ui/overlay-layer'
import { ComposePopup } from './ui/compose-popup'
import { ThreadPopup } from './ui/thread-popup'
import { ActionBar } from './ui/action-bar'
import { serializeAnchor } from './anchor/descriptor'

interface CommentsAppProps {
  slug: string
  frame: FrameRef
}

/** Composant interne : suppose le QueryClientProvider déjà monté. */
function CommentsInner({ slug, frame }: Readonly<CommentsAppProps>) {
  const picker = useMemo(() => new SameOriginPicker(frame), [frame])
  const adapter = useMemo(() => createVisitorAdapter(slug), [slug])

  const list = useCommentList(slug, adapter)
  const createPin = useCreatePin(slug, adapter)
  const addReply = useAddReply(slug, adapter)
  const editMessage = useEditMessage(slug, adapter)
  const deleteMessage = useDeleteMessage(slug, adapter)
  const deletePin = useDeletePin(slug, adapter)

  const pins = list.data?.pins ?? []
  const pinInputs: PinInput[] = useMemo(
    () =>
      pins
        .map((p) => {
          const anchor = parseAnchor(p.anchor)
          return anchor ? { id: p.id, anchor } : null
        })
        .filter((x): x is PinInput => x !== null),
    [pins],
  )

  const positions = useFollow(picker, pinInputs)
  const [pick, dispatch] = useReducer(pickReducer, initialPickState)
  const [pinsVisible, setPinsVisible] = useState(true)
  const [activePinId, setActivePinId] = useState<number | null>(null)

  const activePin = pins.find((p) => p.id === activePinId) ?? null
  const activePosition = positions.find((p) => p.id === activePinId) ?? null

  function onPick(anchor: AnchorDescriptor, rect: ShellRect) {
    dispatch({ type: 'CAPTURE', anchor, rect })
  }

  function submitNewComment(v: { author_name: string; body: string }) {
    if (pick.mode !== 'compose') return
    createPin.mutate(
      { anchor: serializeAnchor(pick.anchor), author_name: v.author_name, body: v.body },
      { onSuccess: () => dispatch({ type: 'SUBMITTED' }) },
    )
  }

  return (
    <>
      <OverlayLayer
        picker={picker}
        positions={pinsVisible ? positions : []}
        pickMode={pick.mode === 'pick'}
        onPick={onPick}
        onPinClick={setActivePinId}
        activePinId={activePinId}
        countOf={(id) => pins.find((p) => p.id === id)?.messages.length ?? 1}
      />
      {pick.mode === 'compose' && (
        <ComposePopup
          rect={pick.rect}
          submitting={createPin.isPending}
          onSubmit={submitNewComment}
          onCancel={() => dispatch({ type: 'CANCEL' })}
        />
      )}
      {activePin && activePosition && (
        <ThreadPopup
          pin={activePin}
          position={activePosition}
          capabilities={adapter.capabilities}
          busy={addReply.isPending || editMessage.isPending || deleteMessage.isPending}
          onReply={(body) =>
            addReply.mutate({ pinId: activePin.id, author_name: lastAuthor(activePin), body })
          }
          onEdit={(messageId, body) => editMessage.mutate({ messageId, body })}
          onDelete={(messageId) => deleteMessage.mutate(messageId)}
          onDeletePin={() => {
            deletePin.mutate(activePin.id)
            setActivePinId(null)
          }}
          onClose={() => setActivePinId(null)}
        />
      )}
      <ActionBar
        capabilities={adapter.capabilities}
        pinCount={pins.length}
        pickActive={pick.mode === 'pick'}
        pinsVisible={pinsVisible}
        onTogglePick={() =>
          dispatch(pick.mode === 'pick' ? { type: 'CANCEL' } : { type: 'ENTER_PICK' })
        }
        onToggleVisible={() => setPinsVisible((v) => !v)}
        onOpenList={() => setPinsVisible(true)}
      />
    </>
  )
}

/** Nom d'auteur pour une réponse : réutilise le nom du dernier message du fil. */
function lastAuthor(pin: { messages: { author_name: string }[] }): string {
  return pin.messages.at(-1)?.author_name ?? ''
}

export function CommentsApp(props: Readonly<CommentsAppProps>) {
  const client = useMemo(
    () => new QueryClient({ defaultOptions: { queries: { retry: false } } }),
    [],
  )
  return (
    <QueryClientProvider client={client}>
      <CommentsInner {...props} />
    </QueryClientProvider>
  )
}
```

> NOTE produit : `lastAuthor` réutilise le dernier nom du fil pour les replies (v1 privée,
> un seul auteur par fil). Si le fil est vide (ne devrait pas arriver), le nom est vide et
> le backend renverra 422 — acceptable, mais préférer pré-remplir depuis `getStoredName()`.
> Lors de l'implémentation, remplacer `lastAuthor(activePin)` par `getStoredName() || lastAuthor(activePin)`.

- [ ] **Step 4: Mettre à jour l'export du module**

Remplacer le contenu de `frontend/src/comments/index.ts` par :
```ts
export { CommentsApp as default } from './comments-app'
```

- [ ] **Step 5: Lancer le test, vérifier le succès**

Run : `pnpm exec vitest run src/comments/comments-app.test.tsx`
Attendu : PASS (1 test). Le compteur « 1 comment » prouve que la liste est chargée, parsée et comptée.

- [ ] **Step 6: Suite complète du module + lint + typecheck**

Run : `pnpm exec vitest run src/comments && pnpm lint && pnpm typecheck`
Attendu : PASS (tous les fichiers du module).

- [ ] **Step 7: Commit**

```bash
git add frontend/src/comments/comments-app.tsx frontend/src/comments/comments-app.test.tsx frontend/src/comments/index.ts
git commit -m "✨ feat(comments): comments-app (racine du module, QueryClient confiné) + export lazy"
```

---

## Phase E — Intégration dans le shell visiteur

### Task E1: Shell lit `PublicMeta` + monte le module en lazy

**Files:**
- Create: `frontend/src/shell/comments-mount.tsx`
- Modify: `frontend/src/shell/shell-page.tsx`
- Test: `frontend/src/shell/comments-mount.test.tsx`
- Test: `frontend/src/shell/shell-page.test.tsx` (créer si absent)

**Interfaces:**
- Consumes: `CommentsApp` (export par défaut de `@/comments`, D9) ; `FrameRef` satisfait par `HTMLIFrameElement`.
- Produces:
  - `CommentsMount({ slug, frame }: { slug: string; frame: HTMLIFrameElement })` — `React.lazy(() => import('@/comments'))` sous `Suspense`, ré-instancie sur l'événement `load` de l'iframe (changement de scène/navigation interne).
  - `shell-page.tsx` : fetch `GET /api/public/{slug}` → `comments_enabled` ; quand `true` ET iframe montée, rend `<CommentsMount …>`.

- [ ] **Step 1: Écrire le test du mount (échoue)**

Create `frontend/src/shell/comments-mount.test.tsx` :
```tsx
import { describe, expect, it, beforeEach } from 'vitest'
import { http, HttpResponse } from 'msw'
import { render, screen } from '@testing-library/react'
import { I18nextProvider } from 'react-i18next'
import i18n from './i18n'
import { server } from '@/test/msw'
import { CommentsMount } from './comments-mount'

const ORIGIN = globalThis.location.origin
const SLUG = 'demo-aB3dEf9z'

beforeEach(() => {
  i18n.changeLanguage('en')
  server.use(
    http.get(`${ORIGIN}/c/${SLUG}/comments`, () =>
      HttpResponse.json({ version: 1, pins: [] }, { status: 200 }),
    ),
  )
})

describe('CommentsMount', () => {
  it('lazy-loads the comments module and renders its action bar', async () => {
    const iframe = document.createElement('iframe')
    document.body.appendChild(iframe)
    render(
      <I18nextProvider i18n={i18n}>
        <CommentsMount slug={SLUG} frame={iframe} />
      </I18nextProvider>,
    )
    // le chunk se charge async (Suspense) ; la barre apparaît une fois la liste chargée
    expect(await screen.findByText('0 comments')).toBeInTheDocument()
  })
})
```

- [ ] **Step 2: Lancer, vérifier l'échec**

Run : `pnpm exec vitest run src/shell/comments-mount.test.tsx`
Attendu : FAIL (module introuvable).

- [ ] **Step 3: Implémenter `comments-mount.tsx`**

Create `frontend/src/shell/comments-mount.tsx` :
```tsx
import { lazy, Suspense, useEffect, useState } from 'react'

const CommentsApp = lazy(() => import('@/comments'))

interface CommentsMountProps {
  slug: string
  frame: HTMLIFrameElement
}

/**
 * Monte la couche commentaire en lazy (chunk Vite séparé du bundle shell).
 * Se ré-instancie au `load` de l'iframe : nouvelle scène/navigation interne du proto
 * → on reconstruit picker + contrôleur sur le DOM courant.
 */
export function CommentsMount({ slug, frame }: Readonly<CommentsMountProps>) {
  const [reloadKey, setReloadKey] = useState(0)

  useEffect(() => {
    const bump = () => setReloadKey((k) => k + 1)
    frame.addEventListener('load', bump)
    return () => frame.removeEventListener('load', bump)
  }, [frame])

  return (
    <div data-testid="comments-mount">
      <Suspense fallback={null}>
        <CommentsApp key={reloadKey} slug={slug} frame={frame} />
      </Suspense>
    </div>
  )
}
```

- [ ] **Step 4: Modifier `shell-page.tsx`**

Dans `frontend/src/shell/shell-page.tsx` :

a) Ajouter l'import en tête : `import { CommentsMount } from './comments-mount'`.

b) Dans `ShellPage`, après `const [notes, setNotes] = useState<Notes | null>(null)`, ajouter :
```tsx
const [iframeEl, setIframeEl] = useState<HTMLIFrameElement | null>(null)
const [commentsEnabled, setCommentsEnabled] = useState(false)

useEffect(() => {
  let cancelled = false
  fetch(`/api/public/${slug}`)
    .then((res) => (res.ok ? res.json() : null))
    .then((meta: { comments_enabled?: boolean } | null) => {
      if (!cancelled && meta?.comments_enabled) setCommentsEnabled(true)
    })
    .catch(() => {
      /* best-effort : un échec meta ne doit jamais masquer le proto */
    })
  return () => {
    cancelled = true
  }
}, [slug])
```

c) Remplacer la balise `<iframe … />` par la version avec callback ref :
```tsx
<iframe
  title="prototype"
  src={`/c/${slug}/raw`}
  ref={setIframeEl}
  className="h-full w-full border-0"
/>
```

d) Juste avant la fermeture `</div>` racine (après le bloc `{notes && …}`), ajouter :
```tsx
{commentsEnabled && iframeEl && <CommentsMount slug={slug} frame={iframeEl} />}
```

- [ ] **Step 5: Écrire/compléter le test de `shell-page`**

Create (ou compléter) `frontend/src/shell/shell-page.test.tsx` :
```tsx
import { describe, expect, it, beforeEach } from 'vitest'
import { http, HttpResponse } from 'msw'
import { render, screen, waitFor } from '@testing-library/react'
import { I18nextProvider } from 'react-i18next'
import i18n from './i18n'
import { server } from '@/test/msw'
import { ShellPage } from './shell-page'

const ORIGIN = globalThis.location.origin

beforeEach(() => {
  i18n.changeLanguage('en')
  // slug courant dérivé du pathname : on force /c/<slug>
  window.history.pushState({}, '', '/c/demo-aB3dEf9z')
  server.use(
    http.get(`${ORIGIN}/c/demo-aB3dEf9z/notes`, () => new HttpResponse(null, { status: 204 })),
    http.get(`${ORIGIN}/c/demo-aB3dEf9z/comments`, () =>
      HttpResponse.json({ version: 1, pins: [] }, { status: 200 }),
    ),
  )
})

describe('ShellPage — comments gating', () => {
  it('mounts the comments layer when comments_enabled is true', async () => {
    server.use(
      http.get(`${ORIGIN}/api/public/demo-aB3dEf9z`, () =>
        HttpResponse.json({ code_enabled: false, comments_enabled: true }, { status: 200 }),
      ),
    )
    render(
      <I18nextProvider i18n={i18n}>
        <ShellPage />
      </I18nextProvider>,
    )
    expect(await screen.findByTestId('comments-mount')).toBeInTheDocument()
  })

  it('does NOT mount the comments layer when comments_enabled is false', async () => {
    server.use(
      http.get(`${ORIGIN}/api/public/demo-aB3dEf9z`, () =>
        HttpResponse.json({ code_enabled: false, comments_enabled: false }, { status: 200 }),
      ),
    )
    render(
      <I18nextProvider i18n={i18n}>
        <ShellPage />
      </I18nextProvider>,
    )
    // laisser les effets se résoudre
    await waitFor(() => expect(screen.getByTitle('prototype')).toBeInTheDocument())
    expect(screen.queryByTestId('comments-mount')).not.toBeInTheDocument()
  })
})
```

- [ ] **Step 6: Lancer les tests shell, vérifier le succès**

Run : `pnpm exec vitest run src/shell`
Attendu : PASS. NOTE : le callback ref `setIframeEl` se déclenche au montage (RTL), donc `iframeEl` est non-null sans dépendre de l'événement `load`. La readiness du `contentDocument` est gérée par le picker (lecture à l'appel) + le re-mount sur `load`.

- [ ] **Step 7: Suite complète + lint + typecheck**

Run : `pnpm test && pnpm lint && pnpm typecheck`
Attendu : PASS (toute la suite frontend).

- [ ] **Step 8: Commit**

```bash
git add frontend/src/shell/comments-mount.tsx frontend/src/shell/comments-mount.test.tsx frontend/src/shell/shell-page.tsx frontend/src/shell/shell-page.test.tsx
git commit -m "✨ feat(comments): shell lit PublicMeta + monte la couche commentaire en lazy"
```

---

## Phase F — e2e Playwright (desktop) + clôture

### Task F0: Recon du harnais e2e existant

> Le plan ne peut pas figer le seeding e2e sans connaître le harnais. Cette étape produit
> les faits nécessaires à F1 (pas un placeholder : livrable = notes concrètes).

- [ ] **Step 1: Lire la config et un spec existant**

Lire : `frontend/playwright.config.ts` (ou racine), et au moins un spec sous `frontend/e2e/*.spec.ts`.
Relever : (a) `baseURL` et comment l'app+backend sont lancés (webServer ?) ; (b) comment un **projet de test est seedé** (déploiement via MCP/admin API ? fixture SQLite ? helper `beforeAll` ?) ; (c) comment on déverrouille un projet à code ; (d) les sélecteurs/`data-testid` conventionnels.

- [ ] **Step 2: Consigner les faits**

Écrire ces faits en commentaire en tête de `frontend/e2e/comments.spec.ts` (créé en F1) — ils guident l'implémentation et la revue.

### Task F1: Parcours visiteur desktop

**Files:**
- Create: `frontend/e2e/comments.spec.ts`

**Interfaces:**
- Consumes: le harnais e2e (F0) ; `data-testid="comments-mount"`, `data-testid="pick-surface"`, la barre d'action.

- [ ] **Step 1: Écrire le spec (parcours nominal)**

Create `frontend/e2e/comments.spec.ts` — structure (adapter le seeding aux faits de F0) :
```ts
import { test, expect } from '@playwright/test'
import { seedFreeProjectWithProto } from './helpers' // ← nom réel relevé en F0

test.describe('Commentaires visiteur (projet libre)', () => {
  let slug: string

  test.beforeAll(async () => {
    // Seed d'un projet LIBRE (comments_enabled = code_enabled : activer comments).
    // Le proto contient un élément cible stable, ex. <button id="cta">En savoir plus</button>.
    slug = await seedFreeProjectWithProto({
      html: '<html><body><button id="cta">En savoir plus</button></body></html>',
      commentsEnabled: true,
    })
  })

  test('cibler → écrire → pin apparaît → persiste après reload', async ({ page }) => {
    await page.goto(`/c/${slug}`)
    await expect(page.getByTestId('comments-mount')).toBeVisible()

    // Entrer en mode commentaire
    await page.getByRole('button', { name: /Comment|Commenter/ }).click()

    // Cibler l'élément dans l'iframe : cliquer sur la surface de pick au-dessus du bouton.
    const frame = page.frameLocator('iframe[title="prototype"]')
    const target = frame.locator('#cta')
    const box = await target.boundingBox()
    expect(box).not.toBeNull()
    await page.mouse.click(box!.x + box!.width / 2, box!.y + box!.height / 2)

    // Composer
    await page.getByLabel(/Your name|Votre nom/).fill('Léa')
    await page.getByLabel(/^Comment$|^Commentaire$/).fill('À revoir')
    await page.getByRole('button', { name: /Post|Publier/ }).click()

    // La pastille apparaît
    const badge = page.locator('[data-status="anchored"]')
    await expect(badge.first()).toBeVisible()

    // Persistance : reload → la pastille est toujours là
    await page.reload()
    await expect(page.locator('[data-status]').first()).toBeVisible()
  })
})
```

- [ ] **Step 2: Lancer le spec**

Run (depuis `frontend/`) : `pnpm exec playwright test e2e/comments.spec.ts`
Attendu : PASS. Si le clic ne capture pas (surface de pick vs iframe), ajuster l'ordre des coordonnées (la surface de pick est dans l'espace shell ; `boundingBox()` du locator iframe est déjà en coords page). Itérer sur le ciblage jusqu'au vert.

- [ ] **Step 3: Commit**

```bash
git add frontend/e2e/comments.spec.ts
git commit -m "✅ test(comments): e2e visiteur desktop (cibler/écrire/persister)"
```

### Task F2: Gate finale + mémoire projet

**Files:**
- Modify: `docs/HANDOFF.md`, `docs/INDEX.md`
- Modify (si pièges découverts) : `docs/QUIRKS.md`, `docs/CONVENTIONS.md`

- [ ] **Step 1: Gate complète**

Run (depuis `frontend/`) :
```bash
pnpm lint && pnpm typecheck && pnpm test && pnpm exec playwright test
```
Attendu : tout vert. Si la gate Sonar locale est dispo (cf. `docs/ENVIRONMENT.md §Scan local`), vérifier `new_coverage ≥ 80 %` sur le code neuf.

- [ ] **Step 2: Mettre à jour `docs/INDEX.md`**

Ajouter sous une section Frontend une ligne par livrable, ex. :
`- [x] Couche commentaires visiteur (module \`src/comments/\` + montage shell lazy) — Plan 2 commentaires — 2026-06-30`

- [ ] **Step 3: Mettre à jour `docs/HANDOFF.md`**

Ajouter une entrée datée en haut : Plan 2 (frontend visiteur) LIVRÉ ; **Plan 3 (admin Review §10 + toggle `ProjectForm` §10.1 + docs publiques §13) reste à faire** ; pièges rencontrés (shims jsdom Observers, transposition iframe, lazy chunk).

- [ ] **Step 4: Mettre à jour `docs/QUIRKS.md` + `docs/CONVENTIONS.md` si pertinent**

QUIRKS : shim `IntersectionObserver` requis ; `getBoundingClientRect` = 0 en jsdom (stubber par test) ; floating-ui retombe en (0,0) sans layout. CONVENTIONS : pattern « seam `Picker` + adaptateur + capabilities », hooks React Query paramétrés par `adapter`, module lazy avec QueryClient propre.

- [ ] **Step 5: Commit**

```bash
git add docs/HANDOFF.md docs/INDEX.md docs/QUIRKS.md docs/CONVENTIONS.md
git commit -m "📝 docs(memory): Plan 2 commentaires (frontend visiteur) livré — resync mémoire"
```

---

## Self-Review (rédacteur du plan)

**Couverture de la spec (§8 + §9, périmètre visiteur) :**
- §8.1 seam `Picker` (5 méthodes) → Task B1 (interface + SameOriginPicker : `getElementAt`/`describe`/`resolve`/`toShellRect`/`subscribe` + `fallbackRect`). ✅
- §8.2 `describe`/`resolve` échelle de résolution → A2 (describe : finder+empreinte+textQuote+offset+fallback) + A4 (resolve cascade) + A3 (scorer). ✅
- §8.3 pins dormants (MutationObserver re-resolve) → B1 (`subscribe` branche MutationObserver) + B2 (re-`measure` à chaque signal). ✅
- §8.4 contrôleur de suivi (1 rAF dirty-flag, lecture groupée) → B2. ✅
- §8.5 overlay (surlignage, pastilles, popups floating-ui, badge approximate/orphaned) → D4/D5/D6/D7. **Clustering des pastilles denses** → NON couvert v1 (voir « Écarts » ci-dessous). ⚠️
- §8.6 barre d'action 3 boutons, gated `canAuthor` → D8. ✅
- §8.7 machine à états pick → D1, câblée en D9. ✅
- §8.8 adaptateur + capabilities (visiteur) → C1 ; hooks → C2. ✅
- §9 montage shell (lit `comments_enabled`, lazy, React Query confiné, swallow-on-error, nom localStorage) → E1 + D9 (QueryClient propre) + D2. ✅

**Écarts assumés (à acter en exécution) :**
- **Clustering** des pastilles denses (§8.5) : reporté — l'overlay rend une pastille par pin. Si l'UX le réclame, ajouter un regroupement par proximité dans `OverlayLayer` (post-v1). À signaler dans HANDOFF, pas un blocage de la spec « dans la v1 » stricte mais nice-to-have. → candidat `docs/BACKLOG.md`.
- **« My comments » list** (§8.6, 3ᵉ bouton 💬 saut au pin) : D8 expose `onOpenList` ; D9 le câble sur `setPinsVisible(true)` (rend les pins visibles) mais **pas** un panneau-liste avec saut/scroll-to-pin. Implémentation minimale viable ; le panneau-liste complet est un raffinement. → noter en BACKLOG si non fait.

**Scan placeholders :** aucun « TBD/TODO/à compléter ». Les `NOTE`/`VÉRIFIER` sont des points de contrôle d'exécution (signatures `cn`/`Button.loading`/icônes lucide/`headers` openapi-fetch) avec l'action exacte à faire — pas des trous.

**Cohérence des types :** `ShellRect` (B1) utilisé partout ; `PinPosition` (B2) → D4/D7/D9 ; `AnchorDescriptor`/`parseAnchor`/`serializeAnchor` (A1) → D9 ; `Capabilities`/`CommentPin`/`CommentMessage` (C1) → D6/D8/D9 ; hooks (C2) paramétrés `(slug, adapter)` → appelés ainsi en D9. `FrameRef` (B1) satisfait par `HTMLIFrameElement` → E1.

> RAPPEL exécution : ce plan introduit le **premier `React.lazy` du repo** et le **premier usage**
> de `@floating-ui/dom` / `@medv/finder`. Lire le fichier réel avant chaque `VÉRIFIER`.



