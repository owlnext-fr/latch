# Phase 7 — Lot 1 : Fondations transverses (i18n + thème) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rendre l'i18n du frontend auto-découvert (déposer un JSON = ajouter une langue, sans toucher au code) et monter un `ThemeProvider` (défaut `system`, persistance, anti-FOUC) sur le bundle admin.

**Architecture:** Une fonction pure `parseLocales` transforme le résultat de `import.meta.glob` (résolu au build par Vite, donc séparation public/admin garantie) en `{ resources, locales }`. L'admin et l'unlock l'appellent chacun sur leur dossier (`locales/admin/` / `locales/unlock/`), gardant leurs instances i18next distinctes. Le thème repose sur `next-themes` (déjà en dépendance) monté dans `src/main.tsx`, plus un script inline anti-FOUC dans `index.html`.

**Tech Stack:** React 19, Vite 8, TypeScript, i18next ^26 / react-i18next ^17 / i18next-browser-languagedetector ^8, next-themes ^0.4.6, Vitest ^4 (jsdom, `globals: true`, alias `@` → `src`).

## Global Constraints

- **Confidentialité (NON-NÉGOCIABLE)** : aucun nom de client réel dans le code/tests/fixtures. Utiliser des placeholders fictifs (`ACME`, `Mon Projet`).
- **Bundle public minimal** : l'unlock (`unlock.html` → `src/unlock/*`) ne doit JAMAIS embarquer les clés admin. Garanti par deux globs distincts résolus au build.
- **Zéro régression i18n** : les 108 clés admin + 8 clés unlock doivent continuer à résoudre. L'export par défaut `i18n` des modules `@/i18n` et `./i18n` (unlock) reste inchangé (9 importeurs en dépendent).
- **Couverture** : SonarCloud `new_coverage ≥ 80 %` sur le code neuf (`parseLocales` porte la couverture).
- **Vérif doc API** : i18next 26 / react-i18next 17 / next-themes 0.4 — APIs utilisées (`init`, `createInstance`, `use`, `supportedLngs`, `ThemeProvider` props) stables sur ces majeures ; vérifier via Context7/doc si un comportement surprend.
- **Périmètre thème** : admin + login uniquement (`src/main.tsx`). `src/unlock/main.tsx` n'est PAS touché côté thème (unlock reste clair-only).
- **Commandes** : depuis `frontend/`. `pnpm test` (Vitest), `pnpm lint`, `pnpm typecheck`, `pnpm build`. Préfixer les commandes par `rtk` quand pertinent.

---

## File Structure

| Fichier | Responsabilité | Action |
|---|---|---|
| `frontend/src/i18n/available-locales.ts` | Fonction pure `parseLocales` + types `LocaleInfo`/`ParsedLocales` | **Créer** |
| `frontend/src/i18n/available-locales.test.ts` | Tests unitaires de `parseLocales` | **Créer** |
| `frontend/src/i18n/locales/admin/en.json` | 108 clés admin EN + `_meta` | **Déplacer** depuis `locales/en.json` + éditer |
| `frontend/src/i18n/locales/admin/fr.json` | 108 clés admin FR + `_meta` | **Déplacer** depuis `locales/fr.json` + éditer |
| `frontend/src/i18n/locales/unlock/en.json` | 8 clés unlock EN + `_meta` | **Créer** (depuis inline) |
| `frontend/src/i18n/locales/unlock/fr.json` | 8 clés unlock FR + `_meta` | **Créer** (depuis inline) |
| `frontend/src/i18n/index.ts` | Init i18next admin via glob, export `i18n` (défaut) + `locales` | **Modifier** |
| `frontend/src/i18n/i18n.test.ts` | Test comportement i18n admin (résolution, switch, `locales`) | **Créer** |
| `frontend/src/unlock/i18n.ts` | Init i18next unlock via glob, instance séparée | **Modifier** |
| `frontend/src/unlock/i18n.test.ts` | Test i18n unlock (interpolation, fallback) | **Créer** |
| `frontend/src/main.tsx` | Wrap `<ThemeProvider>` (admin) | **Modifier** |
| `frontend/index.html` | Script inline anti-FOUC | **Modifier** |
| `frontend/src/theme.test.tsx` | Smoke test `ThemeProvider` | **Créer** |
| `docs/INDEX.md`, `docs/HANDOFF.md`, `docs/CONVENTIONS.md`, `docs/QUIRKS.md` | Mémoire projet | **Modifier** |

---

## Task 1 : `parseLocales` — fonction pure de découverte

**Files:**
- Create: `frontend/src/i18n/available-locales.ts`
- Test: `frontend/src/i18n/available-locales.test.ts`

**Interfaces:**
- Consumes: rien (fonction pure).
- Produces:
  ```ts
  export type LocaleInfo = { code: string; name: string; flag: string }
  export type ParsedLocales = {
    resources: Record<string, { translation: Record<string, string> }>
    locales: LocaleInfo[]
  }
  export function parseLocales(
    glob: Record<string, { default: Record<string, unknown> }>,
  ): ParsedLocales
  ```

- [ ] **Step 1 : Écrire les tests qui échouent**

Create `frontend/src/i18n/available-locales.test.ts` :
```ts
import { describe, expect, it, vi } from 'vitest'
import { parseLocales } from './available-locales'

const fakeGlob = {
  './locales/admin/fr.json': {
    default: { _meta: { name: 'Français', flag: 'FR' }, 'login.title': 'latch — admin' },
  },
  './locales/admin/en.json': {
    default: { _meta: { name: 'English', flag: 'GB' }, 'login.title': 'latch — admin' },
  },
}

describe('parseLocales', () => {
  it('derives the language code from the filename', () => {
    const { locales } = parseLocales(fakeGlob)
    expect(locales.map((l) => l.code).sort()).toEqual(['en', 'fr'])
  })

  it('sorts locales by code (stable order)', () => {
    const { locales } = parseLocales(fakeGlob)
    expect(locales[0].code).toBe('en')
    expect(locales[1].code).toBe('fr')
  })

  it('exposes name and flag from _meta', () => {
    const { locales } = parseLocales(fakeGlob)
    expect(locales).toEqual([
      { code: 'en', name: 'English', flag: 'GB' },
      { code: 'fr', name: 'Français', flag: 'FR' },
    ])
  })

  it('strips _meta from translation resources', () => {
    const { resources } = parseLocales(fakeGlob)
    expect(resources.en.translation).toEqual({ 'login.title': 'latch — admin' })
    expect('_meta' in resources.en.translation).toBe(false)
  })

  it('falls back to CODE/CODE and warns when _meta is missing', () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {})
    const { locales } = parseLocales({
      './locales/admin/de.json': { default: { 'login.title': 'x' } },
    })
    expect(locales[0]).toEqual({ code: 'de', name: 'DE', flag: 'DE' })
    expect(warn).toHaveBeenCalledOnce()
    warn.mockRestore()
  })

  it('falls back per-field when _meta is partial', () => {
    const { locales } = parseLocales({
      './locales/admin/es.json': { default: { _meta: { name: 'Español' } } },
    })
    expect(locales[0]).toEqual({ code: 'es', name: 'Español', flag: 'ES' })
  })
})
```

- [ ] **Step 2 : Lancer les tests, vérifier l'échec**

Run: `rtk vitest run src/i18n/available-locales.test.ts`
Expected: FAIL — `parseLocales` introuvable (module non créé).

- [ ] **Step 3 : Implémenter `parseLocales`**

Create `frontend/src/i18n/available-locales.ts` :
```ts
export type LocaleMeta = { name: string; flag: string }
export type LocaleInfo = { code: string } & LocaleMeta
export type ParsedLocales = {
  resources: Record<string, { translation: Record<string, string> }>
  locales: LocaleInfo[]
}

type GlobModule = { default: Record<string, unknown> }

function codeFromPath(filePath: string): string {
  return filePath.split('/').pop()!.replace(/\.json$/, '')
}

function normalizeMeta(meta: unknown, code: string): LocaleMeta {
  const fallback: LocaleMeta = { name: code.toUpperCase(), flag: code.toUpperCase() }
  if (!meta || typeof meta !== 'object') {
    console.warn(`[i18n] locale "${code}" has no _meta; falling back to "${fallback.name}"`)
    return fallback
  }
  const m = meta as Record<string, unknown>
  return {
    name: typeof m.name === 'string' && m.name ? m.name : fallback.name,
    flag: typeof m.flag === 'string' && m.flag ? m.flag : fallback.flag,
  }
}

/**
 * Transforme le résultat d'un `import.meta.glob('...', { eager: true })` de fichiers
 * locale JSON en ressources i18next + métadonnées de langue. Fonction pure (la
 * découverte glob, primitive Vite, reste chez l'appelant) → unitairement testable.
 */
export function parseLocales(glob: Record<string, GlobModule>): ParsedLocales {
  const resources: ParsedLocales['resources'] = {}
  const locales: LocaleInfo[] = []

  for (const [filePath, mod] of Object.entries(glob)) {
    const code = codeFromPath(filePath)
    const { _meta, ...translation } = mod.default
    resources[code] = { translation: translation as Record<string, string> }
    locales.push({ code, ...normalizeMeta(_meta, code) })
  }

  locales.sort((a, b) => a.code.localeCompare(b.code))
  return { resources, locales }
}
```

- [ ] **Step 4 : Lancer les tests, vérifier le succès**

Run: `rtk vitest run src/i18n/available-locales.test.ts`
Expected: PASS (6 tests).

- [ ] **Step 5 : Commit**

```bash
rtk git add frontend/src/i18n/available-locales.ts frontend/src/i18n/available-locales.test.ts
rtk git commit -m "✨ feat(i18n): parseLocales — découverte pure des locales + _meta"
```

---

## Task 2 : Migration des locales admin + rewire `index.ts`

**Files:**
- Move: `frontend/src/i18n/locales/en.json` → `frontend/src/i18n/locales/admin/en.json`
- Move: `frontend/src/i18n/locales/fr.json` → `frontend/src/i18n/locales/admin/fr.json`
- Modify: les deux fichiers (ajout `_meta`)
- Modify: `frontend/src/i18n/index.ts`
- Test: `frontend/src/i18n/i18n.test.ts`

**Interfaces:**
- Consumes: `parseLocales` (Task 1).
- Produces: `export default i18n` (inchangé) **et** `export const locales: LocaleInfo[]` depuis `@/i18n`. Le sélecteur du Lot 2 consommera `locales`.

- [ ] **Step 1 : Écrire le test qui échoue**

Create `frontend/src/i18n/i18n.test.ts` :
```ts
import { describe, expect, it, beforeEach } from 'vitest'
import i18n, { locales } from '@/i18n'

describe('admin i18n', () => {
  beforeEach(async () => {
    await i18n.changeLanguage('en')
  })

  it('resolves flat keys in English', () => {
    expect(i18n.t('common.cancel')).toBe('Cancel')
  })

  it('switches to French', async () => {
    await i18n.changeLanguage('fr')
    expect(i18n.t('common.cancel')).toBe('Annuler')
  })

  it('exposes discovered locales with _meta', () => {
    expect(locales).toEqual([
      { code: 'en', name: 'English', flag: 'GB' },
      { code: 'fr', name: 'Français', flag: 'FR' },
    ])
  })

  it('derives supportedLngs from discovered locales', () => {
    expect(i18n.options.supportedLngs).toContain('en')
    expect(i18n.options.supportedLngs).toContain('fr')
  })

  it('does not expose _meta as a translation key', () => {
    expect(i18n.t('_meta')).toBe('_meta')
  })
})
```

- [ ] **Step 2 : Lancer le test, vérifier l'échec**

Run: `rtk vitest run src/i18n/i18n.test.ts`
Expected: FAIL — `locales` non exporté par `@/i18n`.

- [ ] **Step 3 : Déplacer les fichiers locale**

```bash
cd frontend
mkdir -p src/i18n/locales/admin
git mv src/i18n/locales/en.json src/i18n/locales/admin/en.json
git mv src/i18n/locales/fr.json src/i18n/locales/admin/fr.json
```

- [ ] **Step 4 : Ajouter `_meta` à `admin/en.json`**

Edit `frontend/src/i18n/locales/admin/en.json` — insérer la clé `_meta` juste après la première accolade :
```json
{
  "_meta": { "name": "English", "flag": "GB" },
  "common.loading": "Loading…",
```
(Le reste du fichier est inchangé. `GB` est un choix éditorial — modifiable dans ce seul fichier.)

- [ ] **Step 5 : Ajouter `_meta` à `admin/fr.json`**

Edit `frontend/src/i18n/locales/admin/fr.json` — insérer juste après la première accolade :
```json
{
  "_meta": { "name": "Français", "flag": "FR" },
```
(Garder la 2ᵉ ligne existante du fichier juste après.)

- [ ] **Step 6 : Réécrire `index.ts`**

Replace tout le contenu de `frontend/src/i18n/index.ts` par :
```ts
import i18n from 'i18next'
import { initReactI18next } from 'react-i18next'
import LanguageDetector from 'i18next-browser-languagedetector'
import { parseLocales } from './available-locales'

const { resources, locales } = parseLocales(
  import.meta.glob('./locales/admin/*.json', { eager: true }),
)

export { locales }

i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources,
    fallbackLng: 'en',
    supportedLngs: locales.map((l) => l.code),
    keySeparator: false, // clés plates "login.title"
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

- [ ] **Step 7 : Lancer le test ciblé, vérifier le succès**

Run: `rtk vitest run src/i18n/i18n.test.ts`
Expected: PASS (5 tests).

- [ ] **Step 8 : Lancer toute la suite Vitest (non-régression des 108 clés)**

Run: `rtk vitest run`
Expected: PASS — tous les tests existants (composants/routes qui consomment `i18n`) restent verts.

- [ ] **Step 9 : Commit**

```bash
rtk git add frontend/src/i18n/
rtk git commit -m "♻️ refactor(i18n): admin locales auto-découvertes (glob + _meta)"
```

---

## Task 3 : Migration des locales unlock + rewire `unlock/i18n.ts`

**Files:**
- Create: `frontend/src/i18n/locales/unlock/en.json`
- Create: `frontend/src/i18n/locales/unlock/fr.json`
- Modify: `frontend/src/unlock/i18n.ts`
- Test: `frontend/src/unlock/i18n.test.ts`

**Interfaces:**
- Consumes: `parseLocales` (Task 1) via `@/i18n/available-locales`.
- Produces: `export default instance` (inchangé — instance i18next séparée). Importé par `src/unlock/main.tsx` et `src/unlock/unlock-page.test.tsx`.

- [ ] **Step 1 : Écrire le test qui échoue**

Create `frontend/src/unlock/i18n.test.ts` :
```ts
import { describe, expect, it, beforeEach } from 'vitest'
import i18n from './i18n'

describe('unlock i18n', () => {
  beforeEach(async () => {
    await i18n.changeLanguage('en')
  })

  it('interpolates the brand placeholder', () => {
    expect(i18n.t('unlock.title_brand', { brand: 'ACME' })).toBe('Prototype prepared for ACME')
  })

  it('switches to French', async () => {
    await i18n.changeLanguage('fr')
    expect(i18n.t('unlock.submit')).toBe('Déverrouiller')
  })

  it('does not expose _meta as a translation key', () => {
    expect(i18n.t('_meta')).toBe('_meta')
  })
})
```

- [ ] **Step 2 : Lancer le test, vérifier l'échec**

Run: `rtk vitest run src/unlock/i18n.test.ts`
Expected: PASS pour l'interpolation/switch (le catalogue inline marche encore) — MAIS le test `_meta` passe aussi trivialement. **Note** : ce test passe avant refactor ; il sert de filet de non-régression. Si tu veux un vrai red d'abord, ajoute temporairement une assertion sur la source glob ; sinon, considère Step 2 comme baseline verte et procède au refactor (Steps 3-5), le test devant rester vert après.

- [ ] **Step 3 : Créer `locales/unlock/en.json`**

Create `frontend/src/i18n/locales/unlock/en.json` :
```json
{
  "_meta": { "name": "English", "flag": "GB" },
  "unlock.title_brand": "Prototype prepared for {{brand}}",
  "unlock.title_neutral": "Protected prototype",
  "unlock.instructions": "An access code was shared with this link. Enter it to unlock the prototype.",
  "unlock.pin_label": "Access code",
  "unlock.submit": "Unlock",
  "unlock.error_wrong": "Incorrect code.",
  "unlock.error_throttled": "Too many attempts. Please try again in a moment.",
  "unlock.error_generic": "Something went wrong. Please try again."
}
```

- [ ] **Step 4 : Créer `locales/unlock/fr.json`**

Create `frontend/src/i18n/locales/unlock/fr.json` :
```json
{
  "_meta": { "name": "Français", "flag": "FR" },
  "unlock.title_brand": "Prototype préparé pour {{brand}}",
  "unlock.title_neutral": "Prototype protégé",
  "unlock.instructions": "Un code d'accès vous a été transmis avec ce lien. Saisissez-le pour déverrouiller le prototype.",
  "unlock.pin_label": "Code d'accès",
  "unlock.submit": "Déverrouiller",
  "unlock.error_wrong": "Code incorrect.",
  "unlock.error_throttled": "Trop de tentatives. Réessaie dans un moment.",
  "unlock.error_generic": "Une erreur s'est produite. Réessaie."
}
```

- [ ] **Step 5 : Réécrire `unlock/i18n.ts`**

Replace tout le contenu de `frontend/src/unlock/i18n.ts` par :
```ts
import i18next from 'i18next'
import { initReactI18next } from 'react-i18next'
import LanguageDetector from 'i18next-browser-languagedetector'
import { parseLocales } from '@/i18n/available-locales'

const { resources, locales } = parseLocales(
  import.meta.glob('../i18n/locales/unlock/*.json', { eager: true }),
)

const instance = i18next.createInstance()
instance
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources,
    fallbackLng: 'en',
    supportedLngs: locales.map((l) => l.code),
    keySeparator: false,
    nsSeparator: false,
    interpolation: { escapeValue: false },
    detection: { order: ['localStorage', 'navigator'], lookupLocalStorage: 'latch.locale' },
  })

export default instance
```

- [ ] **Step 6 : Lancer le test ciblé + le test unlock-page existant**

Run: `rtk vitest run src/unlock/`
Expected: PASS — `i18n.test.ts` (3 tests) + `unlock-page.test.tsx` existant restent verts.

- [ ] **Step 7 : Commit**

```bash
rtk git add frontend/src/i18n/locales/unlock/ frontend/src/unlock/i18n.ts frontend/src/unlock/i18n.test.ts
rtk git commit -m "♻️ refactor(i18n): unlock locales auto-découvertes (glob), bundle public minimal"
```

---

## Task 4 : Montage `ThemeProvider` + anti-FOUC

**Files:**
- Modify: `frontend/src/main.tsx`
- Modify: `frontend/index.html`
- Test: `frontend/src/theme.test.tsx`

**Interfaces:**
- Consumes: `next-themes` (`ThemeProvider`). storageKey `latch.theme`.
- Produces: contexte thème disponible pour toute l'app admin (consommé par `src/components/ui/sonner.tsx` déjà existant, et par le toggle du Lot 2).

- [ ] **Step 1 : Écrire le smoke test qui échoue**

Create `frontend/src/theme.test.tsx` :
```tsx
import { describe, expect, it } from 'vitest'
import { render, screen } from '@testing-library/react'
import { ThemeProvider, useTheme } from 'next-themes'

function Probe() {
  const { theme } = useTheme()
  return <span data-testid="theme">{theme ?? 'pending'}</span>
}

describe('ThemeProvider (config)', () => {
  it('provides a resolved theme value to consumers', () => {
    render(
      <ThemeProvider attribute="class" defaultTheme="system" enableSystem storageKey="latch.theme">
        <Probe />
      </ThemeProvider>,
    )
    expect(screen.getByTestId('theme')).toHaveTextContent(/^(system|light|dark)$/)
  })
})
```

- [ ] **Step 2 : Lancer le test, vérifier l'état**

Run: `rtk vitest run src/theme.test.tsx`
Expected: PASS (next-themes est déjà installé ; ce test verrouille la config attendue). Si `theme` reste `pending` (effet non flush), wrap le `render` dans `await act(...)` ou ajoute `await screen.findByText(...)`. Ce test sert de garde de configuration.

- [ ] **Step 3 : Monter le `ThemeProvider` dans `main.tsx`**

Edit `frontend/src/main.tsx` — ajouter l'import et wrapper l'arbre. Remplacer :
```tsx
import { I18nextProvider } from 'react-i18next'
```
par :
```tsx
import { I18nextProvider } from 'react-i18next'
import { ThemeProvider } from 'next-themes'
```
puis remplacer le bloc `render(...)` par :
```tsx
createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <ThemeProvider
      attribute="class"
      defaultTheme="system"
      enableSystem
      storageKey="latch.theme"
      disableTransitionOnChange
    >
      <I18nextProvider i18n={i18n}>
        <QueryClientProvider client={queryClient}>
          <RouterProvider router={router} />
          <Toaster richColors position="top-right" />
        </QueryClientProvider>
      </I18nextProvider>
    </ThemeProvider>
  </StrictMode>,
)
```

- [ ] **Step 4 : Ajouter le script anti-FOUC dans `index.html`**

Edit `frontend/index.html` — insérer le script juste avant `</head>` (NE PAS toucher `unlock.html`) :
```html
    <script>
      (function () {
        try {
          var t = localStorage.getItem('latch.theme') || 'system'
          var dark =
            t === 'dark' ||
            (t === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches)
          if (dark) document.documentElement.classList.add('dark')
        } catch (e) {}
      })()
    </script>
  </head>
```

- [ ] **Step 5 : Vérifier typecheck + build (entrées non testées unitairement)**

Run: `rtk tsc` puis `cd frontend && pnpm build`
Expected: typecheck sans erreur ; build produit `dist/index.html` (avec le script inline) + `dist/unlock.html` (sans).

- [ ] **Step 6 : Lancer toute la suite Vitest**

Run: `rtk vitest run`
Expected: PASS — y compris `src/theme.test.tsx` et les tests sonner/topbar qui montent `useTheme`.

- [ ] **Step 7 : Commit**

```bash
rtk git add frontend/src/main.tsx frontend/index.html frontend/src/theme.test.tsx
rtk git commit -m "✨ feat(theme): ThemeProvider next-themes (admin, défaut system) + anti-FOUC"
```

---

## Task 5 : Vérification finale + mémoire projet

**Files:**
- Modify: `docs/INDEX.md`, `docs/HANDOFF.md`, `docs/CONVENTIONS.md`, `docs/QUIRKS.md`

**Interfaces:** aucune (clôture).

- [ ] **Step 1 : Vérification complète (gate « terminé »)**

Run (depuis `frontend/`) :
```bash
rtk lint
pnpm typecheck
rtk vitest run --coverage
pnpm build
```
Expected: lint 0 erreur ; typecheck 0 erreur ; tous tests verts ; couverture `src/i18n/available-locales.ts` ≈ 100 % ; build OK. Vérifier dans le rapport que la couverture du code neuf est ≥ 80 %.

- [ ] **Step 2 : Vérifier la non-fuite du bundle public**

Run: `cd frontend && rtk grep "common.new_project\|danger.delete" dist/assets/*.js` (une clé admin, après build)
Expected: AUCUN match dans les chunks de l'entrée `unlock` (la séparation glob garantit que les clés admin ne sont pas dans le bundle public). Inspecter le mapping d'entrées si doute (`dist/.vite/manifest.json` non émis par défaut → se fier à l'absence des chaînes admin côté unlock).

- [ ] **Step 3 : Mettre à jour `docs/CONVENTIONS.md`**

Ajouter une section documentant le pattern réutilisable (Lot 2/3) :
```markdown
## i18n — locales auto-découvertes (Phase 7 Lot 1)

Ajouter une langue = déposer `src/i18n/locales/{admin,unlock}/<code>.json` avec une
clé `_meta` en tête : `{ "_meta": { "name": "<Nom natif>", "flag": "<ISO pays>" }, ... }`.
Aucun code à toucher : `parseLocales` (`src/i18n/available-locales.ts`) découvre les
fichiers via `import.meta.glob(..., { eager: true })`, strip `_meta`, dérive
`supportedLngs`, et expose `locales: LocaleInfo[]` (lu par le sélecteur de langue).
Le drapeau est un code pays ISO (rendu décidé au Lot 2). Deux dossiers/globs distincts
= séparation garantie admin (108 clés) / unlock (8 clés, bundle public minimal).
```

- [ ] **Step 4 : Mettre à jour `docs/QUIRKS.md`**

Ajouter :
```markdown
## import.meta.glob sous Vitest (Phase 7 Lot 1)
`import.meta.glob` est une primitive Vite — disponible sous Vitest (qui passe par Vite),
mais la logique de transformation a été isolée en fonction pure `parseLocales(glob)` pour
être testée avec des maps factices, sans dépendre du glob réel. Les modules JSON eager
exposent l'objet parsé sous `.default`.

## Thème : anti-FOUC en SPA CSR (Phase 7 Lot 1)
next-themes n'injecte son script anti-flash qu'en environnement Next.js. En SPA Vite pure
(CSR), `<html>` n'a pas `.dark` avant le montage React → flash possible. Mitigation : script
inline bloquant dans `index.html` (lit `localStorage['latch.theme']` / `prefers-color-scheme`
et pose `.dark` avant le 1er paint). `unlock.html` n'a PAS ce script (clair-only assumé).
```

- [ ] **Step 5 : Mettre à jour `docs/INDEX.md`**

Ajouter une ligne dans la table des features livrées :
```markdown
| Phase 7 Lot 1 — Fondations i18n/thème | i18n auto-découvert (glob + `_meta`), `ThemeProvider` (défaut `system`, anti-FOUC) | `docs/superpowers/specs/2026-06-25-phase-7-lot-1-fondations-i18n-theme-design.md` · plan associé |
```

- [ ] **Step 6 : Mettre à jour `docs/HANDOFF.md`**

Ajouter une entrée datée en haut (sous le H1) : `Dernière chose faite` (Lot 1 livré : i18n auto-découvert + ThemeProvider), `Trucs en suspens` (toggle thème + vrai sélecteur langue = Lot 2 ; unlock clair-only ; Context7 non connecté cette session), `Prochaine chose à creuser` (Lot 2), `Notes pour future Claude` (pattern glob+`_meta` dans CONVENTIONS).

- [ ] **Step 7 : Commit**

```bash
rtk git add docs/
rtk git commit -m "📝 docs(phase-7): Lot 1 livré — mémoire (INDEX/HANDOFF/CONVENTIONS/QUIRKS)"
```

---

## Self-Review (effectuée à l'écriture du plan)

- **Couverture du spec** : i18n auto-découvert (T1+T2+T3) ✓ ; `_meta` nom/drapeau (T1) ✓ ; deux dossiers/globs (T2/T3) ✓ ; instance unlock séparée + `escapeValue` (T3) ✓ ; `ThemeProvider` admin défaut system + storageKey + anti-FOUC (T4) ✓ ; `LocaleSwitcher` inchangé (aucune tâche ne le touche) ✓ ; unlock non touché côté thème (T4 ne modifie que `main.tsx`/`index.html`) ✓ ; tests parseLocales/i18n/unlock/theme (T1-T4) ✓ ; critères de sortie + mémoire (T5) ✓.
- **Placeholders** : aucun TODO/TBD ; tout le code est explicite.
- **Cohérence des types** : `parseLocales` / `LocaleInfo` / `locales` / `resources` cohérents entre T1, T2, T3. `storageKey: 'latch.theme'` identique entre `main.tsx` (T4-S3), script `index.html` (T4-S4) et le test (T4-S1).
- **Risque connu** : le « red » de Task 3 Step 2 n'est pas un vrai échec (catalogue inline encore valide) — documenté comme baseline de non-régression plutôt que TDD strict, car c'est un refactor à comportement constant.
```
