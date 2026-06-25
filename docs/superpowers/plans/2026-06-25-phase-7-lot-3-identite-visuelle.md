# Phase 7 — Lot 3 : Identité visuelle & confort admin — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Poser l'identité visuelle (logo `latch` en favicon/topbar/login/unlock), des titres de page dynamiques, une largeur de contenu admin bornée, et deux liens sortants (GitHub login + « ? » doc topbar).

**Architecture:** Un composant `Logo` mutualisé (importe le SVG `src/assets/latch-logo.svg`), un hook `useDocumentTitle`, et un module `lib/links.ts` pour les URLs externes. Favicon SVG-only référencé via `/src/assets/...` (bundlé sous `/assets`). Largeur via `mx-auto max-w-6xl` sur le `<main>` de list/detail.

**Tech Stack:** React 19, Vite 8, TypeScript, react-i18next ^17, lucide-react (`Github`, `CircleHelp`), radix-ui `Slot` (via `Button asChild`), Vitest ^4 + Testing Library (jsdom, `globals: true`, alias `@` → `src`).

## Global Constraints

- **Confidentialité (NON-NÉGOCIABLE)** : aucun nom de client réel. `latch`, `owlnext-fr/latch`, `latch.owlnext.fr` = projet/org propriétaire (OK). Placeholders fictifs sinon.
- **Favicon SVG-only** : `<link rel="icon" type="image/svg+xml" href="/src/assets/latch-logo.svg">` dans index.html ET unlock.html. AUCUN fichier favicon à la racine (le backend ne sert que `/assets`).
- **Isolation bundle public** : le `Logo` (unlock) n'ajoute que le SVG (~4 Ko), aucune dépendance admin.
- **Liens externes** : `target="_blank" rel="noopener noreferrer"` sur GitHub + doc. URLs centralisées dans `lib/links.ts` : `GITHUB_URL = 'https://github.com/owlnext-fr/latch'`, `DOCS_URL = 'https://latch.owlnext.fr/docs'`.
- **`Button asChild`** : disponible (Slot.Root) — l'utiliser pour les liens-boutons (`<a>` enfant).
- **Largeur** : `mx-auto w-full max-w-6xl` sur le `<main>` de list + detail uniquement (topbar pleine largeur).
- **Titres** : schéma « Page — latch admin » (admin traduit), unlock « {brand} — déverrouillage » / « Déverrouillage — latch ».
- **Couverture** : SonarCloud `new_coverage ≥ 80 %` sur le code neuf.
- **Commandes** : depuis `frontend/`. `rtk vitest run`, `pnpm typecheck`, `rtk lint`, `pnpm build`.
- **Subagents** : IGNORER le protocole load-memory du CLAUDE.md, ne pas répondre « Mémoire chargée ».

---

## File Structure

| Fichier | Responsabilité | Action |
|---|---|---|
| `frontend/src/components/logo.tsx` | `<Logo className?>` | **Créer** |
| `frontend/src/components/logo.test.tsx` | Test | **Créer** |
| `frontend/src/hooks/use-document-title.ts` | Hook titre | **Créer** |
| `frontend/src/hooks/use-document-title.test.ts` | Test | **Créer** |
| `frontend/src/lib/links.ts` | URLs externes | **Créer** |
| `frontend/index.html` + `frontend/unlock.html` | `<link rel="icon">` | **Modifier** |
| `frontend/public/vite.svg`, `frontend/src/assets/react.svg` | scaffold mort | **Supprimer** |
| `frontend/src/routes/list.tsx` | titre + largeur | **Modifier** |
| `frontend/src/routes/detail.tsx` | titre + largeur | **Modifier** |
| `frontend/src/routes/login.tsx` | logo + GitHub + titre | **Modifier** |
| `frontend/src/components/topbar.tsx` | logo badge+texte + « ? » | **Modifier** |
| `frontend/src/unlock/unlock-page.tsx` | logo + titre | **Modifier** |
| `frontend/src/i18n/locales/admin/{en,fr}.json` | `title.*`, `login.github`, `topbar.help` | **Modifier** |
| `frontend/src/i18n/locales/unlock/{en,fr}.json` | `unlock.page_title_*` | **Modifier** |
| Tests : `login.test.tsx`, `topbar.test.tsx`, `unlock-page.test.tsx`, `list.test.tsx`, `detail.test.tsx` | maj | **Modifier** |
| `docs/INDEX.md`, `docs/HANDOFF.md`, `docs/CONVENTIONS.md`, `docs/QUIRKS.md` | mémoire | **Modifier** |

---

## Task 1 : Identity infra — `Logo`, `useDocumentTitle`, `lib/links`, favicon, purge

**Files:**
- Create: `frontend/src/components/logo.tsx`, `frontend/src/components/logo.test.tsx`
- Create: `frontend/src/hooks/use-document-title.ts`, `frontend/src/hooks/use-document-title.test.ts`
- Create: `frontend/src/lib/links.ts`
- Modify: `frontend/index.html`, `frontend/unlock.html`
- Delete: `frontend/public/vite.svg`, `frontend/src/assets/react.svg`

**Interfaces:**
- Produces: `<Logo className?: string />` ; `useDocumentTitle(title: string): void` ; `GITHUB_URL`, `DOCS_URL` from `@/lib/links`.

- [ ] **Step 1 : Écrire les tests (échouent)**

Create `frontend/src/components/logo.test.tsx` :
```tsx
import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { Logo } from './logo'

describe('Logo', () => {
  it('renders an image with the latch alt text', () => {
    render(<Logo className="size-6" />)
    const img = screen.getByAltText('latch')
    expect(img).toBeInTheDocument()
    expect(img).toHaveClass('size-6')
  })
})
```

Create `frontend/src/hooks/use-document-title.test.ts` :
```ts
import { describe, it, expect } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useDocumentTitle } from './use-document-title'

describe('useDocumentTitle', () => {
  it('sets document.title to the given value', () => {
    renderHook(() => useDocumentTitle('Hello — latch admin'))
    expect(document.title).toBe('Hello — latch admin')
  })

  it('updates the title when the value changes', () => {
    const { rerender } = renderHook(({ t }) => useDocumentTitle(t), {
      initialProps: { t: 'First' },
    })
    expect(document.title).toBe('First')
    rerender({ t: 'Second' })
    expect(document.title).toBe('Second')
  })
})
```

- [ ] **Step 2 : Lancer, vérifier l'échec**

Run: `rtk vitest run src/components/logo.test.tsx src/hooks/use-document-title.test.ts`
Expected: FAIL — modules introuvables.

- [ ] **Step 3 : Implémenter `Logo`**

Create `frontend/src/components/logo.tsx` :
```tsx
import logoUrl from '@/assets/latch-logo.svg'

export function Logo({ className }: Readonly<{ className?: string }>) {
  return <img src={logoUrl} alt="latch" className={className} />
}
```

- [ ] **Step 4 : Implémenter `useDocumentTitle`**

Create `frontend/src/hooks/use-document-title.ts` :
```ts
import { useEffect } from 'react'

export function useDocumentTitle(title: string) {
  useEffect(() => {
    document.title = title
  }, [title])
}
```

- [ ] **Step 5 : Créer `lib/links.ts`**

Create `frontend/src/lib/links.ts` :
```ts
export const GITHUB_URL = 'https://github.com/owlnext-fr/latch'
export const DOCS_URL = 'https://latch.owlnext.fr/docs'
```

- [ ] **Step 6 : Ajouter le favicon dans `index.html`**

Edit `frontend/index.html` — ajouter dans `<head>`, après la ligne `<title>` :
```html
    <link rel="icon" type="image/svg+xml" href="/src/assets/latch-logo.svg" />
```

- [ ] **Step 7 : Ajouter le favicon dans `unlock.html`**

Edit `frontend/unlock.html` — ajouter dans `<head>`, après la ligne `<title>` :
```html
    <link rel="icon" type="image/svg+xml" href="/src/assets/latch-logo.svg" />
```

- [ ] **Step 8 : Purger le scaffold mort**

```bash
cd /srv/owlnext/latch/frontend
git rm public/vite.svg src/assets/react.svg
```

- [ ] **Step 9 : Lancer les tests + typecheck**

Run: `rtk vitest run src/components/logo.test.tsx src/hooks/use-document-title.test.ts` puis `pnpm typecheck`
Expected: PASS (3 tests) ; typecheck 0 erreur (le `import logoUrl from '@/assets/latch-logo.svg'` résout via les types Vite SVG — si TS se plaint de l'import SVG, vérifier `vite-env.d.ts`/`env.d.ts` qui déclare déjà `*.svg`; le projet importait déjà `react.svg` donc la déclaration existe).

- [ ] **Step 10 : Commit**

```bash
git add frontend/src/components/logo.tsx frontend/src/components/logo.test.tsx frontend/src/hooks/use-document-title.ts frontend/src/hooks/use-document-title.test.ts frontend/src/lib/links.ts frontend/index.html frontend/unlock.html
git commit -m "✨ feat(ui): Logo + useDocumentTitle + lib/links + favicon SVG, purge scaffold"
```

---

## Task 2 : Titres + largeur des pages admin (list + detail)

**Files:**
- Modify: `frontend/src/routes/list.tsx`, `frontend/src/routes/detail.tsx`
- Modify: `frontend/src/i18n/locales/admin/en.json`, `frontend/src/i18n/locales/admin/fr.json`
- Modify (tests): `frontend/src/routes/list.test.tsx`, `frontend/src/routes/detail.test.tsx`

**Interfaces:**
- Consumes: `useDocumentTitle` (Task 1), clés i18n `title.projects` / `title.detail`.

- [ ] **Step 1 : Ajouter les clés i18n admin (en)**

Edit `frontend/src/i18n/locales/admin/en.json` — ajouter ces clés (en bloc, avant le `}` final ; la clé actuellement dernière doit recevoir une virgule) :
```json
  "title.projects": "Projects — latch admin",
  "title.login": "Sign in — latch admin",
  "title.detail": "{{name}} — latch admin",
  "login.github": "View on GitHub",
  "topbar.help": "Documentation"
```

- [ ] **Step 2 : Ajouter les clés i18n admin (fr)**

Edit `frontend/src/i18n/locales/admin/fr.json` — de même :
```json
  "title.projects": "Projets — latch admin",
  "title.login": "Connexion — latch admin",
  "title.detail": "{{name}} — latch admin",
  "login.github": "Voir sur GitHub",
  "topbar.help": "Documentation"
```

- [ ] **Step 3 : Écrire/ajuster le test list (échoue sur le titre)**

Edit `frontend/src/routes/list.test.tsx` — ajouter un test (réutiliser le harness `renderWithRouter('/')` déjà importé) :
```tsx
  it('sets the document title to the projects title', async () => {
    renderWithRouter('/')
    await waitFor(() => expect(document.title).toBe('Projects — latch admin'))
  })
```
(Si `waitFor` n'est pas importé dans ce fichier, l'ajouter à l'import `@testing-library/react`.)

- [ ] **Step 4 : Lancer, vérifier l'échec**

Run: `rtk vitest run src/routes/list.test.tsx`
Expected: FAIL — `document.title` n'est pas encore posé par ListPage.

- [ ] **Step 5 : Modifier `list.tsx` (titre + largeur)**

Edit `frontend/src/routes/list.tsx` :
- Ajouter l'import : `import { useDocumentTitle } from '@/hooks/use-document-title'`.
- Dans `ListPage`, après le `const { t } = useTranslation()` (ou près des hooks du haut), ajouter : `useDocumentTitle(t('title.projects'))`.
- Remplacer `<main className="flex-1 p-6">` par `<main className="mx-auto w-full max-w-6xl flex-1 p-6">`.

- [ ] **Step 6 : Modifier `detail.tsx` (titre + largeur)**

Edit `frontend/src/routes/detail.tsx` :
- Ajouter l'import : `import { useDocumentTitle } from '@/hooks/use-document-title'`.
- Dans `DetailPage`, après l'obtention de `project` et `t`, ajouter : `useDocumentTitle(t('title.detail', { name: project?.name ?? '…' }))`.
- Remplacer `<main className="flex-1 p-6">` par `<main className="mx-auto w-full max-w-6xl flex-1 p-6">`.

- [ ] **Step 7 : Ajouter le test détail (titre)**

Edit `frontend/src/routes/detail.test.tsx` — ajouter un test vérifiant que `document.title` contient le nom du projet chargé (réutiliser le harness/MSW existant du fichier ; s'inspirer d'un test existant qui rend le détail avec un projet nommé). Exemple de forme (adapter au harness du fichier) :
```tsx
  it('sets the document title to the project name', async () => {
    // … rendre la route détail avec un projet nommé via le harness existant …
    await waitFor(() => expect(document.title).toMatch(/— latch admin$/))
  })
```
Si le harness du fichier ne fournit pas trivialement un projet nommé, se limiter à asserter le suffixe `/— latch admin$/` une fois la page chargée.

- [ ] **Step 8 : Lancer la suite ciblée**

Run: `rtk vitest run src/routes/list.test.tsx src/routes/detail.test.tsx`
Expected: PASS.

- [ ] **Step 9 : Commit**

```bash
git add frontend/src/routes/list.tsx frontend/src/routes/detail.tsx frontend/src/routes/list.test.tsx frontend/src/routes/detail.test.tsx frontend/src/i18n/locales/admin/
git commit -m "✨ feat(admin): titres de page dynamiques + largeur bornée max-w-6xl"
```

---

## Task 3 : Login — logo + bouton GitHub + titre

**Files:**
- Modify: `frontend/src/routes/login.tsx`
- Modify (test): `frontend/src/routes/login.test.tsx`

**Interfaces:**
- Consumes: `<Logo />`, `useDocumentTitle`, `GITHUB_URL` (Task 1), clés `title.login` / `login.github` (Task 2).

- [ ] **Step 1 : Ajouter les tests login (échouent)**

Edit `frontend/src/routes/login.test.tsx` — ajouter (réutiliser `renderWithRouter('/login')` déjà importé) :
```tsx
  it('shows the logo and a GitHub link, and sets the document title', async () => {
    renderWithRouter('/login')
    expect(await screen.findByAltText('latch')).toBeInTheDocument()
    const gh = screen.getByRole('link', { name: /GitHub/i })
    expect(gh).toHaveAttribute('href', 'https://github.com/owlnext-fr/latch')
    expect(gh).toHaveAttribute('target', '_blank')
    expect(gh).toHaveAttribute('rel', expect.stringContaining('noopener'))
    await waitFor(() => expect(document.title).toBe('Sign in — latch admin'))
  })
```
(Ajouter `screen`, `waitFor` à l'import `@testing-library/react` s'ils manquent.)

- [ ] **Step 2 : Lancer, vérifier l'échec**

Run: `rtk vitest run src/routes/login.test.tsx`
Expected: FAIL — pas de logo, pas de lien GitHub, titre non posé.

- [ ] **Step 3 : Modifier `login.tsx`**

Edit `frontend/src/routes/login.tsx` :
- Imports à ajouter :
  ```tsx
  import { Github } from 'lucide-react'
  import { Logo } from '@/components/logo'
  import { useDocumentTitle } from '@/hooks/use-document-title'
  import { GITHUB_URL } from '@/lib/links'
  ```
- Dans `LoginPage`, après `const { t } = useTranslation()` : `useDocumentTitle(t('title.login'))`.
- Insérer le logo au-dessus de la Card et le lien GitHub sous la Card. Remplacer le bloc :
  ```tsx
      <Card className="w-full max-w-sm">
  ```
  par :
  ```tsx
      <div className="flex w-full max-w-sm flex-col items-center gap-6">
        <Logo className="size-12" />
        <Card className="w-full">
  ```
  et, juste après la fermeture `</Card>` (avant la fermeture du conteneur centré), ajouter le lien GitHub puis fermer le nouveau `<div>` :
  ```tsx
        </Card>
        <Button asChild variant="ghost" size="sm">
          <a href={GITHUB_URL} target="_blank" rel="noopener noreferrer">
            <Github />
            {t('login.github')}
          </a>
        </Button>
      </div>
  ```
  **Important** : la `<Card>` est actuellement enfant direct du `<div className="relative grid min-h-screen place-items-center">`. On l'enveloppe désormais dans le nouveau `<div className="flex ... flex-col items-center gap-6">`, lui-même enfant du grid centré. Vérifier l'équilibre des balises (`<Card>…</Card>` conserve son contenu inchangé).

- [ ] **Step 4 : Lancer, vérifier le succès**

Run: `rtk vitest run src/routes/login.test.tsx`
Expected: PASS.

- [ ] **Step 5 : Commit**

```bash
git add frontend/src/routes/login.tsx frontend/src/routes/login.test.tsx
git commit -m "✨ feat(login): logo + lien GitHub + titre de page"
```

---

## Task 4 : Topbar — logo badge+texte + bouton « ? » doc

**Files:**
- Modify: `frontend/src/components/topbar.tsx`
- Modify (test): `frontend/src/components/topbar.test.tsx`

**Interfaces:**
- Consumes: `<Logo />`, `DOCS_URL` (Task 1), clé `topbar.help` (Task 2).

- [ ] **Step 1 : Ajouter le test topbar (échoue)**

Edit `frontend/src/components/topbar.test.tsx` — ajouter (réutiliser `renderTopbar()` déjà défini) :
```tsx
  it('shows the logo and a help link to the docs', async () => {
    renderTopbar()
    await waitFor(() =>
      expect(screen.getByAltText('latch')).toBeInTheDocument(),
    )
    const help = screen.getByRole('link', { name: 'Documentation' })
    expect(help).toHaveAttribute('href', 'https://latch.owlnext.fr/docs')
    expect(help).toHaveAttribute('target', '_blank')
  })
```

- [ ] **Step 2 : Lancer, vérifier l'échec**

Run: `rtk vitest run src/components/topbar.test.tsx`
Expected: FAIL — pas de logo ni de lien doc.

- [ ] **Step 3 : Modifier `topbar.tsx`**

Edit `frontend/src/components/topbar.tsx` :
- Imports : remplacer `import { Settings } from 'lucide-react'` par `import { Settings, CircleHelp } from 'lucide-react'` ; ajouter `import { Logo } from '@/components/logo'` et `import { DOCS_URL } from '@/lib/links'`.
- Le bouton-lien titre « latch » : insérer le logo avant le texte. Remplacer :
  ```tsx
      <Button
        type="button"
        variant="link"
        className="text-lg font-bold"
        onClick={() => {
          router.navigate({ to: '/' })
        }}
      >
        latch
      </Button>
  ```
  par :
  ```tsx
      <Button
        type="button"
        variant="link"
        className="gap-2 text-lg font-bold"
        onClick={() => {
          router.navigate({ to: '/' })
        }}
      >
        <Logo className="size-6" />
        latch
      </Button>
  ```
- Ajouter le bouton « ? » AVANT le cog Settings, dans le `<div className="flex items-center gap-2">` :
  ```tsx
        <Button asChild variant="ghost" size="icon-sm">
          <a
            href={DOCS_URL}
            target="_blank"
            rel="noopener noreferrer"
            aria-label={t('topbar.help')}
          >
            <CircleHelp />
          </a>
        </Button>
  ```
  (juste avant le `<Button … aria-label={t('settings.title')} …>` du cog.)

- [ ] **Step 4 : Lancer la suite topbar (dont le test d'ouverture du Sheet existant)**

Run: `rtk vitest run src/components/topbar.test.tsx`
Expected: PASS (le nouveau test + les tests existants — titre/logout/ouverture Sheet).

- [ ] **Step 5 : Commit**

```bash
git add frontend/src/components/topbar.tsx frontend/src/components/topbar.test.tsx
git commit -m "✨ feat(topbar): logo badge+texte + bouton ? vers la doc"
```

---

## Task 5 : Unlock — logo + titre dynamique

**Files:**
- Modify: `frontend/src/unlock/unlock-page.tsx`
- Modify: `frontend/src/i18n/locales/unlock/en.json`, `frontend/src/i18n/locales/unlock/fr.json`
- Modify (test): `frontend/src/unlock/unlock-page.test.tsx`

**Interfaces:**
- Consumes: `<Logo />`, `useDocumentTitle` (Task 1), nouvelles clés `unlock.page_title_*`.

- [ ] **Step 1 : Ajouter les clés i18n unlock (en + fr)**

Edit `frontend/src/i18n/locales/unlock/en.json` — ajouter (avant le `}` final ; virgule sur la clé précédemment dernière) :
```json
  "unlock.page_title_brand": "{{brand}} — unlock",
  "unlock.page_title_neutral": "Unlock — latch"
```
Edit `frontend/src/i18n/locales/unlock/fr.json` — de même :
```json
  "unlock.page_title_brand": "{{brand}} — déverrouillage",
  "unlock.page_title_neutral": "Déverrouillage — latch"
```

- [ ] **Step 2 : Ajouter/ajuster le test unlock (échoue)**

Edit `frontend/src/unlock/unlock-page.test.tsx` — ajouter un test logo + titre neutre (réutiliser le harness du fichier ; sans brand → titre neutre) :
```tsx
  it('shows the logo and sets the neutral document title', async () => {
    // rendre UnlockPage via le harness du fichier (sans brand renvoyé par /api/public)
    expect(await screen.findByAltText('latch')).toBeInTheDocument()
    await waitFor(() => expect(document.title).toBe('Unlock — latch'))
  })
```
(Adapter au harness existant : si le fichier mocke déjà `/api/public/<slug>`, renvoyer une meta sans `brand_name` pour ce test.)

- [ ] **Step 3 : Lancer, vérifier l'échec**

Run: `rtk vitest run src/unlock/unlock-page.test.tsx`
Expected: FAIL — pas de logo ni de titre.

- [ ] **Step 4 : Modifier `unlock-page.tsx`**

Edit `frontend/src/unlock/unlock-page.tsx` :
- Imports : `import { Logo } from '@/components/logo'` et `import { useDocumentTitle } from '@/hooks/use-document-title'`.
- Dans `UnlockPage`, après les `useState`, ajouter :
  ```tsx
  useDocumentTitle(
    brand ? t('unlock.page_title_brand', { brand }) : t('unlock.page_title_neutral'),
  )
  ```
- Insérer le logo au-dessus de la Card. Remplacer :
  ```tsx
      <Card className="w-full max-w-sm">
  ```
  par :
  ```tsx
      <div className="flex w-full max-w-sm flex-col items-center gap-6">
        <Logo className="size-12" />
        <Card className="w-full">
  ```
  et fermer le nouveau `<div>` juste après `</Card>` :
  ```tsx
        </Card>
      </div>
  ```
  (Le conteneur racine `<div className="flex min-h-svh items-center justify-center bg-background p-4">` reste ; on insère le wrapper centré entre lui et la Card.)

- [ ] **Step 5 : Lancer, vérifier le succès**

Run: `rtk vitest run src/unlock/unlock-page.test.tsx`
Expected: PASS.

- [ ] **Step 6 : Commit**

```bash
git add frontend/src/unlock/unlock-page.tsx frontend/src/unlock/unlock-page.test.tsx frontend/src/i18n/locales/unlock/
git commit -m "✨ feat(unlock): logo + titre de page dynamique (brand)"
```

---

## Task 6 : Vérification finale + mémoire

**Files:**
- Modify: `docs/INDEX.md`, `docs/HANDOFF.md`, `docs/CONVENTIONS.md`, `docs/QUIRKS.md`

- [ ] **Step 1 : Gate complète (depuis `frontend/`)**

Run:
```bash
rtk lint
pnpm typecheck
rtk vitest run --coverage
pnpm build
```
Expected: lint 0 ; typecheck 0 ; tous tests verts ; couverture du code neuf (`logo`, `use-document-title`) ≥ 80 % ; build OK.

- [ ] **Step 2 : Vérifier le favicon bundlé + l'absence de scaffold mort**

Run:
```bash
cd /srv/owlnext/latch/frontend
grep -o 'rel="icon"[^>]*href="[^"]*"' dist/index.html dist/unlock.html
ls public/vite.svg src/assets/react.svg 2>&1 | grep -i "no such\|cannot" && echo "OK: scaffold purgé"
```
Expected: le `href` du favicon pointe un chemin `/assets/...` (réécrit par Vite) dans les DEUX HTML ; `vite.svg` et `react.svg` n'existent plus.

- [ ] **Step 3 : Mettre à jour `docs/CONVENTIONS.md`**

Ajouter :
```markdown
## Logo, titres de page, liens externes (Phase 7 Lot 3)
- Logo : composant `components/logo.tsx` (`<img src={logoUrl} alt="latch">`, importe
  `src/assets/latch-logo.svg`), mutualisé admin + unlock, taille par CSS (`size-6` topbar,
  `size-12` login/unlock).
- Favicon : SVG-only, `<link rel="icon" type="image/svg+xml" href="/src/assets/latch-logo.svg">`
  dans index.html ET unlock.html. JAMAIS de fichier favicon à la racine (le backend ne sert que
  `/assets` → 404 ; cf. QUIRKS). Vite réécrit `/src/assets/...` vers `/assets/<hash>`.
- Titres : hook `hooks/use-document-title.ts` appelé par route. Schéma « Page — latch admin »
  (clés i18n `title.*`).
- Liens externes : centralisés dans `lib/links.ts` (`GITHUB_URL`, `DOCS_URL`), rendus via
  `Button asChild` enveloppant un `<a target="_blank" rel="noopener noreferrer">`.
```

- [ ] **Step 4 : Mettre à jour `docs/QUIRKS.md`**

Ajouter :
```markdown
## Favicon servi via /assets (Phase 7 Lot 3)
Le backend ne sert que `/assets` (mount ServeDir), pas la racine du dist. Un favicon à la racine
(`/favicon.ico`, `/vite.svg`) fait 404 sous `/admin` (bug Phase 4). Solution : référencer le SVG
via `/src/assets/latch-logo.svg` dans le HTML → Vite le bundle sous `/assets/<hash>.svg`, servi.
Stratégie SVG-only assumée (pas de bundle multi-tailles : outil interne noindex).
```

- [ ] **Step 5 : Mettre à jour `docs/INDEX.md`**

Ajouter une ligne :
```markdown
| Phase 7 Lot 3 — Identité visuelle | Logo (favicon SVG + topbar + login + unlock), titres de page dynamiques, largeur admin max-w-6xl, lien GitHub + bouton ? doc | `docs/superpowers/specs/2026-06-25-phase-7-lot-3-identite-visuelle-design.md` · plan associé |
```

- [ ] **Step 6 : Mettre à jour `docs/HANDOFF.md`**

Entrée datée en haut : `Dernière chose faite` (Lot 3 livré : logo partout + titres dynamiques + largeur bornée + liens GitHub/doc), `Trucs en suspens` (Lot 4 = page d'erreur serving `/c` ; merge Lot 1+2+3 groupé à la fin ; le lien doc pointe une URL Phase 8 pas encore en ligne), `Prochaine chose à creuser` (Lot 4), `Notes pour future Claude` (favicon via /assets, Logo mutualisé, lib/links).

- [ ] **Step 7 : Commit**

```bash
git add docs/
git commit -m "📝 docs(phase-7): Lot 3 livré — mémoire (INDEX/HANDOFF/CONVENTIONS/QUIRKS)"
```

---

## Self-Review (effectuée à l'écriture)

- **Couverture du spec** : Logo + favicon SVG (T1) ✓ ; useDocumentTitle (T1) + titres admin (T2) + login (T3) + unlock (T5) ✓ ; largeur max-w-6xl list/detail (T2) ✓ ; bouton GitHub login (T3) ✓ ; bouton « ? » doc topbar (T4) ✓ ; logo topbar badge+texte (T4) ✓ ; logo login (T3) + unlock (T5) ✓ ; purge scaffold (T1) ✓ ; lib/links (T1) ✓ ; i18n keys (T2 admin, T5 unlock) ✓ ; mémoire (T6) ✓.
- **Placeholders** : aucun ; code/edits complets.
- **Cohérence des types** : `Logo({className})` (T1) = usages T3/T4/T5 ; `useDocumentTitle(string)` (T1) = usages T2/T3/T5 ; `GITHUB_URL`/`DOCS_URL` (T1) = usages T3/T4 ; clés `title.*`/`topbar.help`/`login.github` (T2) consommées T3/T4 — T2 doit passer AVANT T3/T4 (ordre respecté). `unlock.page_title_*` (T5) self-contained.
- **Risque connu** : T2 ajoute `topbar.help`/`login.github` (utilisées en T4/T3) — l'ordre T2→T3→T4 garantit leur présence. Les edits login/unlock enveloppent la Card dans un nouveau `<div>` : bien rééquilibrer les balises (souligné dans les steps).
