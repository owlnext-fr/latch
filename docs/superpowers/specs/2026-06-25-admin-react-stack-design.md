# Design — Base technique de l'admin React (migration Yew → React/Vite)

> **Statut : DESIGN VALIDÉ.** Fait suite à la décision actée dans
> `2026-06-25-admin-react-migration-decision.md` (le *pourquoi* et le *périmètre*).
> Ce document fige la **base technique** (le *comment*) : stack, contrat de typage,
> architecture front, tests, CI, Docker, et l'alignement des docs mémoire.
> Prochaine étape : plan d'implémentation (writing-plans).

## 1. Décisions en un coup d'œil

| Sujet | Décision |
|---|---|
| Base tech | **Vite + React + TypeScript + TanStack Router** — SPA pure, servie en statique par Loco sous `/admin`. Pas de SSR. |
| Composants UI | **shadcn/ui** (Radix dessous) + **Tailwind**. Thème = preset oklch existant (`bJfDPe2y`). |
| Contrat de typage | **OpenAPI** : `utoipa` côté backend → `openapi.json` commité → `openapi-typescript` → client **`openapi-fetch`** 100 % typé. |
| DTO | Crate `latch-dto` **dissoute** → inlinée dans **`backend/src/dto/`** (dérive `serde` + `utoipa::ToSchema`). |
| État serveur | **TanStack Query** (cache, invalidation). **Pas de state manager** (YAGNI). |
| Forms | **react-hook-form + zod**. |
| i18n | **react-i18next** (+ language-detector) — port du catalogue FR/EN existant. |
| Toasts | **sonner** (shadcn natif — fin de la couche maison). |
| Tests front | **Vitest + Testing Library** (unit) · **MSW** (inté isolée) · **Playwright** (e2e). |
| CI | **Pistes parallèles** (reusable workflows) : back / front / (fuma futur) → **e2e** → **docker**. |
| Supply-chain front | **pnpm audit** (advisories) + **license-checker** (allowlist) — pendant cargo-deny côté Rust. |
| Docker | Stage **Node/pnpm** (remplace Trunk/wasm) → `vite build` → `frontend/dist`. Runtime distroless inchangé. |
| Node | Version pinnée via **`.nvmrc`** (source unique : dev `nvm use`, CI `node-version-file`, image alignée). |
| Dossier | **`frontend/`** (réutilise `LATCH_SPA_DIST=../frontend/dist`). |

**Le backend reste Rust** (Loco + SeaORM + SQLite + futur MCP). Seul le rendu admin change.

## 2. Base technique — Vite + React + TanStack Router (SPA)

- **Vite** : build statique pur, HMR rapide. `base: '/admin/'` (assets préfixés).
- **React 18 + TypeScript strict**.
- **TanStack Router** : routing type-safe (`$id` typé), `basepath: '/admin'`, `beforeLoad`
  pour le guard d'auth, deep-link géré (le fallback `index.html` est déjà servi côté Loco).
- **Pas de SSR / pas de framework serveur** : Loco sert le `dist/` statique, donc TanStack
  **Start** (serveur, server functions) serait à contre-emploi. CRA est déprécié — écarté.
- **Package manager : pnpm** (déjà utilisé par l'humain).

> Rappel piège Yew résolu : le bug `strip_basename` de yew-router 0.18 n'existe pas ici —
> côté Vite/React on configure proprement `base: '/admin/'` + `basepath` du routeur.

## 3. shadcn/ui + thème

- `shadcn init` sur le projet **Vite** (génère `components.json`, Tailwind, `globals.css`).
- **Thème** : application du preset oklch existant (`bJfDPe2y`). Tailwind gère oklch
  nativement → **plus aucune conversion oklch→HSL** (la gymnastique de l'ère Yew disparaît).
- ⚠️ La commande `--template start` du preset cible TanStack **Start** : **on ne la copie
  pas telle quelle**. On `init` en Vite et on applique le **thème** seul.
- Les primitives **Radix** sous shadcn/ui gèrent focus-trap / Escape / aria → **résolvent
  d'office** les composants cassés de shadcn-rs (`Sheet`, `Switch`) et les trous d'a11y.
- Composants exacts (CLI shadcn, versions) **résolus via Context7** au moment du plan.

## 4. Contrat de typage — OpenAPI

Source de vérité = **le backend Rust**. Le front consomme un schéma généré, jamais une
crate Rust partagée.

```
backend (utoipa)                              frontend (Node)
─────────────────                             ───────────────
#[derive(serde, ToSchema)] sur les DTO  ┐
#[utoipa::path] sur chaque route        ├─► ApiDoc::openapi()
                                        ┘          │
   bin/test Rust qui sérialise ─────────────► openapi.json  (COMMITÉ)
                                                     │
                       openapi-typescript ───► frontend/src/api/schema.d.ts (COMMITÉ)
                       + openapi-fetch ──────► client HTTP 100 % typé
```

- **`backend/src/dto/`** : les DTO (ex-`latch-dto`) dérivent `serde` + `utoipa::ToSchema`.
  La subtilité `Option<Option<String>>` de `UpdateProjectReq` (absent vs `null` pour
  effacer `brand_name`) est **préservée**.
- **Génération sans serveur** : un petit binaire/test Rust sérialise `openapi.json`
  **sans démarrer Loco** → le build front ne dépend jamais d'un backend qui tourne.
- **`openapi.json` ET `schema.d.ts` sont commités.** Un job CI **drift-check** les
  régénère et fait `git diff --exit-code` → rouge si un DTO a changé sans régénération.
  C'est le garde-fou anti-drift qui remplace l'ancien partage `latch-dto`.
- **Client : `openapi-fetch`** (typé par `schema.d.ts`), `credentials: 'include'`
  (cookie session same-origin). Sémantique connue : un `401` est un `response.status`,
  pas une exception.
- **Swagger UI** : exposé en **dev uniquement** (`utoipa-swagger-ui` derrière env/flag),
  **absent** de l'image distroless de prod.
- Workspace Cargo repasse à `["backend", "backend/migration"]` (retrait `latch-dto`).

## 5. Architecture front

**Principe** : TanStack Query porte tout l'état serveur (fini le `Load::Loading/Ready/Failed`
manuel). L'état UI local reste en `useState`/contexte. **Pas de Redux/Zustand** : l'état
partagé se réduit à auth + locale (déjà couverts par contexte / react-i18next) ; un store
créerait une 2ᵉ source de vérité et la tentation de dupliquer le cache serveur. Additif si
besoin futur — aucune porte fermée.

### Structure cible

```
frontend/
  .nvmrc                  # version Node (ex. 24)
  package.json · pnpm-lock.yaml · vite.config.ts · tsconfig.json
  components.json         # shadcn
  src/
    main.tsx              # providers: QueryClientProvider, I18nextProvider, RouterProvider, <Toaster/>
    router.tsx            # TanStack Router, basepath /admin, guard auth (beforeLoad)
    api/
      schema.d.ts         # généré (openapi-typescript)
      client.ts           # openapi-fetch + middleware 401 → redirect login
    routes/               # login.tsx · index (list) · projects.$id (detail)
    components/
      project-form.tsx · deploy-panel.tsx · delete-project-panel.tsx · delete-version-panel.tsx
      pin-field.tsx · copy-button.tsx · locale-switcher.tsx
      ui/                 # shadcn (button, sheet, input, badge, table, switch, …)
    hooks/                # use-auth.ts · use-projects.ts (wrappers Query)
    i18n/                 # index.ts + locales/{en,fr}.json
    lib/                  # utils (cn, human_size, public_url via window.location.origin)
  dist/                   # build → servi par Loco sous /admin
```

### Auth (cookie session, pas de token)

- Un **middleware `openapi-fetch`** intercepte les `401` → `router.navigate('/admin/login')`.
- Les routes protégées ont un `beforeLoad` qui vérifie l'état de session.
- Login = mutation `POST /api/auth/login` → invalide le cache → navigate liste.
- Logout = `POST /api/auth/logout` → reset cache → login.
- Le front vit sous la **même origine** (`/admin`), l'API sous `/api` → cookies envoyés
  automatiquement, rien à stocker.

### Comportement = contrat §7 à l'identique

Side-panels (Sheet) créer/éditer, suppressions **danger**, détail **lecture seule**,
**dropzone** deploy, **PIN désactivé** quand code off, **slug RO**, **badges colorés**
(trivial en Tailwind), **sélecteur FR/EN** persistant, **helper text** + intros de page.
La réécriture est du **portage**, pas de la conception.

### Packages clés et justification

| Package | Rôle | Pourquoi |
|---|---|---|
| `@tanstack/react-router` | Routing type-safe, `basepath /admin` | params typés, `beforeLoad` (guard), deep-link |
| `@tanstack/react-query` | État serveur : cache, invalidation | invalide après deploy/delete/activate → UI fraîche ; supprime le boilerplate d'états |
| `openapi-fetch` + `openapi-typescript` | Client HTTP typé | types dérivés du schéma Rust, cookies same-origin |
| `react-hook-form` + `zod` + `@hookform/resolvers` | Forms + validation | nom requis, PIN 6 chiffres ; a11y (aria/focus) intégrée ; perf |
| **shadcn/ui** (Radix) | Composants accessibles | résout l'a11y et les composants cassés de shadcn-rs |
| `sonner` | Toasts auto-dismiss | shadcn natif → fin de la couche maison |
| `react-i18next` + `i18next-browser-languagedetector` | i18n FR/EN | port direct du catalogue ; détection + persistance idiomatiques |
| `tailwindcss` | Styling | via shadcn ; thème oklch s'y colle |

## 6. Tests

| Niveau | Outils | Couverture |
|---|---|---|
| Unit / composant | **Vitest** + Testing Library (`@testing-library/react` + `user-event`) + jsdom | `PinField` (masque/révèle), `CopyButton`, validation `ProjectForm`, utils (`human_size`, `public_url`) |
| Intégration front isolé | **MSW** (mocke `/api/*`) | flows sans backend : liste → affiche ; `401` → redirige login ; deploy → toast + invalidation |
| E2E | **Playwright** contre la stack réelle (Loco + DB test + dist React buildé) | login → créer → déployer → activer → supprimer → logout |

- **Backend** : tests Rust **inchangés** (`nextest`) ; `spa_serving.rs` + `security_invariants.rs`
  conservés. Ajout d'un **test de drift OpenAPI**.
- **Périmètre e2e de la migration** : config Playwright + harness (build front, lance backend,
  DB test) + **1-2 smoke admin** (login → créer → déployer). La **suite e2e complète**
  (switch, delete, serving `/c` + unlock, logout) reste en **Phase 6** (`/c/<slug>` = Phase 4,
  pas encore implémenté).
- **Outillage** : TypeScript **strict**, **ESLint** (`typescript-eslint` + `react-hooks` +
  **`jsx-a11y`**) + **Prettier**. Pas de husky/pre-commit (la CI suffit).

## 7. CI — pistes parallèles

```
ci.yml (orchestrateur — push/PR)
│
├─ piste BACK  → ci-backend.yml    : fmt-clippy · test-backend (nextest + drift OpenAPI) · cargo-deny
├─ piste FRONT → ci-frontend.yml   : lint+typecheck · test (vitest) · build (vite) · supply-chain-front
├─ (futur) piste FUMA → ci-fuma.yml
│
├─ e2e   needs:[back, front]       : monte la stack (front buildé + backend Loco), Playwright
└─ docker needs:[back, front, e2e] : build + push GHCR
```

- **Reusable workflows** (un sous-workflow par piste) : lisible et extensible (Fuma s'ajoute
  sans toucher le reste).
- **Parallélisme** : back et front tournent en parallèle → feedback rapide des deux.
- **Fail-fast** : un step qui échoue rougit sa piste immédiatement ; surtout, `docker` ne
  démarre **jamais** si une piste échoue (on ne gâche pas le long build d'image). Par défaut
  les pistes rendent leur verdict indépendamment (feedback complet) ; un cancel inter-pistes
  reste optionnel.
- **Node** : `actions/setup-node` avec `node-version-file: frontend/.nvmrc`.
- **Supply-chain front** (job bloquant, dont `docker` dépend via la piste front) :
  - **Advisories** : `pnpm audit --audit-level=high` ; ignore-list via
    `pnpm.auditConfig.ignoreCves` (analogue `[advisories] ignore` de `deny.toml`).
  - **Licences** : `license-checker --onlyAllow 'MIT;Apache-2.0;BSD-2-Clause;BSD-3-Clause;ISC;0BSD;CC0-1.0;Unlicense'` (allowlist stricte, esprit cargo-deny).
  - *Alternative notée* : `osv-scanner` (un binaire pour `pnpm-lock.yaml` **et** `Cargo.lock`)
    pour les vulns, mais sans les licences → on garde `license-checker` à côté.

### Fuma (cadrage, chantier séparé)

Fumadocs = landing + doc **statique sur GH Pages**, *a priori hors image runtime latch*. Sa
piste CI aura **sa propre cible** (deploy GH Pages). On décidera au moment venu si elle doit
*aussi* gater `docker` ou rester indépendante. Hors périmètre de cette migration.

## 8. Docker

```dockerfile
# Stage 1 — build SPA React (REMPLACE le stage Trunk/wasm)
FROM node:24-bookworm-slim AS frontend      # version alignée sur frontend/.nvmrc
RUN corepack enable                          # pnpm via corepack
WORKDIR /src/frontend
COPY frontend/package.json frontend/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile           # couche cache : deps seules
COPY frontend/ ./
COPY openapi.json /src/openapi.json          # schéma commité → schema.d.ts au build
RUN pnpm build                               # vite build → frontend/dist

# Stage 2 — build backend Rust (INCHANGÉ)
FROM rust:1-bookworm AS backend
WORKDIR /src
COPY . .
RUN cargo build -p latch --release

# Stage 3 — runtime distroless (quasi inchangé)
FROM gcr.io/distroless/cc-debian12 AS runtime
WORKDIR /app
COPY --from=backend  /src/target/release/latch-cli  /app/latch-cli
COPY --from=backend  /src/backend/config            /app/config
COPY --from=frontend /src/frontend/dist             /app/frontend/dist
ENV LOCO_ENV=production
ENV LATCH_SPA_DIST=/app/frontend/dist
EXPOSE 5150
ENTRYPOINT ["/app/latch-cli"]
CMD ["start"]
```

- `pnpm install` en **couche séparée** (lock copié avant la source) → cache Docker, l'équivalent
  Node de cargo-chef.
- **Runtime identique** (distroless, **aucun Node au runtime**).
- `.dockerignore` : ajouter `frontend/node_modules`, `frontend/dist`.
- `LATCH_SPA_DIST=/app/frontend/dist` **inchangé** → serving Loco intact.

## 9. Node via nvm

- **`frontend/.nvmrc`** = version Node (ex. `24`). Source unique de vérité.
- Dev local : `nvm use` lit `.nvmrc`.
- CI : `actions/setup-node` → `node-version-file: frontend/.nvmrc`.
- Docker : image `node:<version>-bookworm-slim` alignée sur le `.nvmrc`.

## 10. Contraintes inchangées (NON négociables)

- **Sécu §9** : aucune réponse ne contient de hash ; le PIN n'apparaît qu'au détail (jamais
  en liste, jamais via MCP). Le DTO liste **n'a structurellement pas** de champ `pin`
  (vérifié côté Rust + reflété dans le schéma OpenAPI).
- **Auth** : cookie session same-origin (`HttpOnly` ; `Secure`+`__Host-` en prod) ; garde
  **Origin** sur les mutations. Front sous `/admin`, API sous `/api` → cookies auto, pas de token.
- **Confidentialité** : aucun nom de client réel nulle part (placeholders fictifs).
- **`/c/<slug>`** (Phase 4) : `no-store`, server-rendered — hors SPA admin.

## 11. Alignement des docs mémoire

Stratégie pour QUIRKS / CONVENTIONS : **archiver, pas supprimer** — les patterns/quirks Yew
vont sous une section « Historique Yew — obsolète depuis migration React » (la branche Yew
reste une référence). Les éléments **backend restent intacts**.

| Fichier | Changements |
|---|---|
| `CLAUDE.md` | En-tête (retrait `latch-ui`/Yew → app React `frontend/`) ; tableau Context7 (Yew/shadcn-rs → TanStack Router/Query, shadcn/ui, react-hook-form, react-i18next, **utoipa**) |
| `docs/contrat-deploy.md` | §2 structure (`frontend/` React, `backend/src/dto/`, retrait `latch-dto`) ; §4 rendu (React/Vite statique, **contrat = OpenAPI généré**, openapi-fetch, retrait Yew/basename) ; §9 invariants inchangés |
| `docs/BOOTSTRAP.md` | §1 stack · §2 versions · §3 commandes (pnpm/playwright) · §5 tests (Vitest/MSW/Playwright) · §6 CI pistes · §7 Docker Node |
| `docs/ROADMAP.md` | Phase 3 : migration React comme chantier courant + critères de sortie |
| `docs/ENVIRONMENT.md` | Toolchain (retrait Trunk/wasm → Node+pnpm via nvm) ; commandes front ; `LATCH_SPA_DIST` inchangé |
| `docs/QUIRKS.md` | Archiver les quirks Yew/shadcn-rs ; quirks backend conservés |
| `docs/CONVENTIONS.md` | Archiver les patterns Yew ; patterns backend conservés (nouveaux patterns React ajoutés *pendant* l'impl) |
| `docs/INDEX.md` | « Frontend (SPA Yew) » → superseded ; nouvelle section « Frontend (SPA React) » au fil des livrables |
| `docs/BACKLOG.md` | Annoter les items résolus (toast globale → sonner ; polish login.rs / activate_version / dropzone flicker = Yew, clos) ; `PUBLIC_BASE_URL` / slug éditable conservés |
| `README.md` | Stack et commandes (Yew/Trunk → React/Vite/pnpm) |

> Cet alignement se fait **au fil du plan d'implémentation**, pas en bloc — sauf la présente
> spec. `HANDOFF.md` reçoit son entrée datée **en fin de session**.

## 12. Dette à résorber (laissée cassée sur `feat/admin-react`)

Références au front Yew à remplacer **pendant** la migration (CI/Docker rouges attendus
jusqu'au setup React) :
- `Dockerfile` : stage Trunk → stage Node/pnpm.
- `.github/workflows/ci.yml` : job `frontend` wasm → pistes back/front + e2e (reusable workflows).
- `backend/src/web/mod.rs` : défaut `../frontend/dist` (reconfirmé — inchangé, le dossier reste `frontend/`).
- `.env.example` (commentaire `LATCH_SPA_DIST`), `.gitignore` (`/frontend/dist`, `frontend/node_modules`).

## 13. Critères de sortie de la migration

- `frontend/` : `pnpm build` produit `dist/` ; `pnpm lint` + `tsc --noEmit` + `pnpm test`
  (Vitest + MSW) verts ; supply-chain front (audit + licences) vert.
- Backend : `nextest` vert (dont drift OpenAPI) ; `openapi.json` + `schema.d.ts` à jour et commités.
- Parcours admin complet vérifié au navigateur ; **1-2 smoke e2e Playwright** verts.
- CI : pistes back/front + e2e vertes ; image Docker (stage Node) build OK.
- Docs mémoire alignées (table §11) ; `HANDOFF.md` + `INDEX.md` à jour.
- Invariants §10 préservés ; aucun nom de client réel.

## 14. Hors périmètre

- **Fumadocs** (landing + doc GH Pages) — chantier séparé, après l'admin React.
- **Phase 4** (`/c/<slug>` serving + unlock) et **Phase 5** (MCP) — backend Rust, indépendants.
- Suite e2e Playwright complète — Phase 6.
- State manager client — non requis (réévaluable si besoin futur, additif).
