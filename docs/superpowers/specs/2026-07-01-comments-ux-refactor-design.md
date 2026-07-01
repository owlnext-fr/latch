# Spec — Refactor UX commentaires (`/c` + `/admin`)

> Date : 2026-07-01
> Statut : design validé, prêt pour plan d'implémentation.
> Suite de la feature « prototype-comments » (`2026-06-30-prototype-comments-design.md`).

## Intention

Lot de corrections + refactor UX de la couche commentaires, **100 % frontend**
(aucun nouvel endpoint, aucun changement backend). Sept points, côté serving client
(`/c/<slug>`) et côté admin (page Review `/admin/projects/{id}/versions/{n}/review`) :

1. **Drawer « Commentaires »** — le bouton *Liste* de la barre est un stub ; construire
   le panneau latéral droit qui liste les threads.
2. **Label du pin** — afficher la 1ʳᵉ lettre de l'auteur au lieu du nombre de messages.
3. **Couleur des pins** — bleu fluo fixe (indépendant du thème), pour la visibilité sur
   maquettes claires comme sombres.
4. **Positionnement des popups** — empêcher les popups de thread de sortir du viewport
   (notamment à gauche).
5. **Ciblage DOM** — remplacer le carré sombre par un highlight bleu fluo + dégradé
   intérieur capé.
6. **Adapter côté admin** — propagation des changements 1–5 (composants partagés).
7. **Décalage des pins admin** — corriger le décalage vertical égal à la hauteur de la
   topbar.

## Contexte technique (existant, cause racine)

- **Composants partagés client ⇄ admin** : `comments-app.tsx`, `ui/overlay-layer.tsx`,
  `ui/pin-badge.tsx`, `ui/thread-popup.tsx`, `ui/compose-popup.tsx`, `ui/action-bar.tsx`,
  `ui/use-floating-rect.ts`, `follow/*`, `picker/*`, `data/use-comments.ts`. Seul
  l'`adapter` diffère (`createVisitorAdapter` vs `createAdminAdapter`). Toute
  modification d'un composant partagé profite aux deux surfaces.
- **Espace de coordonnées « shell »** : `SameOriginPicker.toShellRect` renvoie des
  coordonnées **viewport** (`iframe.getBoundingClientRect().top + element.top`, etc.).
  - `ThreadPopup`/`ComposePopup` via `useFloatingRect` se positionnent en
    `position: fixed` → déjà en espace viewport → corrects partout.
  - `OverlayLayer` (donc les pins + la hover-box) est en `absolute inset-0` de son
    conteneur positionné. Côté client (`shell-page.tsx`), ce conteneur (`relative
    h-svh w-svw`) est à l'origine viewport `(0,0)` → OK. Côté admin (`review.tsx`), le
    conteneur `relative flex-1` démarre **sous la topbar** (`h-14` = 56 px) →
    double-comptage → pins décalés de 56 px vers le bas.
- **Couleur `bg-primary`** : `--primary` vaut du stone quasi-noir en light
  (`oklch(0.216 …)`) et stone clair en dark (`oklch(0.923 …)`). L'overlay hérite du
  thème admin/shell alors qu'il est rendu **sur un proto arbitraire** → d'où
  l'invisibilité. Il faut une couleur **fixe**, hors variables de thème.
- **Bouton *Liste*** : `ActionBar` a un bouton *Liste* dont `onOpenList` fait seulement
  `setPinsVisible(true)`. Aucun panneau n'existe.
- **`@floating-ui/dom` v1.7.6**, **`@medv/finder` v4.0.2** (déjà dépendances).

## Décisions validées (brainstorm)

| Sujet | Décision |
|---|---|
| Bleu fluo | `#18A0FB` (Figma blue), **couleur en dur**, hors `--primary`. |
| Pins orphaned/moved | **Restent en ambre** `#f59e0b` (avertissement sémantique). |
| Label pin | 1ʳᵉ lettre **majuscule** de `pin.messages[0].author_name` (créateur du thread) ; fallback `•` si nom vide. |
| Ciblage DOM | **Inset edge glow** : bordure fluo + `box-shadow: inset`, profondeur **capée ~30 px** (non proportionnelle). |
| Liste | **Drawer latéral droit**, ouvert par le bouton *Liste*. |
| Clic sur une ligne | **Scroll vers le pin + ouverture directe du thread** (comportement A). |
| Contenu de ligne | avatar fluo (lettre), auteur, âge, extrait du 1ᵉʳ message, nb de réponses, badge « orphelin » ; orphelins listés en bas. |

---

## 1. Pin — label + couleur

**Fichiers** : `ui/pin-badge.tsx`, `ui/overlay-layer.tsx`, `comments-app.tsx`.

- `PinBadge` : remplacer la prop `count: number` par `label: string`. Afficher `{label}`
  au lieu de `{count}`.
- Couleur : supprimer `status === 'anchored' ? 'bg-primary' : 'bg-amber-500'`. Le pin
  ancré prend le fluo **en dur** (style inline `background:'#18A0FB'` ou classe
  utilitaire dédiée type `bg-[#18A0FB]`), l'ambre reste pour `orphaned`/`moved`. Le
  liseré blanc, l'ombre et l'anneau actif sont conservés (l'anneau actif passe d'une
  teinte `ring-primary/40` à une teinte fluo cohérente, ex. `ring-[#18A0FB]/40`).
- `overlay-layer.tsx` : remplacer la prop `countOf` par un calcul de label. Le plus
  simple : passer `label` par pin. `comments-app.tsx` calcule
  `firstLetter(pin.messages[0]?.author_name)` (helper `firstLetter` : trim → premier
  caractère → `toUpperCase()` → `•` si vide).

## 2. Ciblage DOM — inset edge glow

**Fichier** : `ui/overlay-layer.tsx` (bloc `pickMode && hover`).

- Bordure : `border-primary` → bordure fluo (`#18A0FB`).
- Ajouter un **halo intérieur** via `box-shadow: inset`. Profondeur (blur) calculée à
  partir de la taille du rect **et capée** : `depth = min(CAP, k · min(hover.width,
  hover.height))` avec `CAP ≈ 30 px`. Opacité du halo ~0.4–0.55. Valeurs exactes
  affinées à l'implémentation ; l'important est la **règle « capée, non
  proportionnelle »**.
- `pointer-events: none` conservé.

## 3. Positionnement des popups — moteur

**Fichier** : `ui/use-floating-rect.ts`.

Root cause (confirmée via context7) : `placement: 'right-start'` + `shift({ padding: 8 })`
ne borne que l'axe vertical ; quand `flip` bascule le popup à gauche et qu'il n'y a pas
la place (largeur 320 px), il déborde à gauche.

Nouveau pipeline middleware :

```ts
middleware: [
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
```

- `shift({ crossAxis: true, limiter: limitShift() })` : borne aussi l'axe horizontal
  sans détacher le popup de sa référence.
- `size` : borne la hauteur des longs threads pour qu'ils ne dépassent pas le viewport
  (le `ul` interne du `ThreadPopup` a déjà `overflow-y-auto`).
- Applicable aussi bien à `ThreadPopup` qu'à `ComposePopup` (même hook).

## 4. Décalage des pins admin — coordonnées

**Fichier** : `ui/overlay-layer.tsx` (le conteneur racine).

`absolute inset-0` → **`fixed inset-0`**. Le conteneur `fixed` établit un bloc
conteneur ancré au viewport ; ses enfants `absolute` (pins, hover-box, pick-surface)
se positionnent alors en coordonnées **viewport**, cohérentes avec `toShellRect` et les
popups `fixed`.

- **Client** : aucun changement visuel (l'overlay était déjà à l'origine viewport).
- **Admin** : les pins s'alignent correctement (plus de +56 px).
- **Sûreté** : en admin `capabilities.canAuthor === false` → la pick-surface plein-écran
  ne se rend jamais ; hors pick, le conteneur a `pointer-events: none` (seuls les pins
  captent les clics), donc le `fixed` plein-viewport ne masque pas la topbar.

## 5. Drawer « Commentaires »

**Nouveau fichier** : `ui/comments-drawer.tsx`. **Branchement** : `comments-app.tsx`,
`ui/action-bar.tsx`.

- **État** : `drawerOpen` (`useState`) dans `CommentsInner`. Le bouton *Liste* de
  l'`ActionBar` (`onOpenList`) ouvre/bascule le drawer ; on retire l'ancien
  `onOpenList → setPinsVisible(true)`.
- **Rendu** : panneau latéral droit (position `fixed`, `right:0`, hauteur pleine,
  `z-[60]`), stylé shadcn/stone. En-tête « Commentaires (N) » + fermeture. Liste
  scrollable.
- **Ligne** (par pin) : avatar rond fluo avec la lettre (ambre si orphaned), auteur
  (`messages[0].author_name`), âge relatif (`created_at`), extrait tronqué du 1ᵉʳ
  message, nombre de réponses (`messages.length - 1`), badge « orphelin » si le statut
  du pin (via `positions`) est `orphaned`/`moved`.
- **Tri** : threads sains d'abord (par récence), orphelins en bas.
- **Clic sur une ligne (comportement A)** :
  1. Résoudre l'élément dans l'iframe : `picker.resolve(parseAnchor(pin.anchor))`.
  2. Si un élément est trouvé → `element.scrollIntoView({ block: 'center' })`
     (défilement du DOM du proto).
  3. `setActivePinId(pin.id)` → le `ThreadPopup` s'ouvre (repositionné par le
     `FollowController` au prochain frame).
  4. Orphelin sans élément : ouvrir le thread au `fallbackRect`, sans scroll.
  - Fermer le drawer à l'ouverture du thread (évite le chevauchement popup/drawer).
- **État vide** : message « Aucun commentaire » si `pins.length === 0`.
- **Admin** : fonctionne tel quel (adapter admin ; lecture + modération déjà gérées par
  `ThreadPopup` via `capabilities.canModerate`).

## 6. Admin — propagation

Aucun code dédié hors point 4. Les points 1/2/3/5 vivent dans des composants partagés
et profitent automatiquement à la page Review. `createAdminAdapter` (caps
`canAuthor:false`, `canModerate:true`) inchangé.

## Découpage en unités

- `pin-badge.tsx` : présentation d'un pin (label + couleur). Testable isolément.
- `overlay-layer.tsx` : surface d'overlay (pins + ciblage + coordonnées). Le highlight
  et le passage `fixed` y sont contenus.
- `use-floating-rect.ts` : politique de positionnement flottant. Isolé, réutilisé par
  les deux popups.
- `comments-drawer.tsx` : **nouvelle unité** — liste + interaction de focus. Dépend de
  l'`adapter` (données) et du `picker` (scroll/résolution) via props ; ne connaît pas
  l'origine (client/admin).
- `comments-app.tsx` : composition/état (câble le drawer, l'activePin, le label).

## Tests (partie « terminé »)

- **Vitest / Testing-Library** :
  - `pin-badge` : lettre affichée, fallback `•`, couleur fluo pour ancré / ambre pour
    orphaned.
  - `overlay-layer` : highlight rendu en pick, profondeur de glow capée pour un grand
    rect ; conteneur `fixed`.
  - `use-floating-rect` : présence des middlewares (borne viewport) ; non-régression du
    style de base sous jsdom (retombe en `fixed` `(0,0)` sans layout).
  - `comments-drawer` (**nouveau**) : rendu des lignes, tri (orphelins en bas), état
    vide, clic → `scrollIntoView` appelé + `activePinId` posé + drawer fermé.
  - non-régression `thread-popup`.
- **Playwright** :
  - `comments.spec.ts` : ouvrir le drawer visiteur, cliquer une ligne, le thread
    s'ouvre.
  - `comments-admin.spec.ts` : pins alignés sous la topbar (position attendue), drawer
    admin ouvrable, modération inchangée.
  - Suites **build (`:5150`) et `test:vite` (`:5173`)** vertes (on ne touche pas
    `vite.config.ts` → suite Vite non impactée mais on vérifie).
- **Gate complète** : `pnpm lint && pnpm typecheck && pnpm test`, `cargo nextest`
  inchangé (aucun changement backend), Sonar **new-coverage ≥ 80 %**.

## Hors périmètre (YAGNI)

- Pas de nouveaux endpoints ni de changements de schéma OpenAPI.
- Pas de recherche/filtre texte dans le drawer (v1 = liste + tri statut/récence).
- Pas de temps-réel / websocket (le cache TanStack Query reste la source).
- Pas de refonte de l'ancrage (`@medv/finder`) ni du `FollowController`.

## Mémoire à mettre à jour en fin d'implémentation

- `docs/INDEX.md` : ligne feature « Refactor UX commentaires ».
- `docs/HANDOFF.md` : entrée datée.
- `docs/QUIRKS.md` : piège « espace shell = viewport ; l'overlay des pins doit être
  `fixed` sinon décalage sous une topbar ».
- `docs/CONVENTIONS.md` : constante couleur fluo commentaires `#18A0FB` (fixe, hors
  thème).
