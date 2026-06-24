# Décision — Migration de l'admin SPA : Yew → React/Vite/shadcn-ui

> **Statut : DÉCISION ACTÉE. Base technique à brainstormer en session neuve.**
> Ce document range la discussion de refonte pour reprendre à froid. Il ne fige PAS
> la stack React détaillée (routeur, data layer, i18n lib, pipeline build) — c'est
> l'objet du prochain brainstorm. Il fige le **pourquoi**, le **périmètre**, **ce qui
> est gardé/recyclé**, et les **questions ouvertes**.

## 1. Contexte & décision

L'admin a été bâti en **Yew + shadcn-rs** (choix « PoC technique, fun > simplicité »,
explicitement assumé au cadrage). À l'usage : **beaucoup de friction pour peu de gain**.
La cause précise est **`shadcn-rs` en 0.1** (lib immature : `Switch` cassé, vars CSS
manquantes, keyframes dupliquées, toasts sans auto-dismiss, badge écrasé par spécificité…
on a vendorisé/patché à chaque écran) **et l'outillage wasm** (boucle build→test lente,
bugs visibles seulement au rendu, pas de mock/Storybook/HMR mûrs).

**Décision** : migrer l'admin SPA vers **React + Vite + shadcn/ui + Tailwind** (écosystème
mature : docs abondantes, composants automatisés, HMR, MSW, Storybook, component-testing).
Motivations : vélocité de dev, **qualité produit visée « top »**, et cohérence avec
**Fumadocs** (React/Next) prévu pour la landing + doc (item ROADMAP, chantier séparé).

**Le backend reste Rust** (Loco + SeaORM + SQLite + futur MCP) — c'est une force, pas en cause.

## 2. Périmètre — ce qui est gardé vs jeté

**Gardé (backend Phase 3, agnostique du framework front)** — c'est ce qui fait qu'une SPA
React se branche proprement, et qui n'était PAS sur `main` (uniquement sur la branche) :
- API JSON re-préfixée sous **`/api/*`** (`controllers/admin.rs`, `auth.rs`).
- **Serving statique sous `/admin`** : `nest_service("/admin", ServeDir + ServeFile fallback)`
  dans `app.rs`, racine `LATCH_SPA_DIST` (`web::spa_dist_dir`). Sert **n'importe quel** dist
  statique (test `backend/tests/spa_serving.rs` utilise un faux dist tempdir → agnostique).
- Garde **Origin** sur les mutations, **session cookie** same-origin, invariants sécu §9
  (tests `security_invariants.rs`).
- Crate **`latch-dto`** : reste la source Rust des shapes sérialisées côté backend
  (le partage Rust back↔front disparaît ; côté React → types TS, cf. §4).

**Jeté (fait dans cette session)** :
- La crate Yew **`frontend/`** supprimée (`git rm -r frontend`), retirée des `members` du
  workspace racine. Backend compile + **86 tests verts** sans elle.
- Branche : **`feat/admin-react`** créée depuis l'état polished (backend Phase 3 + thème).
  La branche Yew `feat/phase-3-spa-yew-admin` reste en référence dans git ; `main` intouché.
  Le code Yew vit dans l'historique git (à consulter comme référence d'implémentation).

## 3. Ce qui se recycle (NE PAS réinventer)

Le comportement et l'UX de l'admin sont **entièrement spécifiés** — la réécriture React est
du portage, pas de la conception :
- **Comportement par page = contrat `docs/contrat-deploy.md` §7** (rails par page) : login,
  liste, détail (lecture seule), side-panels création/édition (`ProjectForm`), déploiement
  (`DeployPanel`), suppressions danger, prévisualisation `no-store`, slug RO, URL via origin.
- **Catalogue i18n FR+EN complet** : les chaînes existent (anciennement
  `frontend/locales/{en,fr}.yml` — récupérables dans l'historique git du commit Yew). À
  porter en JSON pour la lib i18n React. Clés groupées : `login/list/detail/form/deploy/danger/common/toast/error`.
- **Endpoints API** (client à réécrire en TS, tous sous `/api/*`) : `list_projects`,
  `get_project`, `create_project`, `update_project`, `delete_project`, `set_code`,
  `clear_code`, `deploy`, `activate_version`, `delete_version`, `login`, `logout`,
  `preview_url`. Sémantique gloo-net → fetch : un 401 n'est pas une erreur réseau, inspecter `status`.
- **Shapes DTO** : `latch-dto/src/lib.rs` (ProjectListItem sans PIN, ProjectDetail avec PIN,
  VersionItem, CreateProjectReq, UpdateProjectReq, SetCodeReq, DeployReq, LoginReq) → types TS.
- **Thème de marque** : l'export **oklch** fourni par l'humain se **colle directement** dans
  `globals.css` shadcn/ui (Tailwind gère oklch nativement) — **plus aucune conversion**
  (la gymnastique oklch→HSL du Yew devient inutile). C'est un gain net de la migration.
- **Décisions UX du polish** (toutes à reproduire) : badges accès colorés (vert PIN requis /
  orange libre), **toasts sur toutes les actions** (création/édition/déploiement/activation/
  suppression/copie + erreurs), **PIN affiché mais désactivé** quand code off, **slug
  désactivé** en édition, **dropzone drag-and-drop**, **helper text** par champ + **intros**
  de page, **accessibilité** (boutons focusables, pas de `<a onclick>` sans href), **sélecteur
  FR/EN persistant** (localStorage + détection navigateur, défaut EN), espacements/login soignés.

## 4. Contraintes (inchangées, NON négociables)
- **Sécu §9** : aucune réponse ne contient de hash ; le PIN n'apparaît qu'au détail (jamais
  en liste, jamais via MCP). Le DTO liste n'a structurellement pas de champ pin.
- **Auth** : cookie session same-origin (HttpOnly ; Secure+`__Host-` en prod), garde Origin
  sur mutations. Le front React vit sous la même origine (`/admin`), API sous `/api` → cookies
  envoyés automatiquement, pas de token à stocker.
- **Confidentialité** : aucun nom de client réel nulle part (placeholders fictifs).
- **`/c/<slug>`** (Phase 4) : `no-store`, server-rendered — hors SPA admin.

## 5. Questions ouvertes — à BRAINSTORMER en session neuve (la « base technique »)
- **Stack exacte** : Vite + React 18 + TS + Tailwind + shadcn/ui (à confirmer ; vs Next/TanStack
  Start — probablement **Vite SPA simple** servie en statique, pas de SSR car serving Loco).
- **Routeur** : react-router vs TanStack Router ; base path `/admin` ; deep-link fallback (déjà
  géré côté serveur). (Rappel piège Yew : basename cassé — côté React, configurer `base: '/admin/'`
  Vite + basename routeur, plus propre.)
- **Types TS depuis `latch-dto`** : écrits à la main vs générés (ts-rs / schemars→OpenAPI). Trancher.
- **Data fetching / state** : fetch + TanStack Query ? ou minimal. **Forms** : react-hook-form + zod.
- **i18n** : react-i18next (ou Lingui/Paraglide). Recycler le catalogue FR/EN.
- **Toasts** : shadcn **sonner** (natif, auto-dismiss — fin de la couche maison).
- **Tests** : Vitest + Testing Library + **MSW** (front isolé sans backend) + Playwright e2e.
- **Pipeline build** : nouveau dossier (`frontend/` Vite ou `admin/`) ; **package manager** (pnpm ?).
- **Gestion mono-repo** : app Node hors workspace Cargo (déjà retirée des members).
- **Fumadocs** (landing + doc, GH Pages) = **chantier SÉPARÉ** (item ROADMAP), à brainstormer après.

## 6. Dette laissée sur la branche (à traiter DANS la migration, pas avant)
Ces références au front Yew sont volontairement laissées cassées sur `feat/admin-react`
(le pipeline build React est l'objet de la session neuve) — **CI/Docker rouges attendus sur
cette branche WIP** jusqu'au setup React :
- `Dockerfile` : stage `frontend` (`trunk build --release`) → à remplacer par un stage **node/pnpm**
  (`vite build`) copiant le `dist` vers `/app/frontend/dist` (ou nouveau chemin).
- `.github/workflows/ci.yml` : job `frontend` (trunk + wasm) + `needs:[…frontend…]` du job docker →
  à remplacer par un job node (install/lint/typecheck/test/build).
- `backend/src/web/mod.rs` : défaut `../frontend/dist` (à reconfirmer selon le dossier React).
- `.env.example` (commentaire `LATCH_SPA_DIST`), `.gitignore` (`/frontend/dist`) : à réaligner.
- `docs/BOOTSTRAP.md` (§1 stack, §3 commandes, §6 CI, §7 Docker) + `docs/contrat-deploy.md` §4
  (rendu) : à mettre à jour vers la stack React une fois tranchée.

## 7. Reprise — pour la session neuve
1. Lire ce doc + `docs/contrat-deploy.md` §7 (UX exacte à porter) + `latch-dto/src/lib.rs` (shapes).
2. Récupérer le catalogue i18n depuis la branche Yew (qui conserve `frontend/`) :
   `git show feat/phase-3-spa-yew-admin:frontend/locales/en.yml` (et `fr.yml`).
   Idem pour tout composant Yew de référence (`pages/`, `panels/`, `api/client.rs`).
3. Brainstormer la base technique (§5), trancher, écrire le spec de migration, puis le plan.
4. Garder le backend tel quel ; livrer l'app React + reconnecter Docker/CI (§6).
