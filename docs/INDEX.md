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

## Frontend (SPA Yew)
- [x] Crate `latch-ui` (Yew 0.21 CSR) buildée par Trunk → bundle wasm `dist/` — Phase 0 — 2026-06-24

## Infra (CI / Docker / déploiement)
- [x] Dockerfile multi-stage (Trunk wasm → build Rust → distroless), image ~85 Mo, boot vérifié — Phase 0 — 2026-06-24
- [x] CI GitHub Actions **verte sur main** (fmt/clippy, tests, build SPA, cargo-deny bloquant, docker GHCR) — Phase 0 — 2026-06-24
- [x] Images GHCR versionnées (`metadata-action` : semver+latest+sha) + pin déploiement (`LATCH_IMAGE_TAG`) — Phase 0 — 2026-06-24
- [x] `docker-compose.yml` + `deploy.sh` + `.env.example` + dual-license MIT/Apache — Phase 0 — 2026-06-24

## Phases closes
- [x] Phase 0 — scaffold & squelette CI/Docker — 2026-06-24
- [x] Phase 1 — cœur + modèle + migrations — 2026-06-24
- [ ] Phase 2 — adaptateur web admin
- [ ] Phase 3 — SPA Yew admin
- [ ] Phase 4 — serving `/c/<slug>`
- [ ] Phase 5 — endpoint MCP
- [ ] Phase 6 — e2e, durcissement, packaging
