# Phase 7 — Lot 3 : Identité visuelle & confort admin

> Design doc. Troisième des 4 lots de la Phase 7 (« Peaufinage graphique / web »).
> Statut : design validé (brainstorming 2026-06-25), à implémenter via un plan dédié.

## Contexte & motivation

Les Lots 1-2 ont livré les fondations (i18n auto-découvert, thème) et le panneau Settings.
Ce lot pose l'**identité visuelle** et le confort de lecture : logo `latch` partout, titres de
page dynamiques, largeur de contenu bornée, et deux liens sortants (GitHub, doc). C'est le lot
le plus visible côté finition.

### État actuel (constaté)

- **Logo** : aucun. Le SVG fourni a été placé en `frontend/src/assets/latch-logo.svg` (badge
  carré 240×240, fond blanc + marque sombre `#2C323B`, auto-contenu).
- **Favicon** : aucun `<link rel="icon">` (retiré en Phase 4 car `public/vite.svg` faisait 404
  sous `/admin` — le backend ne sert que `/assets`). `public/vite.svg` et `src/assets/react.svg`
  subsistent (scaffold mort).
- **Titres** : statiques — `index.html` = « latch — admin », `unlock.html` = « latch ». Aucun
  mécanisme dynamique (TanStack Router n'a pas de gestion head ici).
- **Topbar** (`components/topbar.tsx`) : bouton-lien texte « latch » (→ `/`), puis cog Settings
  (ouvre le Sheet, Lot 2) + Logout.
- **Login** (`routes/login.tsx`) : Card `max-w-sm` centrée, `LocaleSwitcher` top-right, pas de
  logo, pas de lien GitHub.
- **Unlock** (`unlock/unlock-page.tsx`) : Card `max-w-sm`, titre brand/neutral, pas de logo.
- **Largeur admin** : `routes/list.tsx` + `routes/detail.tsx` rendent `<main className="flex-1 p-6">`
  (pleine largeur).

## Décisions de design (tranchées au brainstorming)

| # | Décision | Choix retenu |
|---|---|---|
| D1 | Source/placement du logo | `frontend/src/assets/latch-logo.svg`, exposé via un composant `Logo` mutualisé (admin + unlock). |
| D2 | Stratégie favicon | **SVG seul** (`rel="icon" type="image/svg+xml"`), référencé via `/src/assets/...` → bundlé sous `/assets` (servi en prod). Pas de bundle multi-tailles (outil interne `noindex` ; les chemins racine `/favicon.ico` ne sont pas servis par le backend). |
| D3 | Largeur admin | Conteneur centré `mx-auto max-w-6xl` (1152px) sur le `<main>` de list/detail. |
| D4 | Schéma de titres | « Page — latch admin » (admin, traduit FR/EN) ; unlock « {brand} — déverrouillage » / « Déverrouillage — latch ». Via hook `useDocumentTitle`. |
| D5 | Logo topbar | Badge logo (`size-6`) + texte « latch », ensemble cliquable → `/`. |
| D6 | Liens sortants | Bouton GitHub sur login + bouton « ? » (doc) dans la topbar (à gauche du cog). URLs centralisées dans `lib/links.ts`. |

## Objectifs (ce que le lot livre)

1. **Logo** présent : favicon (2 entrées), topbar (badge+texte), login, unlock.
2. **Titres de page dynamiques** par route (admin) + unlock (brand).
3. **Pages admin bornées** à `max-w-6xl`, centrées.
4. **Bouton GitHub** sur login + **bouton « ? » doc** dans la topbar (liens externes sûrs).
5. **Purge** du scaffold mort (`public/vite.svg`, `src/assets/react.svg`).

### Non-objectifs (hors lot)

- Page d'erreur stylée serving `/c` → Lot 4.
- Bundle favicon multi-tailles / PWA manifest → écarté (cf. D2) ; BACKLOG si besoin un jour.
- Doc Phase 8 elle-même (le bouton « ? » pointe une URL future, pas encore en ligne).

## Architecture

### Fichiers

| Fichier | Responsabilité | Action |
|---|---|---|
| `frontend/src/assets/latch-logo.svg` | Le logo (déjà déplacé) | (présent) |
| `frontend/src/components/logo.tsx` | `<Logo className?>` → `<img src={logoUrl} alt="latch">` | **Créer** |
| `frontend/src/hooks/use-document-title.ts` | `useDocumentTitle(title)` (effect → `document.title`) | **Créer** |
| `frontend/src/lib/links.ts` | `GITHUB_URL`, `DOCS_URL` | **Créer** |
| `frontend/index.html` + `frontend/unlock.html` | `<link rel="icon" ...>` SVG | **Modifier** |
| `frontend/src/components/topbar.tsx` | Badge logo + texte ; bouton « ? » avant le cog | **Modifier** |
| `frontend/src/routes/login.tsx` | Logo au-dessus de la Card + bouton GitHub + titre | **Modifier** |
| `frontend/src/unlock/unlock-page.tsx` | Logo au-dessus de la Card + titre dynamique | **Modifier** |
| `frontend/src/routes/list.tsx` | Conteneur `max-w-6xl` + titre | **Modifier** |
| `frontend/src/routes/detail.tsx` | Conteneur `max-w-6xl` + titre | **Modifier** |
| `frontend/src/i18n/locales/admin/{en,fr}.json` | `title.*`, `login.github`, `topbar.help` | **Modifier** |
| `frontend/src/i18n/locales/unlock/{en,fr}.json` | `unlock.page_title_*` | **Modifier** |
| `frontend/public/vite.svg`, `frontend/src/assets/react.svg` | scaffold mort | **Supprimer** |
| Tests associés | Couverture | **Créer/Modifier** |

### `Logo` (mutualisé)

```tsx
import logoUrl from '@/assets/latch-logo.svg'
export function Logo({ className }: { className?: string }) {
  return <img src={logoUrl} alt="latch" className={className} />
}
```
Importé par topbar, login, unlock. Vite hashe le SVG par bundle → source unique, pas de couplage
admin/unlock. Tailles par CSS : `size-6` (topbar), `size-12` (login/unlock).

### Favicon (SVG seul)

Dans `index.html` ET `unlock.html`, dans `<head>` :
```html
<link rel="icon" type="image/svg+xml" href="/src/assets/latch-logo.svg" />
```
Vite réécrit `/src/assets/latch-logo.svg` → `/assets/latch-logo-<hash>.svg` au build, servi par le
mount `ServeDir("/assets")`. C'est la correction propre du favicon-404 de la Phase 4 (pas de
fichier à la racine).

### `useDocumentTitle`

```ts
import { useEffect } from 'react'
export function useDocumentTitle(title: string) {
  useEffect(() => {
    document.title = title
  }, [title])
}
```
Hook partagé. Les titres statiques HTML restent la valeur initiale avant hydratation.

Schéma (admin, clés `title.*`, traduites) :
- `title.projects` : « Projects — latch admin » / « Projets — latch admin »
- `title.login` : « Sign in — latch admin » / « Connexion — latch admin »
- `title.detail` : « {{name}} — latch admin » (les 2 langues ; `name` = nom du projet, non traduit)

Appels :
- `list` → `useDocumentTitle(t('title.projects'))`
- `detail` → `useDocumentTitle(t('title.detail', { name: project?.name ?? '…' }))` (réagit au chargement)
- `login` → `useDocumentTitle(t('title.login'))`
- Settings : aucun (c'est un Sheet, pas une route).

Unlock (catalogue séparé, clés `unlock.page_title_*`) :
- `unlock.page_title_brand` : « {{brand}} — unlock » / « {{brand}} — déverrouillage »
- `unlock.page_title_neutral` : « Unlock — latch » / « Déverrouillage — latch »
- Appel : `useDocumentTitle(brand ? t('unlock.page_title_brand', { brand }) : t('unlock.page_title_neutral'))`

### Topbar

Groupe gauche cliquable → `/` : `<Logo className="size-6" />` + « latch » (même `Button variant="link"`).
Groupe droit : bouton « ? » (`CircleHelp`, lien externe `DOCS_URL`) **avant** le cog Settings, puis Logout.

### Largeur admin

`routes/list.tsx` et `routes/detail.tsx` : `<main className="flex-1 p-6">` →
`<main className="mx-auto w-full max-w-6xl flex-1 p-6">`. Topbar reste pleine largeur ; seul le
contenu est borné/centré.

### Bouton GitHub (login)

Lien discret sous la Card, centré : `Button variant="ghost" size="sm" asChild` enveloppant un
`<a href={GITHUB_URL} target="_blank" rel="noopener noreferrer">` avec icône `Github` (lucide) +
`t('login.github')` (« View on GitHub » / « Voir sur GitHub »).

### `lib/links.ts`

```ts
export const GITHUB_URL = 'https://github.com/owlnext-fr/latch'
export const DOCS_URL = 'https://latch.owlnext.fr/docs'
```
`owlnext-fr/latch` = org propriétaire (déjà public README/CLAUDE.md), pas un client.

## i18n — clés à ajouter

**Admin (en + fr)** : `title.projects`, `title.login`, `title.detail`, `login.github`, `topbar.help`
(« Documentation » / « Documentation »).
**Unlock (en + fr)** : `unlock.page_title_brand`, `unlock.page_title_neutral`.

## Plan de tests

Lot 100 % frontend. Vitest + Testing Library ; e2e Playwright reste vert.

- `logo.test.tsx` : `<img alt="latch">` rendu.
- `use-document-title.test.ts` : `document.title` mis à jour.
- `topbar.test.tsx` (maj) : badge logo présent ; lien « ? » (`aria-label` doc, href `DOCS_URL`,
  `target=_blank`) ; cog présent ; titre « latch » présent.
- `login.test.tsx` (maj) : logo présent ; lien GitHub (href `GITHUB_URL`, `target=_blank`,
  `rel=noopener noreferrer`) ; `document.title` = valeur de `title.login`.
- `unlock-page.test.tsx` (maj) : logo présent ; `document.title` bascule brand/neutral.
- `list.test.tsx` / `detail.test.tsx` (maj légère) : `document.title` posé ; conteneur `max-w-6xl`.

**Vérifs build** : favicon SVG présent dans `dist/index.html` + `dist/unlock.html` (réécrit sous
`/assets`) ; `public/vite.svg` + `src/assets/react.svg` supprimés ; build vert.

## Critères de sortie du Lot 3

1. Logo présent : favicon (2 entrées), topbar (badge+texte), login, unlock.
2. Titres dynamiques par route (« Page — latch admin ») + unlock (brand).
3. Pages admin bornées `max-w-6xl`, centrées.
4. Bouton GitHub (login) + bouton « ? » doc (topbar), liens externes `target=_blank rel=noopener`.
5. Scaffold mort purgé (`vite.svg`, `react.svg`).
6. `lint && typecheck && test` verts ; SonarCloud `new_coverage ≥ 80 %` ; build vert.
7. Mémoire à jour : INDEX, HANDOFF, CONVENTIONS (Logo + `asChild` lien + `lib/links`), QUIRKS si
   piège favicon `/assets`.

## Risques & points de vigilance

- **Favicon serving** : vérifier au build que le `<link>` est bien réécrit vers `/assets/...`
  (pas un chemin racine non servi). Test build.
- **`Button asChild`** : confirmer que `components/ui/button.tsx` expose `asChild` (Radix Slot)
  avant de l'utiliser pour le lien GitHub ; sinon, styliser un `<a>` directement.
- **Bundle public unlock** : le `Logo` n'ajoute que le SVG (4 Ko) ; pas de dépendance admin.
- **Lien doc** : `DOCS_URL` pointe une page Phase 8 pas encore en ligne (assumé).

## Dépendances

- Consomme du Lot 2 : la topbar (cog Settings) — on insère le « ? » à côté.
- Aucune dépendance bloquante : le SVG logo est déjà en place.
