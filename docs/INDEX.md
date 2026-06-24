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
- [x] Crate `latch-dto` (workspace, serde, cible native + wasm32) — types partagés `ProjectListItem`, `ProjectDetail`, `VersionItem`, `CreateProjectReq`, `UpdateProjectReq`, `SetCodeReq`, `DeployReq` — Phase 3 — 2026-06-24
- [x] API JSON re-préfixée sous `/api/*` (depuis `/admin/*`) + conversions libres `dto::to_list_item`/`to_detail` — Phase 3 — 2026-06-24
- [x] `web::spa_dist_dir()` + `nest_service("/admin", ServeDir + ServeFile fallback)` dans `after_routes` — serving SPA Yew sous `/admin` avec fallback `index.html` — Phase 3 — 2026-06-24

## Frontend (SPA Yew)
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

## Infra (CI / Docker / déploiement)
- [x] Dockerfile multi-stage (Trunk wasm → build Rust → distroless), image ~85 Mo, boot vérifié — Phase 0 — 2026-06-24
- [x] CI GitHub Actions **verte sur main** (fmt/clippy, tests, build SPA, cargo-deny bloquant, docker GHCR) — Phase 0 — 2026-06-24
- [x] Images GHCR versionnées (`metadata-action` : semver+latest+sha) + pin déploiement (`LATCH_IMAGE_TAG`) — Phase 0 — 2026-06-24
- [x] `docker-compose.yml` + `deploy.sh` + `.env.example` + dual-license MIT/Apache — Phase 0 — 2026-06-24

## Phases closes
- [x] Phase 0 — scaffold & squelette CI/Docker — 2026-06-24
- [x] Phase 1 — cœur + modèle + migrations — 2026-06-24
- [x] Phase 2 — adaptateur web admin — 2026-06-24
- [x] Phase 3 — SPA Yew admin — 2026-06-24
- [ ] Phase 4 — serving `/c/<slug>`
- [ ] Phase 5 — endpoint MCP
- [ ] Phase 6 — e2e, durcissement, packaging
