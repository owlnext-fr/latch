# Spec — Popups de commentaires ancrés au pin (Figma-like)

> Date : 2026-07-01
> Statut : design validé, prêt pour plan d'implémentation.
> Suite de « prototype-comments » (`2026-06-30-prototype-comments-design.md`) et du
> refactor UX (`2026-07-01-comments-ux-refactor-design.md`).

## Intention

Correctif UX **100 % frontend** (aucun endpoint, aucun changement backend ni du modèle
d'ancrage). Aujourd'hui, ouvrir un commentaire fait apparaître le popup **collé au côté
de l'élément cible**. Sur un gros composant (container de page), le popup s'ouvre au bord
de ce conteneur — potentiellement à des centaines de pixels du pin — ce qui est
impraticable.

Cible : comme Figma, le fil de commentaire s'ouvre **collé au pin** (au point de clic),
quelle que soit la taille de l'élément ancré. S'applique aux **deux** popups :

- `ThreadPopup` — fil d'un commentaire existant, ouvert au clic sur un pin.
- `ComposePopup` — rédaction d'un nouveau commentaire, ouvert après le pick.

## Contexte technique (cause racine)

- **Le popup s'ancre au rect complet de l'élément.** `ThreadPopup` et `ComposePopup`
  se positionnent via `useFloatingRect(rect)` où `rect` est le **bounding box entier**
  de l'élément cible (`ShellRect`), avec `placement: 'right-start'` dans
  `ui/use-floating-rect.ts`. Sur un grand conteneur, le bord droit du rect est loin du
  pin → popup éloigné.

- **Le pin, lui, est déjà un point.** `ui/pin-badge.tsx` place le pin à :
  ```
  left = rect.x + offset.x * rect.width
  top  = rect.y + offset.y * rect.height
  ```
  où `offset` (`Point` normalisé 0..1) est le point de clic capté au pick et porté par
  l'`AnchorDescriptor`. L'information « où est le pin » existe donc déjà ; elle est
  simplement **dupliquée** inline dans `PinBadge` et **non réutilisée** par les popups.

- **Le pipeline de bornage viewport existe déjà.** `floatingMiddleware()` enchaîne
  `offset(8) → flip → shift(crossAxis, limitShift) → size(maxHeight)`. Il borne déjà le
  popup au viewport ; seul le point de **référence** est mauvais (rect au lieu du point).

- **Espace de coordonnées.** `SameOriginPicker.toShellRect` renvoie des coordonnées
  **viewport**. Les popups sont en `position: fixed` → l'ancrage sur un point viewport
  est correct sur les deux surfaces (client + admin), sans le piège de double-comptage
  d'offset connu de l'`OverlayLayer` (cf. QUIRKS « Overlay commentaires = espace
  viewport »). Ce correctif ne touche pas `OverlayLayer`.

## Décisions (issues du brainstorming)

1. **Périmètre** : les deux popups (`ThreadPopup` + `ComposePopup`).
2. **Visibilité du pin** : le pin reste **pleinement visible à côté** du fil (choix
   Figma). L'écart horizontal dégage le rayon du pin + un petit gap.
3. **Placement par défaut inchangé** : `right-start` + `flip`/`shift` (déjà Figma-like :
   ouvre à droite, se rabat près des bords).

## Design

Approche retenue : **ancrer les popups au point du pin via un `VirtualElement` de taille
nulle**, en réutilisant tout le pipeline `@floating-ui/dom` déjà en place.

### 1. Helper de point partagé — `anchorPoint`

Nouvelle fonction pure, source unique du calcul point (supprime la duplication) :

```ts
// signature
anchorPoint(rect: ShellRect, offset: Point): { x: number; y: number }
// x = rect.x + offset.x * rect.width
// y = rect.y + offset.y * rect.height
```

Emplacement : à côté de `PinPosition` (`follow/controller.ts`) ou un petit
`ui/anchor-point.ts` — tranché au plan selon les cycles d'import (éviter un import
`ui/ → follow/` circulaire). Consommée par : `PinBadge`, `ThreadPopup`, `ComposePopup`.

### 2. `useFloatingRect` → ancrage sur un point

Le hook prend désormais un **point** (`{ x, y } | null`) au lieu d'un `ShellRect`, et
construit un `VirtualElement` de **taille nulle** à ce point :

```ts
const reference = {
  getBoundingClientRect: () => ({
    x, y, width: 0, height: 0, top: y, left: x, right: x, bottom: y,
  }) as DOMRect,
}
```

- Le hook et son fichier sont renommés pour refléter la sémantique point
  (`useFloatingPoint` / `use-floating-point.ts`).
- Pipeline de middleware **inchangé** sauf l'`offset`, qui passe de `8` à une constante
  qui **dégage le pin** :
  ```
  PIN_RADIUS = 14   // size-7 (28px) ÷ 2, cf. pin-badge.tsx
  GAP        = 8
  offset(PIN_RADIUS + GAP)  // = 22
  ```
  L'`offset` de floating-ui est la distance **perpendiculaire au placement** : il
  dégage donc le pin quel que soit le côté après `flip` (droite/gauche/haut/bas). Comme
  la référence est un point (le centre du pin) et le pin s'étend de `PIN_RADIUS` autour,
  un offset ≥ `PIN_RADIUS + GAP` garantit un écart visible → **pin pleinement visible**.

### 3. Câblage

- `ThreadPopup` : `useFloatingPoint(anchorPoint(position.rect, position.offset))`.
- `ComposePopup` : `useFloatingPoint(anchorPoint(pick.rect, pick.anchor.offset))` — le
  point de clic est déjà dans le descripteur capté par le pick-machine.
- `PinBadge` : calcule sa position via `anchorPoint(...)` (dédup, comportement
  identique au pixel près).
- Aucun changement : `OverlayLayer`, suivi rAF (`FollowController`), statut `hidden`,
  drawer, modération, adapters, backend.

### 4. Comportement attendu

- Petit élément : identique à aujourd'hui, le popup s'ouvre à côté du pin.
- Gros conteneur : le popup s'ouvre **collé au pin** (et non plus au bord du conteneur).
- Près d'un bord de viewport : `flip`/`shift` rabattent le popup ; le pin reste dégagé.
- Pin `hidden` (vue `display:none`) : inchangé — le fil ne s'ouvre pas (garde existante
  `!activePosition.hidden` dans `comments-app.tsx`).

## Tests

- **Unit `anchorPoint`** : formule pour offsets `{0,0}`, `{0.5,0.5}`, `{1,1}` sur un rect
  décalé (vérifie `x/y` exacts).
- **`use-floating-point.test.ts`** (ex-`use-floating-rect.test.ts`, renommé) : la
  référence passée à `computePosition` a `width/height === 0` et `top/left/right/bottom`
  au point ; l'`offset` appliqué = `PIN_RADIUS + GAP`.
- **Régression** : `overlay-layer`, `pin-badge`, `thread-popup`, `compose-popup`,
  `comments-drawer` restent verts (mise à jour des imports/props seulement).
- **jsdom** : `computePosition` retombe en `(0,0)` sans layout réel (cf. QUIRKS) → les
  tests de **valeur** de position ne sont pas significatifs en jsdom ; la vérification
  visuelle finale se fait **au navigateur** (Playwright/à la main), comme les correctifs
  UX commentaires précédents. Pas de nouvelle fixture e2e requise (le pin ancré est déjà
  couvert par `comments*.spec.ts`).

## Gate « terminé »

- `pnpm lint` + `pnpm typecheck` : 0 erreur (attention `eslint-plugin-react-hooks` v7 et
  `erasableSyntaxOnly`, cf. QUIRKS).
- `pnpm test` (Vitest) : vert, y compris les nouveaux tests `anchorPoint` /
  `use-floating-point`.
- `pnpm exec playwright test` : vert (suite commentaires inchangée).
- `cargo nextest run` : inchangé (backend non touché) — vérifié quand même.
- Vérification **au navigateur** : le fil s'ouvre collé au pin sur un gros conteneur, le
  pin reste visible, rabattement correct près des bords.
- Mémoire : `INDEX`, `HANDOFF`, et `QUIRKS`/`CONVENTIONS` si un piège/pattern émerge
  (notamment le pattern `VirtualElement` zéro-size + la sémantique de l'`offset`).

## Hors périmètre

- Aucun changement backend, endpoint, modèle d'ancrage, ou format de descripteur.
- Pas de changement de `OverlayLayer` ni du calcul de position des pins (seule la
  *source* du calcul est factorisée dans `anchorPoint`).
- Clustering de pastilles denses, retour-sur-vue, `visibility:hidden` : restent au
  BACKLOG, non concernés.

## Note Context7 (à faire au plan)

Réutilise `computePosition` + `VirtualElement` **déjà présents** dans le repo, à
l'identique (seules les valeurs du rect de référence changent). Confirmer le pattern
`VirtualElement` de taille nulle et la sémantique de `offset()` via Context7
(`@floating-ui/dom`, version épinglée) au moment du plan — usage standard, faible risque.
