# Phase 7 — Lot 2 : Panneau Settings unifié (side-panel) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transformer la route Settings plein écran en un side-panel (`<Sheet>`) regroupant infos MCP + vrai sélecteur de langue (drapeaux SVG, peuplé depuis `locales`) + toggle de thème segmenté, chaque réglage explicité.

**Architecture:** Trois composants neufs (`LanguageSelect`, `ThemeToggle`, `SettingsSheet`) + un wrapper `ui/select.tsx` (radix). La `Topbar` porte l'état d'ouverture du Sheet et le rend (route `/settings` supprimée). Le `LocaleSwitcher` est dé-hardcodé (dérivé de `locales`) et conservé pour le login. Consomme `locales` et `useTheme/setTheme` du Lot 1.

**Tech Stack:** React 19, Vite 8, TypeScript, radix-ui (Select), next-themes ^0.4.6, react-i18next ^17, flag-icons (nouveau), Vitest ^4 + Testing Library + MSW (jsdom, `globals: true`, alias `@` → `src`).

## Global Constraints

- **Confidentialité (NON-NÉGOCIABLE)** : aucun nom de client réel (code/tests/fixtures/docs). Placeholders fictifs (`ACME`, `Mon Projet`). `latch — admin` / `latch` = marque produit (OK).
- **Isolation bundle public** : la CSS `flag-icons` et les chaînes `settings.*` ne doivent PAS entrer dans le bundle unlock. Garanti en important la CSS flag-icons **dans `language-select.tsx`** (jamais dans `index.css` partagé).
- **Thème** : le `ThemeToggle` lit **`theme`** (préférence `system`/`light`/`dark`), PAS `resolvedTheme`.
- **Secret** : `useSettings` doit accepter un `enabled` et le Sheet passe `enabled: open` → `/api/settings` (donc le `deploy_token`) n'est fetché qu'à l'ouverture.
- **Zéro `['en','fr']` en dur** : `LanguageSelect` ET `LocaleSwitcher` dérivent de `locales` (export `@/i18n`).
- **Style d'import radix** : calquer `ui/sheet.tsx` → `import { Select as SelectPrimitive } from "radix-ui"`.
- **Couverture** : SonarCloud `new_coverage ≥ 80 %` sur le code neuf.
- **Commandes** : depuis `frontend/`. `rtk vitest run`, `pnpm typecheck`, `rtk lint`, `pnpm build`.
- **Subagents** : IGNORER le protocole load-memory du CLAUDE.md, ne pas répondre « Mémoire chargée ».

---

## File Structure

| Fichier | Responsabilité | Action |
|---|---|---|
| `frontend/src/components/ui/select.tsx` | Wrapper shadcn du `Select` radix | **Créer** |
| `frontend/src/components/language-select.tsx` | `Select` peuplé depuis `locales` + drapeaux ; importe la CSS flag-icons | **Créer** |
| `frontend/src/components/language-select.test.tsx` | Test | **Créer** |
| `frontend/src/components/theme-toggle.tsx` | Segmented 3 boutons (système/clair/sombre) | **Créer** |
| `frontend/src/components/theme-toggle.test.tsx` | Test | **Créer** |
| `frontend/src/components/settings-sheet.tsx` | Le panneau `<Sheet>` (MCP puis Préférences) | **Créer** |
| `frontend/src/components/settings-sheet.test.tsx` | Test | **Créer** |
| `frontend/src/hooks/use-settings.ts` | Ajout param `enabled` | **Modifier** |
| `frontend/src/components/locale-switcher.tsx` | Dérive de `locales` (login) | **Modifier** |
| `frontend/src/components/locale-switcher.test.tsx` | Test | **Créer** |
| `frontend/src/components/topbar.tsx` | Ouvre le Sheet ; retrait LocaleSwitcher | **Modifier** |
| `frontend/src/components/topbar.test.tsx` | Test maj (ouvre le Sheet) | **Modifier** |
| `frontend/src/router.tsx` | Retrait `settingsRoute` | **Modifier** |
| `frontend/src/routes/settings.tsx` | — | **Supprimer** |
| `frontend/src/routes/settings.test.tsx` | — | **Supprimer** |
| `frontend/src/test/utils.tsx` | Retrait SettingsPage/route/TestPath | **Modifier** |
| `frontend/src/i18n/locales/admin/{en,fr}.json` | ~12 clés `settings.*` | **Modifier** |
| `frontend/vitest.setup.ts` | Shims jsdom radix Select | **Modifier** |
| `frontend/package.json` | Dépendance `flag-icons` | **Modifier** |
| `docs/INDEX.md`, `docs/HANDOFF.md`, `docs/CONVENTIONS.md`, `docs/QUIRKS.md` | Mémoire | **Modifier** |

---

## Task 1 : i18n keys + `Select` wrapper + shims + `LanguageSelect`

**Files:**
- Modify: `frontend/src/i18n/locales/admin/en.json`, `frontend/src/i18n/locales/admin/fr.json`
- Modify: `frontend/package.json` (+ lockfile)
- Modify: `frontend/vitest.setup.ts`
- Create: `frontend/src/components/ui/select.tsx`
- Create: `frontend/src/components/language-select.tsx`
- Test: `frontend/src/components/language-select.test.tsx`

**Interfaces:**
- Consumes: `locales` (export `@/i18n`, Lot 1) — `LocaleInfo = { code: string; name: string; flag: string }`.
- Produces: `<LanguageSelect />` (no props) ; `ui/select.tsx` exports `Select, SelectValue, SelectTrigger, SelectContent, SelectItem`.

- [ ] **Step 1 : Ajouter les clés i18n (en)**

Edit `frontend/src/i18n/locales/admin/en.json` — remplacer la ligne `"settings.copy_mcp_url": "Copy MCP URL"` (dernière clé) par la version avec virgule + les nouvelles clés (garder le `}` final du fichier) :
```json
  "settings.copy_mcp_url": "Copy MCP URL",
  "settings.section_mcp": "MCP connection",
  "settings.section_preferences": "Preferences",
  "settings.language": "Language",
  "settings.language_help": "Admin interface language.",
  "settings.theme": "Theme",
  "settings.theme_help": "\"System\" follows your OS preference.",
  "settings.theme_system": "System",
  "settings.theme_light": "Light",
  "settings.theme_dark": "Dark",
  "settings.mcp_url_help": "Set this in Claude's MCP connector.",
  "settings.deploy_token_help": "Secret validated by all MCP tools.",
  "settings.public_base_url_help": "Public root of this instance."
```

- [ ] **Step 2 : Ajouter les clés i18n (fr)**

Edit `frontend/src/i18n/locales/admin/fr.json` — de même, après la clé `settings.copy_mcp_url` :
```json
  "settings.copy_mcp_url": "Copier l'URL MCP",
  "settings.section_mcp": "Connexion MCP",
  "settings.section_preferences": "Préférences",
  "settings.language": "Langue",
  "settings.language_help": "Langue de l'interface d'administration.",
  "settings.theme": "Thème",
  "settings.theme_help": "« Système » suit la préférence de l'OS.",
  "settings.theme_system": "Système",
  "settings.theme_light": "Clair",
  "settings.theme_dark": "Sombre",
  "settings.mcp_url_help": "À renseigner dans le connecteur MCP de Claude.",
  "settings.deploy_token_help": "Secret validé par tous les tools MCP.",
  "settings.public_base_url_help": "Racine publique de l'instance."
```
**Note** : la valeur EN/FR exacte de `settings.copy_mcp_url` doit rester celle déjà présente dans chaque fichier (vérifier avant de remplacer — ne change que l'ajout de la virgule + les nouvelles lignes).

- [ ] **Step 3 : Ajouter la dépendance flag-icons**

Run: `cd /srv/owlnext/latch/frontend && pnpm add flag-icons`
Expected: `flag-icons` ajouté à `package.json` dependencies + lockfile mis à jour.

- [ ] **Step 4 : Ajouter les shims jsdom pour radix Select**

Edit `frontend/vitest.setup.ts` — ajouter après le shim `document.elementFromPoint` existant :
```ts
// jsdom lacks these Element methods that Radix Select relies on for positioning.
if (!Element.prototype.scrollIntoView) {
  Element.prototype.scrollIntoView = () => {}
}
if (!Element.prototype.hasPointerCapture) {
  Element.prototype.hasPointerCapture = () => false
}
if (!Element.prototype.releasePointerCapture) {
  Element.prototype.releasePointerCapture = () => {}
}
```

- [ ] **Step 5 : Créer le wrapper `ui/select.tsx`**

Create `frontend/src/components/ui/select.tsx` :
```tsx
import * as React from "react"
import { Select as SelectPrimitive } from "radix-ui"
import { CheckIcon, ChevronDownIcon } from "lucide-react"

import { cn } from "@/lib/utils"

function Select(props: React.ComponentProps<typeof SelectPrimitive.Root>) {
  return <SelectPrimitive.Root data-slot="select" {...props} />
}

function SelectValue(props: React.ComponentProps<typeof SelectPrimitive.Value>) {
  return <SelectPrimitive.Value data-slot="select-value" {...props} />
}

function SelectTrigger({
  className,
  children,
  ...props
}: React.ComponentProps<typeof SelectPrimitive.Trigger>) {
  return (
    <SelectPrimitive.Trigger
      data-slot="select-trigger"
      className={cn(
        "flex h-9 w-full items-center justify-between gap-2 rounded-md border border-input bg-transparent px-3 py-2 text-sm shadow-sm focus:outline-none focus:ring-1 focus:ring-ring disabled:cursor-not-allowed disabled:opacity-50 [&>span]:flex [&>span]:items-center [&>span]:gap-2",
        className,
      )}
      {...props}
    >
      {children}
      <SelectPrimitive.Icon asChild>
        <ChevronDownIcon className="size-4 opacity-50" />
      </SelectPrimitive.Icon>
    </SelectPrimitive.Trigger>
  )
}

function SelectContent({
  className,
  children,
  position = "popper",
  ...props
}: React.ComponentProps<typeof SelectPrimitive.Content>) {
  return (
    <SelectPrimitive.Portal>
      <SelectPrimitive.Content
        data-slot="select-content"
        className={cn(
          "relative z-50 max-h-96 min-w-[8rem] overflow-hidden rounded-md border bg-popover text-popover-foreground shadow-md",
          className,
        )}
        position={position}
        {...props}
      >
        <SelectPrimitive.Viewport
          className={cn(
            "p-1",
            position === "popper" && "w-full min-w-[var(--radix-select-trigger-width)]",
          )}
        >
          {children}
        </SelectPrimitive.Viewport>
      </SelectPrimitive.Content>
    </SelectPrimitive.Portal>
  )
}

function SelectItem({
  className,
  children,
  ...props
}: React.ComponentProps<typeof SelectPrimitive.Item>) {
  return (
    <SelectPrimitive.Item
      data-slot="select-item"
      className={cn(
        "relative flex w-full cursor-default items-center gap-2 rounded-sm py-1.5 pr-8 pl-2 text-sm outline-none select-none focus:bg-accent focus:text-accent-foreground data-disabled:pointer-events-none data-disabled:opacity-50",
        className,
      )}
      {...props}
    >
      <span className="absolute right-2 flex size-3.5 items-center justify-center">
        <SelectPrimitive.ItemIndicator>
          <CheckIcon className="size-4" />
        </SelectPrimitive.ItemIndicator>
      </span>
      <SelectPrimitive.ItemText>{children}</SelectPrimitive.ItemText>
    </SelectPrimitive.Item>
  )
}

export { Select, SelectValue, SelectTrigger, SelectContent, SelectItem }
```

- [ ] **Step 6 : Écrire le test `LanguageSelect` (échoue)**

Create `frontend/src/components/language-select.test.tsx` :
```tsx
import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen, act } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { I18nextProvider } from 'react-i18next'
import i18n from '@/i18n'
import { LanguageSelect } from './language-select'

function renderLS() {
  act(() => {
    render(
      <I18nextProvider i18n={i18n}>
        <LanguageSelect />
      </I18nextProvider>,
    )
  })
}

describe('LanguageSelect', () => {
  beforeEach(async () => {
    await i18n.changeLanguage('en')
  })

  it('shows the current language in the trigger', () => {
    renderLS()
    expect(screen.getByRole('combobox')).toHaveTextContent('English')
  })

  it('lists discovered locales and switches language on selection', async () => {
    const user = userEvent.setup()
    renderLS()
    await user.click(screen.getByRole('combobox'))
    // Options are rendered in a portal once open.
    const frenchOption = await screen.findByRole('option', { name: /Français/ })
    expect(screen.getByRole('option', { name: /English/ })).toBeInTheDocument()
    await user.click(frenchOption)
    expect(i18n.language).toBe('fr')
  })
})
```

- [ ] **Step 7 : Lancer le test, vérifier l'échec**

Run: `rtk vitest run src/components/language-select.test.tsx`
Expected: FAIL — `./language-select` introuvable.

- [ ] **Step 8 : Implémenter `LanguageSelect`**

Create `frontend/src/components/language-select.tsx` :
```tsx
import { useTranslation } from 'react-i18next'
import { locales } from '@/i18n'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import 'flag-icons/css/flag-icons.min.css'

export function LanguageSelect() {
  const { t, i18n } = useTranslation()
  const current = i18n.language.slice(0, 2)

  return (
    <Select value={current} onValueChange={(code) => void i18n.changeLanguage(code)}>
      <SelectTrigger className="w-full" aria-label={t('settings.language')}>
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {locales.map((l) => (
          <SelectItem key={l.code} value={l.code}>
            <span className={`fi fi-${l.flag.toLowerCase()}`} aria-hidden="true" />
            {l.name}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}
```

- [ ] **Step 9 : Lancer le test, vérifier le succès**

Run: `rtk vitest run src/components/language-select.test.tsx`
Expected: PASS (2 tests).

- [ ] **Step 10 : Commit**

```bash
git add frontend/src/components/ui/select.tsx frontend/src/components/language-select.tsx frontend/src/components/language-select.test.tsx frontend/src/i18n/locales/admin/ frontend/vitest.setup.ts frontend/package.json frontend/pnpm-lock.yaml
git commit -m "✨ feat(settings): LanguageSelect (Select radix + flag-icons, locales-driven)"
```

---

## Task 2 : `ThemeToggle`

**Files:**
- Create: `frontend/src/components/theme-toggle.tsx`
- Test: `frontend/src/components/theme-toggle.test.tsx`

**Interfaces:**
- Consumes: `next-themes` `useTheme()` (provider monté au Lot 1) ; clés i18n `settings.theme*` (Task 1).
- Produces: `<ThemeToggle />` (no props).

- [ ] **Step 1 : Écrire le test (échoue)**

Create `frontend/src/components/theme-toggle.test.tsx` :
```tsx
import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen, act } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { ThemeProvider } from 'next-themes'
import { I18nextProvider } from 'react-i18next'
import i18n from '@/i18n'
import { ThemeToggle } from './theme-toggle'

function renderTT() {
  act(() => {
    render(
      <I18nextProvider i18n={i18n}>
        <ThemeProvider attribute="class" defaultTheme="system" enableSystem storageKey="latch.theme">
          <ThemeToggle />
        </ThemeProvider>
      </I18nextProvider>,
    )
  })
}

describe('ThemeToggle', () => {
  beforeEach(() => {
    localStorage.clear()
    Object.defineProperty(window, 'matchMedia', {
      writable: true,
      configurable: true,
      value: (query: string) => ({
        matches: false,
        media: query,
        addEventListener: () => {},
        removeEventListener: () => {},
        addListener: () => {},
        removeListener: () => {},
        dispatchEvent: () => false,
        onchange: null,
      }),
    })
  })

  it('renders the three theme options', async () => {
    renderTT()
    expect(await screen.findByRole('button', { name: /System/ })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Light/ })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Dark/ })).toBeInTheDocument()
  })

  it('marks System as pressed by default and switches on click', async () => {
    const user = userEvent.setup()
    renderTT()
    const system = await screen.findByRole('button', { name: /System/ })
    expect(system).toHaveAttribute('aria-pressed', 'true')

    await user.click(screen.getByRole('button', { name: /Dark/ }))
    expect(screen.getByRole('button', { name: /Dark/ })).toHaveAttribute('aria-pressed', 'true')
  })
})
```

- [ ] **Step 2 : Lancer le test, vérifier l'échec**

Run: `rtk vitest run src/components/theme-toggle.test.tsx`
Expected: FAIL — `./theme-toggle` introuvable.

- [ ] **Step 3 : Implémenter `ThemeToggle`**

Create `frontend/src/components/theme-toggle.tsx` :
```tsx
import { useTheme } from 'next-themes'
import { useTranslation } from 'react-i18next'
import { Monitor, Sun, Moon } from 'lucide-react'
import { Button } from '@/components/ui/button'

const OPTIONS = [
  { value: 'system', icon: Monitor, labelKey: 'settings.theme_system' },
  { value: 'light', icon: Sun, labelKey: 'settings.theme_light' },
  { value: 'dark', icon: Moon, labelKey: 'settings.theme_dark' },
] as const

export function ThemeToggle() {
  const { t } = useTranslation()
  const { theme, setTheme } = useTheme()

  return (
    <fieldset className="m-0 flex items-center gap-1 border-0 p-0">
      <legend className="sr-only">{t('settings.theme')}</legend>
      {OPTIONS.map(({ value, icon: Icon, labelKey }) => (
        <Button
          key={value}
          type="button"
          variant={theme === value ? 'secondary' : 'ghost'}
          size="sm"
          aria-pressed={theme === value}
          onClick={() => setTheme(value)}
        >
          <Icon className="mr-1 size-4" />
          {t(labelKey)}
        </Button>
      ))}
    </fieldset>
  )
}
```

- [ ] **Step 4 : Lancer le test, vérifier le succès**

Run: `rtk vitest run src/components/theme-toggle.test.tsx`
Expected: PASS (2 tests).

- [ ] **Step 5 : Commit**

```bash
git add frontend/src/components/theme-toggle.tsx frontend/src/components/theme-toggle.test.tsx
git commit -m "✨ feat(settings): ThemeToggle segmenté (système/clair/sombre)"
```

---

## Task 3 : `SettingsSheet` + `useSettings(enabled)`

**Files:**
- Modify: `frontend/src/hooks/use-settings.ts`
- Create: `frontend/src/components/settings-sheet.tsx`
- Test: `frontend/src/components/settings-sheet.test.tsx`

**Interfaces:**
- Consumes: `<LanguageSelect />` (Task 1), `<ThemeToggle />` (Task 2), `useSettings(enabled)`, `CopyButton`, `PinField`, clés i18n (Task 1).
- Produces: `<SettingsSheet open={boolean} onOpenChange={(open: boolean) => void} />`.
- `useSettings` signature devient `useSettings(enabled = true)`.

- [ ] **Step 1 : Modifier `use-settings.ts`**

Replace le contenu de `frontend/src/hooks/use-settings.ts` par :
```ts
import { useQuery } from '@tanstack/react-query'
import { api } from '@/api/client'

export function useSettings(enabled = true) {
  return useQuery({
    queryKey: ['settings'],
    enabled,
    queryFn: async () => {
      const { data, error } = await api.GET('/api/settings')
      if (error) throw new Error('settings')
      return data
    },
  })
}
```

- [ ] **Step 2 : Écrire le test `SettingsSheet` (échoue)**

Create `frontend/src/components/settings-sheet.test.tsx` :
```tsx
import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen, act } from '@testing-library/react'
import { http, HttpResponse } from 'msw'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { I18nextProvider } from 'react-i18next'
import { ThemeProvider } from 'next-themes'
import { server } from '@/test/msw'
import i18n from '@/i18n'
import { SettingsSheet } from './settings-sheet'

const ORIGIN = globalThis.location.origin

function renderSheet(open: boolean) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  act(() => {
    render(
      <I18nextProvider i18n={i18n}>
        <ThemeProvider attribute="class" defaultTheme="system" enableSystem storageKey="latch.theme">
          <QueryClientProvider client={qc}>
            <SettingsSheet open={open} onOpenChange={() => {}} />
          </QueryClientProvider>
        </ThemeProvider>
      </I18nextProvider>,
    )
  })
}

describe('SettingsSheet', () => {
  beforeEach(async () => {
    server.resetHandlers()
    localStorage.clear()
    Object.defineProperty(window, 'matchMedia', {
      writable: true,
      configurable: true,
      value: (query: string) => ({
        matches: false, media: query,
        addEventListener: () => {}, removeEventListener: () => {},
        addListener: () => {}, removeListener: () => {},
        dispatchEvent: () => false, onchange: null,
      }),
    })
    await i18n.changeLanguage('en')
    server.use(
      http.get(`${ORIGIN}/api/settings`, () =>
        HttpResponse.json({
          mcp_url: 'https://latch.example/mcp',
          deploy_token: 'tok-123456',
          public_base_url: 'https://latch.example',
        }),
      ),
    )
  })

  it('renders MCP infos with a help text per field when open', async () => {
    renderSheet(true)
    expect(await screen.findByText('https://latch.example/mcp')).toBeInTheDocument()
    expect(screen.getByText('Set this in Claude\'s MCP connector.')).toBeInTheDocument()
    expect(screen.getByText('Secret validated by all MCP tools.')).toBeInTheDocument()
    expect(screen.getByText('Public root of this instance.')).toBeInTheDocument()
    // deploy_token masqué par défaut
    expect(screen.getByText('••••••')).toBeInTheDocument()
  })

  it('renders the preferences controls (language + theme)', async () => {
    renderSheet(true)
    await screen.findByText('https://latch.example/mcp')
    expect(screen.getByRole('combobox')).toBeInTheDocument() // LanguageSelect
    expect(screen.getByRole('button', { name: /System/ })).toBeInTheDocument() // ThemeToggle
  })
})
```

- [ ] **Step 3 : Lancer le test, vérifier l'échec**

Run: `rtk vitest run src/components/settings-sheet.test.tsx`
Expected: FAIL — `./settings-sheet` introuvable.

- [ ] **Step 4 : Implémenter `SettingsSheet`**

Create `frontend/src/components/settings-sheet.tsx` :
```tsx
import { useTranslation } from 'react-i18next'
import { Sheet, SheetContent, SheetHeader, SheetTitle } from '@/components/ui/sheet'
import { CopyButton } from '@/components/copy-button'
import { PinField } from '@/components/pin-field'
import { LanguageSelect } from '@/components/language-select'
import { ThemeToggle } from '@/components/theme-toggle'
import { useSettings } from '@/hooks/use-settings'

interface SettingsSheetProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function SettingsSheet({ open, onOpenChange }: Readonly<SettingsSheetProps>) {
  const { t } = useTranslation()
  const { data, isLoading, isError } = useSettings(open)

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent className="w-full overflow-y-auto sm:max-w-md">
        <SheetHeader>
          <SheetTitle>{t('settings.title')}</SheetTitle>
        </SheetHeader>

        <div className="flex flex-1 flex-col gap-6 p-4">
          <section className="flex flex-col gap-4">
            <h3 className="text-muted-foreground text-xs font-medium uppercase">
              {t('settings.section_mcp')}
            </h3>

            {isLoading ? (
              <p className="text-muted-foreground text-sm">{t('common.loading')}</p>
            ) : isError ? (
              <p className="text-destructive text-sm">{t('error.network')}</p>
            ) : data ? (
              <>
                <div className="flex flex-col gap-1.5">
                  <span className="text-sm font-medium">{t('settings.mcp_url')}</span>
                  <span className="flex items-center gap-2">
                    <span className="font-mono text-sm break-all">{data.mcp_url}</span>
                    <CopyButton text={data.mcp_url} ariaLabel={t('settings.copy_mcp_url')} />
                  </span>
                  <p className="text-muted-foreground text-xs">{t('settings.mcp_url_help')}</p>
                </div>

                <div className="flex flex-col gap-1.5">
                  <span className="text-sm font-medium">{t('settings.deploy_token')}</span>
                  <PinField pin={data.deploy_token} />
                  <p className="text-muted-foreground text-xs">{t('settings.deploy_token_help')}</p>
                </div>

                <div className="flex flex-col gap-1.5">
                  <span className="text-sm font-medium">{t('settings.public_base_url')}</span>
                  <span className="font-mono text-sm break-all">{data.public_base_url}</span>
                  <p className="text-muted-foreground text-xs">{t('settings.public_base_url_help')}</p>
                </div>
              </>
            ) : null}
          </section>

          <section className="flex flex-col gap-4">
            <h3 className="text-muted-foreground text-xs font-medium uppercase">
              {t('settings.section_preferences')}
            </h3>

            <div className="flex flex-col gap-1.5">
              <span className="text-sm font-medium">{t('settings.language')}</span>
              <LanguageSelect />
              <p className="text-muted-foreground text-xs">{t('settings.language_help')}</p>
            </div>

            <div className="flex flex-col gap-1.5">
              <span className="text-sm font-medium">{t('settings.theme')}</span>
              <ThemeToggle />
              <p className="text-muted-foreground text-xs">{t('settings.theme_help')}</p>
            </div>
          </section>
        </div>
      </SheetContent>
    </Sheet>
  )
}
```

- [ ] **Step 5 : Lancer le test, vérifier le succès**

Run: `rtk vitest run src/components/settings-sheet.test.tsx`
Expected: PASS (2 tests).

- [ ] **Step 6 : Commit**

```bash
git add frontend/src/hooks/use-settings.ts frontend/src/components/settings-sheet.tsx frontend/src/components/settings-sheet.test.tsx
git commit -m "✨ feat(settings): SettingsSheet (MCP + préférences) + useSettings(enabled)"
```

---

## Task 4 : `LocaleSwitcher` dé-hardcodé (login)

**Files:**
- Modify: `frontend/src/components/locale-switcher.tsx`
- Test: `frontend/src/components/locale-switcher.test.tsx`

**Interfaces:**
- Consumes: `locales` (export `@/i18n`).
- Produces: `<LocaleSwitcher />` (no props), comportement inchangé (toggle de boutons), mais options dérivées de `locales`.

- [ ] **Step 1 : Écrire le test (échoue)**

Create `frontend/src/components/locale-switcher.test.tsx` :
```tsx
import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen, act } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { I18nextProvider } from 'react-i18next'
import i18n from '@/i18n'
import { LocaleSwitcher } from './locale-switcher'

function renderLS() {
  act(() => {
    render(
      <I18nextProvider i18n={i18n}>
        <LocaleSwitcher />
      </I18nextProvider>,
    )
  })
}

describe('LocaleSwitcher', () => {
  beforeEach(async () => {
    await i18n.changeLanguage('en')
  })

  it('renders one button per discovered locale', () => {
    renderLS()
    expect(screen.getByRole('button', { name: 'EN' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'FR' })).toBeInTheDocument()
  })

  it('switches language on click', async () => {
    const user = userEvent.setup()
    renderLS()
    await user.click(screen.getByRole('button', { name: 'FR' }))
    expect(i18n.language).toBe('fr')
  })
})
```

- [ ] **Step 2 : Lancer le test, vérifier l'état**

Run: `rtk vitest run src/components/locale-switcher.test.tsx`
Expected: PASS (le composant actuel rend déjà EN/FR via la liste en dur — baseline verte avant refactor à comportement constant).

- [ ] **Step 3 : Refactorer `locale-switcher.tsx`**

Replace le contenu de `frontend/src/components/locale-switcher.tsx` par :
```tsx
import { useTranslation } from 'react-i18next'
import { locales } from '@/i18n'
import { Button } from '@/components/ui/button'

export function LocaleSwitcher() {
  const { i18n } = useTranslation()
  const current = i18n.language.slice(0, 2)

  return (
    <fieldset className="m-0 flex items-center gap-1 border-0 p-0">
      <legend className="sr-only">Language</legend>
      {locales.map((l) => (
        <Button
          key={l.code}
          type="button"
          variant={current === l.code ? 'secondary' : 'ghost'}
          size="xs"
          aria-pressed={current === l.code}
          onClick={() => void i18n.changeLanguage(l.code)}
        >
          {l.code.toUpperCase()}
        </Button>
      ))}
    </fieldset>
  )
}
```

- [ ] **Step 4 : Lancer le test, vérifier le succès**

Run: `rtk vitest run src/components/locale-switcher.test.tsx`
Expected: PASS (2 tests) — comportement identique, liste désormais dérivée de `locales`.

- [ ] **Step 5 : Commit**

```bash
git add frontend/src/components/locale-switcher.tsx frontend/src/components/locale-switcher.test.tsx
git commit -m "♻️ refactor(i18n): LocaleSwitcher dérivé de locales (supprime ['en','fr'] en dur)"
```

---

## Task 5 : Wiring Topbar + suppression route `/settings` + nettoyage

**Files:**
- Modify: `frontend/src/components/topbar.tsx`
- Modify: `frontend/src/components/topbar.test.tsx`
- Modify: `frontend/src/router.tsx`
- Modify: `frontend/src/test/utils.tsx`
- Delete: `frontend/src/routes/settings.tsx`, `frontend/src/routes/settings.test.tsx`

**Interfaces:**
- Consumes: `<SettingsSheet open onOpenChange />` (Task 3).
- Produces: Topbar ouvrant le Sheet ; plus de route `/settings` ; `TestPath = '/login' | '/'`.

- [ ] **Step 1 : Modifier `topbar.test.tsx` (ajouter le test d'ouverture, échoue)**

Edit `frontend/src/components/topbar.test.tsx` — ajouter en tête les imports `http, HttpResponse` (déjà importés) puis ajouter ce test dans le `describe('Topbar', …)` :
```tsx
  it('opens the settings sheet when the settings icon is clicked', async () => {
    const user = userEvent.setup()
    server.use(
      http.get(`${ORIGIN}/api/settings`, () =>
        HttpResponse.json({
          mcp_url: 'https://latch.example/mcp',
          deploy_token: 'tok-123456',
          public_base_url: 'https://latch.example',
        }),
      ),
    )
    renderTopbar()
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'Settings' })).toBeInTheDocument(),
    )
    await user.click(screen.getByRole('button', { name: 'Settings' }))
    expect(await screen.findByText('https://latch.example/mcp')).toBeInTheDocument()
  })
```
*(Le harness `renderTopbar()` existant fournit déjà `I18nextProvider` + `QueryClientProvider` ; le Sheet portale dans le body et `useSettings(open)` fetch à l'ouverture.)*

- [ ] **Step 2 : Lancer le test, vérifier l'échec**

Run: `rtk vitest run src/components/topbar.test.tsx`
Expected: FAIL — l'icône Settings navigue encore (pas de Sheet), le texte mcp_url n'apparaît pas.

- [ ] **Step 3 : Modifier `topbar.tsx`**

Replace le contenu de `frontend/src/components/topbar.tsx` par :
```tsx
import { useState } from 'react'
import { useRouter } from '@tanstack/react-router'
import { useTranslation } from 'react-i18next'
import { Settings } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { SettingsSheet } from '@/components/settings-sheet'
import { useLogout } from '@/hooks/use-auth'

export function Topbar() {
  const router = useRouter()
  const { t } = useTranslation()
  const logout = useLogout()
  const [settingsOpen, setSettingsOpen] = useState(false)

  function handleLogout() {
    logout.mutate(undefined, {
      onSettled: () => {
        router.navigate({ to: '/login' })
      },
    })
  }

  return (
    <header className="flex h-14 items-center justify-between border-b px-4">
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
      <div className="flex items-center gap-2">
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          aria-label={t('settings.title')}
          onClick={() => setSettingsOpen(true)}
        >
          <Settings />
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={handleLogout}
          loading={logout.isPending}
        >
          {t('common.logout')}
        </Button>
      </div>
      <SettingsSheet open={settingsOpen} onOpenChange={setSettingsOpen} />
    </header>
  )
}
```

- [ ] **Step 4 : Supprimer la route `/settings` du routeur**

Replace le contenu de `frontend/src/router.tsx` par :
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

- [ ] **Step 5 : Supprimer les fichiers Settings (route + test)**

```bash
cd /srv/owlnext/latch/frontend
git rm src/routes/settings.tsx src/routes/settings.test.tsx
```

- [ ] **Step 6 : Nettoyer `test/utils.tsx`**

Edit `frontend/src/test/utils.tsx` :
- supprimer la ligne `import { SettingsPage } from '@/routes/settings'` ;
- changer `type TestPath = '/login' | '/' | '/settings'` en `type TestPath = '/login' | '/'` ;
- supprimer le bloc `settingsRoute` (lignes `const settingsRoute = createRoute({ … path: '/settings', component: SettingsPage })`) ;
- retirer `settingsRoute` de `rootRoute.addChildren([loginRoute, listRoute, settingsRoute])` → `addChildren([loginRoute, listRoute])`.

- [ ] **Step 7 : Lancer la suite complète**

Run: `rtk vitest run`
Expected: PASS — `topbar.test.tsx` (dont le nouveau test d'ouverture), plus aucun test ne référence `/settings` (settings.test.tsx supprimé), `list.test`/`login.test` inchangés.

- [ ] **Step 8 : Typecheck**

Run: `pnpm typecheck`
Expected: 0 erreur (plus de référence à `SettingsPage` / route `/settings`).

- [ ] **Step 9 : Commit**

```bash
git add frontend/src/components/topbar.tsx frontend/src/components/topbar.test.tsx frontend/src/router.tsx frontend/src/test/utils.tsx
git commit -m "✨ feat(settings): Topbar ouvre le Sheet, route /settings supprimée"
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
Expected: lint 0 erreur ; typecheck 0 erreur ; tous tests verts ; couverture du code neuf (`language-select`, `theme-toggle`, `settings-sheet`) ≥ 80 % ; build OK (`dist/index.html` + `dist/unlock.html`).

- [ ] **Step 2 : Vérifier l'isolation du bundle public**

Run:
```bash
cd /srv/owlnext/latch/frontend
grep -rl "flag-icons\|fi-gb\|section_preferences" dist/assets/ | grep -i unlock || echo "OK: unlock bundle sans flag-icons ni settings.*"
```
Expected: `OK: …` — aucun chunk `unlock-*` ne contient la CSS flag-icons ni les chaînes `settings.*`. (La CSS flag-icons doit apparaître uniquement dans le CSS de l'entrée admin.)

- [ ] **Step 3 : Mettre à jour `docs/CONVENTIONS.md`**

Ajouter :
```markdown
## Composant Select (radix) + helper-text généralisé (Phase 7 Lot 2)

`components/ui/select.tsx` vendorise le Select radix via le package unifié
(`import { Select as SelectPrimitive } from "radix-ui"`, même style que `ui/sheet.tsx`).
Pattern de réglage dans un panneau : `flex flex-col gap-1.5` → label (`text-sm font-medium`)
+ contrôle + helper text (`text-muted-foreground text-xs`). Pour un sélecteur dépendant des
locales découvertes, mapper sur l'export `locales` de `@/i18n` (jamais de liste en dur).
La CSS d'un asset spécifique-admin (ex. `flag-icons`) s'importe DANS le composant qui
l'utilise (`language-select.tsx`), pas dans `index.css` partagé, pour ne pas alourdir le
bundle public unlock.
```

- [ ] **Step 4 : Mettre à jour `docs/QUIRKS.md`**

Ajouter :
```markdown
## radix Select sous jsdom (Phase 7 Lot 2)
Le Select radix appelle `scrollIntoView`, `hasPointerCapture`, `releasePointerCapture`,
absents de jsdom. Shims ajoutés dans `vitest.setup.ts` (à côté de `ResizeObserver`/
`elementFromPoint`). Les tests ciblent le câblage (option courante, `onValueChange` →
`changeLanguage`) plutôt que le cycle pointer interne de radix.
```

- [ ] **Step 5 : Mettre à jour `docs/INDEX.md`**

Ajouter une ligne :
```markdown
| Phase 7 Lot 2 — Panneau Settings unifié | Side-panel Settings (route /settings supprimée), LanguageSelect (Select+flag-icons, locales-driven), ThemeToggle 3 états, helper text par réglage | `docs/superpowers/specs/2026-06-25-phase-7-lot-2-settings-side-panel-design.md` · plan associé |
```

- [ ] **Step 6 : Mettre à jour `docs/HANDOFF.md`**

Entrée datée en haut : `Dernière chose faite` (Lot 2 livré : Settings en side-panel + sélecteur langue avec drapeaux + toggle thème), `Trucs en suspens` (Lot 3 = logo/titres/largeur/GitHub ; Lot 4 = page erreur serving ; merge Lot 1+2 d'un coup à la fin), `Prochaine chose à creuser` (Lot 3), `Notes pour future Claude` (Select vendorisé + flag-icons réutilisables ; isolation CSS via import composant).

- [ ] **Step 7 : Commit**

```bash
git add docs/
git commit -m "📝 docs(phase-7): Lot 2 livré — mémoire (INDEX/HANDOFF/CONVENTIONS/QUIRKS)"
```

---

## Self-Review (effectuée à l'écriture)

- **Couverture du spec** : Sheet Settings (T3) ✓ ; route supprimée + topbar wiring (T5) ✓ ; LanguageSelect Select+flag-icons locales-driven (T1) ✓ ; ThemeToggle 3 états lisant `theme` (T2) ✓ ; helper text par réglage (T3) ✓ ; LocaleSwitcher dé-hardcodé conservé login (T4) ✓ ; `useSettings(enabled)` (T3) ✓ ; i18n keys (T1) ✓ ; shims jsdom (T1) ✓ ; isolation CSS flag-icons (T1 import + T6 check) ✓ ; ordre MCP→Préférences (T3) ✓ ; nettoyage settings.test/test-utils (T5) ✓ ; mémoire (T6) ✓.
- **Placeholders** : aucun ; code complet à chaque étape.
- **Cohérence des types** : `useSettings(enabled=true)` (T3) cohérent avec l'appel `useSettings(open)` du Sheet ; `SettingsSheet` props `{open, onOpenChange}` identiques entre T3 (def) et T5 (usage topbar) ; `locales`/`LocaleInfo` du Lot 1 ; `ui/select.tsx` exports (T1) = imports de `language-select.tsx` (T1).
- **Risque connu** : Task 4 Step 2 = baseline verte (refactor à comportement constant), documenté comme tel. Tests radix Select dépendent des shims de T1 — T1 les pose avant tout test Select.
