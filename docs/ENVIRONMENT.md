# Environment — spécifique à l'instance

> Ce qui est propre à *ta* machine / *ton* déploiement : paths réels, ports, contenu
> du `.env`, secrets (jamais les valeurs ici — juste les clés attendues). Pour les
> commandes génériques de build/test, voir `docs/BOOTSTRAP.md §3` plutôt que dupliquer.

## Variables d'environnement attendues (`.env`)
- `ADMIN_USER` — identifiant admin.
- `ADMIN_PASS` — mot de passe admin (comparé à temps constant, non hashé).
- `DEPLOY_TOKEN` — secret applicatif validé par les tools MCP.
- `UNLOCK_COOKIE_SECRET` — clé HMAC de signature du cookie de déverrouillage client.
- `SESSION_SECRET` — clé HMAC de signature du cookie de session admin (≥ 64 bytes). En dev : clé de secours déterministe (voir `web/mod.rs`). **Obligatoire en prod.**
- `LATCH_STORAGE_ROOT` — racine du volume HTML des versions. Défaut : `data`. En prod : `/data` (volume Docker). Utilisé par `storage_from_ctx`.
- `LATCH_SPA_DIST` — racine des assets buildés de la SPA (Trunk `dist/`). Défaut dev (CWD `backend/`) : `../frontend/dist`. Prod (image) : `/app/frontend/dist` (posé par le Dockerfile). Lu par `web::spa_dist_dir()`.
- `DATABASE_URL` — URI SQLite. Dev (défaut) : `sqlite://latch_development.sqlite?mode=rwc`.
  Prod (image) : `sqlite:///data/latch.sqlite?mode=rwc` (volume monté). Modèle : `.env.example`.
- `PORT` — port d'écoute backend (défaut `5150`).

## Repo & exécution (cette instance)
- **Path repo** : `/srv/owlnext/latch` · **branche par défaut** : `main` (commits directs / branches courtes).
- **Toolchain** : Rust 1.96, `wasm32-unknown-unknown`, Trunk 0.21, Docker 29, Node 24,
  **`sea-orm-cli`** (≈ 1.1.x, aligné sur `sea-orm`) — requis par `cargo loco db entities`
  (`cargo install sea-orm-cli`), cf. QUIRKS.
- **Lancer le serveur** : `cd backend && cargo loco start` (Loco lit `./config` depuis le
  CWD → impératif depuis `backend/`, cf. QUIRKS). `fmt`/`clippy`/`test` : depuis la racine.
- **Build image locale** : `docker build -t ghcr.io/owlnext-fr/latch:dev .` (multi-stage).

## Serving
- Domaine : `latch.owlnext.fr` (Caddy en façade, TLS + reverse proxy).
- Path MCP : `/mcp` _(option : path non devinable — à figer si retenu)._

## Box de déploiement
- _(host, chemin du repo/compose, emplacement du volume `data/` — à remplir)._

## GHCR
- Package : `ghcr.io/owlnext-fr/latch` — **public** (pas de `docker login` au pull).
- **Schéma de tags** (CI, `docker/metadata-action`) :
  - release `vX.Y.Z` → `X.Y.Z` (immuable, à pinner en prod), `X.Y`, `latest`, `sha-xxxxxxx` ;
  - `main` → `main` (dernier état intégré, pour staging), `sha-xxxxxxx`.
- **Pin du déploiement** : `LATCH_IMAGE_TAG` dans `.env` (défaut `latest`). Rollback =
  remettre l'ancien tag + `./deploy.sh`.

## Connexion du connecteur MCP côté Claude web
- _(procédure de branchement aux designers — dépend de la formule OWLNEXT,
  laissée hors périmètre build ; à documenter au moment du branchement)._
