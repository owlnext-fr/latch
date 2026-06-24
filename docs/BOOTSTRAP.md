# BOOTSTRAP — latch

> Stack, versions épinglées, outillage, structure du repo, règles de test, CI,
> Docker, déploiement. Le « comment ». Les décisions d'archi sont dans le contrat.

## 1. Stack

- **Backend** : Loco (sur axum) + SeaORM + **SQLite**.
  `libsqlite3-sys` en feature **`bundled`** → le binaire embarque SQLite, l'image
  runtime n'a aucune lib système à fournir.
- **MCP** : `rmcp` (transport `transport-streamable-http-server`), **≥ 1.4.0**.
- **Frontend** : ⚠️ **migration en cours Yew → React/Vite/shadcn-ui/Tailwind** (2026-06-25 ;
  crate Yew `frontend/` retirée, stack React à figer en session neuve — cf.
  `docs/superpowers/specs/2026-06-25-admin-react-migration-decision.md`). Reste servi en
  **statique** par Loco sous `/admin` (mécanisme inchangé). _(Ancien : crate Yew `latch-ui` +
  `shadcn-rs`, build Trunk wasm.)_
- **Cookie signé** (déverrouillage client) : `axum-extra` (`SignedCookieJar`) ou
  `cookie` — résoudre l'API exacte via Context7.
- Pas de hachage de mot de passe : le PIN est récupérable (contrat §3), l'`ADMIN_PASS`
  est comparé à temps constant depuis l'env. Aucun `argon2`/`bcrypt` requis en v1.
- **Pas de Redis, pas de worker.** La file de jobs Loco est désactivée (ou backend
  in-process). Aucun job dans le périmètre.

## 2. Versions épinglées

Épingler dans `Cargo.toml`, et **ne pas recopier un numéro traîné dans un tuto**.
Résoudre via Context7 la version courante au moment du bootstrap.

- **Loco** : pré-1.0 (lignée 0.16.x), historique de breaking changes → **figé**.
- **rmcp** : **≥ 1.4.0** impératif (CVE Host-header < 1.4.0). A sauté 0.x → 1.x.
- **Yew** : 0.21. **shadcn-rs** : 0.1 (API instable, lib jeune — cf. QUIRKS).
- **SeaORM** : aligné sur la version embarquée par Loco.

## 3. Commandes

```bash
# Backend
cargo loco start                 # lancer l'app
cargo loco db migrate            # migrations
cargo nextest run                # tests backend (unit + intégration)
cargo clippy --all-targets -- -D warnings
cargo fmt --all

# Frontend (dans frontend/)
trunk serve                      # dev server SPA
trunk build --release            # build wasm de prod (sert d'input au Docker)
wasm-pack test --headless --firefox   # ou cargo test --target wasm32 selon setup

# E2E
npx playwright test              # contre la stack montée (SPA buildée + Loco + DB de test)

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
- **Frontend Yew** : `wasm-bindgen-test` en headless, **à dose mesurée** (sur 3-4
  écrans, l'e2e porte la confiance réelle, pas le test unitaire de composant).
- **E2E Playwright** : navigateur réel contre la stack montée — login, création de
  projet, deploy, bascule de version, `/c/<slug>` qui sert l'active, projet protégé
  qui affiche la page de déverrouillage + flux unlock, logout.

> Honnêteté : Playwright tire un toolchain **Node en CI/dev**. Le « pas de Node »
> qu'on s'offre vaut pour le **runtime**, pas pour l'outillage de test.

## 6. CI — GitHub Actions

Jobs (séparés, cache agressif pour rester rapide : cache cargo `target` + registry,
cache wasm/trunk) :

1. `fmt` + `clippy` (warnings = erreurs).
2. Tests backend (`cargo nextest`).
3. `trunk build` + `wasm-bindgen-test`.
4. E2E Playwright sur la stack montée.
5. `cargo deny` / `cargo audit` (licences + advisories — c'est ce qui aurait levé le
   CVE rmcp).
6. Sur **tag** (ou `main`) : build de l'image multi-stage → **push GHCR**, package
   **public** du repo (`ghcr.io/owlnext-fr/latch`). Tags dérivés par
   `docker/metadata-action` (modèle *release-driven*) :
   - tag git `vX.Y.Z` → `X.Y.Z`, `X.Y`, `latest`, `sha-xxxxxxx` ;
   - push `main` → `main`, `sha-xxxxxxx` (pas `latest` : il pointe la dernière *release*).
   Le déploiement pin une version via `LATCH_IMAGE_TAG` (`docker-compose.yml`).
   Le job docker dépend de **tous** les contrôles (dont `cargo-deny`) : pas de publication
   d'une image qui échoue fmt/clippy/tests/supply-chain.

Badge CI dans le README, dual-license, CHANGELOG en commits conventionnels.

## 7. Docker

- **Dockerfile multi-stage** :
  1. étape **Trunk/wasm** : build de la SPA Yew (`trunk build --release`).
  2. étape **build Rust** : compile le backend (statique, SQLite `bundled`).
  3. **runtime minimal** (distroless ou alpine) : binaire + assets SPA, rien d'autre.
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
