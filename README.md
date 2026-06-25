# latch

[![CI](https://github.com/owlnext-fr/latch/actions/workflows/ci.yml/badge.svg)](https://github.com/owlnext-fr/latch/actions/workflows/ci.yml)

Petite app Rust qui sert des prototypes HTML mono-fichier derrière un host
contrôlé, avec versioning et code d'accès optionnel par projet. Trois surfaces
sur un seul binaire [Loco](https://loco.rs) :

- **Serving client** `/c/<slug>` — sert la version active d'un prototype, avec
  page de déverrouillage stylée + PIN si le projet est protégé.
- **Admin** `/admin` — SPA React/Vite (session cookie) : projets,
  versions, déploiement manuel, configuration des codes.
- **MCP** `/mcp` — endpoint appelé par Claude pour déployer un prototype.

## Stack

**Backend** : Loco (axum) + SeaORM + SQLite (`bundled`) · rmcp ≥ 1.4.

**Frontend** : React + Vite + TypeScript + pnpm · TanStack Router/Query ·
shadcn/ui (Radix, stone oklch) + Tailwind v4 · react-i18next (FR/EN) · sonner ·
openapi-fetch/openapi-typescript (client typé depuis `openapi.json`).

## Développement

```bash
# Serveur : se lance DEPUIS backend/ (Loco lit ./config relativement au CWD).
# L'alias `cargo loco` est câblé à la racine (.cargo/config.toml) et reste
# disponible depuis backend/ par recherche ascendante. Cf. docs/QUIRKS.md.
cd backend && cargo loco start        # lance l'app (auto-migrate au boot)
cd backend && cargo loco db migrate   # migrations explicites

# Qualité backend (depuis la racine)
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo nextest run                     # tests backend (ou: cargo test)

# Frontend (depuis frontend/)
cd frontend && pnpm dev               # dev server React/Vite (HMR, port 5173)
cd frontend && pnpm build             # bundle de prod → dist/
```

## Qualité

```bash
# Backend
cargo fmt --all && cargo clippy --all-targets -- -D warnings
cargo nextest run
cargo deny check

# Frontend
cd frontend && pnpm lint
cd frontend && pnpm typecheck
cd frontend && pnpm test

# E2E
cd frontend && pnpm exec playwright test
```

## Déploiement

Image multi-stage publiée sur GHCR public (`ghcr.io/owlnext-fr/latch`).
Déploiement manuel sur la box via `./deploy.sh` (pull + up + prune).
Voir `docs/BOOTSTRAP.md §7-8`.

## Architecture & documentation

Le contrat d'architecture fait loi : `docs/contrat-deploy.md`. Stack et outillage :
`docs/BOOTSTRAP.md`. Phases : `docs/ROADMAP.md`.

## Licence

Dual-license [MIT](LICENSE-MIT) OU [Apache-2.0](LICENSE-APACHE), au choix.
