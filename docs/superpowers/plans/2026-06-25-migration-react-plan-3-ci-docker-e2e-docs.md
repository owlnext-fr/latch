# Plan 3 — CI / Docker / e2e Playwright / Docs (migration React) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Reconnecter le pipeline (Docker stage Node, CI pistes back/front/e2e/docker), ajouter un smoke e2e Playwright qui valide l'UX admin réelle, et aligner toute la doc mémoire sur la stack React.

**Architecture:** Le frontend Yew/Trunk/wasm est remplacé partout par l'app React/Vite/pnpm (Plan 2). Le backend Rust est inchangé. La CI passe de 5 jobs mono-fichier à des pistes parallèles (backend / frontend) qui gatent un job e2e puis le build/push Docker. L'e2e monte la stack réelle (backend Loco sert le `dist/` React buildé) et la pilote avec Playwright.

**Tech Stack:** Docker multi-stage (node:24 + rust + distroless) · GitHub Actions · Playwright · pnpm.

## Global Constraints

- **Dossier front** : `frontend/` (app Node/Vite). `LATCH_SPA_DIST` inchangé (`../frontend/dist` en dev, `/app/frontend/dist` en image).
- **Node** : version pinnée `frontend/.nvmrc` (= `24`). CI : `actions/setup-node` avec `node-version-file: frontend/.nvmrc`. Docker : `node:24-bookworm-slim`.
- **Aucun nom de client réel** nulle part (placeholders fictifs).
- **Invariants §9** inchangés (sécu : pas de hash, PIN qu'au détail). Le backend ne change pas.
- **openapi.json + schema.d.ts commités** ; un drift-check CI les régénère et `git diff --exit-code`.
- **Pas de push / pas de PR** dans ce plan (la branche reste locale ; l'humain validera et poussera). Les workflows CI sont **écrits et validés syntaxiquement**, pas exécutés à distance.
- **Commits** : gitmoji + conventionnel. Un commit par task.

### Assets / faits de référence

- Recette build front : `cd frontend && pnpm install --frozen-lockfile && pnpm build` → `frontend/dist` (assets préfixés `/admin/`).
- Lancement backend (dev, sert le dist sous `/admin`, API `/api`) :
  `cd backend && LATCH_SPA_DIST=../frontend/dist ADMIN_USER=admin ADMIN_PASS=secret DATABASE_URL='sqlite://<file>?mode=rwc' cargo loco start` (LOCO_ENV par défaut = `development` → cookie session non-`Secure`, OK en http localhost). Health : `GET /_health`.
- `backend/src/web/mod.rs` : défaut `LATCH_SPA_DIST = ../frontend/dist` (inchangé — reconfirmer).
- Drift backend : `UPDATE_OPENAPI=1 cargo test --test openapi_drift` régénère `openapi.json`. Drift front : `pnpm gen:api` régénère `frontend/src/api/schema.d.ts`.

---

## File Structure

```
Dockerfile                         # stage 1 Yew/Trunk → stage Node/pnpm vite
.dockerignore                      # déjà: frontend/node_modules, frontend/dist
.env.example                       # commentaire "SPA Yew" → "SPA React"
.github/workflows/
  ci.yml                           # orchestrateur : pistes back + front (parallèle) → e2e → docker
frontend/
  playwright.config.ts             # webServer = build front + run backend ; baseURL :5150/admin
  e2e/
    admin-smoke.spec.ts            # login → créer projet → déployer → activer
  package.json                     # scripts: e2e, gen:api (déjà), + license-checker
```

---

## Task 1 : Dockerfile stage Node + alignements env/gitignore

**Files:**
- Modify: `Dockerfile` (stage 1), `.env.example` (commentaire), `.dockerignore` (vérifier)
- Verify: `backend/src/web/mod.rs` (défaut `../frontend/dist` — inchangé)

- [ ] **Step 1 : Remplacer le stage 1 (Trunk/wasm) par un stage Node/pnpm**

Remplacer le stage `frontend` du `Dockerfile` par :

```dockerfile
###############################################################################
# Stage 1 — build de la SPA React (Vite + pnpm)
###############################################################################
FROM node:24-bookworm-slim AS frontend
RUN corepack enable
WORKDIR /src/frontend
# Couche cache : deps seules (lock copié avant la source)
COPY frontend/package.json frontend/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
# Source + schéma OpenAPI commité (pour gen:api si lancé ; sinon schema.d.ts est commité)
COPY frontend/ ./
COPY openapi.json /src/openapi.json
RUN pnpm build      # vite build → /src/frontend/dist
```

Garder le stage 2 (backend Rust) **inchangé**, mais s'assurer que le stage runtime copie depuis le bon chemin : `COPY --from=frontend /src/frontend/dist /app/frontend/dist` (le `WORKDIR /src/frontend` met le dist sous `/src/frontend/dist`).

> Le stage 2 fait `COPY . .` puis `cargo build -p latch --release`. Le workspace Cargo n'inclut plus de crate frontend (membres = backend + migration), donc `COPY . .` embarque `frontend/` mais Cargo l'ignore. OK.

- [ ] **Step 2 : `.env.example` — commentaire SPA**

Remplacer le bloc `# --- SPA Yew (interface admin) ---` par `# --- SPA React (interface admin) ---` (le reste du commentaire `LATCH_SPA_DIST` est correct).

- [ ] **Step 3 : Vérifier `.dockerignore` et `web/mod.rs`**

`.dockerignore` doit contenir `frontend/node_modules` et `frontend/dist` (posés au Plan 2 T1 — vérifier). `backend/src/web/mod.rs` : confirmer que le défaut de `LATCH_SPA_DIST` est bien `../frontend/dist` (inchangé). Aucune modif si déjà correct.

- [ ] **Step 4 : Build de l'image en local**

```bash
cd /srv/owlnext/latch
docker build -t ghcr.io/owlnext-fr/latch:dev . 2>&1 | tail -30
```
Expected : build OK des 3 stages. (Long : compile Rust release.) Si le runtime démarre, smoke optionnel :
`docker run --rm -p 5151:5150 -e ADMIN_USER=admin -e ADMIN_PASS=secret -e SESSION_SECRET=$(head -c 64 /dev/urandom | base64 | head -c 64) -e DATABASE_URL='sqlite:///data/latch.sqlite?mode=rwc' -v /tmp/latchdata:/data ghcr.io/owlnext-fr/latch:dev` puis `curl -s localhost:5151/_health`.

- [ ] **Step 5 : Commit**

```bash
git add Dockerfile .env.example .dockerignore
git commit -m "🐳 chore(docker): stage build SPA Node/pnpm (Vite) en remplacement de Trunk/wasm"
```

---

## Task 2 : CI — pistes backend + frontend (parallèle) → e2e → docker + drift checks

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1 : Réécrire `ci.yml`**

Conserver les triggers (`push` main+tags, `pull_request` main). Jobs :

- **`fmt-clippy`** (backend) : inchangé.
- **`test-backend`** : `cargo nextest run` (inclut déjà `openapi_drift`). **Ajouter un step drift OpenAPI explicite** : après les tests, `UPDATE_OPENAPI=1 cargo test --test openapi_drift` puis `git diff --exit-code openapi.json` (rouge si un DTO a changé sans régénération).
- **`supply-chain`** (cargo-deny) : inchangé.
- **`frontend`** (REMPLACE le job Trunk/wasm) :
  ```yaml
  frontend:
    name: front (lint/typecheck/test/build)
    runs-on: ubuntu-latest
    defaults: { run: { working-directory: frontend } }
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version-file: frontend/.nvmrc
          cache: pnpm
          cache-dependency-path: frontend/pnpm-lock.yaml
      - uses: pnpm/action-setup@v4
      - run: pnpm install --frozen-lockfile
      - run: pnpm lint
      - run: pnpm typecheck
      - run: pnpm test
      - run: pnpm build
      # drift schema : le schema.d.ts commité doit correspondre à openapi.json
      - run: pnpm gen:api && git diff --exit-code src/api/schema.d.ts
  ```
  > Ordre des actions : `pnpm/action-setup` peut devoir précéder `setup-node` pour que `cache: pnpm` trouve pnpm. Mettre `pnpm/action-setup@v4` AVANT `setup-node`.
- **`supply-chain-front`** :
  ```yaml
  supply-chain-front:
    name: front supply-chain (audit + licences)
    runs-on: ubuntu-latest
    defaults: { run: { working-directory: frontend } }
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
      - uses: actions/setup-node@v4
        with: { node-version-file: frontend/.nvmrc, cache: pnpm, cache-dependency-path: frontend/pnpm-lock.yaml }
      - run: pnpm install --frozen-lockfile
      - run: pnpm audit --audit-level=high
      - run: pnpm dlx license-checker-rseidelsohn --production --onlyAllow 'MIT;Apache-2.0;BSD-2-Clause;BSD-3-Clause;ISC;0BSD;CC0-1.0;Unlicense;BlueOak-1.0.0;Python-2.0;CC-BY-4.0' --excludePrivatePackages
  ```
  > `pnpm audit` peut faussement rougir sur des advisories de devDeps ; `--prod`/`--audit-level=high` limite. Si une licence légitime manque à l'allowlist, l'ajouter (même esprit que `deny.toml`). Marquer ce job `continue-on-error: false` mais documenter qu'il peut nécessiter un ajustement d'allowlist au 1er run.
- **`e2e`** :
  ```yaml
  e2e:
    name: e2e Playwright (smoke admin)
    runs-on: ubuntu-latest
    needs: [fmt-clippy, test-backend, frontend]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - uses: pnpm/action-setup@v4
      - uses: actions/setup-node@v4
        with: { node-version-file: frontend/.nvmrc, cache: pnpm, cache-dependency-path: frontend/pnpm-lock.yaml }
      - run: cd frontend && pnpm install --frozen-lockfile
      - run: cd frontend && pnpm exec playwright install --with-deps chromium
      - run: cd frontend && pnpm build
      - run: cd frontend && pnpm exec playwright test
        env:
          ADMIN_USER: admin
          ADMIN_PASS: secret
  ```
  > La config Playwright (Task 3) démarre le backend elle-même via `webServer` (build déjà fait). Si plus simple en CI, séparer build et test.
- **`docker`** : `needs: [fmt-clippy, test-backend, supply-chain, frontend, supply-chain-front, e2e]` (mettre à jour la liste `needs`, retirer l'ancien job `frontend` Trunk). Le reste (metadata-action, login GHCR, build-push) **inchangé**.

> **Note d'architecture** : la spec §7 préconise des *reusable workflows* (un fichier par piste). Ici on garde un `ci.yml` à jobs parallèles (équivalent fonctionnel, plus simple). Refactor en reusable workflows = BACKLOG.

- [ ] **Step 2 : Valider la syntaxe**

```bash
# Si actionlint dispo :
command -v actionlint && actionlint .github/workflows/ci.yml || python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/ci.yml')); print('YAML ok')"
```
Expected : pas d'erreur de syntaxe. (Le run réel des Actions n'est pas exécuté localement.)

- [ ] **Step 3 : Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "👷 ci: pistes back/front parallèles + drift OpenAPI/schema + supply-chain front + e2e gate docker"
```

---

## Task 3 : Playwright e2e — harness + smoke admin (login → créer → déployer)

**Files:**
- Create: `frontend/playwright.config.ts`, `frontend/e2e/admin-smoke.spec.ts`, `frontend/e2e/fixtures/proto.html`
- Modify: `frontend/package.json` (deps + script `e2e`), `frontend/.gitignore` (playwright-report, test-results), racine `.gitignore` si besoin

**Interfaces:**
- Produces : `pnpm e2e` (= `playwright test`) qui monte la stack et valide le parcours admin.

- [ ] **Step 1 : Installer Playwright**

```bash
cd frontend
pnpm add -D @playwright/test
pnpm exec playwright install chromium
```

- [ ] **Step 2 : `playwright.config.ts` (webServer = backend servant le dist buildé)**

```ts
import { defineConfig } from '@playwright/test'

const PORT = 5150
const DB = '/tmp/latch-e2e.sqlite'

export default defineConfig({
  testDir: './e2e',
  timeout: 30_000,
  fullyParallel: false,
  workers: 1,
  retries: 0,
  use: { baseURL: `http://127.0.0.1:${PORT}/admin`, trace: 'on-first-retry' },
  webServer: {
    // Build front (au cas où) puis lance le backend qui sert dist/ sous /admin.
    // rm -f de la DB e2e → base fraîche migrée au boot (auto_migrate).
    command: `pnpm build && rm -f ${DB} && cd ../backend && LATCH_SPA_DIST=../frontend/dist ADMIN_USER=admin ADMIN_PASS=secret LATCH_STORAGE_ROOT=/tmp/latch-e2e-data DATABASE_URL='sqlite://${DB}?mode=rwc' cargo loco start`,
    url: `http://127.0.0.1:${PORT}/_health`,
    timeout: 180_000,
    reuseExistingServer: !process.env.CI,
    stdout: 'pipe',
    stderr: 'pipe',
  },
})
```

> LOCO_ENV non défini → `development` → cookie session non-`Secure` (OK en http). `auto_migrate` crée le schéma sur la DB e2e vierge. Le 1er `cargo loco start` compile le backend (long) ; `webServer.timeout` = 180 s.

- [ ] **Step 3 : Fixture HTML**

`frontend/e2e/fixtures/proto.html` : un mono-fichier trivial `<!doctype html><html><body><h1>Demo proto</h1></body></html>` (placeholder fictif).

- [ ] **Step 4 : Smoke spec `e2e/admin-smoke.spec.ts`**

Couvre le parcours (sélecteurs par rôle/texte i18n — l'UI démarre en EN par défaut) :
1. **login** : aller sur `/admin/login` (baseURL), remplir Username `admin` / Password `secret`, cliquer `Sign in` → attendre la liste (intro `list.intro` ou le bouton `+ New project`).
2. **créer un projet** : cliquer `+ New project` → le side-panel s'ouvre → remplir Name `Mon Projet` → (code activé par défaut, PIN auto) → Save → attendre le toast `Project created.` et la ligne `Mon Projet` dans la table.
3. **détail + déployer** : cliquer la ligne `Mon Projet` → page détail → cliquer `Deploy` → dans le DeployPanel, uploader `e2e/fixtures/proto.html` (via `setInputFiles` sur l'input file caché), cocher `Activate immediately`, cliquer `Deploy` → attendre le toast `Version deployed.` et une ligne de version dans la table Versions.

Garder le spec robuste : `await expect(...).toBeVisible()` avec timeouts par défaut, pas de `waitForTimeout` arbitraire. Pour l'input file caché : `page.setInputFiles('input[type=file]', path.resolve(__dirname, 'fixtures/proto.html'))`.

Script `package.json` : `"e2e": "playwright test"`.

- [ ] **Step 5 : `.gitignore`**

Ajouter à `frontend/.gitignore` : `playwright-report/`, `test-results/`, `e2e/.auth/`.

- [ ] **Step 6 : Lancer l'e2e en local (VALIDATION UX)**

```bash
cd frontend && pnpm e2e 2>&1 | tail -30
```
Expected : le(s) test(s) passent (login → créer → déployer). Si un sélecteur ne matche pas, l'ajuster au texte i18n EN réel (`src/i18n/locales/en.json`). **C'est la validation UX demandée.**

- [ ] **Step 7 : Commit**

```bash
cd /srv/owlnext/latch
git add frontend/playwright.config.ts frontend/e2e frontend/package.json frontend/pnpm-lock.yaml frontend/.gitignore
git commit -m "✅ test(e2e): smoke Playwright admin (login → créer → déployer) + harness stack réelle"
```

---

## Task 4 : Alignement de la doc mémoire sur la stack React

**Files (modifier):** `CLAUDE.md`, `docs/contrat-deploy.md`, `docs/BOOTSTRAP.md`, `docs/ROADMAP.md`, `docs/ENVIRONMENT.md`, `docs/QUIRKS.md`, `docs/CONVENTIONS.md`, `docs/INDEX.md`, `docs/BACKLOG.md`, `README.md`

> Stratégie (spec §11) : **archiver, pas supprimer** les patterns/quirks Yew (section « Historique Yew — obsolète depuis migration React »). Le backend reste intact. **Aucun nom de client réel.**

- [ ] **Step 1 : `CLAUDE.md`** — en-tête : retirer `latch-ui`/Yew → « app React `frontend/` (Vite) ». Tableau Context7 : remplacer `yew`/`shadcn-rs` par `@tanstack/react-router`+`@tanstack/react-query`, `shadcn/ui` (Radix), `react-hook-form`/`zod`, `react-i18next`, `openapi-typescript`/`openapi-fetch` ; garder `loco-rs`/`sea-orm`/`rmcp`/`axum-extra`.

- [ ] **Step 2 : `docs/contrat-deploy.md`** — §2 structure : `frontend/` = app React (Vite), `backend/src/dto/` (plus de `latch-dto`). §4 rendu : SPA React/Vite servie statique sous `/admin`, **contrat de fil = OpenAPI généré** (`schema.d.ts` via openapi-typescript + `openapi-fetch`, `credentials: 'include'`) ; retirer les mentions Yew/Trunk/basename/`latch-dto`/DTO-partagé. **§9 invariants inchangés.** Remplacer le bandeau « migration en cours » par « migré ».

- [ ] **Step 3 : `docs/BOOTSTRAP.md`** — §1 stack (front React/Vite/pnpm/shadcn/Tailwind), §2 versions (Node 24, retrait yew/shadcn-rs), §3 commandes (`pnpm dev/build/test/lint/typecheck`, `playwright test` ; retrait Trunk/wasm-pack), §5 tests (Vitest+Testing Library / MSW / Playwright), §6 CI (pistes back/front→e2e→docker), §7 Docker (stage Node).

- [ ] **Step 4 : `docs/ROADMAP.md`** — Phase 3 : noter la migration React **livrée** (Plans 1-3) + critères de sortie atteints. Garder Phases 4-6.

- [ ] **Step 5 : `docs/ENVIRONMENT.md`** — Toolchain : retirer Trunk/wasm32, ajouter Node 24 + pnpm (corepack) + Playwright. Commandes front (`cd frontend && pnpm dev/build`). `LATCH_SPA_DIST` inchangé. DB e2e (`/tmp/latch-e2e.sqlite`) si utile.

- [ ] **Step 6 : `docs/QUIRKS.md`** — Encadrer les quirks Yew/shadcn-rs/yew-router/gloo-net/Trunk sous une section « ## Historique Yew — obsolète depuis migration React (2026-06-25) ». **Conserver intacts** les quirks backend (Loco/sea-orm/nextest/cargo-deny/axum-session/utoipa…). Ajouter les quirks React découverts : openapi-fetch capture `globalThis.fetch` au load → wrapper pour MSW (cf. `client.ts`) ; ResizeObserver polyfill pour Radix en jsdom ; `shadcn init --preset` nécessite `npm_config_ignore_workspace_root_check=true` (template pose un `pnpm-workspace.yaml`) ; thème via `--preset bJfDPe2y` (base stone oklch).

- [ ] **Step 7 : `docs/CONVENTIONS.md`** — Encadrer les patterns Yew sous « Historique Yew — obsolète ». Conserver les patterns backend. Ajouter les patterns React : hook Query par endpoint (`use-projects`, invalidation + toast), side-panel `<Sheet>` Radix, form RHF+zod, client `openapi-fetch` typé, test MSW avec `renderWithProviders`/`renderWithRouter`.

- [ ] **Step 8 : `docs/INDEX.md`** — marquer « Frontend (SPA Yew) » comme **superseded** (migré). Nouvelle section « Frontend (SPA React) » listant les livrables Plan 2 (scaffold, client typé, shell, harness test, login, liste, ProjectForm, détail/deploy/danger) + Plan 3 (Docker Node, CI pistes, e2e Playwright). Garder le backend.

- [ ] **Step 9 : `docs/BACKLOG.md`** — annoter résolus par React (toast globale → sonner ; polish login/activate_version/dropzone-flicker = Yew, clos). Ajouter les nouveaux items (cf. ledger Plan 2) : enrichir `ProjectListItem` (active_version_n + version_count) ; bouton activer pending state ; code-splitting bundle 604 kB ; reusable workflows CI ; `deny.toml` transitives utoipa-swagger-ui (zlib-rs « Zlib »). Conserver `PUBLIC_BASE_URL`/slug éditable.

- [ ] **Step 10 : `README.md`** — Stack (Yew/Trunk → React/Vite/pnpm/shadcn) ; commandes dev (`cd frontend && pnpm dev` / `pnpm build`) ; section qualité (pnpm lint/typecheck/test, playwright). Garder badge CI, licence, archi.

- [ ] **Step 11 : Commit**

```bash
git add CLAUDE.md docs README.md
git commit -m "📝 docs: alignement mémoire sur la stack React (contrat §2/§4, BOOTSTRAP, ROADMAP, ENV, QUIRKS, CONVENTIONS, INDEX, BACKLOG, README)"
```

---

## Task 5 : HANDOFF + lancement serveur pour validation humaine

- [ ] **Step 1 : `docs/HANDOFF.md`** — entrée datée 2026-06-25 en haut : migration React livrée (Plans 1-3), état (tests front 25/25, backend 88, e2e smoke vert, docker build OK ou noté), décisions/quirks clés, points à trancher (enrichir ProjectListItem DTO), prochaine étape (Phase 4 `/c/<slug>`). **Ne pas dupliquer** l'entrée T6 déjà posée par le commit hors-scope 5f1ba97 — consolider.

- [ ] **Step 2 : Commit**

```bash
git add docs/HANDOFF.md && git commit -m "📝 docs: HANDOFF — migration React livrée (Plans 1-3), prêt validation"
```

- [ ] **Step 3 : Lancer le serveur pour la validation du matin**

Build front + lancer le backend en arrière-plan (laissé tournant pour l'humain) :
```bash
cd /srv/owlnext/latch/frontend && pnpm build
cd /srv/owlnext/latch/backend && LATCH_SPA_DIST=../frontend/dist ADMIN_USER=admin ADMIN_PASS=secret LATCH_STORAGE_ROOT=/tmp/latch-dev-data DATABASE_URL='sqlite:///tmp/latch-dev.sqlite?mode=rwc' cargo loco start
```
Indiquer à l'humain : URL `http://127.0.0.1:5150/admin`, identifiants `admin` / `secret`.

---

## Critères de sortie (Plan 3)

- `Dockerfile` build les 3 stages (stage Node) → image OK (ou build noté si non lancé).
- `ci.yml` : pistes back + front parallèles, drift OpenAPI + schema, supply-chain front, e2e gate docker ; YAML valide.
- `pnpm e2e` (Playwright) vert : login → créer → déployer contre la stack réelle. **UX validée.**
- Doc mémoire alignée (table spec §11) ; HANDOFF + INDEX à jour.
- Invariants §9 préservés ; aucun nom de client réel.
- Serveur lancé et joignable sur `/admin` pour la validation humaine.

## Self-review (anti-placeholder)

Paths réels vérifiés (Dockerfile stage names, ci.yml jobs, `LATCH_SPA_DIST`, endpoints `/api/login` etc.). Le harness e2e démarre le backend qui sert le `dist/` React (même mécanisme que la prod). Les sélecteurs Playwright s'appuient sur le catalogue i18n EN réel (`en.json`). La doc archive le Yew sans le supprimer (spec §11). Aucun secret commité (env au runtime).
