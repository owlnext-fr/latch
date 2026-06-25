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
- `LATCH_SPA_DIST` — racine des assets buildés de la SPA React (Vite `dist/`). Défaut dev (CWD `backend/`) : `../frontend/dist`. Prod (image) : `/app/frontend/dist` (posé par le Dockerfile). Lu par `web::spa_dist_dir()`.
- `DATABASE_URL` — URI SQLite. Dev (défaut) : `sqlite://latch_development.sqlite?mode=rwc`.
  Prod (image) : `sqlite:///data/latch.sqlite?mode=rwc` (volume monté). Modèle : `.env.example`.
- `PORT` — port d'écoute backend (défaut `5150`).
- `LATCH_BODY_LIMIT` — taille max du body des requêtes (le deploy envoie le HTML mono-fichier en JSON). Valeurs `byte_unit` (`5mb`, `10mb`, `32mb`) ou `disable`. **Défaut `5mb`** (l'ancien défaut Loco `limit_payload` était 2 Mo → 413 sur un gros proto). Configuré dans `backend/config/*.yaml` via `server.middlewares.limit_payload.body_limit`.

## Repo & exécution (cette instance)
- **Path repo** : `/srv/owlnext/latch` · **branche par défaut** : `main` (commits directs / branches courtes).
- **Toolchain backend** : Rust 1.96, Docker 29,
  **`sea-orm-cli`** (≈ 1.1.x, aligné sur `sea-orm`) — requis par `cargo loco db entities`
  (`cargo install sea-orm-cli`), cf. QUIRKS.
- **Toolchain frontend** : Node 24 (`.nvmrc` dans `frontend/`), **pnpm** via corepack (épinglé
  `pnpm@9.15.9` dans `packageManager`), Playwright (installé dans `frontend/node_modules`).
- **Lancer le serveur** : `cd backend && cargo loco start` (Loco lit `./config` depuis le
  CWD → impératif depuis `backend/`, cf. QUIRKS). `fmt`/`clippy`/`test` : depuis la racine.
- **Frontend dev** : `cd frontend && pnpm dev` (Vite HMR, port 5173 par défaut).
- **Frontend build** : `cd frontend && pnpm build` (bundle → `frontend/dist/`).
- **Tests frontend** : `cd frontend && pnpm test` (Vitest) ; `pnpm exec playwright test` (e2e).
- **Build image locale** : `docker build -t ghcr.io/owlnext-fr/latch:dev .` (multi-stage Node + Rust + runtime).
- **DB e2e** : `LATCH_E2E_DB=/tmp/latch-e2e.sqlite` (SQLite de test pour Playwright, séparée de la dev).

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
