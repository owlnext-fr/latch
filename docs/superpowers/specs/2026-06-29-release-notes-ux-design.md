# Spec — Patchs UX release-notes

> Date : 2026-06-29
> Statut : design validé, prêt pour plan d'implémentation.
> Suite de la feature « notes de version » (`2026-06-29-release-notes-design.md`).

## Intention

Trois ajustements UX admin, **100 % frontend** (aucun nouvel endpoint) :

1. **Preview depuis la liste projets** — pouvoir prévisualiser la version active d'un
   projet directement depuis la liste, sans entrer dans le détail.
2. **Icône au lieu de l'emoji** — l'indicateur « a des notes » dans la liste des
   versions passe de l'emoji `📝` à une icône lucide (cohérence visuelle).
3. **Panel de détail de version** — un bouton « Détail » sur chaque ligne de version
   ouvre un side-panel read-only montrant les métadonnées et les notes de version
   rendues (revoir les notes après déploiement).

## Contexte technique (existant réutilisé)

- Route preview admin : `GET /api/projects/{id}/versions/{n}/preview` (déjà utilisée
  dans `detail.tsx`, ouverte en nouvel onglet).
- `ProjectListItem` porte `id` et `active_version_n` (numéro de la version active, ou
  `null`).
- `VersionItem` porte `id`, `n`, `created_at`, `is_active`, `release_notes` — déjà
  chargé dans `project.versions` via `useProject`. **Aucun fetch supplémentaire.**
- `MarkdownView` (`@/lib/markdown`) : rendu markdown restreint, identique à l'overlay
  visiteur.
- Pattern de side-panel : `Sheet` (cf. `delete-version-panel.tsx`).
- Icônes : `lucide-react` (déjà dépendance).

## Décisions validées

| Sujet | Décision |
|---|---|
| Panel de détail | **Bouton « Détail » par ligne**, actions de ligne (Activer/Preview/Supprimer) **conservées**. Panel **read-only**. |
| Preview liste | **Colonne d'actions** en fin de ligne, bouton Preview (icône `Eye`). |
| Icône notes | `FileText` (lucide), `size-4 text-muted-foreground`. |

---

## 1. Preview depuis la liste projets

**Fichier** : `frontend/src/routes/list.tsx`.

- Ajouter une **colonne d'actions** (en-tête vide) en fin de table.
- Par ligne : un bouton Preview (icône `Eye`, `aria-label` via i18n) rendu comme un
  `<a href={previewUrl(project.id, project.active_version_n)} target="_blank"
  rel="noopener noreferrer">` quand `active_version_n != null` ; sinon un bouton
  **désactivé** (ou rien) avec un `title` expliquant l'absence de version active.
- Helper `previewUrl(projectId, n)` : identique à celui de `detail.tsx`
  (`/api/projects/${projectId}/versions/${n}/preview`). À factoriser dans
  `@/lib/utils` (ou un petit module partagé) pour éviter la duplication entre
  `list.tsx` et `detail.tsx`.
- i18n : `list.preview_aria` (« Preview active version » / « Aperçu de la version active »).

## 2. Icône au lieu de l'emoji

**Fichier** : `frontend/src/routes/detail.tsx` (cellule statut de la table versions).

- Remplacer le `<span>📝</span>` par `<FileText className="size-4 text-muted-foreground" />`
  (import depuis `lucide-react`), en conservant `title`/`aria-label` = `t('detail.has_notes')`.
- Aucune nouvelle clé i18n.

## 3. Panel de détail de version (nouveau composant)

**Fichier créé** : `frontend/src/components/version-detail-panel.tsx`.

- Props : `{ projectId: number; version: VersionItem; open: boolean; onOpenChange: (open: boolean) => void }`.
- `Sheet` (read-only), même structure que `delete-version-panel.tsx` :
  - `SheetTitle` : `t('version_detail.title', { n: version.n })` → « Version v{n} ».
  - Métadonnées : date de déploiement (`new Date(version.created_at).toLocaleDateString()`),
    badge **active** si `version.is_active` (réutilise le `Badge` vert existant).
  - Notes : si `version.release_notes` non vide → `<MarkdownView source={version.release_notes} />`
    sous un libellé `t('version_detail.notes_label')` ; sinon état vide
    `t('version_detail.no_notes')`.
  - Pas d'actions (read-only) ; un bouton **Close** (`common.close` si existe, sinon
    réutiliser le pattern `common.cancel`/fermeture du Sheet).
- i18n : `version_detail.title`, `version_detail.date_label`, `version_detail.notes_label`,
  `version_detail.no_notes`.

**Fichier modifié** : `frontend/src/routes/detail.tsx`.

- État : `const [detailVersion, setDetailVersion] = useState<VersionItem | null>(null)`
  (même pattern que `deleteVersion`).
- Ajouter un bouton **« Détail »** dans la cellule d'actions de chaque ligne (à côté
  d'Activer/Preview/Supprimer), `onClick={() => setDetailVersion(v)}`. i18n :
  `detail.detail_aria`.
- Monter `<VersionDetailPanel>` en bas (comme `DeleteVersionPanel`), conditionné par
  `detailVersion`.

---

## i18n (clés à ajouter, EN + FR, parité obligatoire)

| Clé | EN | FR |
|---|---|---|
| `list.preview_aria` | Preview active version | Aperçu de la version active |
| `detail.detail_aria` | Details | Détail |
| `version_detail.title` | Version v{{n}} | Version v{{n}} |
| `version_detail.date_label` | Deployed on | Déployée le |
| `version_detail.notes_label` | Release notes | Notes de version |
| `version_detail.no_notes` | No release notes for this version. | Aucune note pour cette version. |

(`detail.has_notes` existe déjà ; réutilisé pour l'icône.)

## Tests (Vitest)

- **`list.tsx`** : le bouton Preview pointe `/api/projects/{id}/versions/{active_version_n}/preview`
  quand une version active existe ; absent/désactivé quand `active_version_n == null`.
- **`detail.tsx`** : l'icône `FileText` (et non l'emoji) s'affiche pour une version
  avec notes ; le bouton « Détail » ouvre le panel.
- **`version-detail-panel.tsx`** : rend les notes via `MarkdownView` quand présentes ;
  affiche l'état vide sinon ; affiche v{n} + date + badge actif.
- **Parité i18n** EN/FR (couverte par le test de parité existant).
- Gates : `pnpm lint`, `pnpm typecheck`, `pnpm test`, `pnpm build`. Pas de e2e
  (UI admin, couverte par Vitest). SonarCloud `new_coverage ≥ 80 %`, duplication < 3 %.

## Documentation publique (Fumadocs — `public_docs/`)

À mettre à jour **dans le même chantier** :

- **`content/docs/admin/projects.mdx`** — section « project list » / « detail view » :
  mentionner l'action **Preview** disponible **depuis la liste** (ouvre la version
  active dans un nouvel onglet, route admin `no-store` ; indisponible tant qu'aucune
  version n'est active).
- **`content/docs/admin/versions.mdx`** :
  - « Version indicator » (ligne ~53) — remplacer « show a 📝 indicator » par la
    mention d'une **icône** (icône de notes) ; ne plus citer l'emoji.
  - « Per-version actions » (liste ~65-72) — ajouter une puce **Details** : ouvre un
    side-panel **read-only** montrant le numéro de version, la date, le statut, et les
    **notes de version rendues** (identiques à ce que voit le visiteur).

## Mémoire projet (fin d'implémentation)

- `docs/INDEX.md` : compléter la ligne release-notes (patchs UX) ou ajouter une entrée.
- `docs/HANDOFF.md` : entrée datée.
- `docs/CONVENTIONS.md` : si `previewUrl` est factorisé dans `@/lib/utils`, noter le helper.
- (Pas de changement contrat/QUIRKS attendu — comportement serveur inchangé.)
