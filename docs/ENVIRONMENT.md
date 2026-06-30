# Environment — spécifique à l'instance

> Ce qui est propre à *ta* machine / *ton* déploiement : paths réels, ports, contenu
> du `.env`, secrets (jamais les valeurs ici — juste les clés attendues). Pour les
> commandes génériques de build/test, voir `docs/BOOTSTRAP.md §3` plutôt que dupliquer.

## Variables d'environnement attendues (`.env`)
- `ADMIN_USER` — identifiant admin.
- `ADMIN_PASS` — mot de passe admin (comparé à temps constant, non hashé).
- `DEPLOY_TOKEN` — secret applicatif validé par TOUS les tools MCP (`deploy_prototype` + `list_projects`). Comparaison à temps constant (`secure_compare`). **OBLIGATOIRE en prod** (le boot refuse de démarrer si absent hors Dev/Test — fail-secure). Injecter via `.env` ou secret Docker.
- `LATCH_PUBLIC_BASE_URL` — URL publique racine de l'instance (ex. `https://latch.owlnext.fr`). Slash terminal normalisé automatiquement. **OBLIGATOIRE en prod** (fail-secure). Utilisée par : (a) `allowed_hosts` rmcp (dérivé via `web::host_authority()`, défense contre DNS rebinding) ; (b) champ `url` retourné par `deploy_prototype` (`<LATCH_PUBLIC_BASE_URL>/c/<slug>`) ; (c) réponse `GET /api/settings` (`public_base_url` + `mcp_url`). En dev : `http://localhost:5150`.
- `UNLOCK_COOKIE_SECRET` — clé HMAC de signature du cookie de déverrouillage client (≥ 64 bytes, `Key::from()` panique en dessous). **OBLIGATOIRE en prod** (le boot refuse de démarrer si absente hors Dev/Test — fail-secure). En dev, un fallback déterministe de 64 chars est utilisé. Générer : `openssl rand -hex 32`.
- `LATCH_UNLOCK_TTL_DAYS` — durée de vie du cookie d'unlock (jours). Défaut : 30.
- `LATCH_UNLOCK_RL_IP_BURST` — governor IP : burst (réservation burst). Défaut : 5.
- `LATCH_UNLOCK_RL_IP_PER_SECOND` — governor IP : taux de remplissage (req/s). Défaut : 1.
- `LATCH_UNLOCK_RL_SLUG_BURST` — governor slug-global : burst. Défaut : 20.
- `LATCH_UNLOCK_RL_SLUG_PERIOD_SECS` — governor slug-global : période de remplissage (secondes). Défaut : 3.
- `SESSION_SECRET` — clé HMAC de signature du cookie de session admin (≥ 64 bytes). En dev : clé de secours déterministe (voir `web/mod.rs`). **Obligatoire en prod.**
- `LATCH_STORAGE_ROOT` — racine du volume HTML des versions. Défaut : `data`. En prod : `/data` (volume Docker). Utilisé par `storage_from_ctx`.
- `LATCH_SPA_DIST` — racine des assets buildés de la SPA React (Vite `dist/`). Défaut dev (CWD `backend/`) : `../frontend/dist`. Prod (image) : `/app/frontend/dist` (posé par le Dockerfile). Lu par `web::spa_dist_dir()`. **Note** : `unlock.html` (page de déverrouillage client) est servie depuis cette même racine (`dist/unlock.html`) — c'est la 2ᵉ entrée Vite build (Phase 4) ; depuis la refonte assets (base Vite `'/'`), les deux bundles référencent `/assets/...` (sans préfixe `/admin/`), servis par le mount `nest_service("/assets", ServeDir::new(dist.join("assets")))` dans `after_routes`.
- `DATABASE_URL` — URI SQLite. Dev (défaut) : `sqlite://latch_development.sqlite?mode=rwc`.
  Prod (image) : `sqlite:///data/latch.sqlite?mode=rwc` (volume monté). Modèle : `.env.example`.
- `PORT` — port d'écoute backend (défaut `5150`).
- `LATCH_BINDING` — interface sur laquelle le serveur bind (`server.binding`). **Défaut `localhost`** (dev local). Les **e2e Playwright** exportent `127.0.0.1` pour forcer un bind IPv4 explicite, cohérent avec le poll `127.0.0.1/_health` (sinon `localhost` peut résoudre vers `::1`/IPv6 sur les runners CI → timeout webServer flaky, cf. QUIRKS). Configuré dans `backend/config/development.yaml` via Tera.
- `LATCH_BODY_LIMIT` — taille max du body des requêtes (le deploy envoie le HTML mono-fichier en JSON). Valeurs `byte_unit` (`5mb`, `10mb`, `32mb`) ou `disable`. **Défaut `5mb`** (l'ancien défaut Loco `limit_payload` était 2 Mo → 413 sur un gros proto). Configuré dans `backend/config/*.yaml` via `server.middlewares.limit_payload.body_limit`.

## SonarCloud
- **Secret GitHub** : `SONAR_TOKEN` — token d'accès SonarCloud (projet `owlnext-fr_latch`, org `owlnext-fr`). À créer dans les settings GitHub du repo (`Settings > Secrets and variables > Actions`). Générer depuis `sonarcloud.io > My Account > Security`.
- **Identifiants SonarCloud** :
  - `sonar.organization=owlnext-fr`
  - `sonar.projectKey=owlnext-fr_latch`
  - Ces valeurs sont dans `sonar-project.properties` (commité).
- **`.env.local`** (gitignoré) : fichier optionnel pour stocker le token localement pour les scans manuels. Format : `SONAR_TOKEN=<votre_token>`. Ne jamais commiter ce fichier.
- **Automatic Analysis** : DÉSACTIVÉ dans les settings SonarCloud (`Administration > Analysis Method`) — le scanner CI est l'unique source (les deux modes sont exclusifs, cf. QUIRKS).

### Scan local (Docker) — recette complète

**Prérequis :** `cargo-llvm-cov` installé (`cargo install cargo-llvm-cov`) + composant `llvm-tools-preview` (`rustup component add llvm-tools-preview`) + `cargo-deny` + Docker + `.env.local` contenant `SONAR_TOKEN=<token>`.

**Étape 1 — Générer la couverture Rust (depuis la racine du repo) :**
```bash
cargo llvm-cov nextest --lcov --output-path backend-lcov.info
```

**Étape 2 — Générer la couverture frontend :**
```bash
cd frontend && pnpm test:cov
# produit frontend/coverage/lcov.info
```

**Étape 3 — CRITIQUE : remappe les chemins absolus avant le scan local.**
`cargo-llvm-cov` écrit des chemins absolus (`/srv/owlnext/latch/…`) dans `backend-lcov.info`.
Le container `sonarsource/sonar-scanner-cli` monte le repo sous `/usr/src` → chemin différent →
le sensor Rust LCOV **ignore silencieusement** tout le backend → couverture spurieusement basse
et **faux échec de gate**. Fix obligatoire avant le scan local (pas nécessaire en CI où les chemins correspondent) :
```bash
sed -i "s#$(pwd)/#/usr/src/#g" backend-lcov.info
```

**Étape 4 — Lancer le scan (scoped à la branche courante) :**
```bash
docker run --rm \
  -e SONAR_TOKEN="$(grep SONAR_TOKEN .env.local | cut -d= -f2)" \
  -v "$(pwd):/usr/src" \
  sonarsource/sonar-scanner-cli \
  -Dsonar.branch.name="$(git rev-parse --abbrev-ref HEAD)" \
  -Dsonar.qualitygate.wait=true
```
`-Dsonar.branch.name=<branch>` scoped le scan à la branche, sans polluer `main`.
`sonar.qualitygate.wait=true` → sortie non-zéro si la gate échoue.

**Gate :** `new_coverage ≥ 80%` sur le code neuf, ratings A, 0 duplication new code.
Cf. QUIRKS pour le détail du piège chemin absolu/`/usr/src`.

## Toolchain couverture Rust
- **`cargo-llvm-cov`** : installé en CI via `taiki-e/install-action@v2` (`tool: cargo-llvm-cov,nextest`). Localement : `cargo install cargo-llvm-cov`.
- **`llvm-tools-preview`** : composant Rust requis par `cargo-llvm-cov`. En CI, ajouté à `dtolnay/rust-toolchain` via `components: llvm-tools-preview`. Localement : `rustup component add llvm-tools-preview`.
- **Commande locale** : `cargo llvm-cov nextest --lcov --output-path backend-lcov.info` (depuis la racine).

## Toolchain CHANGELOG (git-cliff)
- **`git-cliff`** : installé via Cargo (`cargo install git-cliff`). Utilisé pour générer `CHANGELOG.md` depuis l'historique git.
- **Configuration** : `cliff.toml` à la racine du repo (2 passes preprocessor gitmoji : retire les émojis en tête et en milieu de sujet, parsers réordonnés : Sécurité avant `^feat`).
- **Régénérer** : `git cliff --output CHANGELOG.md` (depuis la racine). Ajouter `--tag vX.Y.Z` pour un nouveau tag.
- **Note** : git-cliff n'est pas en CI (génération manuelle avant chaque release). Entrée BACKLOG si on veut l'automatiser.

## Captures Playwright (screenshots)
- **Condition** : les tests de capture (`e2e/screenshots.capture.ts`) sont skippés par défaut.
  Activer avec : `CAPTURE=1 pnpm exec playwright test screenshots.capture` (depuis `frontend/`).
- **`CAPTURE=1`** : contrôle le skip (`test.skip(!process.env.CAPTURE, ...)`). Seule variable requise.
- **`CI=1`** : active `reuseExistingServer: true` dans `playwright.config.ts` (réutilise le serveur déjà lancé). Indépendant du CAPTURE — utile pour ne pas relancer le build si le serveur tourne déjà.
- **Résultat** : `docs/assets/admin-list.png` (liste admin, 2 projets fictifs) + `docs/assets/unlock.png` (page unlock formulaire OTP).
- **Données** : toujours des placeholders fictifs (`Mon Projet`, `ACME`) — jamais de nom client (cf. règle confidentialité CLAUDE.md).

## Badges SonarCloud (README)
- **Visibilité publique requise** : les badges SonarCloud (`Quality Gate`, `Coverage`) ne s'affichent que si le projet SonarCloud est **public** (`Administration > Visibility > Public`). Si le badge renvoie une icône « brisée », vérifier la visibilité dans les settings SonarCloud.
- **Clé du projet** : `owlnext-fr_latch` (organisation `owlnext-fr`). URLs des badges dans le README.

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
- Path MCP : `/mcp` (figé — monté via `nest_service("/mcp", …)` dans `after_routes`).
  URL complète : `<LATCH_PUBLIC_BASE_URL>/mcp` (ex. `https://latch.owlnext.fr/mcp`).
  `allowed_hosts` = autorité de `LATCH_PUBLIC_BASE_URL` (ex. `latch.owlnext.fr`) — dérivé automatiquement.
- Path admin : `/admin` (SPA React statique + API JSON sous `/api/*`).
  Routes SPA notables : `/admin` (liste), `/admin/projects/{id}` (détail),
  `/admin/projects/{id}/versions/{n}/review` (page Review commentaires, full-screen, créée en Plan 3).
- Path serving client : `/c/<slug>` (page de déverrouillage ou proto HTML actif).

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

Wiring déduit de la doc rmcp + architecture Phase 5 (non encore validé en prod — à confirmer au 1er branchement) :

1. Récupérer `mcp_url` et `deploy_token` depuis le panneau Settings (`/admin/settings`).
2. Dans Claude web → Paramètres → Connecteurs MCP (ou équivalent selon la formule) : renseigner l'URL MCP (`https://latch.owlnext.fr/mcp`). Pas d'OAuth, pas de header d'auth — l'auth est dans l'argument `deploy_token` de chaque tool.
3. Tester avec `list_projects(deploy_token=<valeur>)`.

> _(Procédure UI exacte côté Claude web à documenter au moment du branchement réel — dépend de la formule OWLNEXT.)_

## Site de documentation publique (`public_docs/`, Phase 8)

- **App** : `public_docs/` — **Fumadocs** (Next.js + MDX), **isolée** du workspace Rust et de
  `frontend/` (son propre `package.json` + lockfile). Node 24 (`.nvmrc`), **pnpm 9.15.9**.
- **URL publique** : **`https://owlnext-fr.github.io/latch`** (GitHub Pages, sous-chemin projet ;
  **pas** de domaine custom). `basePath`/`assetPrefix` = `/latch` (env `DOCS_BASE_PATH`, défaut `/latch`).
- **Build/dev** (depuis `public_docs/`) : `pnpm dev` (→ `http://localhost:3000/latch/`),
  `pnpm build` (export statique → `public_docs/out/`), `pnpm lint`, `pnpm types:check`.
- **Déploiement** : porté par **`ci.yml`** (pas de workflow séparé). Job **`docs`** = build à chaque
  push/PR (+ `upload-pages-artifact` sur `out/`). Job **`deploy-docs`** = `deploy-pages`, **`main`
  only**, `permissions: pages/id-token`, `environment: github-pages`, `concurrency: pages`.
- **Pré-requis (déjà fait)** : Settings repo → Pages = « GitHub Actions ». **Pas** de DNS/domaine custom.
- **Lien depuis le produit** : `frontend/src/lib/links.ts` `DOCS_URL` = `https://owlnext-fr.github.io/latch/docs`.
- **Captures** : `public_docs/public/img/{admin-list,unlock}.png` (réutilisées de `docs/assets/`, Phase 6).
- **Contenu** : EN, **uniquement** sous `public_docs/content/docs/` — jamais le `docs/` interne.

## Variables d'environnement — commentaires ancrés (Plan 1, 2026-06-30)
- `LATCH_COMMENT_RL_IP_BURST` — rate-limit écriture commentaires, par-IP : burst. Défaut : **10**.
- `LATCH_COMMENT_RL_IP_PER_SECOND` — par-IP : taux de remplissage (req/s). Défaut : **1**.
- `LATCH_COMMENT_RL_SLUG_BURST` — par-slug global : burst. Défaut : **60**.
- `LATCH_COMMENT_RL_SLUG_PERIOD_SECS` — par-slug : période de remplissage (s). Défaut : **1**.
- **Cookie d'identité visiteur** (`latch_comment`, ULID opaque, TTL 365 j, `Path=/c/<slug>`) : signé avec la
  clé **`UNLOCK_COOKIE_SECRET` réutilisée** — **aucun nouveau secret à provisionner**.
- Dépendance Cargo ajoutée au backend : **`ulid`** (1.x) — génération des `owner_token`.
- Scan Sonar local (rappel recette §Scan local) : la branche commentaires a validé la gate
  (couverture 97.7 %, duplication 2.1 % après 2 passes de dédup).
