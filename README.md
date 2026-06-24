# latch

[![CI](https://github.com/owlnext-fr/latch/actions/workflows/ci.yml/badge.svg)](https://github.com/owlnext-fr/latch/actions/workflows/ci.yml)

Petite app Rust qui sert des prototypes HTML mono-fichier derrière un host
contrôlé, avec versioning et code d'accès optionnel par projet. Trois surfaces
sur un seul binaire [Loco](https://loco.rs) :

- **Serving client** `/c/<slug>` — sert la version active d'un prototype, avec
  page de déverrouillage stylée + PIN si le projet est protégé.
- **Admin** `/admin` — SPA [Yew](https://yew.rs) (session cookie) : projets,
  versions, déploiement manuel, configuration des codes.
- **MCP** `/mcp` — endpoint appelé par Claude pour déployer un prototype.

> ⚠️ Projet en cours de construction (Phase 0 — scaffold). Voir `docs/ROADMAP.md`.

## Stack

Loco (axum) + SeaORM + SQLite (`bundled`) · Yew + shadcn-rs (Trunk) · rmcp ≥ 1.4.

## Développement

```bash
# Serveur : se lance DEPUIS backend/ (Loco lit ./config relativement au CWD).
# L'alias `cargo loco` est câblé à la racine (.cargo/config.toml) et reste
# disponible depuis backend/ par recherche ascendante. Cf. docs/QUIRKS.md.
cd backend && cargo loco start        # lance l'app (auto-migrate au boot)
cd backend && cargo loco db migrate   # migrations explicites

# Qualité (depuis la racine — default-members = backend, le frontend wasm est à part)
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo nextest run                     # tests backend (ou: cargo test)

# Frontend (SPA Yew)
cd frontend && trunk serve            # dev server
cd frontend && trunk build --release  # bundle wasm de prod
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
