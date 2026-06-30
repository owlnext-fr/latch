# Index — ce qui est livré et marche

> Le **réalisé**, par opposition au ROADMAP (le prévu). Quand les critères de sortie
> d'une phase sont verts, on coche ici avec une ligne par livrable (+ entrée HANDOFF).
> Format : `- [x] <livrable> — <phase> — <date>`.

## Backend (cœur + adaptateurs)
- [x] Scaffold app Loco (`backend/`, crate `latch`, bin `latch-cli`) — SQLite `bundled`,
  sans users/JWT, sans worker (`--bg none`) — Phase 0 — 2026-06-24
- [x] Workspace 2 membres + `migration` (sea-orm 1.1 aligné Loco), `auto_migrate` au boot — Phase 0 — 2026-06-24
- [x] `CoreError` + squelette `services/` (no axum/loco) — Phase 1 — 2026-06-24
- [x] Service `slug` (génération `<nom>-<8xbase62>`) — Phase 1 — 2026-06-24
- [x] Service `security` (`secure_compare` timing-safe) — Phase 1 — 2026-06-24
- [x] Service `pin` (génération 6 chiffres) — Phase 1 — 2026-06-24
- [x] Trait `Storage` + implémentation `FsStorage` — Phase 1 — 2026-06-24
- [x] Migrations `projects`/`versions` + entités SeaORM générées + `test_support` (in-memory SQLite) — Phase 1 — 2026-06-24
- [x] `ProjectsService` (create/list/get_by_slug/set_code/clear_code/verify_code) — Phase 1 — 2026-06-24
- [x] `DeployService` (n=max+1, storage-first, transaction flip pointeur) — Phase 1 — 2026-06-24
- [x] Garde d'architecture `backend/tests/architecture.rs` (contrat §1 : cœur sans axum/loco) — Phase 1 — 2026-06-24
- [x] Migration table `sessions` (schéma axum-session : id TEXT PK, expires INTEGER NULL, session TEXT) — Phase 2 — 2026-06-24
- [x] `backend/src/web/mod.rs` : helpers `SessionPool`/`AdminSession`/`storage_from_ctx`/`build_session_store` — Phase 2 — 2026-06-24
- [x] `after_routes` dans `app.rs` : monte `SessionLayer` (axum-session 0.16 + SQLite pool Loco) — Phase 2 — 2026-06-24
- [x] `controllers/error.rs` : mapping `CoreError`→`loco_rs::Error` (NotFound→404, Validation→400, Db/Io→500) — Phase 2 — 2026-06-24
- [x] `controllers/dto.rs` : DTOs admin (`ProjectListItem` sans PIN, `ProjectDetail` avec PIN, `VersionItem`, `CreateProjectReq`/`UpdateProjectReq`/`SetCodeReq`/`DeployReq`) — Phase 2 — 2026-06-24
- [x] `controllers/auth.rs` : login/logout session + extracteur `AdminAuth` (FromRequestParts, 401 sans session) + rate-limit `tower_governor` sur `/admin/login` — Phase 2 — 2026-06-24
- [x] `controllers/middleware/origin.rs` : middleware `require_same_origin` (axum from_fn), 403 sur cross-origin, helpers `url_host`/`same_host` testés unitairement — Phase 2 — 2026-06-24
- [x] `controllers/admin.rs` : API JSON lecture `GET /admin/projects` + `GET /admin/projects/{id}`, protégés par `AdminAuth` — Phase 2 — 2026-06-24
- [x] `controllers/admin.rs` : API JSON écriture `POST /admin/projects`, `PUT /admin/projects/{id}`, `DELETE /admin/projects/{id}`, `POST /admin/projects/{id}/code`, `DELETE /admin/projects/{id}/code` — tous protégés par `AdminAuth` + garde `require_same_origin` — Phase 2 — 2026-06-24
- [x] `controllers/admin.rs` : `POST /admin/projects/{id}/deploy` (DeployService), `POST /admin/projects/{id}/versions/{n}/activate`, `DELETE /admin/projects/{id}/versions/{n}` (garde actif→400), `GET /admin/projects/{id}/versions/{n}/preview` (HTML brut + `Cache-Control: no-store`, AdminAuth) — Phase 2 — 2026-06-24
- [x] `backend/tests/security_invariants.rs` : invariants §9.1 (pas de hash) et §9.2 (PIN absent de la liste, présent au détail) — Phase 2 — 2026-06-24
- [x] Crate `latch-dto` (workspace, serde, cible native + wasm32) — types partagés `ProjectListItem`, `ProjectDetail`, `VersionItem`, `CreateProjectReq`, `UpdateProjectReq`, `SetCodeReq`, `DeployReq` — Phase 3 — 2026-06-24 **(retirée feat/admin-react, inlinée dans `backend/src/dto/` — 2026-06-25)**
- [x] DTO inlinés dans `backend/src/dto/` (ex-`latch-dto`) + dérivation `utoipa::ToSchema` — Migration React Plan 1 — 2026-06-25
- [x] Réponses typées `OkResponse`/`DeployResponse`/`ActivateResponse` (fin des `serde_json::json!` ad-hoc) — Migration React Plan 1 — 2026-06-25
- [x] `#[utoipa::path]` sur toutes les routes `/api/*` + `openapi::ApiDoc` (paths + schemas) — Migration React Plan 1 — 2026-06-25
- [x] `openapi.json` commité (racine) + test de drift `backend/tests/openapi_drift.rs` — Migration React Plan 1 — 2026-06-25
- [x] Swagger UI sous `/api-docs` en dev uniquement — Migration React Plan 1 — 2026-06-25
- [x] API JSON re-préfixée sous `/api/*` (depuis `/admin/*`) + conversions libres `dto::to_list_item`/`to_detail` — Phase 3 — 2026-06-24
- [x] `web::spa_dist_dir()` + `nest_service("/admin", ServeDir + ServeFile fallback)` dans `after_routes` — serving SPA sous `/admin` avec fallback `index.html` — Phase 3 — 2026-06-24
- [x] Redirection racine `GET /` → `/admin` (307 temporaire, `root_redirect` dans `after_routes`) — remplace la page welcome Loco (dev) / 404 (prod) — micro-feature — 2026-06-30
- [x] `LATCH_BODY_LIMIT` — taille max body configurable (Loco `limit_payload`, défaut 5 Mo, `disable` possible) + test de régression deploy > 2 Mo — Migration React Plan 3 (post-validation) — 2026-06-25
- [x] `ProjectListItem` enrichi : `active_version_n` + `version_count` (retrait `active_version_id`), service `list_with_versions` (pas de N+1), `openapi.json`/`schema.d.ts` régénérés — Plan 2 (post-validation) — 2026-06-25
- [x] Endpoints admin commentaires : `GET /api/projects/{id}/versions/{n}/comments` + `DELETE /api/projects/{id}/comments/messages/{cid}` (modération) ; `comment_count` réel dans `detail`/`update` ; `#[utoipa::path]` sur 6 handlers serve ; 9 schémas + 8 paths OpenAPI ; `openapi.json`/`schema.d.ts` régénérés — Task 10 — 2026-06-30

## Frontend (SPA Yew) — **SUPERSEDED** (migré vers React, Plans 1-3, 2026-06-25)

> La crate Yew (`latch-ui`) est retirée du workspace. Les livrables ci-dessous restent dans
> l'historique git (branche pré-migration). Le comportement UI (contrat §7) est repris par la SPA React.

- [x] Crate `latch-ui` (Yew 0.21 CSR) buildée par Trunk → bundle wasm `dist/` — Phase 0 — 2026-06-24
- [x] Router Yew (yew-router 0.18, BrowserRouter basename="/admin", routes absolues `#[at("/admin/...")]`) + scaffold SPA (AuthProvider, Protected, pages Login/List/Detail) — Phase 3 — 2026-06-24
- [x] Utilitaires SPA : `pin::generate()`, `url::public_url(slug)` (window.location.origin), `clipboard::copy_to_clipboard(text)` — Phase 3 — 2026-06-24
- [x] Client API gloo-net 0.6 (`api/client.rs`) : `list_projects`, `get_project`, `create_project`, `update_project`, `delete_project`, `set_code`, `clear_code`, `deploy_version`, `activate_version`, `delete_version` + `check_status` / `ApiError` — Phase 3 — 2026-06-24
- [x] Auth dérivée : `AuthProvider` (contexte global), extracteur `Protected` (redirige → /admin/login si non connecté), `AuthContext::logout()` → `POST /api/auth/logout` — Phase 3 — 2026-06-24
- [x] Page `Login` (`/admin/login`) : formulaire, erreur sur mauvais credentials, indicateur busy — Phase 3 — 2026-06-24
- [x] Composants `CopyButton` (bouton-icône copie + « Copié ! » éphémère) et `PinField` (masque `••••••`, œil de révélation, bouton copier) — Phase 3 — 2026-06-24
- [x] Page `List` (`/admin`) : tableau shadcn-rs (nom, URL publique + CopyButton, badge code, version active, nb versions), état vide, bouton « Nouveau projet », navigation vers détail — Phase 3 — 2026-06-24
- [x] Side-panel `ProjectForm` (créer/éditer) : champs nom/slug(RO)/code/PIN/brand_name, validation, reset à l'ouverture — Phase 3 — 2026-06-24
- [x] Side-panel `DeployPanel` : upload fichier HTML, case « activer immédiatement », appel `deploy_version`, reset à l'ouverture — Phase 3 — 2026-06-24
- [x] Side-panels danger : suppression projet (`DeleteProjectPanel`) et suppression version (`DeleteVersionPanel`) — confirmation texte + bouton destructif — Phase 3 — 2026-06-24
- [x] Page `Detail` (`/admin/projects/:id`) : accès public (URL + PIN via PinField), actions haut-droite (Modifier/Déployer/Supprimer), liste versions (activer/prévisualiser/supprimer), état vide premier déploiement — Phase 3 — 2026-06-24
- [x] CSS shadcn-rs vendorisée (5 fichiers `frontend/styles/`, patch `--color-card*`/`--color-popover*` manquants, dark-mode via `.dark`) — Phase 3 — 2026-06-24
- [x] Tests wasm-bindgen-test (3 tests T5 : pin, url, clipboard) verts — Phase 3 — 2026-06-24
- [x] `ToastProvider` + `use_toast()` + `ToastHandle` maison (gloo-timers, auto-dismiss 4 s) — SDD Task 3 — 2026-06-24
- [x] `CopyButton` rewired : toast + i18n `t!("toast.copied")` + `t!("common.copied")` — SDD Task 3 — 2026-06-24
- [x] **i18n FR+EN** (`rust-i18n 3`) : `LocaleProvider` réactif + `use_locale()`, `frontend/locales/{en,fr}.yml`, macro `t!` crate-wide — Polish UX — 2026-06-25
- [x] `LocaleSwitcher` (boutons FR/EN) : persistance localStorage `latch.locale` + détection `navigator.language` au boot, défaut EN — Polish UX — 2026-06-25
- [x] `components/toggle.rs` : `Toggle` vendorisé (patch du `Switch` shadcn-rs, état contrôlé pur, classe `size-md`) — remplace `<Switch>` dans ProjectForm + DeployPanel — Polish UX — 2026-06-25
- [x] Badges d'accès colorés (vert PIN requis / orange libre) : vars `--color-success`/`--color-warning` (`:root`+`.dark`), sélecteurs `.badge.badge--success/--warning` — Polish UX — 2026-06-25
- [x] `DeployPanel` dropzone drag-and-drop (dragover/drop + input caché via NodeRef, `human_size`) — Polish UX — 2026-06-25
- [x] `ProjectForm` : PIN toujours affiché + `disabled` quand code off (plus de saut de layout), slug `disabled` en édition, helper text par champ — Polish UX — 2026-06-25
- [x] Toasts câblés sur tous les retours d'action (création/édition/déploiement/activation/suppression/copie) + erreurs — Polish UX — 2026-06-25
- [x] i18n complet + intros de page + accessibilité (`<a onclick>` → `<button class="linkish">`, breadcrumb `<button>`) sur Login/List/Detail/panels — Polish UX — 2026-06-25

## Frontend React (SPA — feat/admin-react, Plans 1-3)

### Plan 2 — Scaffold, composants, shell, liste
- [x] Harness Vitest (jsdom, globals) + MSW (`src/test/msw.ts` : `server` + `jsonOnce`) + `vitest.setup.ts` — Plan 2 T4 — 2026-06-25
- [x] Helper `src/test/utils.tsx` : `renderWithProviders` (I18nextProvider + QueryClientProvider retry:false) — Plan 2 T4 — 2026-06-25
- [x] `CopyButton` React : `navigator.clipboard.writeText` + `toast.success(t('toast.copied'))`, bouton-icône `Copy` lucide + aria-label — Plan 2 T4 — 2026-06-25
- [x] `PinField` React : lecture (masque `••••••`, œil révéler/masquer, CopyButton si pin), édition (Input 6 chiffres, disabled, onChange filtré) — Plan 2 T4 — 2026-06-25
- [x] `LocaleSwitcher` React : boutons FR/EN, `i18n.changeLanguage`, `aria-pressed` sur langue active — Plan 2 T4 — 2026-06-25
- [x] Tests Vitest : 10 tests verts (utils×2, PinField×5, CopyButton×2) ; typecheck + lint propres — Plan 2 T4 — 2026-06-25
- [x] `hooks/use-projects.ts` : `useProjects`, `useProject`, 8 mutations (create/update/delete/setCode/clearCode/deploy/activateVersion/deleteVersion) avec invalidation Query + toasts sonner — Plan 2 T6 — 2026-06-25
- [x] `components/topbar.tsx` : titre latch → /, LocaleSwitcher, logout — Plan 2 T6 — 2026-06-25
- [x] `routes/list.tsx` : Table shadcn (nom bouton, URL+CopyButton, badge accès coloré, version active), état vide, ProjectForm stub — Plan 2 T6 — 2026-06-25
- [x] `routes/list.test.tsx` : 6 tests MSW (hooks réels, PIN jamais rendu §9.2) — Plan 2 T6 — 2026-06-25
- [x] `components/ui/{table,badge}.tsx` ajoutés (shadcn@latest) — Plan 2 T6 — 2026-06-25
- [x] `routes/login.tsx` + `routes/detail.tsx` : login formulaire, page détail (accès, versions, actions) — Plan 2 — 2026-06-25
- [x] `components/project-form.tsx` : side-panel Sheet créer/éditer (RHF+zod, PinField, Toggle accès) — Plan 2 — 2026-06-25
- [x] `components/deploy-panel.tsx` : side-panel Sheet deploy (upload HTML, case activer, dropzone) — Plan 2 — 2026-06-25
- [x] Side-panels danger : `DeleteProjectPanel` / `DeleteVersionPanel` (Sheet danger, confirmation) — Plan 2 — 2026-06-25
- [x] `frontend/src/api/schema.d.ts` généré par openapi-typescript depuis `openapi.json` — Plan 2 T1 — 2026-06-25
- [x] `frontend/src/api/client.ts` : client openapi-fetch typé, `credentials: 'include'`, wrapper MSW — Plan 2 T1 — 2026-06-25

### Plan 3 — Infra, CI, e2e
- [x] Stage Docker Node 24 (`node:24-bookworm-slim`) : `pnpm install` + `pnpm build` en multi-stage — Plan 3 — 2026-06-25
- [x] CI pistes parallèles back/front → e2e → docker (GitHub Actions) — Plan 3 — 2026-06-25
- [x] Playwright e2e : login, liste, création projet, deploy, détail, activation version, logout — Plan 3 — 2026-06-25

## Infra (CI / Docker / déploiement)
- [x] Dockerfile multi-stage (Node 24 pnpm build → build Rust → distroless), image ~85 Mo, boot vérifié — Phase 0 + Plan 3 — 2026-06-24/2026-06-25
- [x] Dockerfile cargo-chef (couche deps cachée) + runtime `gcr.io/distroless/cc-debian12:nonroot` (uid 65532) + durcissements S8549/S6471/S6596/S6505 — Toolchain Task 5 — 2026-06-25
- [x] Couverture Rust sur SonarQube (`cargo-llvm-cov` → `backend-lcov.info`) : `backend/src` scanné, artefact CI `backend-lcov` (upload `test-backend` → download `sonar`), `sonar.rust.clippy.enabled=false`, gate PASSED, 0 issue Rust — Toolchain Task 8c — 2026-06-25
- [x] `sonar-project.properties` + job CI `sonar` **bloquant** (gate PASSED, front + IaC + couverture Rust lcov), SonarCloud Automatic Analysis désactivé — Toolchain Task 8c — 2026-06-25
- [x] `[workspace.lints.clippy] unwrap_used/expect_used=warn` + `[lints] workspace=true` ×2 crates (backend + migration) — Toolchain Task 7 — 2026-06-25
- [x] Couverture Vitest lcov (`@vitest/coverage-v8`, script `test:cov`, `coverage/lcov.info`) — Toolchain Task 4 — 2026-06-25
- [x] CI confort : 15 uses SHA-pinned, `--ignore-scripts ×3`, `concurrency cancel-in-progress`, cache Playwright, `clippy --all-features` — Toolchain Task 6 — 2026-06-25
- [x] Remédiation 64 issues Sonar front (22 `void` S3735, 12 `Readonly`/`globalThis` S6606/S1128, ternaires/FormEvent/fieldset — Tasks T1–T3) — Toolchain Tasks 1-3 — 2026-06-25
- [x] CI GitHub Actions **verte sur main** (back: fmt/clippy/nextest/cargo-deny ; front: lint/typecheck/vitest ; e2e Playwright ; docker GHCR) — Phase 0 + Plan 3 — 2026-06-24/2026-06-25
- [x] Images GHCR versionnées (`metadata-action` : semver+latest+sha) + pin déploiement (`LATCH_IMAGE_TAG`) — Phase 0 — 2026-06-24
- [x] `docker-compose.yml` + `deploy.sh` + `.env.example` + dual-license MIT/Apache — Phase 0 — 2026-06-24

## Phase 4 — Serving `/c/<slug>`
- [x] `services/unlock_cookie.rs` : `issue_token` / `verify_token` — cœur pur (sans axum/loco), empreinte HMAC du PIN, TTL — Phase 4 Task 1 — 2026-06-25
- [x] `controllers/serve.rs` : handler `serve` (GET /c/{slug}) — arbre de décision (slug inconnu→404, pas de version→404, libre→HTML no-store, protégé sans cookie→unlock.html 200 no-store, protégé avec cookie valide→HTML no-store) ; handler `unlock` (POST /c/{slug}/unlock — vérif PIN, émission cookie signé, 204) ; handler `public_meta` (GET /api/public/{slug} — PublicMeta sans hash ni PIN) — Phase 4 Tasks 4-6 — 2026-06-25
- [x] `controllers/serve_ratelimit.rs` : deux `GovernorLayer` in-memory (par-IP + slug-global) via `ServiceBuilder`, montés sur POST /c/{slug}/unlock — Phase 4 Task 7 — 2026-06-25
- [x] `frontend/src/unlock/` (`main.tsx`, `unlock-page.tsx`, `i18n.ts`, `reload.ts`) + `unlock.html` : page de déverrouillage standalone (2ᵉ entrée Vite, formulaire PIN, fetch POST /unlock, `brand_name`) — Phase 4 Task 8 — 2026-06-25
- [x] Config env unlock : `UNLOCK_COOKIE_SECRET` (≥ 64 B), `LATCH_UNLOCK_TTL_DAYS`, `LATCH_UNLOCK_RL_*` documentés dans `.env.example` + `docs/ENVIRONMENT.md` — Phase 4 Task 10 — 2026-06-25
- [x] `InputOTP` shadcn (6 slots, `REGEXP_ONLY_DIGITS`) remplace `<Input>` dans la page unlock — itération UI — 2026-06-25
- [x] `CardDescription` (clé `unlock.instructions` EN+FR) dans la page unlock — itération UI — 2026-06-25
- [x] `vite.config.ts` base `'/'` + mount `/assets` dans `after_routes` (backend) — découplage assets admin/unlock — 2026-06-25
- [x] `Button` prop `loading` réutilisable (spinner `Loader2` + disabled effectif) + câblage sur 7 sites d'action — itération UX — 2026-06-25
- [x] OTP auto-submit sur saisie complète (`onComplete`) + clear-on-error (401) dans `unlock-page.tsx` — itération UX — 2026-06-25
- [x] État d'erreur unlock : cases OTP en rouge (`aria-invalid` par slot) + message centré + reset au re-typage — itération UI — 2026-06-25
- [x] Bordure OTP foncée (`oklch(0.85 0.003 48.717)`, même teinte que `--input`) + retrait favicon `/vite.svg` — itération UI — 2026-06-25
- [x] Fix sécu : fail-secure `UNLOCK_COOKIE_SECRET`/`SESSION_SECRET` (`resolve_cookie_secret`, refus de boot prod sans secret) — revue auto — 2026-06-25

## Phase 5 — Endpoint MCP + panneau Settings

- [x] Helpers `backend/src/web/mod.rs` : `deploy_token(ctx)`, `public_base_url(ctx)` (trailing-slash normalisé), `host_authority(base)` — fail-secure — Phase 5 — 2026-06-25
- [x] `backend/src/mcp/mod.rs` : `LatchMcp` (struct + `#[tool_router]`/`#[tool_handler]`/`ServerHandler`), monté via `nest_service("/mcp", StreamableHttpService)` + `LocalSessionManager` dans `after_routes` — Phase 5 — 2026-06-25
- [x] Tool `deploy_prototype(slug, html, deploy_token, activate?)` : gate token FIRST (`secure_compare`), slug préexistant (pas d'auto-création), `activate` défaut `true`, retourne `DeployResult { url, version, code_protected }` — Phase 5 — 2026-06-25
- [x] Tool `list_projects(deploy_token)` : gate token FIRST, retourne `ProjectListResult { projects: Vec<ProjectSummary> }` (enveloppe objet, jamais tableau racine), `ProjectSummary { slug, name, code_protected, active_version: Option<i32> }` — Phase 5 — 2026-06-25
- [x] `rmcp` épinglé `"1.4"` floor (CVE-2026-42559), résout **1.8.0** ; `allowed_hosts` dérivé de `LATCH_PUBLIC_BASE_URL` via `host_authority()` — Phase 5 — 2026-06-25
- [x] `GET /api/settings` (`AdminAuth`) : `SettingsResponse { deploy_token, mcp_url, public_base_url }`, enregistré dans `openapi.rs` + `openapi.json` + `schema.d.ts` régénérés — Phase 5 — 2026-06-25
- [x] `frontend/src/hooks/use-settings.ts` + `frontend/src/routes/settings.tsx` : topbar, `mcp_url` copyable, `deploy_token` via `PinField` masqué/révéler/copier, `public_base_url` texte, loading/error — Phase 5 — 2026-06-25
- [x] Route `/settings`, icône Settings dans la topbar, i18n `settings.*` (EN+FR) — Phase 5 — 2026-06-25
- [x] `LATCH_PUBLIC_BASE_URL` (nouvelle variable runtime, fail-secure, source hôte public + `allowed_hosts`) — Phase 5 — 2026-06-25
- [x] Tests Phase 5 : 127 backend (dont gate token, deploy_prototype, slug inconnu, invariants sécu, settings 401), 54 frontend. Clippy `--all-features` clean. SonarCloud gate PASSED (~94.8% new_coverage) — Phase 5 — 2026-06-25

## Phase 6 — E2E, durcissement, packaging

- [x] `backend/tests/mcp_http.rs` : 6 tests e2e MCP transport Streamable HTTP réel (initialize handshake, tools/list, deploy_prototype + invariant §9 PIN absent, list_projects enveloppe objet, gate token rejeté ×2 bad-token no-side-effect). `axum-test` ajouté en dev-dep. 136/136 tests backend verts — Phase 6 T2 — 2026-06-25
- [x] `frontend/e2e/serve-unlock.spec.ts` + `frontend/e2e/fixtures/proto-v2.html` : 3 tests Playwright navigateur réel sur la surface `/c` (projet libre no-store, unlock par PIN + auto-submit OTP, bascule v1→v2). Setup API-driven (login + create + deploy via `request` fixture, `Origin` on mutations). 4/4 e2e verts (smoke + serve-unlock) — Phase 6 T3 — 2026-06-25
- [x] `frontend/e2e/screenshots.capture.ts` + `docs/assets/admin-list.png` + `docs/assets/unlock.png` : script de capture Playwright conditionnel (skip sauf `CAPTURE=1` ; `CI=1` active `reuseExistingServer`, indépendant du skip). Captures générées : liste admin (2 projets fictifs, badges d'accès) + page unlock (formulaire OTP). `playwright.config.ts` étendu avec `testMatch: /.*\.(spec|capture)\.ts$/` — Phase 6 T5 — 2026-06-25

## Phase 7 — Peaufinage graphique / web

### Lot 1 — Fondations i18n/thème
- [x] `src/i18n/available-locales.ts` : fonction pure `parseLocales(glob)` — découverte locales + strip `_meta` + sortage — Phase 7 Lot 1 T1 — 2026-06-25
- [x] `src/i18n/locales/{admin,unlock}/{en,fr}.json` : 106 clés admin + 8 clés unlock, chacune avec `_meta` (name, flag ISO) — Phase 7 Lot 1 T2-T3 — 2026-06-25
- [x] `src/i18n/index.ts` réwirée : glob auto-découverte, `locales` exporté, support multi-langue déclaratif — Phase 7 Lot 1 T2 — 2026-06-25
- [x] `src/unlock/i18n.ts` réwirée : instance séparée, glob auto-découverte `locales/unlock/` — Phase 7 Lot 1 T3 — 2026-06-25
- [x] `ThemeProvider` (next-themes `^0.4.6`) monté dans `src/main.tsx` (défaut `system`, storageKey `latch.theme`, anti-FOUC script dans `index.html` uniquement) — Phase 7 Lot 1 T4 — 2026-06-25
- [x] Tests complets : parseLocales (100%), admin i18n, unlock i18n, ThemeProvider, zéro régression 106+8 clés — Phase 7 Lot 1 T1-T4 — 2026-06-25
- [x] Gate finale SDD : lint/typecheck/test+coverage/build, bundle isolation (admin key ≠ unlock bundle), anti-FOUC script présent/absent (index/unlock) — Phase 7 Lot 1 T5 — 2026-06-25
- [x] Mémoire projet : CONVENTIONS (pattern glob+_meta), QUIRKS (import.meta.glob, anti-FOUC), INDEX (Lot 1), HANDOFF (entrée datée) — Phase 7 Lot 1 T5 — 2026-06-25

### Lot 2 — Panneau Settings unifié
- [x] `components/ui/select.tsx` : Select radix wrapper (radix-ui + shadcn style) — Phase 7 Lot 2 T1 — 2026-06-25
- [x] `components/language-select.tsx` : Select + flag-icons CSS (locales-driven auto-découverte) — Phase 7 Lot 2 T1 — 2026-06-25
- [x] `components/theme-toggle.tsx` : Segmented 3-state (system/light/dark), lit `theme` context — Phase 7 Lot 2 T2 — 2026-06-25
- [x] `components/settings-sheet.tsx` : MCP section + Preferences, helper text par réglage, `useSettings(open)` lazy fetch — Phase 7 Lot 2 T3 — 2026-06-25
- [x] Topbar ouvre le Settings sheet ; route `/settings` retirée ; `routes/settings.tsx`+`settings.test.tsx` effacés — Phase 7 Lot 2 T4-T5 — 2026-06-25
- [x] New dep: `flag-icons` CSS (import seulement dans `language-select.tsx`, bundle unlock isolation vérifiée) — Phase 7 Lot 2 T1 — 2026-06-25
- [x] i18n : ~12 nouvelles clés `settings.*` (EN+FR) — Phase 7 Lot 2 T1 — 2026-06-25
- [x] jsdom shims radix Select (`scrollIntoView`, `hasPointerCapture`, `releasePointerCapture`) dans `vitest.setup.ts` — Phase 7 Lot 2 T1 — 2026-06-25
- [x] Gate finale : lint 0 err, typecheck 0 err, tests 76 verts (new coverage ≥ 80%), build OK ; bundle isolation unlock vérifiée (no flag-icons CSS, no `settings.*` strings) — Phase 7 Lot 2 T6 — 2026-06-25
- [x] Mémoire projet : CONVENTIONS (Select+helper-text pattern), QUIRKS (radix Select jsdom), INDEX (Lot 2), HANDOFF (entrée datée) — Phase 7 Lot 2 T6 — 2026-06-25

## Phase 7 Lot 4 — Page d'erreur serving /c
- [x] 3ᵉ entrée Vite `error.html` + `src/error/{main,error-page,i18n}.tsx` + `locales/error/*.json` auto-découverts — Phase 7 Lot 4 T1 — 2026-06-26
- [x] `web::error_index()` retourne `PathBuf` vers `dist/error.html` — Phase 7 Lot 4 T2 — 2026-06-26
- [x] `serve.rs::serve_error_page(status)` lit le HTML, renvoie HTML + `no-store` + status code, fallback texte inline si fichier manque — Phase 7 Lot 4 T2 — 2026-06-26
- [x] 5 branches `Err` terminales de `serve` → `Ok(serve_error_page(...))` (404 slug inconnu, 404 pas de version, 500 DB/storage/version manquante) ; logs `tracing::error!` — Phase 7 Lot 4 T2 — 2026-06-26
- [x] `fake_dist()` écrit `error.html` + test fallback inline quand absent — Phase 7 Lot 4 T2 — 2026-06-26
- [x] Gate finale : cargo fmt/clippy/nextest + pnpm lint/typecheck/vitest/build, couverture error-page ≥ 80%, bundle isolation OK (no admin code), `dist/error.html` présent — Phase 7 Lot 4 T3 — 2026-06-26
- [x] Mémoire projet : CONVENTIONS (page d'erreur pattern), QUIRKS (fake_dist + error.html), BACKLOG (mark Phase 4 item RÉSOLU), INDEX (Lot 4), ROADMAP (Phase 7 ✅ LIVRÉE 2026-06-26 + 4 lots), HANDOFF (entrée datée) — Phase 7 Lot 4 T3 — 2026-06-26

## Phase 8 — Documentation publique (Fumadocs / GitHub Pages)

> Implémentée sur `feat/phase-8-public-docs`. Déploiement Pages au merge sur `main` (job CI `deploy-docs`).

- [x] Scaffold **Fumadocs** (Next 16 + MDX, template `+next+fuma-docs-mdx+static`) dans `public_docs/`, export statique, `basePath '/latch'` + `assetPrefix` + `public/.nojekyll` — Phase 8 T1 — 2026-06-26
- [x] Identité produit : preset `shadcn.css` + tokens stone/oklch (clair/sombre), logo inline `currentColor`, nav, Inter — Phase 8 T2 — 2026-06-26
- [x] Landing produit (hero, parcours 3 étapes avec **conversation Claude simulée** CSS, features, CTA, footer) + page 404 — Phase 8 T3 — 2026-06-26
- [x] Shell docs + **recherche statique Orama** (`staticGET` + `oramaStaticClient`) + intro + ordre sidebar — Phase 8 T4 — 2026-06-26
- [x] CI : jobs `docs` (build push/PR) + `deploy-docs` (Pages, main only) dans `ci.yml`, SHA-pinned — Phase 8 T5 — 2026-06-26
- [x] Contenu EN (sourcé du contrat/BOOTSTRAP, jamais le `docs/` interne) : `how-it-works/` (architecture, security-model, contributing), `deploy/` (docker, compose, **reverse-proxy Caddy/Nginx/Traefik/Apache**, from-source, **configuration 17 clés**, backup-upgrade, releases), `admin/` (projects, access-codes, versions, co-branding), `publish-from-claude/` (connect-mcp, **tools-reference 2 tools**, why-token-not-oauth), `quickstart`, `troubleshooting` — Phase 8 T6-T10 — 2026-06-26
- [x] Schéma flux Claude (composant `ClaudeFlow` themeable) + captures réutilisées (Phase 6) — Phase 8 T11 — 2026-06-26
- [x] Finitions : liens internes vérifiés (0 cassé), `README` + `links.ts` `DOCS_URL` → URL Pages, mémoire projet — Phase 8 T12 — 2026-06-26
- [ ] **Post-merge** : 1ᵉʳ déploiement Pages vert + vérif basePath sur l'URL live (charger une page profonde, confirmer `_next/` chargé)

## Phase 9 — Notes de version (feat/release-notes)

- [x] **Modèle** : colonne `versions.release_notes` (TEXT NULL, max 10 000 chars `chars()`) — migration SeaORM — 2026-06-29
- [x] **Backend** : `deploy()` accepte `release_notes` optionnel ; validation 400/`invalid_params` MCP au-delà de 10 000 ; `GET /c/<slug>/notes` (JSON `{ n, notes_md }` ou 204, gardé unlock, no-store) ; `GET /c/<slug>/raw` (HTML brut cible iframe, `frame-ancestors 'self'`, no-store, gardé unlock) ; `GET /c/<slug>` sert désormais un shell + `<iframe src=/c/<slug>/raw>` — 2026-06-29
- [x] **MCP** : `deploy_prototype` + argument optionnel `release_notes` (markdown léger, max 10 000) — 2026-06-29
- [x] **Admin** : panneau déploiement avec éditeur Tiptap (WYSIWYG léger) + onglet Aperçu ; indicateur icône (notes) sur les lignes de version ; composant `MarkdownView` restreint partagé (admin aperçu + overlay visiteur) — 2026-06-29
- [x] **Admin UX patchs** : action Preview depuis la liste projets (version active, nouvel onglet, route admin `no-store`) ; indicateur notes en icône `FileText` (lucide, remplace l'emoji) ; panel Détail read-only par version (numéro, date, statut, notes rendues via `MarkdownView`) — Spec `docs/superpowers/specs/2026-06-29-release-notes-ux-design.md`, Plan `docs/superpowers/plans/2026-06-29-release-notes-ux.md` — 2026-06-29
- [x] **Shell visiteur** : bundle Vite isolé (`src/shell/`) avec sa propre instance i18n (`src/shell/i18n.ts` + `locales/shell/`) ; overlay notes au premier passage (localStorage `latch:seen:<slug>` = dernier `n` vu) ; dismiss mémorisé — 2026-06-29
- Spec : `docs/superpowers/specs/2026-06-29-release-notes-design.md`
- Plan : `docs/superpowers/plans/2026-06-29-release-notes.md`

## Correctifs post-déploiement (prod)

- [x] **Fix session admin en prod** (`v0.3.1`) : bug `axum_session 0.16.0` — `with_prefix_with_host(true)`
  écrit le cookie `__Host-latch_admin` mais le relit sous le nom brut `latch_admin` → session jamais
  restaurée en prod (login 200 puis 401 silencieux sur routes protégées, rebond vers login). Fix dans
  `web/mod.rs::build_session_store` : nom `__Host-…` posé manuellement en prod, sans `with_prefix_with_host`.
  Durcissement `__Host-` préservé. Cf. `docs/QUIRKS.md`. — 2026-06-26

## Phases closes
- [x] Phase 0 — scaffold & squelette CI/Docker — 2026-06-24
- [x] Phase 1 — cœur + modèle + migrations — 2026-06-24
- [x] Phase 2 — adaptateur web admin — 2026-06-24
- [x] Phase 3 — SPA admin (Yew, puis migrée React — Plans 1-3) — 2026-06-24/2026-06-25
- [x] Phase 4 — serving `/c/<slug>` — 2026-06-25
- [x] Phase 5 — endpoint MCP + panneau Settings — 2026-06-25
- [x] Phase 6 — e2e, durcissement, packaging (robots.txt + X-Robots-Tag, e2e MCP HTTP ×6, e2e /c + unlock ×3, captures Playwright, CHANGELOG git-cliff, README refonte + badges) — 2026-06-25
- [x] Phase 7 ✅ LIVRÉE (2026-06-26) — Lot 1: Fondations i18n/thème (auto-découverte, ThemeProvider, anti-FOUC, mémoire) ; Lot 2: Panneau Settings unifié (Select radix, language/theme toggles, MCP section, lazy fetch, mémoire) ; Lot 3: Identité visuelle (Logo favicon SVG + topbar + login + unlock, titres de page dynamiques, largeur admin max-w-6xl, lien GitHub + bouton ? doc, inline GitHub SVG, mémoire) ; Lot 4: Page d'erreur serving /c (3ᵉ entrée Vite, serve_error_page + fallback, logs 500, page générique, mémoire)

## Phase 10 — Commentaires ancrés (Plan 1 Backend, branche feat/prototype-comments)
- [x] Schéma `comment_pins` + `comments` (FK CASCADE, soft-delete `deleted_at`) + `projects.comments_enabled` (backfill = `code_enabled`) — Plan 1 T2 — 2026-06-30
- [x] `CommentsService` cœur (create_pin/add_reply/list/count/edit/delete/delete_pin/moderate ; owner-check `secure_compare`→NotFound ; validation 2000/80 ; plafond 200 pins) — Plan 1 T3-4 — 2026-06-30
- [x] Toggle `comments_enabled` par projet (service + DTO + admin, défaut sécurité-aware) — Plan 1 T5 — 2026-06-30
- [x] Identité visiteur : cookie signé `latch_comment` (ULID, réutilise `UNLOCK_COOKIE_SECRET`) + garde `X-Comment-Client` — Plan 1 T6 — 2026-06-30
- [x] DTOs commentaires (`owner_token` **jamais sérialisé**, `editable`) + `comment_count` — Plan 1 T7 — 2026-06-30
- [x] Endpoints publics `/c/{slug}/comments` (GET/POST/replies/PUT/DELETE), gated `unlock_ok`+`comments_enabled`, rate-limit `LATCH_COMMENT_RL_*`, Origin + `X-Comment-Client` — Plan 1 T8-9 — 2026-06-30
- [x] Endpoints admin : `GET .../versions/{n}/comments` (`list_version_comments`) + `DELETE .../comments/messages/{cid}` (modération, walk projet) — Plan 1 T10 — 2026-06-30
- [x] Contrat amendé (§3/§6.4/§7/§9 invariant `owner_token`) + OpenAPI/`schema.d.ts` régénérés + invariants build-breaking (`owner_token` 3 surfaces, gate verrouillé 403) — Plan 1 T1,10,11 — 2026-06-30
- [x] Gate complète : 181 nextest, drift green, clippy/fmt/cargo-deny clean, **revue finale opus = YES**, **SonarCloud PASSED** (97.7 % couverture, 2.1 % duplication) — Plan 1 — 2026-06-30
- [x] **Plan 2 (frontend visiteur) LIVRÉ** — plan : `docs/superpowers/plans/2026-06-30-prototype-comments-frontend.md` ; spec : `docs/superpowers/specs/2026-06-30-prototype-comments-design.md` ; commits `fc7d616..d9d4a33` ; gate verte (vitest 173, e2e 6, lint/typecheck clean), revue finale opus = YES — 2026-06-30
- [x] Couche commentaires visiteur — module partagé `frontend/src/comments/` (seam Picker + SameOriginPicker, moteur d'ancrage describe/resolve/similarité, contrôleur de suivi rAF, adaptateur visiteur + hooks React Query confinés, overlay/pastilles/popups @floating-ui, barre d'action, machine pick) — Plan 2 commentaires — 2026-06-30
- [x] Montage shell visiteur : `/c/<slug>` lit `PublicMeta.comments_enabled` et charge le module en lazy (React.lazy, 1er du repo ; React Query confiné au chunk) — Plan 2 commentaires — 2026-06-30
- [x] e2e Playwright visiteur (cibler→écrire→pin ancré→persistance reload) — Plan 2 commentaires — 2026-06-30
- [ ] **Plan 3** (frontend admin Review + passe `public_docs`) — à faire
