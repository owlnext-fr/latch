# Site de documentation publique (`latch`) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Construire et déployer le site de documentation publique de `latch` (landing produit + docs) en statique sur GitHub Pages, via Fumadocs.

**Architecture:** App **Fumadocs (Next.js)** isolée dans `public_docs/`, exportée en **statique** (`output: 'export'`) et servie sous le **sous-chemin projet** `owlnext-fr.github.io/latch` (`basePath: '/latch'`). Build + déploiement Pages portés par **deux jobs ajoutés à `ci.yml`** (`docs` = build sur push/PR ; `deploy-docs` = `actions/deploy-pages`, `main` only). Contenu **EN**, sourcé uniquement de `public_docs/content/`, identité produit réutilisée (logo SVG, stone/oklch, clair/sombre).

**Tech Stack:** Fumadocs (Next.js + MDX, version résolue via Context7), Node 24, pnpm 9.15.9, Orama (recherche statique), GitHub Pages + Actions, Playwright (captures, harnais existant).

## Global Constraints

- **Spec de référence :** `docs/superpowers/specs/2026-06-26-phase-8-public-docs-design.md` (fait foi).
- **Langue du contenu publié : anglais uniquement.** La spec et le kit `docs/` restent en français.
- **Source de contenu : `public_docs/content/` UNIQUEMENT.** Jamais d'import depuis le `docs/` interne (kit de contrôle, non publiable). Réécriture manuelle des infos du contrat/BOOTSTRAP.
- **Confidentialité (CLAUDE.md, non-négociable) :** aucun nom client réel — placeholders fictifs (`Mon Projet`, `ACME`, `demo`).
- **`basePath: '/latch'`** explicite + `assetPrefix` + `.nojekyll` au build (sinon 404 d'assets). Liens internes **root-relative** (`/docs/...`) — jamais `/latch` codé en dur (Next préfixe via `basePath`).
- **App isolée :** `public_docs/` a son propre `package.json` + lockfile, **n'est pas** membre du workspace Rust ni lié à `frontend/`.
- **Workflow Context7 obligatoire** avant d'utiliser l'API Fumadocs (loader `source.config.ts`, recherche statique, composants MDX) — la lib bouge vite ; la référence est la version épinglée dans `public_docs/package.json`.
- **Actions GitHub épinglées par SHA** (politique supply-chain, BOOTSTRAP §6).
- **Identité produit :** logo `latch` (source `frontend/src/assets/latch-logo.svg`), palette **stone/oklch**, thème clair/sombre défaut `system`.
- **Commits :** conventionnels + gitmoji (`✨ feat:`, `📝 docs:`, `🧱 chore:`…).
- **Pré-requis humains (hors code, déjà faits / non requis) :** Pages = « GitHub Actions » **déjà activé** ; **pas** de domaine custom.
- **Définition de « terminé » (site) :** voir spec §8 — build OK, déployé à l'URL §2 sans 404 d'asset, toutes les pages §5 présentes en EN, recherche + liens OK, captures à jour, mémoire projet à jour, CI verte.

---

## File Structure

```
public_docs/
  package.json              # app isolée — Fumadocs/Next/React, scripts dev/build/start
  pnpm-lock.yaml
  .nvmrc                    # 24
  next.config.mjs           # output: export, images.unoptimized, basePath/assetPrefix env, .nojekyll via public/
  source.config.ts          # config fumadocs-mdx (résolu via Context7)
  tsconfig.json
  postcss/tailwind config   # selon scaffold Fumadocs
  app/
    layout.tsx              # RootProvider (thème, search provider)
    layout.config.tsx       # baseOptions partagées : nav (logo, Docs, GitHub), title
    global.css              # tokens stone/oklch + Tailwind
    (home)/
      page.tsx              # LANDING produit (7 sections — spec §4.2)
    docs/
      layout.tsx            # DocsLayout (sidebar depuis source)
      [[...slug]]/page.tsx  # rendu MDX
    not-found.tsx           # 404 statique
    api/search/route.ts     # recherche (mode statique pour export — Context7)
  lib/
    source.ts               # loader('/docs', content/docs)
  components/
    landing/                # sections de la landing (Hero, Features, etc.)
  content/docs/             # ← contenu (voir Tasks 6-10) + meta.json par dossier
  public/
    .nojekyll               # copié à la racine de out/ au build
    img/                    # captures + logo + schéma flux Claude
.github/workflows/ci.yml    # MODIFIÉ : + jobs `docs` et `deploy-docs`
README.md                   # MODIFIÉ (Task 12) : lien vers l'URL doc en ligne
docs/INDEX.md ROADMAP.md ENVIRONMENT.md HANDOFF.md QUIRKS.md  # MODIFIÉS (Task 12)
```

---

## PHASE A — Install (scaffold, identité, landing, docs shell, CI)

### Task 1 : Scaffold Fumadocs + export statique

**Files:**
- Create: `public_docs/` (arborescence scaffold), `public_docs/next.config.mjs`, `public_docs/public/.nojekyll`, `public_docs/.nvmrc`
- Modify: `public_docs/package.json` (épinglage versions, scripts)

**Interfaces:**
- Produces: une app Fumadocs qui **build en statique** → `public_docs/out/` contenant `index.html`, `_next/`, `.nojekyll`. Scripts `pnpm build` (export) et `pnpm dev`.

- [ ] **Step 1 : Résoudre la doc Fumadocs via Context7**

Récupérer, pour la version courante de Fumadocs : (a) commande de scaffold non-interactive, (b) forme de `source.config.ts` + `lib/source.ts`, (c) **mode recherche compatible `output: 'export'`** (recherche statique Orama, pas de route serveur), (d) config `next.config.mjs` recommandée pour export statique. Noter la version résolue.

- [ ] **Step 2 : Scaffolder l'app dans `public_docs/`**

Run (depuis la racine repo) : `pnpm create fumadocs-app` en ciblant `public_docs` (template **Next.js**, **Tailwind CSS**, content source **Fumadocs MDX**). Si l'outil exige un dossier vide, scaffolder dans un tmp puis déplacer. Épingler les versions résolues dans `public_docs/package.json` ; ajouter `public_docs/.nvmrc` = `24`.

- [ ] **Step 3 : Configurer l'export statique + basePath**

Écrire `public_docs/next.config.mjs` :

```js
import { createMDX } from 'fumadocs-mdx/next';

const basePath = process.env.DOCS_BASE_PATH ?? '/latch';
const withMDX = createMDX();

/** @type {import('next').NextConfig} */
const config = {
  output: 'export',
  images: { unoptimized: true },
  basePath,
  // assetPrefix doit matcher basePath pour que _next/ se charge sous le sous-chemin
  assetPrefix: basePath || undefined,
  trailingSlash: true,
};

export default withMDX(config);
```

> ⚠️ `createMDX` / l'import exact dépend de la version Fumadocs (Step 1). Adapter si Context7 indique une autre forme.

- [ ] **Step 4 : Forcer `.nojekyll` dans l'export**

Créer `public_docs/public/.nojekyll` (fichier vide). Next copie `public/` à la racine de `out/` → `.nojekyll` se retrouve dans l'artefact (indispensable, sinon Pages/Jekyll ignore `_next/`).

```bash
mkdir -p public_docs/public && : > public_docs/public/.nojekyll
```

- [ ] **Step 5 : Installer et builder**

Run : `cd public_docs && pnpm install && pnpm build`
Expected : build OK, dossier `public_docs/out/` créé.

- [ ] **Step 6 : Vérifier l'artefact d'export**

Run : `ls public_docs/out/.nojekyll && ls public_docs/out/_next >/dev/null && echo OK`
Expected : `OK` (le `.nojekyll` est présent à la racine de `out/`, `_next/` existe).

- [ ] **Step 7 : Ignorer les artefacts de build**

Ajouter à `.gitignore` (racine) : `public_docs/node_modules/`, `public_docs/out/`, `public_docs/.next/`, `public_docs/.source/` (cache fumadocs-mdx si présent).

- [ ] **Step 8 : Commit**

```bash
rtk git add public_docs .gitignore && rtk git commit -m "🧱 chore(docs-site): scaffold Fumadocs + export statique basePath /latch"
```

---

### Task 2 : Identité produit (logo, palette stone/oklch, thème clair/sombre, nav)

**Files:**
- Create: `public_docs/public/img/latch-logo.svg`
- Modify: `public_docs/app/global.css`, `public_docs/app/layout.config.tsx`, `public_docs/app/layout.tsx`

**Interfaces:**
- Consumes: app scaffoldée (Task 1).
- Produces: `baseOptions` (nav partagée) exportées depuis `layout.config.tsx` ; thème clair/sombre fonctionnel ; logo affiché dans la nav.

- [ ] **Step 1 : Importer le logo**

Copier `frontend/src/assets/latch-logo.svg` → `public_docs/public/img/latch-logo.svg`. (SVG `currentColor` → suit le thème.)

```bash
cp frontend/src/assets/latch-logo.svg public_docs/public/img/latch-logo.svg
```

- [ ] **Step 2 : Aligner les tokens de couleur stone/oklch**

Dans `public_docs/app/global.css`, surcharger les variables de thème Fumadocs avec la palette **stone oklch** (mêmes valeurs `--background`/`--foreground`/`--primary`/`--muted…` que `frontend/src/index.css`, en `:root` et `.dark`). Reprendre les valeurs depuis `frontend/src/index.css` (lecture seule, ne pas importer).

- [ ] **Step 3 : Configurer la nav partagée**

Dans `public_docs/app/layout.config.tsx`, définir `baseOptions` : titre `latch` + logo (`<img src="/img/latch-logo.svg" alt="latch" />` — root-relative, Next préfixe le basePath), lien **GitHub** (`https://github.com/owlnext-fr/latch`), lien **Docs** (`/docs`). Le toggle de thème est fourni par Fumadocs (`RootProvider`).

- [ ] **Step 4 : Vérifier le thème**

Run : `cd public_docs && pnpm build`
Expected : build OK. (Vérif visuelle clair/sombre faite en Task 3 quand la landing existe.)

- [ ] **Step 5 : Commit**

```bash
rtk git add public_docs && rtk git commit -m "💄 feat(docs-site): identité produit (logo, stone/oklch, thème clair/sombre)"
```

---

### Task 3 : Landing produit + page 404

**Files:**
- Create: `public_docs/app/(home)/page.tsx`, `public_docs/app/not-found.tsx`, `public_docs/components/landing/*.tsx`
- Modify: `public_docs/app/(home)/layout.tsx` si nécessaire (HomeLayout avec baseOptions)

**Interfaces:**
- Consumes: `baseOptions` (Task 2).
- Produces: route `/` = landing ; route 404 statique.

- [ ] **Step 1 : Composer la landing (spec §4.2)**

`app/(home)/page.tsx` assemblant 7 sections (composants dans `components/landing/`) :
1. **Hero** — logo, titre `latch`, pitch EN (« Serve single-file HTML prototypes behind a controlled host, with versioning and optional per-project access codes. »), CTA **Get Started** (`/docs/quickstart`) + **GitHub**.
2. **Screenshots** — `/img/admin-list.png` + `/img/unlock.png` (placeholders jusqu'à Task 11 ; référencer les chemins).
3. **Features grid** — 6 cartes : Three surfaces (serving `/c`, admin, MCP) · Publish from Claude · PIN access codes · Fail-secure secrets · Single Rust binary / distroless image · FOSS dual-license.
4. **Publish-from-Claude highlight** — bande dédiée (le différenciateur MCP).
5. **How it works teaser** — archi en couches en 3 lignes → lien `/docs/how-it-works/architecture`.
6. **CTA finale** — *Get Started* + bloc copiable `docker pull ghcr.io/owlnext-fr/latch`.
7. **Footer** — GitHub · License (MIT/Apache-2.0) · CHANGELOG.

Style Tailwind + tokens du thème (réutilise l'identité Task 2). Tout le copy en **anglais**.

- [ ] **Step 2 : Page 404**

`app/not-found.tsx` : message simple EN + lien retour `/` et `/docs`. Statique (pas de fetch).

- [ ] **Step 3 : Builder et vérifier le rendu**

Run : `cd public_docs && pnpm build && ls out/index.html out/404.html`
Expected : `out/index.html` (landing) + `out/404.html` présents.

- [ ] **Step 4 : Vérification visuelle locale (dev)**

Run : `cd public_docs && pnpm dev` → ouvrir `http://localhost:3000/latch` ; vérifier hero, features, toggle clair/sombre, logo qui suit le thème, CTA cliquables. (Arrêter le serveur après.)

- [ ] **Step 5 : Commit**

```bash
rtk git add public_docs && rtk git commit -m "✨ feat(docs-site): landing produit (hero, features, CTA) + page 404"
```

---

### Task 4 : Shell docs + recherche statique + ordre sidebar

**Files:**
- Modify: `public_docs/lib/source.ts`, `public_docs/app/docs/layout.tsx`, `public_docs/app/api/search/route.ts`, `public_docs/source.config.ts`
- Create: `public_docs/content/docs/index.mdx`, `public_docs/content/docs/meta.json`

**Interfaces:**
- Consumes: loader scaffoldé (Task 1).
- Produces: route `/docs` fonctionnelle, sidebar ordonnée, recherche statique buildée dans l'artefact.

- [ ] **Step 1 : Recherche statique (Context7)**

Reconfigurer la recherche pour **`output: 'export'`** : index Orama **statique** généré au build (pas de route serveur dynamique). Suivre la recette Context7 (typiquement `staticGET` côté `app/api/search/route.ts` + client `useDocsSearch({ type: 'static' })`). Vérifier que le build n'émet pas de route serveur.

- [ ] **Step 2 : Page d'intro docs**

`content/docs/index.mdx` (frontmatter `title: Introduction`, `description: …`) : « What is latch » + les **trois surfaces** (serving `/c`, admin, MCP) + tableau « I'm an operator / designer / contributor → start here » avec liens. EN.

- [ ] **Step 3 : Ordre de la sidebar racine**

`content/docs/meta.json` :

```json
{
  "title": "Docs",
  "pages": ["index", "quickstart", "deploy", "admin", "publish-from-claude", "how-it-works", "troubleshooting"]
}
```

> Les pages/dossiers non encore créés sont tolérés par Fumadocs (apparaissent au fur et à mesure des Tasks 6-10).

- [ ] **Step 4 : Builder et vérifier**

Run : `cd public_docs && pnpm build && ls out/docs/index.html`
Expected : page docs exportée. Vérifier en `pnpm dev` que la recherche ouvre et indexe l'intro.

- [ ] **Step 5 : Commit**

```bash
rtk git add public_docs && rtk git commit -m "✨ feat(docs-site): shell docs + recherche statique Orama + intro"
```

---

### Task 5 : Intégration CI — jobs `docs` et `deploy-docs` dans `ci.yml`

**Files:**
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: `public_docs` qui build en statique (Tasks 1-4).
- Produces: job `docs` (build + upload artefact Pages) + job `deploy-docs` (déploiement Pages, `main` only).

- [ ] **Step 1 : Ajouter le job `docs` (build, toujours)**

Dans `.github/workflows/ci.yml`, ajouter (SHA des actions identiques à ceux déjà utilisés dans le fichier ; `upload-pages-artifact` à résoudre + épingler) :

```yaml
  # 10. Doc publique — build statique (validé à chaque push/PR)
  docs:
    name: docs (build statique)
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: public_docs
    steps:
      - uses: actions/checkout@34e114876b0b11c390a56381ad16ebd13914f8d5  # v4
      - uses: pnpm/action-setup@b906affcce14559ad1aafd4ab0e942779e9f58b1  # v4
        with:
          version: 9.15.9
      - uses: actions/setup-node@49933ea5288caeca8642d1e84afbd3f7d6820020  # v4
        with:
          node-version-file: public_docs/.nvmrc
          cache: pnpm
          cache-dependency-path: public_docs/pnpm-lock.yaml
      - run: pnpm install --frozen-lockfile
      - run: pnpm build
      - uses: actions/upload-pages-artifact@<SHA>  # v3 — résoudre + épingler
        with:
          path: public_docs/out
```

- [ ] **Step 2 : Ajouter le job `deploy-docs` (Pages, main only)**

```yaml
  # 11. Doc publique — déploiement GitHub Pages (main uniquement)
  deploy-docs:
    name: docs (deploy Pages)
    runs-on: ubuntu-latest
    needs: [docs]
    if: github.event_name == 'push' && github.ref == 'refs/heads/main'
    permissions:
      pages: write
      id-token: write
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    concurrency:
      group: pages
      cancel-in-progress: false
    steps:
      - id: deployment
        uses: actions/deploy-pages@<SHA>  # v4 — résoudre + épingler
```

> `deploy-docs` ne dépend **que** de `docs` (couplage faible voulu — spec §6.2). Pas dans `needs` du job `docker`.

- [ ] **Step 3 : Valider la syntaxe YAML**

Run : `python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/ci.yml')); print('YAML OK')"`
Expected : `YAML OK`.

- [ ] **Step 4 : Commit**

```bash
rtk git add .github/workflows/ci.yml && rtk git commit -m "👷 ci(docs): build + déploiement Pages dans ci.yml (jobs docs + deploy-docs)"
```

> Le déploiement réel se vérifie au 1ᵉʳ push `main` (Task 12) : charger une page profonde et confirmer que `_next/` se charge (piège basePath).

---

## PHASE B — Population du contenu (EN, source = contrat/BOOTSTRAP réécrits)

> **Note d'exécution :** les pages sont du **contenu rédactionnel MDX** — la « preuve » est `pnpm build` vert + **aucun lien interne cassé** + relecture, pas un test unitaire. Chaque page ci-dessous liste : frontmatter, sections obligatoires, source interne (à réécrire en EN, jamais importer). Respecter les invariants de confidentialité (placeholders) et de sécurité (jamais exposer hash/PIN dans les exemples).

### Task 6 : `how-it-works/` (dérivable du contrat)

**Files:**
- Create: `content/docs/how-it-works/{meta.json,architecture.mdx,security-model.mdx,contributing.mdx}`

- [ ] **Step 1 :** `meta.json` → `{ "title": "How it works", "pages": ["architecture", "security-model", "contributing"] }`.
- [ ] **Step 2 :** `architecture.mdx` — *source : contrat §1/§2.* Archi en couches : cœur agnostique HTTP, adaptateurs entrants fins (web/MCP/serve), adaptateurs sortants (SeaORM + trait `Storage`), règle « le cœur suppose l'appelant autorisé ». EN.
- [ ] **Step 3 :** `security-model.mdx` — *source : contrat §9, BOOTSTRAP §9.* Deux cookies (session admin, déverrouillage signé), rate-limit **load-bearing** unlock + login, CVE Host-header / `allowed_hosts`, fail-secure secrets, invariants (jamais de hash ; PIN cantonné au détail), robots/X-Robots-Tag.
- [ ] **Step 4 :** `contributing.mdx` — *source : BOOTSTRAP §3-6.* Build par couche, tests (unit/intégration/MCP/Vitest/Playwright), gate SonarCloud `new_coverage ≥ 80%`, commits gitmoji.
- [ ] **Step 5 :** Run `cd public_docs && pnpm build` → OK, 3 pages exportées.
- [ ] **Step 6 :** Commit `rtk git commit -m "📝 docs(docs-site): section how-it-works (architecture, security, contributing)"`.

### Task 7 : `deploy/` (public opérateur)

**Files:**
- Create: `content/docs/deploy/{meta.json,docker.mdx,docker-compose.mdx,reverse-proxy.mdx,from-source.mdx,configuration.mdx,backup-upgrade.mdx,releases.mdx}`

- [ ] **Step 1 :** `meta.json` → `["docker", "docker-compose", "reverse-proxy", "from-source", "configuration", "backup-upgrade", "releases"]`.
- [ ] **Step 2 :** `docker.mdx` — `docker run` minimal, image GHCR publique (pas de `docker login`), volume `/data`, 6 secrets obligatoires.
- [ ] **Step 3 :** `docker-compose.mdx` — *source : `docker-compose.yml`, `deploy.sh`, `.env.example`.* Compose complet, volume `data/`, `.env`, `deploy.sh` (pull+up+prune), entrypoint migrate→start.
- [ ] **Step 4 :** `reverse-proxy.mdx` — *source : BOOTSTRAP §7/§9, contrat §6.* **Snippets pour Caddy, Nginx, Traefik, Apache** (onglets/code-groups Fumadocs) : TLS + reverse proxy vers `PORT`, forward `X-Forwarded-For`/`X-Real-IP` (load-bearing rate-limit), `X-Robots-Tag: noindex, nofollow`, service `robots.txt` (`Disallow: /`), `Host` cohérent avec `LATCH_PUBLIC_BASE_URL`.
- [ ] **Step 5 :** `from-source.mdx` — *source : BOOTSTRAP §3.* `cargo loco start` (depuis `backend/`) + **`pnpm build`** (React/Vite/pnpm 9.15.9, Node 24 — **pas Trunk**), `LATCH_SPA_DIST`.
- [ ] **Step 6 :** `configuration.mdx` — *source : ENVIRONMENT, `.env.example`.* Référence **exhaustive des 17 clés**, par groupe : **obligatoires prod (fail-secure)** [`ADMIN_USER`, `ADMIN_PASS`, `DEPLOY_TOKEN`, `LATCH_PUBLIC_BASE_URL`, `SESSION_SECRET`, `UNLOCK_COOKIE_SECRET`] ; **réglages** [`LATCH_UNLOCK_TTL_DAYS`, `LATCH_UNLOCK_RL_IP_BURST`, `LATCH_UNLOCK_RL_IP_PER_SECOND`, `LATCH_UNLOCK_RL_SLUG_BURST`, `LATCH_UNLOCK_RL_SLUG_PERIOD_SECS`, `LATCH_BODY_LIMIT`, `LATCH_STORAGE_ROOT`, `LATCH_SPA_DIST`, `DATABASE_URL`, `PORT`, `LATCH_IMAGE_TAG`]. Chaque clé : rôle, défaut, obligatoire prod oui/non, génération (`openssl rand -hex 32`).
- [ ] **Step 7 :** `backup-upgrade.mdx` — volume `data/` (sqlite + html ensemble) = unité de sauvegarde ; upgrade via `deploy.sh` (migrate auto au boot).
- [ ] **Step 8 :** `releases.mdx` — *source : BOOTSTRAP §6/§8.* Tags GHCR (`vX.Y.Z`→`X.Y.Z`/`X.Y`/`latest`/`sha-` ; `main`→`main`/`sha-`), pin `LATCH_IMAGE_TAG`, rollback, lien CHANGELOG.
- [ ] **Step 9 :** Run `pnpm build` → OK. Commit `📝 docs(docs-site): section deploy (docker, compose, reverse-proxy, config, releases…)`.

### Task 8 : `admin/` (public designer)

**Files:**
- Create: `content/docs/admin/{meta.json,projects.mdx,access-codes.mdx,versions.mdx,co-branding.mdx}`

- [ ] **Step 1 :** `meta.json` → `["projects", "access-codes", "versions", "co-branding"]`.
- [ ] **Step 2 :** `projects.mdx` — *source : contrat §7.* Créer en side-panel, slug lisible + suffixe 8 base62 (lecture seule v1), détail lecture seule. (Capture admin réf. Task 11.)
- [ ] **Step 3 :** `access-codes.mdx` — *source : contrat §3/§6/§9.* PIN auto 6 chiffres, `code_enabled` défaut vrai, deux états de `/c`, page de déverrouillage, **rotation PIN = révocation cookies**.
- [ ] **Step 4 :** `versions.mdx` — *source : contrat §7/§8.* Déployer (upload, case activer), prévisualiser (admin-only, no-store), basculer l'active (transactionnel), supprimer (refuse si active).
- [ ] **Step 5 :** `co-branding.mdx` — `brand_name` sur la page de déverrouillage (« Prototype prepared for {brand} »).
- [ ] **Step 6 :** Run `pnpm build` → OK. Commit `📝 docs(docs-site): section admin (projects, access codes, versions, co-branding)`.

### Task 9 : `publish-from-claude/` + `quickstart`

**Files:**
- Create: `content/docs/publish-from-claude/{meta.json,connect-mcp.mdx,tools-reference.mdx,why-token-not-oauth.mdx}`, `content/docs/quickstart.mdx`

- [ ] **Step 1 :** `meta.json` → `["connect-mcp", "tools-reference", "why-token-not-oauth"]`.
- [ ] **Step 2 :** `connect-mcp.mdx` — *source : README, ENVIRONMENT, contrat §5.* Récupérer `mcp_url` + `deploy_token` dans **Settings** ; renseigner l'URL MCP côté Claude ; pas d'OAuth/header ; tester avec `list_projects`.
- [ ] **Step 3 :** `tools-reference.mdx` — *source : contrat §5.1.* **Les deux tools** : `deploy_prototype(slug, html, deploy_token, activate?)` (slug préexistant, `activate` défaut `true`, réponse `DeployResult { url, version, code_protected }` — **jamais PIN/hash**) ; `list_projects(deploy_token)` (enveloppe objet `{ projects: [...] }`, `ProjectSummary` sans PIN/hash/id).
- [ ] **Step 4 :** `why-token-not-oauth.mdx` — *source : contrat §5, BACKLOG.* Modèle 1 expliqué, pourquoi suffisant, Modèle 2 OAuth en évolution future.
- [ ] **Step 5 :** `quickstart.mdx` — *source : README Quickstart.* Chemin doré : (1) `docker compose up` + `.env` minimal → (2) login admin → (3) créer un 1ᵉʳ projet → (4) brancher le MCP → (5) `deploy_prototype` depuis Claude → (6) ouvrir `/c/<slug>` + déverrouiller. Étape Claude = **schéma annoté + résultat** (réf. Task 11), pas de capture claude.ai.
- [ ] **Step 6 :** Run `pnpm build` → OK. Commit `📝 docs(docs-site): publish-from-claude (connect, tools, why-token) + quickstart`.

### Task 10 : `troubleshooting`

**Files:**
- Create: `content/docs/troubleshooting.mdx`

- [ ] **Step 1 :** `troubleshooting.mdx` — *source : ENVIRONMENT, QUIRKS, contrat.* Modes d'échec concrets : boot refusé fail-secure (secret manquant) ; **413** (`LATCH_BODY_LIMIT` trop bas) ; **MCP host rejeté** (`allowed_hosts` ≠ `LATCH_PUBLIC_BASE_URL`) ; **lockout** rate-limit unlock (compteurs in-memory, reset au reboot) ; **404 `/c`** (slug inconnu / pas de version active) ; cookie unlock invalidé après rotation PIN. Format : symptôme → cause → fix.
- [ ] **Step 2 :** Run `pnpm build` → OK.
- [ ] **Step 3 :** Commit `📝 docs(docs-site): page troubleshooting (modes d'échec concrets)`.

---

## PHASE C — Captures + finitions

### Task 11 : Captures (harnais Playwright) + schéma flux Claude

**Files:**
- Create: `public_docs/public/img/admin-list.png`, `public_docs/public/img/unlock.png`, `public_docs/public/img/claude-flow.svg`
- Reference: `frontend/e2e/screenshots.capture.ts` (existant, Phase 6)

- [ ] **Step 1 : Générer les captures**

Réutiliser le harnais existant (`CAPTURE=1`). Run (depuis `frontend/`) : `CAPTURE=1 pnpm exec playwright test screenshots.capture`. Copier les PNG produits (`docs/assets/admin-list.png`, `docs/assets/unlock.png`) → `public_docs/public/img/`. **Vérifier visuellement** : aucun nom client, aucun PIN affiché en clair.

- [ ] **Step 2 : Schéma du flux Claude**

Créer `public_docs/public/img/claude-flow.svg` : schéma annoté du flux « publish from Claude » (Claude → MCP `/mcp` → `deploy_prototype` → nouvelle version active → lien client). **Pas** de capture de claude.ai.

- [ ] **Step 3 : Vérifier les références d'images**

Confirmer que landing (Task 3) et `admin/`/`quickstart` (Tasks 8-9) pointent vers ces chemins. Run `cd public_docs && pnpm build` → OK, images dans `out/img/`.

- [ ] **Step 4 : Commit**

```bash
rtk git add public_docs/public/img && rtk git commit -m "🖼️ docs(docs-site): captures (harnais Playwright) + schéma flux Claude"
```

### Task 12 : Finitions — liens, README, déploiement, mémoire projet

**Files:**
- Modify: `README.md`, `docs/INDEX.md`, `docs/ROADMAP.md`, `docs/ENVIRONMENT.md`, `docs/HANDOFF.md`, `docs/QUIRKS.md`

- [ ] **Step 1 : Vérifier les liens internes**

Builder et scanner l'`out/` pour les `href` cassés vers `/docs/...` (script grep ou `pnpm dlx linkinator out --recurse` si dispo). Corriger les liens morts. Confirmer qu'aucun lien ne code `/latch` en dur.

- [ ] **Step 2 : Premier déploiement réel + vérif basePath**

Après merge sur `main`, vérifier le run CI (`docs` + `deploy-docs` verts), puis charger `https://owlnext-fr.github.io/latch/docs/quickstart/` et **confirmer en DevTools que `_next/` se charge** (pas de 404 d'asset). Si 404 → revoir `basePath`/`assetPrefix`/`.nojekyll` (Task 1).

- [ ] **Step 3 : Aligner le lien doc dans le produit**

Mettre à jour `README.md` (section doc « à venir Phase 8 ») et vérifier `frontend/src/lib/links.ts` `DOCS_URL` → `https://owlnext-fr.github.io/latch`. (Si `links.ts` change, régénérer/tester le front : `cd frontend && pnpm build`.)

- [ ] **Step 4 : Mémoire projet**

- `docs/INDEX.md` : section Phase 8 (site doc livré — pages, infra CI, URL).
- `docs/ROADMAP.md` : **Phase 8 ✅ LIVRÉE** + critères de sortie.
- `docs/ENVIRONMENT.md` : URL doc `owlnext-fr.github.io/latch`, jobs CI `docs`/`deploy-docs`, Pages = GitHub Actions, app `public_docs/` (Node 24, pnpm).
- `docs/QUIRKS.md` : pièges rencontrés (basePath/`.nojekyll`, recherche statique export, scaffold Fumadocs).
- `docs/HANDOFF.md` : entrée datée (dernière chose faite, suspens, prochaine, notes).

- [ ] **Step 5 : Commit**

```bash
rtk git add README.md frontend/src/lib/links.ts docs && rtk git commit -m "📝 docs(phase-8): site doc livré — liens, README, mémoire projet (Phase 8 ✅)"
```

---

## Self-Review (couverture spec)

- Spec §4.1 (séparation landing/docs) → Tasks 1,3,4. §4.2 (7 sections landing) → Task 3. §4.3 (nav/404/search) → Tasks 2,3,4.
- Spec §5 arborescence : `index`→T4 ; `quickstart`→T9 ; `troubleshooting`→T10 ; `deploy/*`→T7 ; `admin/*`→T8 ; `publish-from-claude/*`→T9 ; `how-it-works/*`→T6. **Toutes couvertes.**
- Spec §6.1 (export/basePath/.nojekyll) → T1. §6.2 (CI 2 jobs) → T5. §6.3 (Orama statique) → T4. §6.4 (captures) → T11.
- Spec §8 (definition of done) → T11 (captures), T12 (liens, déploiement, mémoire, URL).
- Global constraints (EN, source public_docs only, confidentialité, basePath, Context7, SHA-pin, identité) → rappelés en tête + dans les tasks concernées.
- **Reverse-proxies les 4** (Caddy/Nginx/Traefik/Apache) → T7 Step 4. **17 clés** → T7 Step 6. **2 tools MCP** → T9 Step 3.
