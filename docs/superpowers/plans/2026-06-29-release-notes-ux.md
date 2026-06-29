# Patchs UX release-notes — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Trois ajustements UX admin pour les notes de version : preview de la version active depuis la liste projets, icône (au lieu de l'emoji) pour l'indicateur de notes, et un side-panel read-only de détail de version montrant les notes rendues.

**Architecture:** 100 % frontend, aucun nouvel endpoint. Réutilise la route preview admin existante, `VersionItem.release_notes` déjà chargé via `useProject`, `MarkdownView` pour le rendu, et le pattern `Sheet` pour le panel.

**Tech Stack:** React + TypeScript + Vite + TanStack Router/Query + react-i18next + lucide-react + Vitest/MSW.

## Global Constraints

- 100 % frontend, **aucun nouvel endpoint** ; route preview réutilisée : `/api/projects/{id}/versions/{n}/preview`.
- Périmètre markdown / rendu inchangé : le panel réutilise `MarkdownView` de `@/lib/markdown` (barrière XSS).
- i18n **clés plates** (`section.key`, `keySeparator:false`), **parité EN/FR obligatoire** (test de parité existant).
- Liens externes/preview : `target="_blank" rel="noopener noreferrer"`.
- Confidentialité : aucun nom de client réel (placeholders `Mon Projet`, `ACME`, `demo`).
- Tests : Vitest + Testing Library + MSW (harness existant dans `src/routes/*.test.tsx`). Pas de e2e.
- Definition of done : `pnpm lint` + `pnpm typecheck` + `pnpm test` + `pnpm build` verts ; SonarCloud `new_coverage ≥ 80 %`, duplication < 3 % ; docs (Fumadocs + mémoire) à jour.
- Commandes frontend DEPUIS `frontend/`, préfixées `rtk`.

---

## File Structure

**Créés**
- `frontend/src/components/version-detail-panel.tsx` — panel read-only de détail de version.
- `frontend/src/components/version-detail-panel.test.tsx` — tests du panel.

**Modifiés**
- `frontend/src/lib/utils.ts` — helper `previewUrl` (factorisé).
- `frontend/src/routes/detail.tsx` — import `previewUrl`, icône `FileText`, bouton « Détail », montage du panel.
- `frontend/src/routes/list.tsx` — colonne d'actions + bouton Preview (`Eye`).
- `frontend/src/routes/detail.test.tsx`, `frontend/src/routes/list.test.tsx` — cas de test ajoutés.
- `frontend/src/i18n/locales/admin/{en,fr}.json` — clés `list.preview_aria`, `detail.detail_aria`, `version_detail.*`.
- `public_docs/content/docs/admin/projects.mdx`, `public_docs/content/docs/admin/versions.mdx` — doc publique.
- `docs/INDEX.md`, `docs/HANDOFF.md`, `docs/CONVENTIONS.md` — mémoire.

---

## Task 1 : Factoriser `previewUrl` dans `@/lib/utils`

**Files:**
- Modify: `frontend/src/lib/utils.ts`
- Modify: `frontend/src/routes/detail.tsx:36-38` (supprimer le helper local, importer celui de utils)
- Test: `frontend/src/lib/utils.test.ts`

**Interfaces:**
- Produces: `export function previewUrl(projectId: number, n: number): string` → `/api/projects/${projectId}/versions/${n}/preview`.

- [ ] **Step 1 : Écrire le test (échec attendu)**

Créer/compléter `frontend/src/lib/utils.test.ts` :

```ts
import { describe, it, expect } from 'vitest'
import { previewUrl } from './utils'

describe('previewUrl', () => {
  it('builds the admin preview route for a project version', () => {
    expect(previewUrl(7, 3)).toBe('/api/projects/7/versions/3/preview')
  })
})
```

(Si `utils.test.ts` existe déjà, ajouter ce `describe` ; sinon le créer avec ces imports.)

- [ ] **Step 2 : Lancer pour vérifier l'échec**

Run : `rtk pnpm test utils`
Expected : FAIL — `previewUrl` n'est pas exporté.

- [ ] **Step 3 : Implémenter le helper**

Dans `frontend/src/lib/utils.ts`, ajouter après `publicUrl` :

```ts
/** Route admin de prévisualisation d'une version (HTML brut, no-store, derrière la session). */
export function previewUrl(projectId: number, n: number): string {
  return `/api/projects/${projectId}/versions/${n}/preview`
}
```

- [ ] **Step 4 : Refactor `detail.tsx` pour utiliser le helper partagé**

Dans `frontend/src/routes/detail.tsx` :
- Supprimer la fonction locale `previewUrl` (lignes ~36-38).
- Ajouter `previewUrl` à l'import existant depuis `@/lib/utils` :
  `import { publicUrl, previewUrl } from '@/lib/utils'`.

- [ ] **Step 5 : Vérifier**

Run : `rtk pnpm test utils && rtk pnpm typecheck`
Expected : PASS.

- [ ] **Step 6 : Commit**

```bash
git add frontend/src/lib/utils.ts frontend/src/lib/utils.test.ts frontend/src/routes/detail.tsx
git commit -m "refactor(front): factorise previewUrl dans lib/utils"
```

---

## Task 2 : Preview de la version active depuis la liste

**Files:**
- Modify: `frontend/src/routes/list.tsx`
- Modify: `frontend/src/i18n/locales/admin/en.json`, `frontend/src/i18n/locales/admin/fr.json`
- Test: `frontend/src/routes/list.test.tsx`

**Interfaces:**
- Consumes: `previewUrl` (Task 1), `ProjectListItem.active_version_n`, `ProjectListItem.id`.

- [ ] **Step 1 : Ajouter les clés i18n**

`en.json` (à côté des autres `list.*`) : `"list.preview_aria": "Preview active version",`
`fr.json` : `"list.preview_aria": "Aperçu de la version active",`

- [ ] **Step 2 : Écrire le test (échec attendu)**

Dans `frontend/src/routes/list.test.tsx`, en suivant le harness existant (render via le helper de ce fichier, MSW pour `/api/projects`), ajouter deux cas. Adapter les noms d'helper de rendu à ceux déjà présents dans le fichier :

```tsx
it('shows a preview link to the active version for a deployed project', async () => {
  // fixture projet avec active_version_n = 2, id = 1 (via le mock MSW du fichier)
  renderList() // helper existant du fichier
  const link = await screen.findByRole('link', { name: /preview active version/i })
  expect(link).toHaveAttribute('href', '/api/projects/1/versions/2/preview')
  expect(link).toHaveAttribute('target', '_blank')
})

it('does not show a preview link when the project has no active version', async () => {
  // fixture projet avec active_version_n = null
  renderList()
  await screen.findByText(/* un texte présent dans la liste, ex. le nom du projet */ 'Mon Projet')
  expect(screen.queryByRole('link', { name: /preview active version/i })).toBeNull()
})
```

> Reprendre exactement les fixtures/MSW du fichier ; ajouter au mock un projet `active_version_n: null` si nécessaire pour le 2ᵉ cas.

- [ ] **Step 3 : Lancer pour vérifier l'échec**

Run : `rtk pnpm test list`
Expected : FAIL — pas de lien preview.

- [ ] **Step 4 : Implémenter la colonne d'actions**

Dans `frontend/src/routes/list.tsx` :
- Importer l'icône : `import { Eye } from 'lucide-react'` et le helper : ajouter `previewUrl` à l'import `@/lib/utils`.
- Ajouter un `<TableHead />` (colonne vide) à la fin du `<TableRow>` d'en-tête.
- Ajouter en fin de chaque ligne la cellule :

```tsx
<TableCell className="text-right">
  {project.active_version_n == null ? (
    <span
      className="text-muted-foreground/40 inline-flex h-8 w-8 items-center justify-center"
      title={t('list.preview_aria')}
      aria-hidden="true"
    >
      <Eye className="size-4" />
    </span>
  ) : (
    <a
      href={previewUrl(project.id, project.active_version_n)}
      target="_blank"
      rel="noopener noreferrer"
      aria-label={t('list.preview_aria')}
      title={t('list.preview_aria')}
      className="text-muted-foreground hover:bg-accent hover:text-accent-foreground inline-flex h-8 w-8 items-center justify-center rounded-md"
    >
      <Eye className="size-4" />
    </a>
  )}
</TableCell>
```

- [ ] **Step 5 : Vérifier**

Run : `rtk pnpm test list && rtk pnpm typecheck && rtk pnpm lint`
Expected : PASS.

- [ ] **Step 6 : Commit**

```bash
git add frontend/src/routes/list.tsx frontend/src/routes/list.test.tsx frontend/src/i18n/locales/admin/
git commit -m "feat(front): preview de la version active depuis la liste projets"
```

---

## Task 3 : Composant `VersionDetailPanel` (read-only)

**Files:**
- Create: `frontend/src/components/version-detail-panel.tsx`
- Create: `frontend/src/components/version-detail-panel.test.tsx`
- Modify: `frontend/src/i18n/locales/admin/en.json`, `frontend/src/i18n/locales/admin/fr.json`

**Interfaces:**
- Consumes: `MarkdownView` (`@/lib/markdown`), `VersionItem` (`components['schemas']['VersionItem']`), `Sheet`/`Badge` UI.
- Produces: `export function VersionDetailPanel({ version, open, onOpenChange }: { version: VersionItem; open: boolean; onOpenChange: (open: boolean) => void }): JSX.Element`.

- [ ] **Step 1 : Ajouter les clés i18n**

`en.json` :
```json
"version_detail.title": "Version v{{n}}",
"version_detail.date_label": "Deployed on",
"version_detail.notes_label": "Release notes",
"version_detail.no_notes": "No release notes for this version.",
```
`fr.json` :
```json
"version_detail.title": "Version v{{n}}",
"version_detail.date_label": "Déployée le",
"version_detail.notes_label": "Notes de version",
"version_detail.no_notes": "Aucune note pour cette version.",
```

- [ ] **Step 2 : Écrire le test (échec attendu)**

Créer `frontend/src/components/version-detail-panel.test.tsx` :

```tsx
import { render, screen } from '@testing-library/react'
import { describe, it, expect, vi } from 'vitest'
import { I18nextProvider } from 'react-i18next'
import i18n from '@/i18n'
import { VersionDetailPanel } from './version-detail-panel'
import type { components } from '@/api/schema'

type VersionItem = components['schemas']['VersionItem']

function renderPanel(version: VersionItem) {
  return render(
    <I18nextProvider i18n={i18n}>
      <VersionDetailPanel version={version} open onOpenChange={vi.fn()} />
    </I18nextProvider>,
  )
}

const base: VersionItem = {
  id: 10,
  n: 2,
  created_at: '2024-01-15T10:00:00Z',
  is_active: true,
}

describe('VersionDetailPanel', () => {
  it('renders rendered release notes when present', () => {
    renderPanel({ ...base, release_notes: '# Hello\n\n- a' })
    expect(screen.getByRole('heading', { name: 'Hello' })).toBeInTheDocument()
    expect(screen.getByRole('list')).toBeInTheDocument()
  })

  it('shows the empty state when there are no notes', () => {
    renderPanel({ ...base, release_notes: null })
    expect(
      screen.getByText(/no release notes for this version/i),
    ).toBeInTheDocument()
  })
})
```

- [ ] **Step 3 : Lancer pour vérifier l'échec**

Run : `rtk pnpm test version-detail-panel`
Expected : FAIL — composant inexistant.

- [ ] **Step 4 : Implémenter le composant**

Créer `frontend/src/components/version-detail-panel.tsx` :

```tsx
import { useTranslation } from 'react-i18next'
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet'
import { Badge } from '@/components/ui/badge'
import { MarkdownView } from '@/lib/markdown'
import type { components } from '@/api/schema'

type VersionItem = components['schemas']['VersionItem']

interface VersionDetailPanelProps {
  version: VersionItem
  open: boolean
  onOpenChange: (open: boolean) => void
}

/**
 * Détail read-only d'une version : métadonnées + notes de version rendues
 * (MarkdownView, identique à l'overlay visiteur) ou état vide. Fermeture via le
 * bouton X intégré du Sheet.
 */
export function VersionDetailPanel({
  version,
  open,
  onOpenChange,
}: Readonly<VersionDetailPanelProps>) {
  const { t } = useTranslation()

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent className="w-full overflow-y-auto sm:max-w-md">
        <SheetHeader>
          <SheetTitle className="flex items-center gap-2">
            {t('version_detail.title', { n: version.n })}
            {version.is_active && (
              <Badge className="bg-green-600 text-white hover:bg-green-600">
                {t('common.active')}
              </Badge>
            )}
          </SheetTitle>
        </SheetHeader>

        <div className="flex flex-col gap-4 p-4">
          <div>
            <p className="text-muted-foreground mb-0.5 text-xs font-medium">
              {t('version_detail.date_label')}
            </p>
            <p className="text-sm">
              {new Date(version.created_at).toLocaleDateString()}
            </p>
          </div>

          <div>
            <p className="text-muted-foreground mb-1 text-xs font-medium">
              {t('version_detail.notes_label')}
            </p>
            {version.release_notes ? (
              <div className="rounded-md border border-input px-3 py-2">
                <MarkdownView source={version.release_notes} />
              </div>
            ) : (
              <p className="text-muted-foreground text-sm">
                {t('version_detail.no_notes')}
              </p>
            )}
          </div>
        </div>
      </SheetContent>
    </Sheet>
  )
}
```

- [ ] **Step 5 : Lancer pour vérifier le succès**

Run : `rtk pnpm test version-detail-panel`
Expected : PASS (2 tests).

- [ ] **Step 6 : Commit**

```bash
git add frontend/src/components/version-detail-panel.tsx frontend/src/components/version-detail-panel.test.tsx frontend/src/i18n/locales/admin/
git commit -m "feat(front): panel read-only de détail de version (notes rendues)"
```

---

## Task 4 : Câbler le détail dans la table versions + icône

**Files:**
- Modify: `frontend/src/routes/detail.tsx`
- Modify: `frontend/src/i18n/locales/admin/en.json`, `frontend/src/i18n/locales/admin/fr.json`
- Test: `frontend/src/routes/detail.test.tsx`

**Interfaces:**
- Consumes: `VersionDetailPanel` (Task 3), `FileText` (lucide).

- [ ] **Step 1 : Ajouter la clé i18n du bouton**

`en.json` : `"detail.detail_aria": "Details",`
`fr.json` : `"detail.detail_aria": "Détail",`

- [ ] **Step 2 : Écrire le test (échec attendu)**

Dans `frontend/src/routes/detail.test.tsx` (harness existant : MSW renvoie un `ProjectDetail` avec une version qui a `release_notes`), ajouter :

```tsx
it('replaces the notes emoji with an icon and opens the version detail panel', async () => {
  // fixture : au moins une version avec release_notes = '# Notes\n\n- x'
  renderDetail() // helper existant du fichier
  // l'emoji ne doit plus apparaître
  expect(screen.queryByText('📝')).toBeNull()
  // le bouton Détail ouvre le panel → notes rendues visibles
  const detailButtons = await screen.findAllByRole('button', { name: /détail|details/i })
  await userEvent.click(detailButtons[0])
  expect(await screen.findByRole('heading', { name: 'Notes' })).toBeInTheDocument()
})
```

> Adapter `renderDetail`/fixtures aux helpers réels du fichier ; s'assurer qu'au moins une version de la fixture porte `release_notes`.

- [ ] **Step 3 : Lancer pour vérifier l'échec**

Run : `rtk pnpm test detail`
Expected : FAIL.

- [ ] **Step 4 : Remplacer l'emoji par l'icône**

Dans `frontend/src/routes/detail.tsx` :
- Import : `import { Zap, FileText } from 'lucide-react'` (ajouter `FileText` à l'import `Zap` existant).
- Remplacer le bloc emoji (lignes ~215-223) par :

```tsx
{v.release_notes ? (
  <FileText
    className="text-muted-foreground size-4"
    aria-label={t('detail.has_notes')}
  />
) : null}
```

(garder le `title` accessible si souhaité : ajouter `aria-label` suffit pour l'a11y ; on peut aussi wrapper dans un `<span title={t('detail.has_notes')}>`.)

- [ ] **Step 5 : Ajouter le bouton « Détail » + monter le panel**

Dans `frontend/src/routes/detail.tsx` :
- Import : `import { VersionDetailPanel } from '@/components/version-detail-panel'`.
- État : ajouter `const [detailVersion, setDetailVersion] = useState<VersionItem | null>(null)` (près de `deleteVersion`).
- Dans la cellule d'actions de chaque ligne (avant ou après le lien Preview), ajouter :

```tsx
<Button
  type="button"
  variant="ghost"
  size="sm"
  aria-label={t('detail.detail_aria')}
  onClick={() => setDetailVersion(v)}
>
  {t('detail.detail_aria')}
</Button>
```

- Monter le panel en bas, à côté de `DeleteVersionPanel` :

```tsx
{detailVersion && (
  <VersionDetailPanel
    version={detailVersion}
    open={detailVersion !== null}
    onOpenChange={(isOpen) => {
      if (!isOpen) setDetailVersion(null)
    }}
  />
)}
```

- [ ] **Step 6 : Lancer pour vérifier le succès**

Run : `rtk pnpm test detail && rtk pnpm typecheck && rtk pnpm lint`
Expected : PASS.

- [ ] **Step 7 : Commit**

```bash
git add frontend/src/routes/detail.tsx frontend/src/routes/detail.test.tsx frontend/src/i18n/locales/admin/
git commit -m "feat(front): icône notes + bouton Détail (panel) sur la table versions"
```

---

## Task 5 : Documentation (Fumadocs + mémoire)

**Files:**
- Modify: `public_docs/content/docs/admin/projects.mdx`
- Modify: `public_docs/content/docs/admin/versions.mdx`
- Modify: `docs/INDEX.md`, `docs/HANDOFF.md`, `docs/CONVENTIONS.md`

- [ ] **Step 1 : `projects.mdx` — preview depuis la liste**

Dans la section sur la liste des projets, ajouter une mention de l'action **Preview** disponible **depuis la liste** : chaque ligne propose une action Preview qui ouvre la **version active** dans un nouvel onglet (route admin, `no-store`, derrière la session) ; indisponible tant que le projet n'a pas de version active. Style/ton cohérent avec l'existant.

- [ ] **Step 2 : `versions.mdx` — icône + action Details**

- Section « Version indicator » (~ligne 53) : remplacer « show a 📝 indicator » par la mention d'une **icône** (icône de notes), sans citer l'emoji.
- Section « Per-version actions » (liste ~65-72) : ajouter une puce **Details** — « open a read-only side-panel showing the version number, deploy date, status, and the release notes rendered exactly as visitors see them ».

- [ ] **Step 3 : Mémoire**

- `docs/INDEX.md` : compléter la ligne release-notes (patchs UX : preview liste, icône, panel détail) ou ajouter une entrée datée.
- `docs/HANDOFF.md` : entrée datée 2026-06-29 (Dernière chose faite / Trucs en suspens / Prochaine chose à creuser / Notes pour future Claude).
- `docs/CONVENTIONS.md` : noter le helper `previewUrl` (`@/lib/utils`) et le pattern « panel read-only via Sheet + MarkdownView ».

- [ ] **Step 4 : Commit**

```bash
git add public_docs/ docs/
git commit -m "docs: patchs UX release-notes (Fumadocs + mémoire)"
```

---

## Task 6 : Vérification finale

**Files:** aucun (gates).

- [ ] **Step 1 : Gates frontend**

Run (depuis `frontend/`) :
```bash
rtk pnpm lint && rtk pnpm typecheck && rtk pnpm test && rtk pnpm build
```
Expected : tout vert (lint 0, tsc 0, vitest tous verts incl. parité i18n, build OK avec `dist/` complet).

- [ ] **Step 2 : Confidentialité + cohérence**

Vérifier : aucun nom de client réel ajouté (docs/tests/fixtures) ; périmètre markdown du panel = `MarkdownView` (inchangé) ; l'emoji `📝` n'apparaît plus dans `detail.tsx`.

- [ ] **Step 3 : Sonar (couverture new-code + duplication)**

Lancer le scan Sonar local (cf. `docs/ENVIRONMENT.md §Scan local`) si exécutable ; confirmer `new_coverage ≥ 80 %` et `new_duplicated_lines_density < 3 %`. Sinon, noter que la gate est déléguée à la CI.

---

## Self-Review (effectuée à la rédaction)

- **Couverture spec** : preview liste (T2), icône (T4 step 4), panel détail + bouton (T3+T4), `previewUrl` factorisé (T1), i18n (T2/T3/T4), Fumadocs `projects.mdx`+`versions.mdx` (T5), mémoire (T5), gates+Sonar (T6). ✔
- **Cohérence des types** : `previewUrl(projectId, n)` défini T1, consommé T2 ; `VersionDetailPanel({version, open, onOpenChange})` défini T3, consommé T4 ; clés i18n `version_detail.*` définies T3, `detail.detail_aria`/`list.preview_aria` cohérentes. ✔
- **Pas de placeholder** : code réel à chaque step ; les seules zones « adapter au harness » sont les helpers de rendu MSW des tests existants (`renderList`/`renderDetail`), signalées explicitement — assertions fournies.
- **YAGNI** : panel read-only sans actions ni `projectId` (non utilisé) ; pas de `common.close` (X intégré du Sheet).
