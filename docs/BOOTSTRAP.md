# BOOTSTRAP — latch

> Stack, versions épinglées, outillage, structure du repo, règles de test, CI,
> Docker, déploiement. Le « comment ». Les décisions d'archi sont dans le contrat.

## 1. Stack

- **Backend** : Loco (sur axum) + SeaORM + **SQLite**.
  `libsqlite3-sys` en feature **`bundled`** → le binaire embarque SQLite, l'image
  runtime n'a aucune lib système à fournir.
- **MCP** : `rmcp` (transport `transport-streamable-http-server`), **≥ 1.4.0**.
- **Frontend** : **React + Vite + TypeScript + pnpm** (livré Plans 1-3, 2026-06-25).
  TanStack Router (code-based, basepath `/admin`) + TanStack Query + openapi-fetch/openapi-typescript
  (client typé depuis `openapi.json` → `frontend/src/api/schema.d.ts`) + shadcn/ui (Radix, base
  **stone**, preset oklch `bJfDPe2y`) + Tailwind v4 + react-hook-form/zod + react-i18next (FR/EN,
  défaut EN) + sonner. Servi en **statique** par Loco sous `/admin` (mécanisme inchangé).
  _(Historique : crate Yew `latch-ui` + `shadcn-rs`, build Trunk wasm — voir branche git pré-migration.)_
- **Cookie signé** (déverrouillage client) : `axum-extra` (`SignedCookieJar`) ou
  `cookie` — résoudre l'API exacte via Context7.
- Pas de hachage de mot de passe : le PIN est récupérable (contrat §3), l'`ADMIN_PASS`
  est comparé à temps constant depuis l'env. Aucun `argon2`/`bcrypt` requis en v1.
- **Pas de Redis, pas de worker.** La file de jobs Loco est désactivée (ou backend
  in-process). Aucun job dans le périmètre.

## 2. Versions épinglées

Épingler dans `Cargo.toml` et `frontend/package.json`, et **ne pas recopier un numéro traîné dans un tuto**.
Résoudre via Context7 la version courante au moment du bootstrap.

- **Loco** : pré-1.0 (lignée 0.16.x), historique de breaking changes → **figé**.
- **rmcp** : **≥ 1.4.0** impératif (CVE Host-header < 1.4.0). A sauté 0.x → 1.x.
- **SeaORM** : aligné sur la version embarquée par Loco.
- **Node** : 24 (`.nvmrc` dans `frontend/`). **pnpm** : via corepack, **épinglé `pnpm@9.15.9`**
  dans `packageManager` du `package.json` (corepack sinon tire pnpm 11 dont la politique
  `minimumReleaseAge` rejette le lockfile — cf. QUIRKS).
- **shadcn/ui** : thème stone oklch via `--preset bJfDPe2y` ; initialisé avec
  `npm_config_ignore_workspace_root_check=true` (le template Vite pose un `pnpm-workspace.yaml` — cf. QUIRKS).
- **Vitest** : jsdom, globals ; **Playwright** : e2e navigateur réel.

## 3. Commandes

```bash
# Backend
cargo loco start                 # lancer l'app (depuis backend/)
cargo loco db migrate            # migrations
cargo nextest run                # tests backend (unit + intégration)
cargo clippy --all-targets -- -D warnings
cargo fmt --all

# Frontend (dans frontend/)
pnpm dev                         # dev server React/Vite (HMR)
pnpm build                       # bundle de prod → dist/ (input Docker)
pnpm test                        # Vitest (unit + composants + MSW)
pnpm lint                        # ESLint
pnpm typecheck                   # tsc --noEmit

# E2E
pnpm exec playwright test        # depuis frontend/ — contre la stack montée
# (ou: npx playwright test si pas de pnpm context)

# Supply-chain
cargo deny check                 # licences + advisories
cargo audit
```

## 4. Standards de code

- `cargo fmt` + `cargo clippy` (warnings = erreurs) verts, non négociable.
- Pas d'`unwrap`/`expect` hors tests et hors `main` d'init. Erreurs propagées.
- **Cœur** : ne dépend ni d'axum ni de loco ; rend un `CoreError` (thiserror).
  Si un `use axum::` ou `use loco_rs::` apparaît dans `src/services/`, c'est un bug
  d'architecture (le contrat est violé).
- Commits **conventionnels + gitmoji**, format `<gitmoji> <type>: <description>`
  (ex. `✨ feat:`, `🐛 fix:`, `🧱 chore:`, `📝 docs:`, `♻️ refactor:`) — le préfixe
  conventionnel alimente le CHANGELOG, le gitmoji donne le coup d'œil. **Obligatoire.**
- Dual-license **MIT / Apache-2.0** (repo publiable).

## 5. Règles de test — « lourd, léger, professionnel »

Couvert en couches. Chaque couche est un critère de sortie de phase (ROADMAP).

- **Unit (cœur, rapides, nombreux)** : génération slug + suffixe, génération/vérif du
  PIN (temps constant), logique de bascule du pointeur, validation du `deploy_token`.
- **Intégration (backend)** via les helpers de test Loco contre une **SQLite de test** :
  chaque endpoint JSON bout-en-bout — 401 sans session, `deploy` qui crée la version
  *et* flippe le pointeur dans une transaction, switch de version, gating code sur
  `/c/<slug>`. **Test-invariant de sécu** : aucune réponse ne contient de hash, et
  aucun PIN n'apparaît dans une liste (casse le build si violé — contrat §9).
- **MCP** : gate `deploy_token` testé sur *tous* les tools (lecture comprise) ;
  `deploy_prototype` crée bien une version.
- **Frontend React** : **Vitest** + Testing Library (jsdom) + **MSW** (mock des routes API).
  Tests au niveau composant (renderWithProviders/renderWithRouter). À dose mesurée :
  l'e2e porte la confiance réelle sur les flux complets.
- **E2E Playwright** : navigateur réel contre la stack montée — login, création de
  projet, deploy, bascule de version, `/c/<slug>` qui sert l'active, projet protégé
  qui affiche la page de déverrouillage + flux unlock, logout.

> Node est requis pour le **frontend et les tests** (Vite, pnpm, Vitest, Playwright).
> Le « pas de Node » ne vaut que pour le **runtime** de l'image (distroless).

## 6. CI — GitHub Actions

Jobs (séparés, cache agressif pour rester rapide : cache cargo `target` + registry,
cache pnpm/node_modules) :

1. **Backend** (`fmt-clippy` + `test-backend`) : `fmt` + `clippy --all-features` (warnings = erreurs) + `cargo nextest` (+ `cargo llvm-cov nextest --lcov` → artefact `backend-lcov`) + `cargo deny`/`audit`.
2. **Frontend** : `pnpm install --ignore-scripts` + `pnpm lint` + `pnpm typecheck` + `pnpm test:cov` (→ `coverage/lcov.info`).
3. **SonarQube** (gate bloquant) : télécharge l'artefact `backend-lcov` + exécute `sonar-scanner`. Analyse front + IaC + couverture Rust (lcov). Gate : `new_code_coverage ≥ 80%`, `new_security_rating = A`. Secret : `SONAR_TOKEN` (GitHub Actions secret). Dépend de #1 + #2.
4. **E2E** : Playwright sur la stack montée (dépend de #1 + #2).
5. Sur **tag** (ou `main`) : build de l'image multi-stage → **push GHCR**, package
   **public** du repo (`ghcr.io/owlnext-fr/latch`). Tags dérivés par
   `docker/metadata-action` (modèle *release-driven*) :
   - tag git `vX.Y.Z` → `X.Y.Z`, `X.Y`, `latest`, `sha-xxxxxxx` ;
   - push `main` → `main`, `sha-xxxxxxx` (pas `latest` : il pointe la dernière *release*).
   Le déploiement pin une version via `LATCH_IMAGE_TAG` (`docker-compose.yml`).
   Le job docker dépend de **tous** les contrôles (dont `cargo-deny` et `sonar`) : pas de publication
   d'une image qui échoue fmt/clippy/tests/supply-chain/gate Sonar.

Badge CI dans le README, dual-license, CHANGELOG en commits conventionnels.

> Toutes les actions GitHub (`uses:`) sont épinglées par **SHA** de commit (sécurité supply-chain). `concurrency: cancel-in-progress` évite les runs orphelins sur force-push.

## 7. Docker

- **Dockerfile multi-stage** (cargo-chef — couche deps cachée) :
  1. étape **Node 24** (`node:24-bookworm-slim`) : `pnpm install --ignore-scripts` + `pnpm build` → `frontend/dist/`.
  2. étape **planner** (`rust:1.96-bookworm`) : `cargo chef prepare` → `recipe.json`.
  3. étape **cook** (`rust:1.96-bookworm`) : `cargo chef cook --locked` (couche deps, invalidée seulement si `Cargo.lock` change).
  4. étape **builder** (`rust:1.96-bookworm`) : `COPY . .` + `cargo build --release --locked`.
  5. étape **dataprep** (`debian:bookworm-slim`) : `chown 65532:65532 /data` (shell requis, distroless n'en a pas).
  6. **runtime minimal** (`gcr.io/distroless/cc-debian12:nonroot`, uid 65532) : binaire + assets SPA, rien d'autre.
- **Entrypoint** : `migrate` **puis** `start` (premier boot sur volume vierge = pas
  de schéma sinon).
- **Volume `data/`** : le `.sqlite` **et** les fichiers HTML des versions ensemble.
- `docker-compose.yml` : image GHCR + volume `data/` + `.env`
  (`ADMIN_USER`, `ADMIN_PASS`, `DEPLOY_TOKEN`, et le secret HMAC du cookie unlock).
- **Caddy en façade** : TLS + reverse proxy, et pose les en-têtes
  `X-Robots-Tag: noindex, nofollow` ; sert/headerise aussi `robots.txt` (`Disallow: /`).

## 8. Déploiement — manuel, sur la box

GHCR public → pas de `docker login` requis sur la box. Un `deploy.sh` :

```bash
#!/usr/bin/env bash
set -euo pipefail
docker compose pull            # pull de l'image GHCR publique
docker compose up -d           # relance avec le .env
docker image prune -f          # nettoie les vieilles images
```

L'image ne contient **aucun secret** : tout est injecté par `.env` au runtime.

## 9. « Hide this thing »

- `robots.txt` à la racine : `Disallow: /` (crawlers honnêtes).
- En-tête `X-Robots-Tag: noindex, nofollow` posé par Caddy sur tout (plus fort).
- Le vrai gating reste l'auth : session admin, `deploy_token` MCP, code par projet.
  Un proto **sans code** reste joignable par quiconque a l'URL — compromis assumé,
  faible enjeu. Cf. le caveat d'énumération du suffixe dans QUIRKS.
- Option de durcissement non retenue en v1 : restreindre `/admin` à l'IP OWLNEXT /
  Tailscale (`/mcp` doit rester public pour le cloud Anthropic). Voir BACKLOG.
