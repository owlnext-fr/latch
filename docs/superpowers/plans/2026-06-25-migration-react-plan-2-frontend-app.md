# Plan 2 — Frontend admin React (migration Yew → React/Vite) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Construire l'app admin React (Vite + TS + TanStack Router/Query + shadcn/ui) qui consomme l'API `/api/*` du backend Loco et reproduit à l'identique l'UX du contrat §7, servie en statique sous `/admin`.

**Architecture:** SPA pure (pas de SSR). TanStack Query porte tout l'état serveur ; client HTTP 100 % typé via `openapi-fetch` + types générés depuis `openapi.json` (racine repo). Auth = cookie session same-origin : un middleware `openapi-fetch` redirige sur `401` (aucun nouvel endpoint backend). Comportement par page = portage du contrat §7, pas de conception.

**Tech Stack:** Vite · React 18 · TypeScript strict · TanStack Router (code-based) · TanStack Query · openapi-fetch/openapi-typescript · shadcn/ui (Radix, base **stone**, preset `bJfDPe2y`) · Tailwind v4 · react-hook-form + zod · react-i18next · sonner · Vitest + Testing Library + MSW.

## Global Constraints

- **Dossier** : `frontend/` (réutilise `LATCH_SPA_DIST=../frontend/dist`). App **hors workspace Cargo**.
- **Node** : pinné via `frontend/.nvmrc` = `24`. Package manager = **pnpm** (corepack).
- **Vite** : `base: '/admin/'`. **TanStack Router** : `basepath: '/admin'`. Build → `frontend/dist`.
- **API** : paths réels = `openapi.json` à la **racine du repo** (`/api/login`, `/api/logout`, `/api/projects`, `/api/projects/{id}`, `/api/projects/{id}/code`, `/api/projects/{id}/deploy`, `/api/projects/{id}/versions/{n}`, `…/activate`, `…/preview`). **C'est la source de vérité des paths** (le design §5 mentionnait `/api/auth/login` à tort).
- **Auth** : cookie session same-origin, `credentials: 'include'`. Pas de token stocké. Un `401` est un `response.status`, **pas une exception**.
- **Sécu §9 (NON négociable)** : aucune réponse n'expose de hash ; le PIN n'apparaît **qu'au détail** (jamais en liste). Le type `ProjectListItem` n'a **structurellement pas** de champ `pin` (vérifié côté schéma). Ne jamais afficher de PIN dans la liste.
- **Confidentialité (NON négociable)** : **aucun nom de client réel** nulle part (code, tests, fixtures, commentaires). Placeholders fictifs uniquement (`Mon Projet`/`mon-projet`, `ACME`, `demo`).
- **Thème** : preset oklch `bJfDPe2y` appliqué via `shadcn init` (Task 1). **Ne PAS** reconstruire un thème depuis l'ancien front Yew.
- **Commits** : gitmoji + conventionnel (`✨ feat:`, `🐛 fix:`, `✅ test:`, `🧱 chore:`…). Un commit par task.
- **Qualité « terminé »** : `pnpm lint` + `pnpm typecheck` (`tsc --noEmit`) + `pnpm test` (Vitest) + `pnpm build` verts.

### Assets de référence (chemins absolus, disponibles cette session)

- **Thème oklch capturé** : `/tmp/claude-1000/-srv-owlnext-latch/cd7e4da8-e767-43ee-bac8-50e6b57ff4ae/scratchpad/assets/theme.index.css` (backup ; normalement écrit automatiquement par `shadcn init`).
- **Catalogue i18n converti (JSON, interpolation `{{var}}`)** : `…/scratchpad/assets/en.json` et `…/scratchpad/assets/fr.json` (97 clés chacun, clés plates pointées).
- **Contrat OpenAPI** : `/srv/owlnext/latch/openapi.json`.
- **Comportement UX exact par page** : `docs/contrat-deploy.md` §7 (rails par page) — **la référence**.

> `$SCRATCH` ci-dessous = `/tmp/claude-1000/-srv-owlnext-latch/cd7e4da8-e767-43ee-bac8-50e6b57ff4ae/scratchpad`.

---

## File Structure

```
frontend/
  .nvmrc · package.json · pnpm-lock.yaml · .npmrc
  vite.config.ts · tsconfig.json · tsconfig.app.json · tsconfig.node.json
  components.json · eslint.config.js · .prettierrc · index.html
  vitest.config.ts · vitest.setup.ts
  src/
    index.css                # thème oklch (preset bJfDPe2y) + Tailwind
    main.tsx                 # providers: QueryClient, I18next, RouterProvider, <Toaster/>
    router.tsx               # TanStack Router code-based, basepath /admin, guard
    vite-env.d.ts
    api/
      schema.d.ts            # généré (openapi-typescript) — COMMITÉ
      client.ts              # openapi-fetch + middleware 401
    routes/
      login.tsx · list.tsx · detail.tsx
    components/
      project-form.tsx · deploy-panel.tsx
      delete-project-panel.tsx · delete-version-panel.tsx
      pin-field.tsx · copy-button.tsx · locale-switcher.tsx · topbar.tsx
      ui/                    # shadcn (button, input, label, badge, table, sheet, switch, sonner, card)
    hooks/
      use-auth.ts · use-projects.ts
    i18n/
      index.ts · locales/en.json · locales/fr.json
    lib/
      utils.ts              # cn (shadcn) + human_size + public_url
    test/
      msw.ts                # MSW server + handlers helpers
  dist/                      # build → servi par Loco sous /admin (gitignoré)
```

---

## Task 1 : Scaffold `frontend/` — Vite + shadcn (thème) + Tailwind + outillage

**Files:**
- Create: tout `frontend/` (scaffold), `frontend/.nvmrc`, `frontend/.npmrc`, `frontend/vite.config.ts`, `frontend/eslint.config.js`, `frontend/.prettierrc`
- Modify: `.gitignore` (racine), `.dockerignore` (racine)

**Interfaces:**
- Produces : projet Vite buildable ; alias `@` → `src/` ; thème oklch appliqué dans `src/index.css` ; `pnpm` scripts `dev/build/preview/lint/typecheck/format`.

- [ ] **Step 1 : Scaffold via shadcn init (Vite + preset thème)**

Depuis la racine repo. La commande crée `frontend/` (Vite React-TS), applique Tailwind v4, écrit `src/index.css` (thème oklch du preset), `components.json`, `button` + `lib/utils`. Le flag d'env neutralise le faux-positif `ERR_PNPM_ADDING_TO_ROOT` (le template pose un `pnpm-workspace.yaml`).

```bash
cd /srv/owlnext/latch
export npm_config_ignore_workspace_root_check=true
pnpm dlx shadcn@latest init --template vite --preset bJfDPe2y --pointer -y --name frontend --cwd "$PWD/frontend" </dev/null
```

Vérifier : `grep -c oklch frontend/src/index.css` ≥ 50 (thème écrit). Si le scaffold a niché le projet (`frontend/frontend/`), remonter le contenu d'un cran. Supprimer le `pnpm-workspace.yaml` posé par le template Vite **dans `frontend/`** (on ne veut pas de workspace pnpm), et créer `frontend/.npmrc` avec `ignore-workspace-root-check=true`.

Si `shadcn init` échoue (réseau/preset indisponible) : fallback = scaffold Vite manuel (`pnpm create vite frontend --template react-ts`), `pnpm dlx shadcn@latest init --template vite -b radix -y`, puis **copier le thème** depuis `$SCRATCH/assets/theme.index.css` → `frontend/src/index.css`.

- [ ] **Step 2 : Pinner Node + nettoyer**

```bash
echo "24" > frontend/.nvmrc
```

Retirer le `README.md` Vite par défaut si présent. Vérifier `frontend/components.json` : `tailwind.css = "src/index.css"`, `baseColor = "stone"`, alias `@/*`.

- [ ] **Step 3 : `vite.config.ts` — base /admin/, alias, build, proxy dev, vitest**

```ts
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import path from 'node:path'

export default defineConfig({
  base: '/admin/',
  plugins: [react(), tailwindcss()],
  resolve: { alias: { '@': path.resolve(__dirname, './src') } },
  build: { outDir: 'dist', emptyOutDir: true },
  server: {
    proxy: {
      '/api': 'http://127.0.0.1:5150',
      '/_health': 'http://127.0.0.1:5150',
      '/c': 'http://127.0.0.1:5150',
    },
  },
})
```

> `@tailwindcss/vite` est déjà installé par `shadcn init` (Tailwind v4). Si absent : `pnpm add -D @tailwindcss/vite`.

- [ ] **Step 4 : tsconfig strict + alias**

Dans `frontend/tsconfig.app.json` : `"strict": true`, `"noUnusedLocals": true`, `"noUnusedParameters": true`, et `"paths": { "@/*": ["./src/*"] }` + `"baseUrl": "."`. (shadcn init configure déjà l'alias ; vérifier.)

- [ ] **Step 5 : ESLint (a11y/react-hooks) + Prettier**

```bash
cd frontend
pnpm add -D eslint @eslint/js typescript-eslint eslint-plugin-react-hooks eslint-plugin-react-refresh eslint-plugin-jsx-a11y globals prettier
```

`frontend/eslint.config.js` (flat config) :

```js
import js from '@eslint/js'
import globals from 'globals'
import tseslint from 'typescript-eslint'
import reactHooks from 'eslint-plugin-react-hooks'
import reactRefresh from 'eslint-plugin-react-refresh'
import jsxA11y from 'eslint-plugin-jsx-a11y'

export default tseslint.config(
  { ignores: ['dist', 'src/api/schema.d.ts'] },
  {
    files: ['**/*.{ts,tsx}'],
    extends: [js.configs.recommended, ...tseslint.configs.recommended],
    languageOptions: { ecmaVersion: 2022, globals: globals.browser },
    plugins: {
      'react-hooks': reactHooks,
      'react-refresh': reactRefresh,
      'jsx-a11y': jsxA11y,
    },
    rules: {
      ...reactHooks.configs.recommended.rules,
      ...jsxA11y.flatConfigs.recommended.rules,
      'react-refresh/only-export-components': 'off',
    },
  },
)
```

`frontend/.prettierrc` : `{ "semi": false, "singleQuote": true, "trailingComma": "all" }`

- [ ] **Step 6 : Scripts package.json**

```jsonc
"scripts": {
  "dev": "vite",
  "build": "tsc -b && vite build",
  "preview": "vite preview",
  "lint": "eslint .",
  "typecheck": "tsc --noEmit -p tsconfig.app.json",
  "format": "prettier --write src",
  "gen:api": "openapi-typescript ../openapi.json -o src/api/schema.d.ts"
}
```

- [ ] **Step 7 : `.gitignore` + `.dockerignore`**

Racine `.gitignore` : ajouter `/frontend/node_modules`, `/frontend/dist`, `frontend/*.tsbuildinfo`. Retirer toute ancienne entrée Yew (`/frontend/dist` Trunk déjà couvert).
Racine `.dockerignore` : ajouter `frontend/node_modules`, `frontend/dist`.

- [ ] **Step 8 : Vérifier le build à blanc**

```bash
cd frontend && pnpm install && pnpm build && pnpm lint
```
Expected : `dist/` produit (assets préfixés `/admin/`), 0 erreur lint. `pnpm typecheck` OK.

- [ ] **Step 9 : Commit**

```bash
cd /srv/owlnext/latch
git add frontend .gitignore .dockerignore
git commit -m "🧱 chore(frontend): scaffold Vite+React+TS+shadcn (thème bJfDPe2y), Tailwind v4, ESLint a11y"
```

---

## Task 2 : Client API typé — `openapi-typescript` + `openapi-fetch`

**Files:**
- Create: `frontend/src/api/schema.d.ts` (généré, commité), `frontend/src/api/client.ts`
- Modify: `frontend/package.json`

**Interfaces:**
- Produces :
  - `frontend/src/api/client.ts` exporte `const api` (typed `openapi-fetch` client) et `setUnauthorizedHandler(fn: () => void)`.
  - `schema.d.ts` exporte `paths` (consommé par `createClient<paths>`).

- [ ] **Step 1 : Installer + générer le schéma**

```bash
cd frontend
pnpm add openapi-fetch
pnpm add -D openapi-typescript
pnpm gen:api      # lit ../openapi.json → src/api/schema.d.ts
```
Vérifier : `schema.d.ts` contient `export interface paths` et les opérations (`/api/projects`, etc.).

- [ ] **Step 2 : `client.ts` — client + middleware 401 + credentials**

```ts
import createClient, { type Middleware } from 'openapi-fetch'
import type { paths } from './schema'

let onUnauthorized: (() => void) | null = null
export function setUnauthorizedHandler(fn: () => void) {
  onUnauthorized = fn
}

const authMiddleware: Middleware = {
  async onResponse({ response }) {
    if (response.status === 401) onUnauthorized?.()
    return response
  },
}

// baseUrl '' = même origine. credentials include = cookie session same-origin.
export const api = createClient<paths>({ baseUrl: '', credentials: 'include' })
api.use(authMiddleware)
```

> Le handler 401 est branché dans `main.tsx` (Task 3) vers `router.navigate({ to: '/login' })` + reset du cache Query. On évite l'import circulaire en l'injectant via `setUnauthorizedHandler`.

- [ ] **Step 3 : typecheck**

```bash
pnpm typecheck
```
Expected : OK.

- [ ] **Step 4 : Commit**

```bash
git add frontend/src/api frontend/package.json frontend/pnpm-lock.yaml
git commit -m "✨ feat(frontend): client openapi-fetch typé + schema.d.ts généré + middleware 401"
```

---

## Task 3 : App shell — providers, router, i18n, sonner, Query

**Files:**
- Create: `frontend/src/router.tsx`, `frontend/src/i18n/index.ts`, `frontend/src/i18n/locales/en.json`, `frontend/src/i18n/locales/fr.json`, `frontend/src/lib/utils.ts` (compléter), `frontend/src/components/ui/sonner.tsx`
- Modify: `frontend/src/main.tsx`, `frontend/index.html`

**Interfaces:**
- Produces :
  - `router.tsx` exporte `router` (createRouter), avec routes `/login`, `/` (list), `/projects/$id`. Routes définies en code-based ; les composants de page sont importés depuis `src/routes/*` (Tasks 5/6/8) — **stubs minimaux créés ici** pour que le routeur compile, remplacés ensuite.
  - `i18n/index.ts` exporte `i18n` (instance i18next initialisée) ; clés plates via `keySeparator: false`, `nsSeparator: false`.
  - `lib/utils.ts` exporte `cn`, `humanSize(bytes: number): string`, `publicUrl(slug: string): string`.

- [ ] **Step 1 : Dépendances**

```bash
cd frontend
pnpm add @tanstack/react-router @tanstack/react-query react-i18next i18next i18next-browser-languagedetector react-hook-form zod @hookform/resolvers
pnpm dlx shadcn@latest add sonner card -y
```

- [ ] **Step 2 : i18n catalog (port FR/EN)**

Copier les catalogues **déjà convertis** (clés plates, interpolation `{{var}}`) :

```bash
mkdir -p frontend/src/i18n/locales
cp "$SCRATCH/assets/en.json" frontend/src/i18n/locales/en.json
cp "$SCRATCH/assets/fr.json" frontend/src/i18n/locales/fr.json
```

`frontend/src/i18n/index.ts` :

```ts
import i18n from 'i18next'
import { initReactI18next } from 'react-i18next'
import LanguageDetector from 'i18next-browser-languagedetector'
import en from './locales/en.json'
import fr from './locales/fr.json'

i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources: { en: { translation: en }, fr: { translation: fr } },
    fallbackLng: 'en',
    supportedLngs: ['en', 'fr'],
    keySeparator: false,   // clés plates "login.title"
    nsSeparator: false,
    interpolation: { escapeValue: false },
    detection: {
      order: ['localStorage', 'navigator'],
      lookupLocalStorage: 'latch.locale',
      caches: ['localStorage'],
    },
  })

export default i18n
```

> Défaut **EN** (fallback + détection navigateur, persistance `localStorage` clé `latch.locale`) — identique à l'ancien comportement.

- [ ] **Step 3 : `lib/utils.ts` (compléter)**

`shadcn init` a créé `cn`. Ajouter :

```ts
export function humanSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  const kb = bytes / 1024
  if (kb < 1024) return `${kb.toFixed(1)} KB`
  return `${(kb / 1024).toFixed(1)} MB`
}

export function publicUrl(slug: string): string {
  return `${window.location.origin}/c/${slug}`
}
```

- [ ] **Step 4 : `router.tsx` (code-based) + stubs de page**

Créer des stubs `src/routes/{login,list,detail}.tsx` exportant chacun un composant trivial (`export function LoginPage(){return <div/>}` etc.) — remplacés Tasks 5/6/8. Puis :

```tsx
import { createRootRoute, createRoute, createRouter, Outlet } from '@tanstack/react-router'
import { LoginPage } from './routes/login'
import { ListPage } from './routes/list'
import { DetailPage } from './routes/detail'

const rootRoute = createRootRoute({ component: Outlet })
const loginRoute = createRoute({ getParentRoute: () => rootRoute, path: '/login', component: LoginPage })
const listRoute = createRoute({ getParentRoute: () => rootRoute, path: '/', component: ListPage })
const detailRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/projects/$id',
  component: DetailPage,
})

const routeTree = rootRoute.addChildren([loginRoute, listRoute, detailRoute])

export const router = createRouter({ routeTree, basepath: '/admin' })

declare module '@tanstack/react-router' {
  interface Register { router: typeof router }
}
```

- [ ] **Step 5 : `main.tsx` — providers**

```tsx
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { RouterProvider } from '@tanstack/react-router'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { I18nextProvider } from 'react-i18next'
import { Toaster } from '@/components/ui/sonner'
import i18n from '@/i18n'
import { router } from '@/router'
import { setUnauthorizedHandler } from '@/api/client'
import './index.css'

const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } })

setUnauthorizedHandler(() => {
  queryClient.clear()
  router.navigate({ to: '/login' })
})

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <I18nextProvider i18n={i18n}>
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
        <Toaster richColors position="top-right" />
      </QueryClientProvider>
    </I18nextProvider>
  </StrictMode>,
)
```

`index.html` : `<title>latch — admin</title>`, `lang="en"`. Vérifier `<div id="root">`.

- [ ] **Step 6 : build + typecheck + lint**

```bash
pnpm build && pnpm typecheck && pnpm lint
```
Expected : OK (stubs compilent, thème chargé).

- [ ] **Step 7 : Commit**

```bash
git add frontend
git commit -m "✨ feat(frontend): app shell (router TanStack, Query, i18n FR/EN, sonner, providers)"
```

---

## Task 4 : Vitest + MSW harness + leaf components (PinField, CopyButton, LocaleSwitcher) + lib tests

**Files:**
- Create: `frontend/vitest.config.ts`, `frontend/vitest.setup.ts`, `frontend/src/test/msw.ts`, `frontend/src/components/{pin-field,copy-button,locale-switcher}.tsx`, tests `*.test.tsx`
- Modify: `frontend/package.json`

**Interfaces:**
- Consumes : `lib/utils.ts` (humanSize, publicUrl), `api/client.ts`.
- Produces :
  - `<PinField pin={string|null} editable?={boolean} onChange?={(v:string)=>void} disabled?={boolean} />` — masque `••••••`, œil révéler/masquer, bouton copier (mode lecture).
  - `<CopyButton text={string} ariaLabel={string} />` — copie + toast `t('toast.copied')`.
  - `<LocaleSwitcher />` — boutons FR/EN, change `i18n.language`, persiste (i18next-browser-languagedetector le fait).
  - `src/test/msw.ts` exporte `server` (setupServer) + helper `jsonOnce`.

- [ ] **Step 1 : Deps de test**

```bash
cd frontend
pnpm add -D vitest @testing-library/react @testing-library/user-event @testing-library/jest-dom jsdom msw
```

- [ ] **Step 2 : Config Vitest**

`frontend/vitest.config.ts` :

```ts
import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'
import path from 'node:path'

export default defineConfig({
  plugins: [react()],
  resolve: { alias: { '@': path.resolve(__dirname, './src') } },
  test: { environment: 'jsdom', globals: true, setupFiles: ['./vitest.setup.ts'] },
})
```

`frontend/vitest.setup.ts` :

```ts
import '@testing-library/jest-dom/vitest'
import { afterAll, afterEach, beforeAll } from 'vitest'
import { server } from './src/test/msw'

beforeAll(() => server.listen({ onUnhandledRequest: 'error' }))
afterEach(() => server.resetHandlers())
afterAll(() => server.close())
```

`src/test/msw.ts` :

```ts
import { setupServer } from 'msw/node'
export const server = setupServer()
```

Script package.json : `"test": "vitest run"`, `"test:watch": "vitest"`.

> `navigator.clipboard` n'existe pas en jsdom → les tests CopyButton stubberont `navigator.clipboard.writeText` (voir test). `window.location.origin` en jsdom = `http://localhost:3000` (suffisant pour `publicUrl`).

- [ ] **Step 3 : Test `lib/utils` (RED→GREEN, déjà implémenté Task 3)**

`src/lib/utils.test.ts` :

```ts
import { describe, expect, it } from 'vitest'
import { humanSize, publicUrl } from './utils'

describe('humanSize', () => {
  it('formats bytes/KB/MB', () => {
    expect(humanSize(512)).toBe('512 B')
    expect(humanSize(2048)).toBe('2.0 KB')
    expect(humanSize(5 * 1024 * 1024)).toBe('5.0 MB')
  })
})
describe('publicUrl', () => {
  it('builds /c/<slug> on current origin', () => {
    expect(publicUrl('mon-projet-k7Qp2maZ')).toContain('/c/mon-projet-k7Qp2maZ')
  })
})
```

- [ ] **Step 4 : `PinField` + test**

Implémenter `src/components/pin-field.tsx` :
- Prop lecture (`editable` absent/false) : affiche `••••••` si `pin` non révélé, sinon `pin` ; bouton œil (`Eye`/`EyeOff` lucide) `aria-label={t('detail.reveal_pin')}` / `t('detail.hide_pin')` ; `<CopyButton text={pin} ariaLabel={t('detail.copy_pin_aria')} />` si `pin`.
- Prop édition (`editable`) : `<Input>` 6 chiffres, `disabled` si `disabled`, `onChange` remonte la valeur (filtrer non-digits, max 6).

`src/components/pin-field.test.tsx` :

```tsx
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import { PinField } from './pin-field'

describe('PinField (read mode)', () => {
  it('masks the pin until revealed', async () => {
    render(<PinField pin="123456" />)
    expect(screen.getByText('••••••')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: /reveal|révéler/i }))
    expect(screen.getByText('123456')).toBeInTheDocument()
  })
})
```

> i18n dans les tests : envelopper le rendu dans `<I18nextProvider i18n={i18n}>` ou importer `@/i18n` (auto-init, défaut EN). Créer un petit helper `renderWithProviders` dans `src/test/utils.tsx` (I18nextProvider + QueryClientProvider) — réutilisé par toutes les tasks suivantes.

- [ ] **Step 5 : `CopyButton` + test (clipboard stub)**

`copy-button.tsx` : `navigator.clipboard.writeText(text)` → `toast.success(t('toast.copied'))`. Bouton-icône (`Copy` lucide) avec `aria-label={ariaLabel}`.

Test : stub `Object.assign(navigator, { clipboard: { writeText: vi.fn().mockResolvedValue(undefined) } })`, cliquer, asserter `writeText` appelé avec le texte.

- [ ] **Step 6 : `LocaleSwitcher` (pas de test obligatoire)**

Deux boutons `FR`/`EN` ; `onClick` → `i18n.changeLanguage('fr'|'en')`. Le bouton de la langue active porte `aria-pressed`.

- [ ] **Step 7 : run tests**

```bash
pnpm test
```
Expected : tous verts (utils, pin-field, copy-button).

- [ ] **Step 8 : Commit**

```bash
git add frontend
git commit -m "✅ test(frontend): harness Vitest+MSW + PinField/CopyButton/LocaleSwitcher (tests verts)"
```

---

## Task 5 : Route Login + hook auth + MSW test

**Files:**
- Create: `frontend/src/routes/login.tsx` (remplace stub), `frontend/src/hooks/use-auth.ts`, `frontend/src/routes/login.test.tsx`

**Interfaces:**
- Consumes : `api` (client), `router`.
- Produces :
  - `use-auth.ts` exporte `useLogin()` (mutation → `POST /api/login`, body `{user,pass}`) et `useLogout()` (`POST /api/logout`).
  - `LoginPage` : formulaire react-hook-form + zod (user requis, pass requis). Succès → `router.navigate({ to: '/' })`. `401` → message `t('login.error_invalid')`. Bouton busy pendant submit (efface l'erreur précédente au re-submit).

- [ ] **Step 1 : `use-auth.ts`**

```ts
import { useMutation } from '@tanstack/react-query'
import { api } from '@/api/client'

export function useLogin() {
  return useMutation({
    mutationFn: async (body: { user: string; pass: string }) => {
      const { data, error, response } = await api.POST('/api/login', { body })
      if (error || !response.ok) throw new Error(String(response.status))
      return data
    },
  })
}

export function useLogout() {
  return useMutation({
    mutationFn: async () => {
      await api.POST('/api/logout')
    },
  })
}
```

- [ ] **Step 2 : `LoginPage` (TDD : écrire le test d'abord)**

`login.test.tsx` (MSW) :

```tsx
import { http, HttpResponse } from 'msw'
import { server } from '@/test/msw'
// renderWithProviders monte LoginPage dans un routeur mémoire + providers
it('shows error on 401', async () => {
  server.use(http.post('/api/login', () => new HttpResponse(null, { status: 401 })))
  // ... render, fill user/pass, submit, expect t('login.error_invalid') visible
})
it('navigates on success', async () => {
  server.use(http.post('/api/login', () => HttpResponse.json({ ok: true })))
  // ... submit, expect navigation spy / router state '/'
})
```

> Pour tester la navigation : utiliser un `createRouter` de test avec `createMemoryHistory({ initialEntries: ['/admin/login'] })` et asserter `router.state.location.pathname`. Helper dans `src/test/utils.tsx`.

- [ ] **Step 3 : Implémenter `login.tsx`**

Form centré (`min-h-screen grid place-items-center`), `Card` shadcn, titre `t('login.title')`, champs `t('login.user')`/`t('login.pass')`, bouton `t('login.submit')`/`t('login.submitting')`. zod schema `{ user: z.string().min(1), pass: z.string().min(1) }`. `onSubmit` : `error.set(null)` puis `mutate`, `onError` → afficher `t('login.error_invalid')`, `onSuccess` → navigate `/`. `<LocaleSwitcher>` en coin.

- [ ] **Step 4 : run tests + lint + typecheck**

```bash
pnpm test && pnpm typecheck && pnpm lint
```

- [ ] **Step 5 : Commit**

```bash
git add frontend
git commit -m "✨ feat(frontend): route login (RHF+zod, 401→erreur, succès→liste) + useLogin/useLogout"
```

---

## Task 6 : Route Liste + hooks projets (Query) + topbar + MSW test

**Files:**
- Create: `frontend/src/routes/list.tsx` (remplace stub), `frontend/src/hooks/use-projects.ts`, `frontend/src/components/topbar.tsx`, `frontend/src/routes/list.test.tsx`

**Interfaces:**
- Consumes : `api`, contrat §7 (liste), `PinField`/`CopyButton`/`LocaleSwitcher`, `ProjectForm` (Task 7 — importer ; créer un stub `project-form.tsx` ici si pas encore fait, remplacé Task 7).
- Produces :
  - `use-projects.ts` exporte : `useProjects()` (query `GET /api/projects`), `useProject(id)` (query `GET /api/projects/{id}`), et mutations `useCreateProject`, `useUpdateProject`, `useDeleteProject`, `useSetCode`, `useClearCode`, `useDeploy`, `useActivateVersion`, `useDeleteVersion` — **toutes** invalident `['projects']` (+ `['project', id]`) et émettent le toast `t('toast.*')` adéquat sur succès, `toast.error` sur échec.
  - `<Topbar>` : titre `latch` cliquable → `/`, `<LocaleSwitcher>`, bouton logout (`useLogout` → navigate `/login`).

- [ ] **Step 1 : `use-projects.ts` (query + mutations + invalidation + toasts)**

Modèle d'une mutation (répéter pour chacune, avec la bonne clé toast) :

```ts
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import { api } from '@/api/client'

export function useProjects() {
  return useQuery({
    queryKey: ['projects'],
    queryFn: async () => {
      const { data, error } = await api.GET('/api/projects')
      if (error) throw new Error('list')
      return data
    },
  })
}

export function useCreateProject() {
  const qc = useQueryClient()
  const { t } = useTranslation()
  return useMutation({
    mutationFn: async (body: /* CreateProjectReq */ {
      name: string; code_enabled?: boolean; pin?: string | null; brand_name?: string | null
    }) => {
      const { data, error } = await api.POST('/api/projects', { body })
      if (error) throw new Error('create')
      return data
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['projects'] })
      toast.success(t('toast.project_created'))
    },
    onError: () => toast.error(t('error.server', { code: '' })),
  })
}
// … useUpdateProject (PUT, toast.project_updated), useDeleteProject (DELETE, toast.project_deleted),
//    useSetCode (POST …/code, body {pin}), useClearCode (DELETE …/code),
//    useDeploy (POST …/deploy, body {html, activate}, toast.version_deployed),
//    useActivateVersion (POST …/versions/{n}/activate, toast.version_activated),
//    useDeleteVersion (DELETE …/versions/{n}, toast.version_deleted).
```

> Types des bodies : dériver de `schema.d.ts` via `paths['/api/projects']['post']['requestBody']['content']['application/json']` plutôt que retaper à la main quand pratique. Sinon types inline ci-dessus (suffisants).

- [ ] **Step 2 : Test liste (MSW) — écrire d'abord**

`list.test.tsx` :
- `GET /api/projects` → 2 projets (un `code_enabled:true`, un `false`) → asserter noms affichés, badge `t('list.badge_code_on')` (PIN requis) et `t('list.badge_free')` (libre) présents, **PIN jamais affiché** (le DTO liste n'a pas de pin).
- liste vide → état vide `t('list.empty')` + bouton `t('list.create_first')`.

- [ ] **Step 3 : Implémenter `list.tsx` (contrat §7 « Liste »)**

`<Topbar>` + intro `t('list.intro')`. `Table` shadcn : colonnes `t('list.col_name')`, `t('list.col_url')` (URL publique `publicUrl(slug)` + `<CopyButton ariaLabel={t('list.copy_url_aria')}>`), `t('list.col_code')` (badge **vert** `PIN requis` si `code_enabled`, **orange** `Libre` sinon — classes Tailwind `bg-green-600`/`bg-amber-500` ou variantes `Badge`), `t('list.col_version')` (n° version active + nb versions ; via `active_version_id` — afficher `common.dash` si null). Clic ligne → `router.navigate({ to: '/projects/$id', params: { id } })`. Bouton `t('common.new_project')` (haut droite) ouvre `<ProjectForm open mode="create">`. État vide conçu (`t('list.empty')`).

> **Accessibilité** : ligne cliquable = `<TableRow>` avec `onClick` + `role` adéquat, OU bouton dans la cellule nom. Pas de `<a onclick>` sans href. Boutons-icône avec `aria-label`.

- [ ] **Step 4 : tests + lint + typecheck + build**

```bash
pnpm test && pnpm typecheck && pnpm lint && pnpm build
```

- [ ] **Step 5 : Commit**

```bash
git add frontend
git commit -m "✨ feat(frontend): route liste (table, badges accès colorés, état vide) + hooks Query + topbar"
```

---

## Task 7 : `ProjectForm` (side-panel créer/éditer) + validation test

**Files:**
- Create/replace: `frontend/src/components/project-form.tsx`, `frontend/src/components/project-form.test.tsx`
- Add shadcn: `sheet`, `switch`, `input`, `label`

**Interfaces:**
- Consumes : `useCreateProject`, `useUpdateProject`, `useSetCode`, `useClearCode`, `PinField`.
- Produces : `<ProjectForm open={boolean} mode={'create'|'edit'} project?={ProjectDetail} onOpenChange={(b)=>void} />`.

- [ ] **Step 1 : shadcn components**

```bash
cd frontend && pnpm dlx shadcn@latest add sheet switch input label -y
```

- [ ] **Step 2 : Test validation (écrire d'abord)**

`project-form.test.tsx` :
- créer, submit vide → message `t('form.err_name')` ; pas d'appel réseau.
- `code_enabled` ON + PIN à 5 chiffres → `t('form.err_pin')`.
- toggle `code_enabled` OFF → champ PIN **présent mais `disabled`** (asserter `toBeDisabled()`).

- [ ] **Step 3 : Implémenter (contrat §7 « Créer / éditer »)**

`<Sheet open onOpenChange>` (Radix gère scrim/Escape/focus-trap). Titre `t('form.title_create')`/`t('form.title_edit')`. Champs :
- **Nom** (`t('form.name')`, requis) + helper `t('form.name_help')`.
- **Slug** (`t('form.slug')`) : affiché **lecture seule** ; en création, placeholder « (auto) » (pas encore de slug) ; en édition, `disabled` avec la valeur ; helper `t('form.slug_help')`.
- **Code activé** (`t('form.code')`) : `<Switch>` (défaut **ON** en création) + helper `t('form.code_help')`.
- **PIN** (`t('form.pin')`) : `<PinField editable disabled={!codeEnabled}>` — **toujours rendu**, `disabled` quand code OFF (pas de saut de layout) ; bouton **régénérer** (`t('common.regenerate')`, génère 6 chiffres aléatoires) ; helper `t('form.pin_help')`. PIN auto-généré à l'ouverture en création.
- **Brand name** (`t('form.brand')`, optionnel) + helper `t('form.brand_help')`.

Validation zod : `name` non vide ; si `code_enabled` → `pin` = exactement 6 chiffres (`/^\d{6}$/`). Reset des champs **à chaque ouverture** (`useEffect` sur `open`).

Submit :
- **create** : `useCreateProject({ name, code_enabled, pin: code_enabled ? pin : null, brand_name })`.
- **edit** : `useUpdateProject({ id, name, brand_name })` ; puis réconcilier le code : si `code_enabled` passé ON → `useSetCode({ id, pin })` ; si passé OFF → `useClearCode({ id })`. (Le slug n'est jamais modifié.)
Fermer le panel sur succès (`onOpenChange(false)`). Les toasts viennent des mutations.

- [ ] **Step 4 : tests + lint + typecheck**

```bash
pnpm test && pnpm typecheck && pnpm lint
```

- [ ] **Step 5 : Commit**

```bash
git add frontend
git commit -m "✨ feat(frontend): ProjectForm side-panel (créer/éditer, PIN disabled si code off, slug RO, validation zod)"
```

---

## Task 8 : Route Détail + DeployPanel + panels danger + MSW smoke

**Files:**
- Create/replace: `frontend/src/routes/detail.tsx`, `frontend/src/components/deploy-panel.tsx`, `frontend/src/components/delete-project-panel.tsx`, `frontend/src/components/delete-version-panel.tsx`, `frontend/src/routes/detail.test.tsx`

**Interfaces:**
- Consumes : `useProject(id)`, mutations (deploy/activate/deleteVersion/deleteProject), `ProjectForm`, `PinField`, `CopyButton`.
- Produces : `DetailPage` (lecture seule) ; `<DeployPanel projectId open onOpenChange>` ; `<DeleteProjectPanel project open onOpenChange>` ; `<DeleteVersionPanel projectId version open onOpenChange>`.

- [ ] **Step 1 : Test détail (MSW) — écrire d'abord**

`detail.test.tsx` : `GET /api/projects/1` → projet `code_enabled:true`, pin `"123456"`, 2 versions (une active). Asserter :
- carte **Accès public** : URL publique + bouton copier ; PIN **masqué** `••••••` (PinField) — révélable.
- carte **Versions** : 2 lignes, badge `t('common.active')` sur l'active.
- actions Modifier / Déployer / Supprimer présentes (boutons).

- [ ] **Step 2 : `deploy-panel.tsx` (dropzone)**

`<Sheet>` titre `t('deploy.title')`. **Dropzone** : zone `onDragOver`/`onDrop` (preventDefault) + `<input type="file" accept="text/html,.html" hidden>` piloté par un clic sur la zone (ref). Texte `t('deploy.dropzone_idle')` / `t('deploy.dropzone_hover')` (état dragover) ; fichier choisi → `t('deploy.file_chosen', { name, size: humanSize(file.size) })`. Lire le fichier via `file.text()`. Case `t('deploy.activate')` (+ helper `t('deploy.activate_help')`). Bouton `t('deploy.btn')`/`t('deploy.deploying')`. Erreurs : pas de fichier → `t('deploy.err_no_file')` ; échec lecture → `t('deploy.err_read')`. Submit → `useDeploy({ id, html, activate })`. Reset à l'ouverture.

- [ ] **Step 3 : panels danger**

`delete-project-panel.tsx` : `<Sheet>` (variant danger visuel — bordure/titre destructive). Titre `t('danger.del_project_title', { name })`, intro + liste `li1/li2 (count=versions.length)/li3`. Bouton destructif `t('danger.del_project_confirm')`/`t('danger.deleting')` → `useDeleteProject(id)` → sur succès `router.navigate({ to: '/' })`.
`delete-version-panel.tsx` : titre `t('danger.del_version_title', { n })`, intro `t('danger.del_version_intro')`, bouton `t('danger.del_version_confirm')` → `useDeleteVersion({ id, n })`. (Le backend refuse la version active en 400 → le bouton de suppression n'est pas proposé sur la ligne active ; en cas de 400 quand même, `toast.error`.)

- [ ] **Step 4 : `detail.tsx` (contrat §7 « Détail », lecture seule)**

`<Topbar>` + breadcrumb `t('detail.back')` (`<button>` → `/`). Intro `t('detail.intro')`. État `useProject(id)` (loading/erreur).
- **Accès public** (`Card`, `t('detail.access_title')`) : URL publique (`publicUrl(slug)`, lecture seule) + `<CopyButton ariaLabel={t('detail.copy_url_aria')}>`. Si `code_enabled` : `t('detail.code_label')` + `<PinField pin={pin}>` (masqué, révélable, copier). Sinon : `t('detail.free_access')`.
- **Configuration** (`Card`, `t('detail.config_title')`) : nom de marque (`t('detail.brand_label')` → `brand_name` ou `common.dash`), état code (`t('detail.code_on')`/`t('detail.code_off')`).
- **Actions** (haut droite) : `t('common.edit')` (ouvre `<ProjectForm mode="edit" project={…}>`), `t('common.deploy')` (ouvre `<DeployPanel>`), `t('common.delete')` (ouvre `<DeleteProjectPanel>`).
- **Versions** (`Card`, `t('detail.versions_title')`) : table `#`/`Date`/`Statut`. Par ligne : badge `t('common.active')` si active ; bouton **activer** (`aria-label={t('detail.activate_aria')}`, masqué si déjà active) → `useActivateVersion` ; **prévisualiser** = vrai lien `<a href={previewUrl} target="_blank" rel="noopener">` avec `aria-label={t('detail.preview_aria')}` (URL = `/api/projects/{id}/versions/{n}/preview`) ; **supprimer** (`aria-label={t('detail.delete_aria')}`, masqué si active) → `<DeleteVersionPanel>`. État vide versions : bloc premier déploiement mis en avant.

> `previewUrl(id, n)` = `` `/api/projects/${id}/versions/${n}/preview` `` (chemin absolu, même origine ; sert le HTML `no-store` derrière la session).

- [ ] **Step 5 : tests + lint + typecheck + build**

```bash
pnpm test && pnpm typecheck && pnpm lint && pnpm build
```

- [ ] **Step 6 : Commit**

```bash
git add frontend
git commit -m "✨ feat(frontend): route détail (lecture seule) + DeployPanel dropzone + panels danger"
```

---

## Task 9 : Polish final, a11y, validation locale complète

**Files:**
- Modify: divers `frontend/src/**` (intros, aria, focus), `frontend/src/routes/detail.tsx` (états vides)

- [ ] **Step 1 : Passe a11y + intros**

Vérifier : tout élément cliquable navigant = `<button>` ou `<a href>` (jamais `<a onclick>` sans href) ; tous les boutons-icône ont `aria-label` (via `t!`) ; les `Sheet` (Radix) gèrent focus-trap/Escape d'office ; chaque page a son intro (`list.intro`, `detail.intro`). Lancer `pnpm lint` (jsx-a11y) → 0 warning.

- [ ] **Step 2 : Réactivité i18n**

Vérifier que changer la langue via `<LocaleSwitcher>` re-render toute l'UI (react-i18next `useTranslation` abonne déjà les composants — pas de piège « use_locale » comme en Yew). Test manuel rapide en `pnpm dev` si possible.

- [ ] **Step 3 : Validation complète**

```bash
cd frontend
pnpm lint && pnpm typecheck && pnpm test && pnpm build
```
Expected : tout vert ; `dist/` produit.

- [ ] **Step 4 : Sanity confidentialité + sécu**

```bash
grep -rniE "pin" frontend/src/routes/list.tsx   # AUCUN affichage de PIN en liste
```
Vérifier qu'aucun nom de client réel n'apparaît (placeholders fictifs uniquement) dans le code et les fixtures de test.

- [ ] **Step 5 : Commit**

```bash
git add frontend
git commit -m "💄 feat(frontend): polish a11y + intros + validation locale complète (lint/type/test/build verts)"
```

---

## Critères de sortie (Plan 2)

- `frontend/` : `pnpm build` produit `dist/` (assets préfixés `/admin/`) ; `pnpm lint` + `pnpm typecheck` + `pnpm test` (Vitest + MSW) verts.
- `src/api/schema.d.ts` généré depuis `openapi.json` et commité.
- Toutes les pages du contrat §7 portées : login, liste (badges colorés, état vide), détail (lecture seule, PIN masqué, versions), side-panels créer/éditer/déployer/danger, sélecteur FR/EN, toasts sur toutes les actions, dropzone, PIN disabled/slug RO.
- Invariants §9 préservés (PIN absent de la liste, pas de hash) ; aucun nom de client réel.
- **NB** : CI/Docker (stage Node, pistes) + smoke e2e Playwright + alignement docs = **Plan 3**. La validation navigateur end-to-end avec backend se fait en Plan 3.

## Self-review (anti-placeholder)

Avant de lancer l'exécution : ce plan référence des assets par chemin absolu (`$SCRATCH/assets/*`) et des commandes reproductibles (`shadcn init --preset`, `gen:api`). Les types des bodies de mutation sont soit inline, soit dérivés de `schema.d.ts`. Les clés i18n utilisées (`login.*`, `list.*`, `detail.*`, `form.*`, `deploy.*`, `danger.*`, `common.*`, `toast.*`, `error.*`) existent toutes dans `en.json`/`fr.json` (97 clés). Les paths API correspondent à `openapi.json`.
