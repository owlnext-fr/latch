# Design — Durcissement toolchain & CI

> Spec validée le 2026-06-25. Trois chantiers indépendants regroupés car ils touchent
> tous le pipeline CI/build : analyse SonarQube Cloud, accélération du build Docker,
> et durcissements bonus. **Hors phasing métier** (s'intercale avant la Phase 5 MCP).

## Contexte & problème

- **Écart éditeur ↔ toolchain** : l'extension **SonarQube for IDE (SonarLint)** dans VSCode
  remonte de nombreux *code smells* sur le **frontend** (TS/React) — règles SonarSource que
  ni ESLint ni `tsc` ne voient. La CI ne lance **aucune** analyse Sonar → ces warnings ne
  sont jamais surfacés dans le pipeline. Constat de terrain : `cargo clippy --all-targets`
  est **propre** (0 warning), donc le problème est purement frontend/Sonar, pas Rust.
- **Build Docker lent** : le stage 2 (build Rust) fait `COPY . . && cargo build -p latch
  --release` → recompile **toutes les dépendances** à chaque changement de source. Aucun
  cache de l'arbre cargo (le `cache-from: type=gha` de la CI ne cache que les *layers*).
- **Trous de toolchain** : la règle BOOTSTRAP §4 « pas d'`unwrap`/`expect` hors tests » n'est
  pas enforcée par un lint ; pas de garde-fou CI contre les runs redondants.

## Décisions (cadrage validé)

| Sujet | Décision |
|---|---|
| Hosting Sonar | **SonarQube Cloud** (gratuit OSS, projet déjà créé sur le repo public) |
| Périmètre Sonar | **Frontend TS/React + IaC** (Dockerfile, GitHub Actions, docker-compose, YAML) + secrets. **Pas le Rust** (reste sur clippy/cargo-deny). |
| Quality Gate | **Bloquant dès le départ** (sur l'existant, pas seulement le New Code) |
| Cache Docker | **cargo-chef** |
| Bonus retenus | Règle *no-unwrap* mécanisée + confort CI |
| Bonus écartés | Alignement IDE↔clippy (`.vscode`), pin du toolchain Rust (`rust-toolchain.toml`) |

---

## §1 — SonarQube Cloud dans la CI

### Nouveau job `sonar` (`.github/workflows/ci.yml`)

- `actions/checkout@v4` avec **`fetch-depth: 0`** — Sonar a besoin de l'historique git
  complet (datation du « New Code », blame des lignes).
- Setup pnpm/node (mêmes étapes que le job `frontend`, pin `pnpm@9.15.9`).
- `pnpm install --frozen-lockfile`.
- **`pnpm test -- --coverage`** (ou script dédié) → produit `frontend/coverage/lcov.info`.
- **`SonarSource/sonarqube-scan-action`** (action officielle, version pinnée dans le plan),
  avec `SONAR_TOKEN` injecté depuis les secrets GitHub.
- Le gate bloquant repose sur **`sonar.qualitygate.wait=true`** → l'action attend le verdict
  et **échoue le job** si le Quality Gate est rouge.
- **Ajout de `sonar` aux `needs:` du job `docker`** → aucune image publiée si Sonar est rouge.

### `sonar-project.properties` (racine du repo)

```properties
sonar.organization=owlnext-fr
sonar.projectKey=owlnext-fr_latch
sonar.sources=frontend/src,Dockerfile,docker-compose.yml,.github
sonar.tests=frontend/src
sonar.test.inclusions=**/*.test.ts,**/*.test.tsx
sonar.javascript.lcov.reportPaths=frontend/coverage/lcov.info
sonar.exclusions=frontend/src/api/schema.d.ts,**/*.config.ts
```

- `schema.d.ts` est **généré** (openapi-typescript) → exclu de l'analyse.
- Les analyseurs **IaC / Docker / GitHub Actions / secrets** de Sonar se déclenchent
  automatiquement sur les fichiers correspondants présents dans `sonar.sources`.

### Couverture (lcov)

- Ajouter le provider de couverture Vitest (`@vitest/coverage-v8`) et activer le reporter
  `lcov` (config `vitest.config.ts` : `coverage: { provider: 'v8', reporter: ['text', 'lcov'] }`).
- La couverture nourrit la condition « Coverage on New Code » du Quality Gate Sonar way.

### Connected Mode (le « local »)

- Dans VSCode : **SonarQube for IDE → Connect to SonarQube Cloud**, lier au projet `latch`.
- Effet : l'éditeur et la CI utilisent **le même quality profile** → ce que tu vois dans
  VSCode == ce que la CI bloque. **C'est le cœur de la réconciliation éditeur ↔ toolchain.**
- Pas de scanner CLI à lancer localement (l'IDE en connected mode tient le rôle « local »).

### Quirk bloquant à connaître

> **SonarQube Cloud : Automatic Analysis et analyse par scanner CI sont MUTUELLEMENT
> EXCLUSIVES.** Comme on veut l'analyse CI (import de couverture + contrôle du périmètre +
> gate bloquant), il faut **désactiver l'Automatic Analysis** dans
> *Projet → Administration → Analysis Method*. Sinon le scanner CI est refusé avec une erreur
> « automatic analysis is enabled ».

### Pré-requis humains (one-shot)

1. ~~Récupérer `ORG_KEY` / `PROJECT_KEY`~~ → **fournis** : `owlnext-fr` / `owlnext-fr_latch`.
2. Générer un **Project Analysis Token** (My Account → Security) et l'ajouter en
   **secret repo GitHub `SONAR_TOKEN`** (Settings → Secrets and variables → Actions).
3. Désactiver l'Automatic Analysis (cf. quirk).

---

## §2 — « Bloquant dès le départ » → résorption du backlog

Le mode *new-code-only* a été **explicitement refusé** : le gate doit être vert **sur
l'existant**. Conséquence assumée : **une passe de remédiation du frontend fait partie de ce
chantier**.

Déroulé :

1. **Premier scan sur une branche** (pas `main`) → récupérer la liste réelle des *issues*.
2. **Triage** de chaque issue : corriger / marquer *false-positive* ou *won't-fix* dans Sonar /
   accepter via exclusion ciblée et justifiée.
3. **Ne déclarer le job bloquant** (ajout aux `needs` de `docker`, gate `wait=true`) **qu'une
   fois le gate vert**, pour ne jamais casser `main` pendant la résorption.

> **Risque honnête** : la taille du backlog est **inconnue avant le 1ᵉʳ scan** (VSCode « rempli
> de warnings »). Si la remédiation s'avère volumineuse, elle pourra basculer dans **son propre
> plan/spec séparé** — décision prise après le premier scan, communiquée avant engagement.

---

## §3 — Docker : cargo-chef

Refonte du **stage 2** uniquement (seul goulot). Les stages 1 (Node/Vite) et 3 (distroless)
sont inchangés.

```dockerfile
FROM rust:1-bookworm AS chef
RUN cargo install cargo-chef --locked
WORKDIR /src

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json     # empreinte des deps

FROM chef AS builder
COPY --from=planner /src/recipe.json recipe.json
RUN cargo chef cook --release -p latch --recipe-path recipe.json   # ← couche cachée (deps)
COPY . .
RUN cargo build -p latch --release                   # ← ne recompile que le code app
```

- La couche `cook` est un **vrai layer**, restauré par le `cache-from: type=gha` déjà en place
  en CI tant que `Cargo.toml`/`Cargo.lock` ne changent pas.
- **Gain attendu** : build « code seul » de plusieurs minutes → quelques secondes.
- Aucune autre étape de la CI à modifier (le job `docker` garde `cache-from/to: type=gha`).

---

## §4 — Bonus

### (a) Règle *no-unwrap* mécanique

`Cargo.toml` racine :
```toml
[workspace.lints.clippy]
unwrap_used = "warn"
expect_used = "warn"
```
+ `[lints] workspace = true` dans `backend/Cargo.toml` **et** `backend/migration/Cargo.toml`.

Comme la CI fait `clippy -D warnings`, ces lints deviennent **bloquants**.

- **Tests** : droit à l'`unwrap`/`expect` → `#![cfg_attr(test, allow(clippy::unwrap_used,
  clippy::expect_used))]` au niveau module, ou `#[allow]` ciblés sur les blocs `#[cfg(test)]`.
- **Code d'init légitime** (`main`, boot, config governor) : chaque `expect` survivant porte un
  `#[allow(clippy::expect_used)]` **explicite et commenté** → chaque exception devient un choix
  visible et revu, pas un oubli.
- **Garde-fou** : si la passe révèle trop de sites légitimes (bruit excessif), repli possible
  vers une politique plus douce (ex. `unwrap_used` seul, ou lints `restriction` ciblés).

### (b) Confort CI

- **`concurrency`** en tête de `ci.yml` :
  ```yaml
  concurrency:
    group: ci-${{ github.ref }}
    cancel-in-progress: true
  ```
  → annule les runs CI redondants sur push rapprochés.
- **Cache des navigateurs Playwright** dans le job `e2e` : `actions/cache` sur
  `~/.cache/ms-playwright`, clé dérivée de la version Playwright du lockfile → évite le
  re-download du chromium à chaque run.
- **`--all-features`** sur le clippy CI : `cargo clippy --all-targets --all-features -- -D
  warnings` → attrape d'éventuels warnings derrière des features.

---

## Vérification (definition of done)

- CI **verte de bout en bout**, **job `sonar` inclus au vert** (gate passé sur branche puis main).
- **Build Docker chronométré avant/après** cargo-chef (preuve du gain sur un changement de code
  seul, deps inchangées).
- `cargo clippy --all-targets --all-features -- -D warnings` **vert** après mécanisation
  no-unwrap (tous les `expect` d'init couverts par `#[allow]` commentés).
- `cargo fmt`, `cargo nextest`, `pnpm lint/typecheck/test/build`, e2e Playwright : restent verts.

## Mise à jour mémoire (fin de chantier)

- `INDEX.md` — livrables : job Sonar CI, `sonar-project.properties`, Dockerfile cargo-chef,
  workspace lints, confort CI.
- `HANDOFF.md` — entrée datée (état, suspens, prochaine étape).
- `QUIRKS.md` — Automatic-Analysis exclusif du scanner CI ; chemin `lcov.info` ; layer
  cargo-chef `cook` ; `[lints] workspace=true` à répliquer par crate.
- `ENVIRONMENT.md` — `SONAR_TOKEN` (secret GitHub), `sonar.organization`, `sonar.projectKey`.
- `BACKLOG.md` — rayer l'item « Cache de build Docker (cargo-chef) » (livré).
- `BOOTSTRAP.md` §6 — documenter l'étape Sonar dans la liste des jobs CI.

## Hors périmètre

- **Analyse Sonar du Rust** (support récent, recoupe clippy) — pas maintenant.
- **Alignement IDE↔clippy** (`rust-analyzer.check.command=clippy` dans `.vscode`) — écarté.
- **Pin du toolchain Rust** (`rust-toolchain.toml`) — écarté.
- **BuildKit cache mounts** — écartés au profit de cargo-chef (bénéfice CI natif).
