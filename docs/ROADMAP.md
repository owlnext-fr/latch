# ROADMAP — latch

> Phases, dépendances, critères de sortie. Identifier la phase courante avant de
> coder (cf. `docs/HANDOFF.md` et `docs/INDEX.md`). Une phase n'est close que si ses
> critères de sortie sont **verts** — alors on le consigne dans INDEX + HANDOFF.

L'ordre suit les dépendances : le cœur d'abord (testable sans HTTP), puis les
adaptateurs un par un, puis l'e2e qui valide le tout assemblé, puis le packaging.

---

## Phase 0 — Scaffold & squelette CI/Docker

Mettre en place le terrain sans logique métier.
- Workspace : `backend/` (Loco, template **avec DB**) + `frontend/` (app React, Vite + pnpm).
- **Retirer l'auth users/JWT** générée par Loco (on n'utilise pas la table `users`).
- Désactiver Redis/worker.
- `Cargo.toml` : versions épinglées (Loco, rmcp ≥ 1.4.0), `libsqlite3-sys` `bundled`.
- Squelette CI (fmt/clippy/test vides mais qui tournent), Dockerfile multi-stage (Node + Rust + runtime),
  `docker-compose.yml`, `deploy.sh`, dual-license, README minimal.

**Sortie :** `cargo loco start` démarre, `pnpm build` produit un bundle React, l'image se
construit, la CI passe au vert sur un projet vide.

## Phase 1 — Cœur (services) + modèle + migrations

Le métier, agnostique HTTP.
- Migrations SeaORM : `projects`, `versions`. _(La table `sessions` est reportée
  en Phase 2 : elle ne sert qu'à l'auth admin via `axum-session` ; la créer à vide
  ici donnerait une table morte et un risque de conflit de schéma. Décision actée
  2026-06-24.)_
- `services/` : `projects` (create/list/get_by_slug/set_code/clear_code/verify_code),
  `deploy` (tx insert version + flip pointeur, ordre fichier→DB du contrat §8),
  `slug` (base + suffixe), trait `Storage` + `FsStorage`, `CoreError`.
- `verify_code` à temps constant ; PIN auto-généré 6 chiffres.

**Sortie :** tests **unit** verts sur slug, code, bascule, `deploy_token`. Un test de
`deploy()` avec un `Storage` sur tempdir (jamais le disque de prod). Aucun `use axum`
ni `use loco_rs` dans `src/services/`.

## Phase 2 — Adaptateur web admin (API JSON + session)

- Migration `sessions` (store de session admin), créée ici — soit auto-gérée par
  `axum-session`, soit migration SeaORM dédiée, à trancher au câblage. _(Reportée
  de la Phase 1, cf. décision 2026-06-24.)_
- `controllers/auth.rs` : login/logout, cookie de session (`axum-session` dans
  `after_routes`), compte unique env, comparaison à temps constant, rate-limit login.
- `controllers/admin.rs` : API JSON — projets CRUD, deploy manuel, switch de version,
  config code, suppression. Vérif `Origin` sur les mutations.

**Sortie :** tests **intégration** (Loco + SQLite de test) verts sur chaque endpoint,
401 sans session, deploy transactionnel, switch, **test-invariant de sécu** (pas de
hash en réponse, pas de PIN en liste).

## Phase 3 — SPA admin ✅ LIVRÉE (2026-06-25)

> Livrée en deux temps : (a) Yew + polish UX/i18n complets ; (b) **migration React/Vite/shadcn-ui**
> décidée (friction `shadcn-rs` 0.1 + wasm), exécutée en Plans 1-3 sur `feat/admin-react`.
> Crate Yew (`latch-ui`) retirée du workspace (reste dans l'historique git).
> Le **comportement (contrat §7) n'a pas changé** — seul le rendu.
> Détail du choix : `docs/superpowers/specs/2026-06-25-admin-react-migration-decision.md`.

Livrables React (Plans 1-3, tous verts) :
- **Plan 1** — Backend : DTO inlinés `backend/src/dto/`, annotations utoipa, `openapi.json` commité
  + test drift, Swagger UI dev.
- **Plan 2** — Frontend React : scaffold Vite/pnpm/shadcn, client typé openapi-fetch, shell TanStack
  Router/Query, harness Vitest+MSW, Login, liste projets, ProjectForm, détail/deploy/danger panels,
  hooks `use-projects`.
- **Plan 3** — Infra : stage Docker Node 24, CI pistes back/front→e2e→docker, Playwright e2e.

**Critères de sortie :** parcours admin manuel complet ; Vitest verts ; Playwright e2e verts.

## Phase 4 — Serving `/c/<slug>` ✅ LIVRÉE (2026-06-25)

> Spec : `docs/superpowers/specs/2026-06-25-phase-4-serving-design.md`
> Plan : `docs/superpowers/plans/2026-06-25-phase-4-serving.md`

- `controllers/serve.rs` : GET deux états (libre / cookie valide / page de
  déverrouillage), `POST /unlock` (vérif + cookie signé HMAC), `no-store` partout,
  page de déverrouillage stylée portant `brand_name`. GET `/api/public/{slug}` (PublicMeta).
- `services/unlock_cookie.rs` : cœur pur (`issue_token`/`verify_token`, empreinte HMAC du PIN).
- `controllers/serve_ratelimit.rs` : **rate-limit *load-bearing*** sur `/unlock`
  (backoff IP+slug via governor, 2 layers `ServiceBuilder`).
- Frontend : `unlock.html` (2ᵉ entrée Vite) + `src/unlock.tsx`.

**Critères de sortie atteints :** tests verts (cœur unit, intégration serve/unlock,
rate-limit) ; frontend Vitest+build verts (`dist/unlock.html`) ; cargo-deny vert ;
validé navigateur (Task 9).

## Phase 5 — Endpoint MCP + panneau Settings ✅ LIVRÉE (2026-06-25)

> Spec/plan : `docs/superpowers/` (tasks 1-8). SonarCloud gate PASSED (~94.8% new_coverage).

**Backend :**
- `mcp/mod.rs` : `LatchMcp { db, storage, deploy_token, public_base_url, tool_router }`,
  macros `#[tool_router]`/`#[tool_handler]`/`ServerHandler`, montés via `after_routes`
  (`nest_service("/mcp", StreamableHttpService)`, `LocalSessionManager`).
- `rmcp` épinglé `"1.4"` (floor CVE-2026-42559), résout en **1.8.0**.
  `allowed_hosts` dérivé de `LATCH_PUBLIC_BASE_URL` via `web::host_authority()`.
- `deploy_prototype(slug, html, deploy_token, activate?)` : token gate FIRST, slug préexistant
  (pas d'auto-création), `activate` défaut `true`, retourne `DeployResult { url, version, code_protected }`.
- `list_projects(deploy_token)` : token gate FIRST, retourne **enveloppe objet**
  `{ projects: [...] }` (`ProjectListResult`, cf. §5.1 contrat).
- Helpers `web/mod.rs` : `deploy_token(ctx)`, `public_base_url(ctx)` (trailing-slash normalisé),
  `host_authority(base)` — fail-secure.
- `GET /api/settings` (AdminAuth) : `SettingsResponse { deploy_token, mcp_url, public_base_url }`.
- Nouvelle variable : `LATCH_PUBLIC_BASE_URL` (runtime, fail-secure, dérive `allowed_hosts`).

**Frontend :**
- `hooks/use-settings.ts`, `routes/settings.tsx` (Topbar + mcp_url copyable +
  deploy_token via `PinField` masqué/révéler/copier + public_base_url texte + loading/error).
- Route `/settings`, icône Settings dans la topbar, i18n `settings.*` (EN+FR).
- Phase 7 (locale/thème) reste hors périmètre.

**Tests :** 127 backend (dont gate token, deploy_prototype, slug inconnu, invariants sécu,
settings 401), 54 frontend. Clippy `--all-features` clean. Cargo-deny OK.

**À confirmer :** branchement réel Claude web (déduit de la doc rmcp, non testé en prod).

## Phase 6 — E2E, durcissement, packaging publiable ✅ LIVRÉE (2026-06-25)

> Spec/plan : `docs/superpowers/` (tasks 1-8). Toutes les gates vertes (136 nextest, 54 vitest, 4 playwright).

**Livré :**
- **E2E MCP transport HTTP** (`backend/tests/mcp_http.rs`) : 6 tests Streamable HTTP réel
  (initialize handshake, tools/list, deploy_prototype + invariant §9, list_projects enveloppe objet,
  gate bad-token ×2 no-side-effect). Harness loco + `axum-test`.
- **E2E Playwright `/c/<slug>`** (`e2e/serve-unlock.spec.ts`) : 3 tests navigateur réel (projet libre
  no-store, unlock par PIN + auto-submit OTP, bascule v1→v2). Setup API-driven.
- **Durcissement en-têtes** : `robots.txt` (text/plain, Disallow: /) + layer `X-Robots-Tag` global
  posé en dernier dans `after_routes`. 3 tests `hardening.rs`.
- **Captures Playwright** (`e2e/screenshots.capture.ts`) : 2 tests conditionnels (skip sauf `CAPTURE=1`),
  génèrent `docs/assets/admin-list.png` + `docs/assets/unlock.png`. `testMatch` étendu aux `.capture.ts`.
- **CHANGELOG** (`cliff.toml` git-cliff) : 2 passes preprocessor gitmoji, `CHANGELOG.md` v0.1.0
  avec 207/208 commits (phases 0-6), zéro gitmoji résiduel.
- **README refondu** : 11 sections FR, badges CI/Quality Gate/Coverage/License, captures, quickstart
  Docker+dev, Connecter Claude MCP, archi+invariants, sécurité robots/X-Robots-Tag.
- `sonar.tests=frontend/src,backend/tests` (T4), `cargo deny` vert.

**Critères de sortie atteints :** e2e vert en CI, image GHCR publiée, `deploy.sh` propre,
repo présentable FOSS. `deploy.sh` testé sur box = responsabilité humaine (hors CI).

## Phase 7 — Peaufinage graphique / web

Polish visuel et confort, une fois le cœur fonctionnel en place. Indépendant des
phases métier ; peut s'intercaler selon les priorités produit.

- **Titres de page** : gestion dynamique du `<title>` par route admin (TanStack Router)
  et sur la page de déverrouillage (ex. « {brand_name} — déverrouillage » / « latch — admin »).
  Aujourd'hui les titres sont statiques (`index.html` = « latch — admin », `unlock.html` = « latch »).
- **Logo** : générer un logo `latch` et l'appliquer — favicon (les deux entrées Vite ; le
  `/vite.svg` placeholder a été retiré en Phase 4), en-tête admin (topbar), et page de
  déverrouillage (au-dessus du `brand_name`).
- **Menu Settings** : un menu de réglages regroupant **le choix de la locale** (FR/EN, déjà
  géré par `react-i18next` + `LocaleSwitcher`) et **le choix du thème** (`system` / `dark` /
  `light`). `next-themes` est déjà en dépendance mais aucun `ThemeProvider` n'est monté
  (retiré au Plan 2) — à recâbler + persister. NB : la page unlock est en fond clair only
  aujourd'hui (cf. BACKLOG « bordure OTP sans variante dark »).
- **Settings en side-panel** : aujourd'hui le panneau Settings (`deploy_token` / `mcp_url` /
  `public_base_url`, livré Phase 5) est une **route plein écran** `/admin/settings`. Le
  transformer en **side-panel** (`<Sheet>`, cohérent avec la grammaire d'interaction admin
  du contrat §7 — créer/éditer en side-panel) ouvert depuis l'icône Settings de la topbar,
  plutôt qu'une navigation de route. À combiner avec le « Menu Settings » ci-dessus (locale +
  thème + infos MCP dans le même panneau).
- **Page d'erreur stylée pour le serving `/c/<slug>`** : aujourd'hui les branches d'erreur de
  `controllers/serve.rs` (slug inconnu, projet sans version active) renvoient l'erreur Loco par
  défaut (**JSON brut**, ex. `404 NotFound`) sur une surface **publique** vue par le client final.
  Servir à la place une **page HTML stylée** (cohérente avec la page de déverrouillage, portant
  éventuellement `brand_name`) pour chaque cas : projet introuvable, aucune version déployée,
  voire erreur interne. Idéalement une 2ᵉ/3ᵉ vue réutilisant le bundle/thème de `unlock.html`
  (ou un mini-template HTML). `no-store` comme le reste de la surface `/c`.
- **i18n centralisé** : centraliser les catalogues de traduction pour qu'ajouter une locale
  soit trivial — idéalement **détection automatique des fichiers JSON** de locale (`locales/*.json`)
  plutôt que les imports statiques en dur actuels (`import en from './locales/en.json'`).
  S'applique à l'i18n admin **et** au mini-catalogue de la page unlock (`src/unlock/i18n.ts`),
  à harmoniser.

**Sortie :** titres cohérents par page ; logo présent (favicon + admin + unlock) ; menu
settings fonctionnel **en side-panel** (locale + thème persistés, défaut thème = `system`, +
infos MCP) ; serving `/c` rend des **pages d'erreur HTML stylées** (plus de JSON brut sur slug
inconnu / sans version) ; ajouter une locale = déposer un JSON (ou une config minimale), sans
toucher au code d'import.

## Phase 8 — Documentation publique (Fumadocs / GitHub Pages)

> Stub — contenu TBD. Référencé depuis le README (lien TBD).

Landing marketing + documentation détaillée hébergées sur **GitHub Pages** (ou équivalent),
construites avec **Fumadocs** (framework MDX adapté aux projets React).

Périmètre pressenti :
- **Landing** : présentation du projet, captures, quickstart en ligne, lien GHCR.
- **Documentation technique** : guide d'installation, référence des variables d'env,
  guide de connexion MCP, sécurité (robots/X-Robots-Tag, rate-limit, fail-secure secrets).
- **Déploiement** : workflow GitHub Actions `pages` déclenché sur push `main` ou tag.
- **Lien** : le README renvoie vers la doc publiée (URL à renseigner quand disponible).

**Sortie :** site docs accessible publiquement, pipeline CI docs vert, lien dans le README.
