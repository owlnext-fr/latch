# Index — ce qui est livré et marche

> Le **réalisé**, par opposition au ROADMAP (le prévu). Quand les critères de sortie
> d'une phase sont verts, on coche ici avec une ligne par livrable (+ entrée HANDOFF).
> Format : `- [x] <livrable> — <phase> — <date>`.

## Backend (cœur + adaptateurs)
- [x] Scaffold app Loco (`backend/`, crate `latch`, bin `latch-cli`) — SQLite `bundled`,
  sans users/JWT, sans worker (`--bg none`) — Phase 0 — 2026-06-24
- [x] Workspace 2 membres + `migration` (sea-orm 1.1 aligné Loco), `auto_migrate` au boot — Phase 0 — 2026-06-24

## Frontend (SPA Yew)
- [x] Crate `latch-ui` (Yew 0.21 CSR) buildée par Trunk → bundle wasm `dist/` — Phase 0 — 2026-06-24

## Infra (CI / Docker / déploiement)
- [x] Dockerfile multi-stage (Trunk wasm → build Rust → distroless), image ~85 Mo, boot vérifié — Phase 0 — 2026-06-24
- [x] CI GitHub Actions (fmt/clippy, tests, build SPA, cargo-deny, docker GHCR) — Phase 0 — 2026-06-24
- [x] `docker-compose.yml` + `deploy.sh` + `.env.example` + dual-license MIT/Apache — Phase 0 — 2026-06-24

## Phases closes
- [x] Phase 0 — scaffold & squelette CI/Docker — 2026-06-24
- [ ] Phase 1 — cœur + modèle + migrations
- [ ] Phase 2 — adaptateur web admin
- [ ] Phase 3 — SPA Yew admin
- [ ] Phase 4 — serving `/c/<slug>`
- [ ] Phase 5 — endpoint MCP
- [ ] Phase 6 — e2e, durcissement, packaging
