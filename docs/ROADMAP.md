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

## Phase 7 ✅ LIVRÉE (2026-06-26) — Peaufinage graphique / web

Polish visuel et confort, une fois le cœur fonctionnel en place. Indépendant des
phases métier ; peut s'intercaler selon les priorités produit. Livrée en 4 lots :

- **Lot 1 — Fondations i18n/thème** : i18n centralisé (auto-découverte locales admin+unlock JSON, strip `_meta`),
  `ThemeProvider` monté (`next-themes`, défaut `system`, anti-FOUC script `index.html` seulement), tests
  parseLocales/theme/i18n complets, mémoire (CONVENTIONS, QUIRKS, INDEX, HANDOFF).

- **Lot 2 — Panneau Settings unifié** : Settings side-panel (`<Sheet>` depuis topbar, plus route `/settings`
  suppressible), Select radix (language + theme toggles, helper text), language-select auto-découverte
  (drapeau `flag-icons` CSS), SonarCloud gate 80% couverture new-code.

- **Lot 3 — Identité visuelle** : Logo favicon SVG (topbar, login, unlock, favicon réel), titres dynamiques
  par route (TanStack Router `useDocumentTitle` hook), largeur admin `max-w-6xl` centré, lien GitHub
  (bouton topbar + logo inline), mémoire complète (CONVENTIONS, QUIRKS, INDEX, HANDOFF).

- **Lot 4 — Page d'erreur serving /c** : 3ᵉ entrée Vite `error.html` + `src/error/{main,error-page,i18n}.tsx`
  + `locales/error/*.json`, `serve.rs::serve_error_page(status)` avec fallback texte inline, 5 branches
  erreur (404 slug/version, 500 DB/storage/version manquante) → HTML + `no-store` + logs
  `tracing::error!`, page générique zéro fuite, bundle isolation vérifiée, gate complète verte (cargo fmt/clippy/nextest +
  pnpm lint/typecheck/vitest/build, couverture ≥ 80%), mémoire (CONVENTIONS, QUIRKS, BACKLOG, INDEX, ROADMAP, HANDOFF).

**Sortie Phase 7** : titres cohérents par page ; logo présent (favicon + topbar admin + login + unlock) ;
menu settings fonctionnel **en side-panel** (locale + thème persistés, défaut thème = `system`, + infos MCP)
avec **chaque réglage explicité** par helper text ; **vrai sélecteur de langue** (drapeau, peuplé depuis
locales découvertes) ; serving `/c` rend des **pages d'erreur HTML stylées** (plus de JSON brut sur slug inconnu /
sans version) ; ajouter une locale = déposer un JSON, sans toucher au code d'import ; **pages admin bornées**
en largeur (conteneur centré) ; **lien GitHub** présent sur la page de login.

## Phase 8 ✅ LIVRÉE (2026-06-26, tag **v0.3.0**) — Documentation publique

> Spec : `docs/superpowers/specs/2026-06-26-phase-8-public-docs-design.md`
> Plan : `docs/superpowers/plans/2026-06-26-phase-8-public-docs.md`
> **Déploiement Pages au merge sur `main`** (job CI `deploy-docs`). Reste : 1ᵉʳ déploiement live + vérif basePath.

Landing marketing + documentation détaillée, **Fumadocs** (Next + MDX) en export statique sur
**GitHub Pages** (sous-chemin `owlnext-fr.github.io/latch`, `basePath '/latch'`, pas de domaine custom).

Livré :
- **Landing** : hero, parcours 3 étapes (avec conversation Claude simulée CSS), grille de features, CTA `docker pull`, footer ; identité produit (logo, stone/oklch, clair/sombre).
- **Documentation EN** (sourcée du contrat/BOOTSTRAP) : `how-it-works/`, `deploy/` (dont reverse-proxy Caddy/Nginx/Traefik/Apache + configuration 17 clés), `admin/`, `publish-from-claude/` (2 tools), `quickstart`, `troubleshooting`. Recherche statique Orama.
- **Déploiement** : jobs `docs` + `deploy-docs` **dans `ci.yml`** (pas de workflow séparé), Pages = GitHub Actions.
- **Lien** : README + `frontend/src/lib/links.ts` `DOCS_URL` → `https://owlnext-fr.github.io/latch`.

**Sortie** : build statique vert (66 pages), basePath OK (assets `/latch/_next`), 0 lien interne
cassé, contenu EN sous `public_docs/content/` uniquement. **Reste (post-merge)** : confirmer le
pipeline `deploy-docs` vert + le site accessible à l'URL Pages (charger une page profonde, vérifier `_next/`).

## Phase 9 — Passe de polish (doc + admin) [À FAIRE]

> Identifiée en clôture de Phase 8 (2026-06-26). Petite passe transverse, non bloquante.

- **Sélecteur de langue du login en ancien modèle** : la page `/admin/login`
  (`frontend/src/routes/login.tsx`) utilise encore l'ancien `LocaleSwitcher` (boutons FR/EN) au lieu
  du nouveau `LanguageSelect` (Select + drapeaux, locales auto-découvertes) introduit en Phase 7 Lot 2.
  → aligner le login sur le modèle unifié.
- **Corrections de pages de doc** : relire/corriger certaines pages du site `public_docs/` (contenu à
  préciser à la relecture).
- **Zoom des images (docs + landing)** : permettre d'agrandir les captures — côté docs via le composant
  Fumadocs `ImageZoom` (déjà dispo) sur les images MDX ; côté landing, un lightbox/zoom sur les captures
  du parcours.
- **Retravailler `.env.example`** : passe de cohérence/pédagogie avant distribution (placeholders uniformes,
  commande de génération par secret, séparation requis-prod / défauts optionnels, dev vs prod). Détail et
  points relevés : `docs/BACKLOG.md`.

**Sortie** : login aligné sur le sélecteur de langue unifié ; pages de doc corrigées ; images
agrandissables sur docs et landing ; `.env.example` propre et homogène.
