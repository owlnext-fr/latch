# Durcissement toolchain & CI — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Brancher SonarQube Cloud (bloquant) dans la CI, accélérer le build Docker via cargo-chef en runtime non-root, durcir les lints Rust, et résorber le backlog Sonar mesuré (64 issues) — éditeur ↔ toolchain réconciliés.

**Architecture:** Trois axes sur le pipeline existant. (1) Remédiation : on corrige les 42 *code smells* front et les 22 *vulnerabilities* CI/Docker. (2) Build : le stage Rust passe en cargo-chef (couche deps cachée) + runtime distroless non-root. (3) Garde-fou : nouveau job CI `sonar` avec Quality Gate bloquant (`qualitygate.wait=true`), ajouté en dépendance du job `docker`. Les corrections vuln tombent en grande partie dans la réécriture `ci.yml`/`Dockerfile` qu'on fait de toute façon.

**Tech Stack:** SonarQube Cloud + `SonarSource/sonarqube-scan-action`, Vitest + `@vitest/coverage-v8` (lcov), Docker BuildKit + cargo-chef, distroless `cc-debian12:nonroot`, clippy `[workspace.lints]`, GitHub Actions.

## Global Constraints

- **Branche de travail** : `chore/toolchain-ci-hardening` (déjà créée, spec committée dessus).
- **Confidentialité** : aucun nom de client réel nulle part (placeholders génériques uniquement).
- **Commits** : conventionnels + gitmoji, format `<gitmoji> <type>: <description>`. Terminer chaque message par les deux lignes de session (cf. commits existants `2b039e2`/`e319543`).
- **Sonar** : `sonar.organization=owlnext-fr`, `sonar.projectKey=owlnext-fr_latch`. Token = secret GitHub `SONAR_TOKEN`. Automatic Analysis désactivée (analyse CI exclusive).
- **pnpm** épinglé `9.15.9` (`packageManager` de `frontend/package.json`).
- **Rust** : pas d'`unwrap`/`expect` hors tests et hors init de boot (devient mécanique en Task 7).
- **Vérif locale Sonar** : `set -a; . ./.env.local; set +a` charge `SONAR_TOKEN` (fichier gitignoré) ; scan via `docker run --rm -e SONAR_TOKEN -e SONAR_HOST_URL=https://sonarcloud.io -v "$PWD:/usr/src" sonarsource/sonar-scanner-cli -D…`.
- **Le job `sonar` n'est ajouté aux `needs` de `docker` qu'en Task 8**, après vérification locale que le gate est VERT — pour ne jamais casser `main` pendant la résorption.
- **eslint** : config `tseslint.configs.recommended` (NON type-checked) → `no-floating-promises` inactif → retirer un `void` ne déclenche aucun lint.

**Référence backlog** (pré-scan, commit `2b039e2`) : 0 bug, 0 hotspot, fiabilité+maintenabilité A. Gate ERROR sur `new_security_rating=C` (porté par les 22 vulns CI/Docker). Détail par règle dans la spec `docs/superpowers/specs/2026-06-25-toolchain-ci-hardening-design.md` §2.

---

### Task 1 : Frontend — supprimer l'opérateur `void` (S3735 ×21)

**Files:**
- Modify: `frontend/src/hooks/use-projects.ts` (lignes 49,75,76,95,96,127,128,147,148,174,175,195,196,216,217)
- Modify: `frontend/src/components/topbar.tsx` (15,26)
- Modify: `frontend/src/components/delete-project-panel.tsx` (35)
- Modify: `frontend/src/routes/detail.tsx` (62)
- Modify: `frontend/src/routes/list.tsx` (65)
- Modify: `frontend/src/routes/login.tsx` (37)
- Modify: `frontend/src/components/deploy-panel.tsx` (100)

**Interfaces:**
- Consumes: rien.
- Produces: rien (refactor interne, comportement identique).

**Deux transformations selon le contexte :**

- **Contexte instruction** (`void expr` comme statement, ex. dans un `onSuccess`) → retirer `void ` :
  ```ts
  // AVANT (use-projects.ts:49)
  void qc.invalidateQueries({ queryKey: ['projects'] })
  // APRÈS
  qc.invalidateQueries({ queryKey: ['projects'] })
  ```
- **Contexte arrow-expression** (`() => void expr`) → passer en corps de bloc (jette le retour, garde le type `void`) :
  ```tsx
  // AVANT (deploy-panel.tsx:100)
  onSubmit={(e) => void handleSubmit(e)}
  // APRÈS
  onSubmit={(e) => { handleSubmit(e) }}

  // AVANT (list.tsx:65)
  onClick={() =>
    void router.navigate({ to: '/projects/$id', params: { id: String(project.id) } })
  }
  // APRÈS
  onClick={() => {
    router.navigate({ to: '/projects/$id', params: { id: String(project.id) } })
  }}
  ```

- [ ] **Step 1: Appliquer les deux transformations à tous les sites listés**
  Ouvrir chaque fichier, traiter chaque ligne flaggée selon son contexte (instruction → retrait `void ` ; arrow → corps de bloc). Dans `use-projects.ts`, tous les sites sont des `void qc.invalidateQueries(...)` en contexte instruction.

- [ ] **Step 2: Vérifier qu'il ne reste aucun `void` opérateur**
  Run: `cd frontend && grep -rn "void " src/ | grep -v "): void\| void {" || echo "OK aucun void operator"`
  Expected: `OK aucun void operator` (les `: void` de types ne sont pas concernés).

- [ ] **Step 3: lint + typecheck + tests**
  Run: `cd frontend && pnpm lint && pnpm typecheck && pnpm test`
  Expected: tout vert (les 30 tests Vitest passent — le comportement est inchangé).

- [ ] **Step 4: Commit**
  ```bash
  rtk git add frontend/src
  rtk git commit -m "$(cat <<'EOF'
  ♻️ refactor(front): retire l'opérateur void (Sonar S3735 ×21)

  no-floating-promises inactif (eslint recommended non type-checked) → retrait sans risque.
  Contexte arrow → corps de bloc pour garder le type void.

  Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01PztH7Hqw6QfatfKG9MpRDV
  EOF
  )"
  ```

---

### Task 2 : Frontend — props read-only (S6759 ×8) + `globalThis` (S7764 ×4)

**Files:**
- Modify (readonly props) : `frontend/src/components/delete-project-panel.tsx:22`, `frontend/src/components/delete-version-panel.tsx:22`, `frontend/src/components/deploy-panel.tsx:25` & `:174`, `frontend/src/components/project-form.tsx:58`, `frontend/src/components/copy-button.tsx:11`, `frontend/src/components/pin-field.tsx:14`, `frontend/src/test/utils.tsx:23`
- Modify (globalThis) : `frontend/src/unlock/reload.ts:1`, `frontend/src/unlock/unlock-page.tsx:12`, `frontend/src/api/client.ts:21`, `frontend/src/lib/utils.ts:16`

**Interfaces:**
- Consumes: rien. Produces: rien (signatures de props inchangées côté appelant — `Readonly<T>` est transparent).

**Transformation A — props read-only** : envelopper le type des props dans `Readonly<…>`.
```tsx
// AVANT (deploy-panel.tsx:25) — type inline destructuré
function DeployPanelContent({
  projectId,
  onOpenChange,
}: {
  projectId: number
  onOpenChange: (open: boolean) => void
}) {
// APRÈS
function DeployPanelContent({
  projectId,
  onOpenChange,
}: Readonly<{
  projectId: number
  onOpenChange: (open: boolean) => void
}>) {

// AVANT (deploy-panel.tsx:174) — type nommé
export function DeployPanel({ projectId, open, onOpenChange }: DeployPanelProps) {
// APRÈS
export function DeployPanel({ projectId, open, onOpenChange }: Readonly<DeployPanelProps>) {
```
Appliquer le même pattern aux 8 sites (selon que le type est inline ou nommé).

**Transformation B — globalThis** : remplacer `window` par `globalThis` (les globals browser sont déjà configurés dans eslint ; `globalThis.location`/`globalThis.fetch` existent en navigateur).
```ts
// AVANT (api/client.ts:21)
fetch: (input) => window.fetch(input),
// APRÈS
fetch: (input) => globalThis.fetch(input),
```
⚠️ Conserver le wrapper `(input) => globalThis.fetch(input)` (load-bearing pour MSW, cf. QUIRKS) — on remplace seulement `window` → `globalThis`.

- [ ] **Step 1: Appliquer Transformation A aux 8 sites de props**

- [ ] **Step 2: Appliquer Transformation B aux 4 sites `window`**
  Vérifier ensuite : `cd frontend && grep -rn "window\." src/ || echo "OK aucun window restant"`
  Expected: `OK aucun window restant` (sauf usages légitimes éventuels — il ne devrait en rester aucun).

- [ ] **Step 3: lint + typecheck + tests**
  Run: `cd frontend && pnpm lint && pnpm typecheck && pnpm test`
  Expected: tout vert (le wrapper MSW garde le même comportement → les tests passent).

- [ ] **Step 4: Commit**
  ```bash
  rtk git add frontend/src
  rtk git commit -m "$(cat <<'EOF'
  ♻️ refactor(front): props Readonly (S6759) + globalThis vs window (S7764)

  Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01PztH7Hqw6QfatfKG9MpRDV
  EOF
  )"
  ```

---

### Task 3 : Frontend — ternaires imbriqués (S3358 ×4) + 4 singletons

**Files:**
- Modify: `frontend/src/components/deploy-panel.tsx` (ternaires :94 & :129 ; FormEvent :70)
- Modify: `frontend/src/routes/list.tsx` (ternaire :40 ; condition négative :99)
- Modify: `frontend/src/routes/detail.tsx` (ternaire :70)
- Modify: `frontend/src/components/locale-switcher.tsx` (rôle ARIA :12)
- Modify: `frontend/src/unlock/unlock-page.test.tsx` (assertion :71)

**Interfaces:** Consumes/Produces: rien.

**S3358 — extraire le ternaire imbriqué.** Exemple réel (deploy-panel.tsx:92-96), le `dropzoneText` :
```tsx
// AVANT
const dropzoneText = file
  ? t('deploy.file_chosen', { name: file.name, size: humanSize(file.size) })
  : isDragOver
    ? t('deploy.dropzone_hover')
    : t('deploy.dropzone_idle')
// APRÈS — fonction d'aide locale, plus de ternaire imbriqué
function computeDropzoneText() {
  if (file) return t('deploy.file_chosen', { name: file.name, size: humanSize(file.size) })
  if (isDragOver) return t('deploy.dropzone_hover')
  return t('deploy.dropzone_idle')
}
const dropzoneText = computeDropzoneText()
```
Pour le `className` imbriqué (deploy-panel.tsx:127-131), extraire dans une variable calculée avant le JSX :
```tsx
// AVANT (inline dans le tableau de classes)
isDragOver ? 'border-primary …' : file ? 'border-green-500 …' : 'border-input …'
// APRÈS — au-dessus du return
let dropzoneBorder: string
if (isDragOver) dropzoneBorder = 'border-primary bg-primary/5 text-primary'
else if (file) dropzoneBorder = 'border-green-500 bg-green-50 text-green-700'
else dropzoneBorder = 'border-input text-muted-foreground hover:border-primary/50 hover:text-foreground'
// …puis utiliser `dropzoneBorder` dans le tableau .join(' ')
```
Pour `list.tsx:40` (le triple `isLoading ? … : !projects||len===0 ? … : (table)`) et `detail.tsx:70` : lire le site, extraire l'état en variable (ex. `let content: ReactNode` rempli par if/else if/else) rendue ensuite, OU des early-returns. Aucune logique ne change.

**S7735 (list.tsx:99) — condition négative avec else** : inverser.
```tsx
// AVANT
{project.active_version_n != null ? (
  <span …>{`v${project.active_version_n}`}…</span>
) : (
  <span className="text-muted-foreground">{t('common.dash')}</span>
)}
// APRÈS — condition positive, branches échangées
{project.active_version_n == null ? (
  <span className="text-muted-foreground">{t('common.dash')}</span>
) : (
  <span …>{`v${project.active_version_n}`}…</span>
)}
```
> Note : si après extraction du ternaire `list.tsx:40` la ligne :99 se décale, re-cibler par le message Sonar (« Unexpected negated condition »), pas par le numéro de ligne.

**S1874 (deploy-panel.tsx:70) — `FormEvent` déprécié** : typer explicitement l'événement de formulaire React.
```tsx
// AVANT
async function handleSubmit(e: React.FormEvent) {
// APRÈS
async function handleSubmit(e: React.FormEvent<HTMLFormElement>) {
```
Si Sonar persiste, vérifier que `e` ne référence pas le type DOM global `FormEvent` (lib.dom déprécié) — forcer le namespace `React.`.

**S5906 (unlock-page.test.tsx:71) — assertion générique** : appliquer la suggestion Sonar.
```ts
// AVANT (assertion générique sur la longueur, ex.)
expect(filled.length).toBe(0)
// APRÈS
expect(filled).toHaveLength(0)
```
Lire la ligne exacte pour adapter le nom de la variable.

**S6819 (locale-switcher.tsx:12) — `role="group"`** : remplacer le `role="group"` par un élément sémantique. Lire le composant (groupe de boutons FR/EN) puis remplacer le conteneur portant `role="group"` par un `<fieldset>` (sans bordure via classes) ou retirer le `role` si la sémantique est déjà portée par un élément approprié. Conserver l'accessibilité (label du groupe).

- [ ] **Step 1: Extraire les 4 ternaires imbriqués** (deploy-panel ×2, list, detail) selon les patterns ci-dessus.

- [ ] **Step 2: Corriger les 4 singletons** (FormEvent, condition négative, assertion test, rôle ARIA).

- [ ] **Step 3: lint + typecheck + tests**
  Run: `cd frontend && pnpm lint && pnpm typecheck && pnpm test`
  Expected: tout vert (comportement inchangé ; le test unlock-page modifié reste vert).

- [ ] **Step 4: Commit**
  ```bash
  rtk git add frontend/src
  rtk git commit -m "$(cat <<'EOF'
  ♻️ refactor(front): dé-imbrication ternaires (S3358) + singletons Sonar

  FormEvent typé (S1874), condition positive (S7735), assertion spécifique (S5906),
  rôle ARIA sémantique (S6819).

  Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01PztH7Hqw6QfatfKG9MpRDV
  EOF
  )"
  ```

---

### Task 4 : Frontend — couverture Vitest en lcov

**Files:**
- Modify: `frontend/package.json` (devDependencies + script)
- Modify: `frontend/vitest.config.ts` (bloc `coverage`) — si la config Vitest est dans `vite.config.ts`, l'éditer là.

**Interfaces:**
- Produces: artefact `frontend/coverage/lcov.info` consommé par Sonar (Task 8) via `sonar.javascript.lcov.reportPaths`.

- [ ] **Step 1: Ajouter le provider de couverture**
  Run: `cd frontend && pnpm add -D @vitest/coverage-v8@^4`
  Expected: ajouté en devDependencies, lockfile mis à jour. (Version alignée sur `vitest ^4`.)

- [ ] **Step 2: Configurer le reporter lcov**
  Localiser le bloc `test:` de la config Vitest (`vitest.config.ts` ou `vite.config.ts`). Ajouter :
  ```ts
  test: {
    // … config existante (environment jsdom, setupFiles, include, etc.) …
    coverage: {
      provider: 'v8',
      reporter: ['text', 'lcov'],
      reportsDirectory: './coverage',
      include: ['src/**'],
      exclude: ['src/api/schema.d.ts', '**/*.test.{ts,tsx}', 'src/test/**'],
    },
  }
  ```

- [ ] **Step 3: Ajouter un script `test:cov`**
  Dans `frontend/package.json`, section `scripts`, ajouter :
  ```json
  "test:cov": "vitest run --coverage",
  ```

- [ ] **Step 4: Générer la couverture et vérifier le lcov**
  Run: `cd frontend && pnpm test:cov && test -f coverage/lcov.info && echo "LCOV OK"`
  Expected: tests verts + `LCOV OK`.

- [ ] **Step 5: Ignorer le dossier coverage dans git**
  Vérifier que `coverage` est ignoré : `cd frontend && git check-ignore coverage || echo "AJOUTER coverage au .gitignore"`. Si à ajouter, l'ajouter dans `frontend/.gitignore` (ou racine).

- [ ] **Step 6: Commit**
  ```bash
  rtk git add frontend/package.json frontend/pnpm-lock.yaml frontend/vitest.config.ts frontend/.gitignore
  rtk git commit -m "$(cat <<'EOF'
  ✨ test(front): couverture Vitest en lcov (pour Sonar)

  @vitest/coverage-v8 + reporter lcov → coverage/lcov.info. Script test:cov.

  Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01PztH7Hqw6QfatfKG9MpRDV
  EOF
  )"
  ```

---

### Task 5 : Docker — cargo-chef + runtime non-root + durcissements (S8549, S6471, S6596, S6505)

**Files:**
- Modify: `Dockerfile` (réécriture stages 1-3)
- Modify: `docker-compose.yml` (note ownership volume) — si nécessaire.

**Interfaces:**
- Produces: image runtime `latch-cli` tournant en uid 65532, écrivant dans `/data`.

**Dockerfile cible complet** (remplace l'intégralité du fichier) :
```dockerfile
# syntax=docker/dockerfile:1

###############################################################################
# Stage 1 — build de la SPA React (Vite + pnpm)
###############################################################################
FROM node:24-bookworm-slim AS frontend
RUN corepack enable
WORKDIR /src/frontend
COPY frontend/package.json frontend/pnpm-lock.yaml ./
# --ignore-scripts : pas de lifecycle scripts à l'install (S6505). Le build SPA
# n'a besoin d'aucun postinstall (esbuild/rollup via optionalDependencies).
RUN pnpm install --frozen-lockfile --ignore-scripts
COPY frontend/ ./
COPY openapi.json /src/openapi.json
RUN pnpm build

###############################################################################
# Stage 2 — build du backend via cargo-chef (couche deps cachée)
###############################################################################
FROM rust:1.90-bookworm AS chef
RUN cargo install cargo-chef --locked
WORKDIR /src

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /src/recipe.json recipe.json
# Build des deps seules → couche cachée tant que Cargo.toml/lock ne changent pas.
RUN cargo chef cook --release -p latch --locked --recipe-path recipe.json
COPY . .
# --locked : respecte Cargo.lock, pas de résolution flottante (S8549).
RUN cargo build -p latch --release --locked

###############################################################################
# Stage 2.5 — préparer /data possédé par nonroot (distroless n'a pas de shell)
###############################################################################
FROM debian:bookworm-slim AS dataprep
RUN mkdir -p /data && chown 65532:65532 /data

###############################################################################
# Stage 3 — runtime minimal NON-ROOT (distroless, tag figé :nonroot)
###############################################################################
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime
WORKDIR /app
COPY --from=builder  /src/target/release/latch-cli  /app/latch-cli
COPY --from=builder  /src/backend/config            /app/config
COPY --from=frontend /src/frontend/dist             /app/frontend/dist
# /data possédé par nonroot (65532) → volume inscriptible au premier boot.
COPY --from=dataprep --chown=65532:65532 /data /data
ENV LOCO_ENV=production
ENV LATCH_SPA_DIST=/app/frontend/dist
EXPOSE 5150
USER nonroot
ENTRYPOINT ["/app/latch-cli"]
CMD ["start"]
```

> Notes :
> - `rust:1.90-bookworm` : tag mineur figé (S6596). **Vérifier la version exacte** de Rust à épingler (`docker run --rm rust:1-bookworm rustc --version` ou la dernière 1.x stable) et ajuster si besoin.
> - `gcr.io/distroless/cc-debian12:nonroot` (uid/gid 65532). Corrige S6471.
> - Le volume `/data` (compose) hérite de l'ownership `65532` à la **première** création. ⚠️ Un volume **préexistant** possédé par root devra être `chown 65532:65532` une fois (à noter en QUIRKS).

- [ ] **Step 1: Réécrire le Dockerfile** avec le contenu cible ci-dessus.

- [ ] **Step 2: Build local de l'image**
  Run: `cd /srv/owlnext/latch && DOCKER_BUILDKIT=1 docker build -t ghcr.io/owlnext-fr/latch:dev .`
  Expected: build réussi (les 3+ stages passent ; le stage `cook` peut être long au 1ᵉʳ build).

- [ ] **Step 3: Vérifier le runtime non-root + boot + écriture /data**
  ```bash
  docker rm -f latch-test 2>/dev/null || true
  docker run -d --name latch-test -e ADMIN_USER=admin -e ADMIN_PASS=secret \
    -e SESSION_SECRET="$(openssl rand -hex 40)" -e UNLOCK_COOKIE_SECRET="$(openssl rand -hex 40)" \
    -e DEPLOY_TOKEN=tok -v latch-test-data:/data ghcr.io/owlnext-fr/latch:dev
  sleep 4
  docker exec latch-test /app/latch-cli --version 2>/dev/null || true   # distroless: pas de shell, on lit les logs
  docker logs latch-test 2>&1 | tail -20      # migrations jouées, "listening on ..."
  docker inspect -f '{{.Config.User}}' latch-test                       # → nonroot ou 65532
  ```
  Expected: logs montrent migrations OK + serveur en écoute ; `User` = `nonroot`. (Le `.sqlite` créé sous `/data` prouve l'écriture non-root.)
  Cleanup: `docker rm -f latch-test && docker volume rm latch-test-data`

- [ ] **Step 4: Mesure du gain cargo-chef (code seul)**
  Toucher un fichier source backend sans changer les deps, rebuild, et constater que le stage `cook` est `CACHED` :
  ```bash
  touch backend/src/app.rs
  DOCKER_BUILDKIT=1 docker build -t ghcr.io/owlnext-fr/latch:dev . 2>&1 | grep -E "cook|CACHED" | tail -5
  ```
  Expected: la ligne `cargo chef cook` apparaît `CACHED` → seule la couche `cargo build` finale recompile.

- [ ] **Step 5: Commit**
  ```bash
  rtk git add Dockerfile docker-compose.yml
  rtk git commit -m "$(cat <<'EOF'
  ⚡ perf(docker): cargo-chef + runtime non-root + durcissements Sonar

  Stage Rust en cargo-chef (couche deps cachée, restaurée par cache gha).
  Runtime distroless cc-debian12:nonroot (uid 65532, /data possédé nonroot).
  Corrige docker S8549 (--locked), S6471 (non-root), S6596 (tag figé), S6505 (--ignore-scripts).

  Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01PztH7Hqw6QfatfKG9MpRDV
  EOF
  )"
  ```

---

### Task 6 : CI — confort + durcissements `ci.yml` (S6505 ×4, S7637 ×15)

**Files:**
- Modify: `.github/workflows/ci.yml`

**Interfaces:** Produces: pipeline durci, `clippy --all-features`, navigateurs Playwright cachés.

- [ ] **Step 1: Bloc `concurrency` en tête du workflow**
  Sous le bloc `env:`, ajouter :
  ```yaml
  concurrency:
    group: ci-${{ github.ref }}
    cancel-in-progress: true
  ```

- [ ] **Step 2: `--ignore-scripts` sur tous les `pnpm install` (S6505 ×4)**
  Remplacer **chaque** `pnpm install --frozen-lockfile` existant par `pnpm install --frozen-lockfile --ignore-scripts` (jobs `frontend`, `supply-chain-front`, `e2e`). Le futur job `sonar` (Task 8) intègre déjà `--ignore-scripts`. Le job e2e garde son `pnpm exec playwright install --with-deps chromium` séparé (le download navigateur n'est plus fait au postinstall, c'est volontaire).

- [ ] **Step 3: `--all-features` sur le clippy CI**
  Dans le job `fmt-clippy`, remplacer :
  ```yaml
  - run: cargo clippy --all-targets -- -D warnings
  ```
  par :
  ```yaml
  - run: cargo clippy --all-targets --all-features -- -D warnings
  ```

- [ ] **Step 4: Cache des navigateurs Playwright (job `e2e`)**
  Avant l'étape `playwright install`, ajouter :
  ```yaml
  - name: Cache Playwright browsers
    uses: actions/cache@v4
    with:
      path: ~/.cache/ms-playwright
      key: playwright-${{ runner.os }}-${{ hashFiles('frontend/pnpm-lock.yaml') }}
  ```

- [ ] **Step 5: Pin toutes les actions à un SHA de commit (S7637 ×15)**
  Pour **chaque** `uses: owner/repo@ref` du fichier, résoudre le SHA du commit pointé par `ref` et l'épingler, en gardant la version en commentaire. Méthode :
  ```bash
  # Exemple pour actions/checkout@v4 :
  gh api repos/actions/checkout/commits/v4 --jq '.sha'
  # → remplacer:  uses: actions/checkout@v4
  #    par:        uses: actions/checkout@<sha>  # v4
  ```
  Actions à épingler (toutes occurrences) : `actions/checkout@v4`, `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2`, `taiki-e/install-action@nextest`, `pnpm/action-setup@v4`, `actions/setup-node@v4`, `EmbarkStudios/cargo-deny-action@v2`, `docker/setup-buildx-action@v3`, `docker/metadata-action@v5`, `docker/login-action@v3`, `docker/build-push-action@v6`, plus `actions/cache@v4` (Step 4) et `SonarSource/sonarqube-scan-action` (Task 8).
  > `dtolnay/rust-toolchain@stable` et `taiki-e/install-action@nextest` sont des refs nommées (pas des tags) : épingler le SHA courant de la branche, commentaire `# stable`/`# nextest`. L'action installe toujours le bon toolchain.

- [ ] **Step 6: Valider la syntaxe du workflow**
  Run: `cd /srv/owlnext/latch && docker run --rm -v "$PWD:/repo" -w /repo rhysd/actionlint:latest -color`
  Expected: 0 erreur (ou installer `actionlint` localement ; à défaut, vérifier le YAML via `python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/ci.yml'))"`).

- [ ] **Step 7: Commit**
  ```bash
  rtk git add .github/workflows/ci.yml
  rtk git commit -m "$(cat <<'EOF'
  👷 ci: durcissement (pin actions SHA S7637, --ignore-scripts S6505) + confort

  concurrency cancel-in-progress, cache navigateurs Playwright, clippy --all-features.

  Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01PztH7Hqw6QfatfKG9MpRDV
  EOF
  )"
  ```

---

### Task 7 : Rust — règle no-unwrap mécanique (`[workspace.lints]`)

**Files:**
- Modify: `Cargo.toml` (racine — ajout `[workspace.lints.clippy]`)
- Modify: `backend/Cargo.toml` (ajout `[lints] workspace = true`)
- Modify: `backend/migration/Cargo.toml` (ajout `[lints] workspace = true`)
- Modify: sites légitimes d'`unwrap`/`expect` hors tests (à recenser au Step 2).

**Interfaces:** Produces: lints `unwrap_used`/`expect_used` actifs et bloquants (via `clippy -D warnings`).

- [ ] **Step 1: Déclarer les lints au workspace**
  Dans `Cargo.toml` racine, après `[workspace.dependencies]`, ajouter :
  ```toml
  [workspace.lints.clippy]
  unwrap_used = "warn"
  expect_used = "warn"
  ```
  Dans `backend/Cargo.toml` ET `backend/migration/Cargo.toml`, ajouter (en fin de fichier) :
  ```toml
  [lints]
  workspace = true
  ```

- [ ] **Step 2: Recenser les violations**
  Run: `cd /srv/owlnext/latch && cargo clippy --all-targets --all-features 2>&1 | grep -E "unwrap_used|expect_used" -A2 | grep -E "^\s*-->" | sort -u`
  Expected: liste des sites. Les sites en code de **test** (`#[cfg(test)]`, `tests/`, `test_support.rs`) recevront un `allow` global ; les sites d'**init de boot légitimes** (ex. `main`, config governor, `Key::from`) recevront un `#[allow]` ciblé **commenté**.

- [ ] **Step 3: Autoriser unwrap/expect dans les tests**
  Pour chaque module de test inline, ajouter en tête du `mod tests` :
  ```rust
  #[cfg(test)]
  #[allow(clippy::unwrap_used, clippy::expect_used)]
  mod tests { /* … */ }
  ```
  Pour les fichiers `backend/tests/*.rs` (tests d'intégration), ajouter en tête de fichier :
  ```rust
  #![allow(clippy::unwrap_used, clippy::expect_used)]
  ```
  Pour `backend/src/services/test_support.rs` (helper de test compilé hors `#[cfg(test)]` si exposé), ajouter `#![allow(...)]` au module ou `#[allow(...)]` sur les fns.

- [ ] **Step 4: Justifier chaque expect d'init légitime**
  Pour chaque `expect`/`unwrap` de boot non-test restant, ajouter un `#[allow]` **commenté** expliquant pourquoi l'échec au boot est acceptable, ex. :
  ```rust
  // Init de boot : config governor invalide = bug de programmation, panique acceptable au démarrage.
  #[allow(clippy::expect_used)]
  let config = GovernorConfigBuilder::default().finish().expect("governor config valide");
  ```
  > Si le nombre de sites légitimes explose (bruit), repli : retirer `expect_used` de la liste workspace (garder `unwrap_used` seul) et le noter en HANDOFF.

- [ ] **Step 5: clippy vert**
  Run: `cd /srv/owlnext/latch && cargo clippy --all-targets --all-features -- -D warnings`
  Expected: 0 warning.

- [ ] **Step 6: Tests backend toujours verts**
  Run: `cd /srv/owlnext/latch && cargo nextest run`
  Expected: tous verts (aucun changement de comportement).

- [ ] **Step 7: Commit**
  ```bash
  rtk git add Cargo.toml backend/Cargo.toml backend/migration/Cargo.toml backend/src backend/tests backend/migration/src
  rtk git commit -m "$(cat <<'EOF'
  🔒 chore(lints): unwrap_used/expect_used = warn (workspace), enforcé en CI

  Règle BOOTSTRAP §4 rendue mécanique. Tests : allow groupé. Init de boot : allow commenté ciblé.

  Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01PztH7Hqw6QfatfKG9MpRDV
  EOF
  )"
  ```

---

### Task 8 : SonarQube — `sonar-project.properties` + job CI bloquant

**Files:**
- Create: `sonar-project.properties` (racine)
- Modify: `.github/workflows/ci.yml` (nouveau job `sonar` + ajout aux `needs` de `docker`)

**Interfaces:**
- Consumes: `frontend/coverage/lcov.info` (Task 4), code remédié (Tasks 1-3,5-7).
- Produces: gate Sonar bloquant dans la CI.

- [ ] **Step 1: Créer `sonar-project.properties`**
  ```properties
  sonar.organization=owlnext-fr
  sonar.projectKey=owlnext-fr_latch
  sonar.sources=frontend/src,Dockerfile,docker-compose.yml,.github
  sonar.tests=frontend/src
  sonar.test.inclusions=**/*.test.ts,**/*.test.tsx
  sonar.javascript.lcov.reportPaths=frontend/coverage/lcov.info
  sonar.exclusions=frontend/src/api/schema.d.ts,**/*.config.ts
  sonar.qualitygate.wait=true
  ```

- [ ] **Step 2: Re-scan local AVANT de câbler la CI — confirmer le gate VERT**
  ```bash
  cd /srv/owlnext/latch
  set -a; . ./.env.local; set +a
  ( cd frontend && pnpm install --frozen-lockfile --ignore-scripts && pnpm test:cov )
  docker run --rm -e SONAR_TOKEN -e SONAR_HOST_URL=https://sonarcloud.io \
    -v "$PWD:/usr/src" sonarsource/sonar-scanner-cli
  ```
  Expected: `QUALITY GATE STATUS: PASSED`. Si des issues subsistent, les corriger (revenir aux Tasks concernées) OU, pour un cas vraiment idiomatique/faux-positif, le marquer *won't-fix* dans l'UI Sonar (avec justification) — puis re-scan jusqu'au PASSED.
  > Le `sonar-project.properties` fournit désormais org/projectKey → plus besoin des `-D…` en ligne de commande.

- [ ] **Step 3: Ajouter le job `sonar` dans `ci.yml`**
  ```yaml
    # 8. Analyse SonarQube Cloud (Quality Gate bloquant)
    sonar:
      name: SonarQube
      runs-on: ubuntu-latest
      steps:
        - uses: actions/checkout@<sha>  # v4 — fetch-depth 0 requis par Sonar
          with:
            fetch-depth: 0
        - uses: pnpm/action-setup@<sha>  # v4
          with:
            version: 9.15.9
        - uses: actions/setup-node@<sha>  # v4
          with:
            node-version-file: frontend/.nvmrc
            cache: pnpm
            cache-dependency-path: frontend/pnpm-lock.yaml
        - run: cd frontend && pnpm install --frozen-lockfile --ignore-scripts
        - run: cd frontend && pnpm test:cov
        - uses: SonarSource/sonarqube-scan-action@<sha>  # pin SHA (cf. Task 6 Step 5)
          env:
            SONAR_TOKEN: ${{ secrets.SONAR_TOKEN }}
  ```
  > Pinner les `<sha>` (méthode Task 6 Step 5). `qualitygate.wait=true` est dans `sonar-project.properties` → l'action échoue si le gate est rouge.

- [ ] **Step 4: Ajouter `sonar` aux dépendances du job `docker`**
  Modifier la ligne `needs:` du job `docker` :
  ```yaml
    needs: [fmt-clippy, test-backend, supply-chain, frontend, supply-chain-front, e2e, sonar]
  ```

- [ ] **Step 5: Valider le YAML**
  Run: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml')); print('YAML OK')"`
  Expected: `YAML OK`.

- [ ] **Step 6: Commit**
  ```bash
  rtk git add sonar-project.properties .github/workflows/ci.yml
  rtk git commit -m "$(cat <<'EOF'
  👷 ci: job SonarQube Cloud bloquant (gate wait) + properties

  Analyse front + IaC, couverture lcov, qualitygate.wait=true. Ajouté aux needs de docker.
  Pré-vérifié localement : QUALITY GATE PASSED.

  Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01PztH7Hqw6QfatfKG9MpRDV
  EOF
  )"
  ```

---

### Task 9 : Vérification finale + mise à jour mémoire

**Files:**
- Modify: `docs/INDEX.md`, `docs/HANDOFF.md`, `docs/QUIRKS.md`, `docs/ENVIRONMENT.md`, `docs/BACKLOG.md`, `docs/BOOTSTRAP.md`

**Interfaces:** Produces: mémoire projet cohérente (règle de fin d'implémentation CLAUDE.md).

- [ ] **Step 1: Vérification complète locale**
  Run, et confirmer chaque sortie verte :
  ```bash
  cd /srv/owlnext/latch
  cargo fmt --all -- --check
  cargo clippy --all-targets --all-features -- -D warnings
  cargo nextest run
  ( cd frontend && pnpm lint && pnpm typecheck && pnpm test:cov && pnpm build )
  ```
  Expected: tout vert.

- [ ] **Step 2: Pousser la branche et surveiller la CI**
  ```bash
  rtk git push -u origin chore/toolchain-ci-hardening
  ```
  Surveiller : `rtk gh run list --branch chore/toolchain-ci-hardening` puis `rtk gh run watch <id>`.
  Expected: **tous les jobs verts, dont `SonarQube`** (gate PASSED) et `docker build`.

- [ ] **Step 3: Mettre à jour `docs/INDEX.md`**
  Ajouter sous une section « Toolchain & CI » : job Sonar CI bloquant + `sonar-project.properties`, Dockerfile cargo-chef + runtime non-root, couverture lcov, `[workspace.lints]` no-unwrap, confort CI (concurrency, cache Playwright, pin SHA, `--all-features`).

- [ ] **Step 4: Mettre à jour `docs/QUIRKS.md`** (entrées nouvelles)
  - SonarQube Cloud : Automatic Analysis **exclusive** du scanner CI (désactiver l'une pour l'autre).
  - Sonar vs ESLint : `void` (S3735) supprimé sans risque car `no-floating-promises` inactif (config recommended non type-checked).
  - cargo-chef : couche `cook` = vrai layer caché par `type=gha` ; `[lints] workspace=true` à répliquer dans **chaque** crate.
  - Runtime non-root : volume `/data` préexistant possédé par root → `chown 65532:65532` une fois.

- [ ] **Step 5: Mettre à jour `docs/ENVIRONMENT.md`**
  Ajouter : secret GitHub `SONAR_TOKEN`, `sonar.organization=owlnext-fr`, `sonar.projectKey=owlnext-fr_latch`, fichier local gitignoré `.env.local` (token pour scan manuel), commande de scan local Docker.

- [ ] **Step 6: Mettre à jour `docs/BACKLOG.md`**
  Rayer/clôturer « Cache de build Docker (cargo-chef) » **et** « Conteneur en utilisateur non-root » (les deux livrés).

- [ ] **Step 7: Mettre à jour `docs/BOOTSTRAP.md` §6**
  Ajouter le job `SonarQube` (gate bloquant, front+IaC, couverture lcov) à la liste des jobs CI ; mentionner cargo-chef au §7 Docker.

- [ ] **Step 8: Mettre à jour `docs/HANDOFF.md`**
  Entrée datée en haut : dernière chose faite (chantier toolchain/CI livré), trucs en suspens (le cas échéant des *won't-fix* Sonar justifiés), prochaine étape (Phase 5 MCP), notes (quirks Sonar/cargo-chef/non-root).

- [ ] **Step 9: Commit mémoire**
  ```bash
  rtk git add docs/
  rtk git commit -m "$(cat <<'EOF'
  📝 docs: clôture chantier toolchain & CI (Sonar + cargo-chef + lints)

  INDEX/QUIRKS/ENVIRONMENT/BACKLOG/BOOTSTRAP/HANDOFF mis à jour.

  Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01PztH7Hqw6QfatfKG9MpRDV
  EOF
  )"
  ```

- [ ] **Step 10: Finaliser la branche**
  Proposer à l'humain : merge `chore/toolchain-ci-hardening` → `main` (fast-forward / PR selon préférence), une fois la CI verte confirmée.

---

## Notes de séquencement

- **Tasks 1-3** (front smells) et **Task 7** (lints Rust) sont indépendantes — parallélisables.
- **Task 4** (coverage) précède **Task 8** (Sonar a besoin du lcov).
- **Tasks 5-6** (Docker/CI) corrigent les vulns → indispensables avant le re-scan vert de **Task 8**.
- **Task 8 Step 2** (re-scan local PASSED) est le **gate dur** avant de rendre le job CI bloquant.
- **Task 9** clôt (vérif + mémoire + push CI).
