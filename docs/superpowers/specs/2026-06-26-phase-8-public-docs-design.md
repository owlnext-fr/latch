# Spec — Phase 8 : site de documentation publique (`latch`)

> Design validé en brainstorming le 2026-06-26. Source d'origine : brief autonome
> `docs/external/fiche-public-docs.md`. Cette spec en est la version arbitrée et
> complétée (manques comblés, décisions tranchées, périmétrage exhaustif) pour permettre
> une implémentation menée en autonomie (install puis population du contenu).
>
> **Le contenu du site est en anglais** (décision §2). Cette spec, comme tout le kit
> `docs/`, reste en français.

## 1. Objectif

Construire la **vitrine publique de `latch`** : une **landing produit** soignée + une
**documentation structurée**, déployées en **statique sur GitHub Pages**. Artefact
**séparé** de l'app Rust : sa propre build, sa propre CI, son propre déploiement. Le
runtime de `latch` n'est pas touché.

But final : présenter `latch` comme un livrable FOSS sérieux, et donner aux trois publics
(opérateur, designer, contributeur) un chemin clair — jusqu'à *publier un prototype depuis
Claude*, qui est le différenciateur.

## 2. Décisions figées (brainstorming 2026-06-26)

| Sujet | Décision |
|---|---|
| **Outil** | **Fumadocs** (Next.js + MDX), **export statique** (`output: 'export'`). Node uniquement au build/CI. |
| **Hébergement** | **GitHub Pages** — **sous-chemin projet** `https://owlnext-fr.github.io/latch` par défaut. |
| **`basePath`** | `basePath` + `assetPrefix` **pilotés par variable d'env** → bascule vers domaine custom (`docs.latch.owlnext.fr`) = changer une seule variable. `.nojekyll` posé au build. |
| **Emplacement** | **`public_docs/`** dans le monorepo `owlnext-fr/latch`. |
| **Langue du site** | **Anglais uniquement** (portée FOSS, un seul corpus). |
| **Identité visuelle** | **Réutiliser l'identité produit** : logo `latch` SVG, palette **stone/oklch**, thème **clair/sombre** (next-themes, défaut `system`). |
| **Versioning doc** | **Version unique** (toujours alignée sur `main`). Pas de doc multi-version en v1. |
| **Recherche** | **Statique**, index **Orama** pré-rendu au build (pas de serveur). |
| **Reverse-proxies couverts** | **Caddy** (principal) + **Nginx** + **Traefik** + **Apache**. |
| **Analytics** | Aucun (privacy, FOSS). |
| **Captures** | Réutiliser le **harnais Playwright** existant (reproductibles). Flux Claude = **schéma annoté**, jamais de capture de claude.ai. |

## 3. Piège structurel à respecter — collision `docs/` interne

Le repo a **deux** dossiers de doc à ne jamais confondre :
- **`docs/`** = kit de contrôle **interne** (contrat, BOOTSTRAP, HANDOFF, QUIRKS…) — **jamais publié**.
- **`public_docs/content/`** = **seule** source du site public.

Fumadocs ne doit sourcer **que** `public_docs/content/`. Toute dérivation depuis `docs/`
(ex. réécrire `architecture.mdx` à partir du contrat) se fait **à la main**, en copiant
l'information pertinente — jamais en important le fichier interne.

## 4. Architecture du site

### 4.1 Séparation landing / docs

Fumadocs/Next sépare deux surfaces dans `app/` :

```
public_docs/
  app/
    (home)/
      page.tsx              # LANDING produit (marketing) — route "/"
    docs/
      [[...slug]]/page.tsx  # rendu des pages MDX — routes "/docs/**"
      layout.tsx            # layout docs (sidebar + TOC)
    layout.config.tsx       # nav partagée (liens, logo, GitHub, version, theme, search)
    layout.tsx              # RootProvider (thème, search)
    global.css              # tokens stone/oklch + Tailwind
  content/docs/             # ← SEULE source de contenu (voir §5)
  lib/
    source.ts               # loader Fumadocs (content/docs)
  public/
    img/                    # captures + assets (logo, schéma flux Claude)
  source.config.ts          # config Fumadocs MDX
  next.config.mjs           # output export, basePath/assetPrefix env, images unoptimized
  package.json              # app isolée (Node 24, pnpm) — distincte de frontend/
  tsconfig.json
  .nojekyll                 # (généré/commit) — évite le pipeline Jekyll de Pages
```

> **Pourquoi une page d'intro docs distincte de la landing** : la home devient une landing
> marketing (hors `content/`). Les docs ont donc besoin de leur propre page d'accueil
> (`content/docs/index.mdx`) qui présente « les trois surfaces » et oriente vers le
> quickstart. C'est un manque créé par la décision « home = landing » — comblé ici.

### 4.2 Landing `/` — structure

Réutilise l'identité produit. Sections, de haut en bas :

1. **Hero** — logo `latch`, pitch une phrase (« Serve single-file HTML prototypes behind a
   controlled host, with versioning and optional per-project access codes. »), deux CTA :
   **Get Started** (→ `/docs/quickstart`) et **GitHub** (→ repo). Toggle thème + lien Docs dans la nav.
2. **Captures** — `admin-list.png` + `unlock.png` (réutilisées depuis le harnais Playwright).
3. **Features** (grille) — les **trois surfaces** (serving `/c`, admin, MCP), **publish from
   Claude** (le différenciateur MCP), **code d'accès PIN** (deux états de `/c`), **fail-secure
   secrets**, **binaire Rust unique / image distroless**, **FOSS dual-license**.
4. **Publish-from-Claude highlight** — bande dédiée : déployer un proto en une phrase depuis Claude.
5. **How it works** (teaser) — l'archi en couches en 3 lignes → lien `/docs/how-it-works/architecture`.
6. **CTA finale** — *Get Started* + bloc copiable `docker pull ghcr.io/owlnext-fr/latch`.
7. **Footer** — GitHub · License (MIT/Apache-2.0) · CHANGELOG.

### 4.3 Navigation & chrome

- **Header** partagé (landing + docs) : logo (→ `/`), lien **Docs**, lien **GitHub**, **version**
  (lue depuis le `package.json` racine ou figée), **toggle thème**, **search** (Orama).
- **Sidebar docs** : ordre piloté par des `meta.json` par dossier (voir §5).
- **404** : page d'erreur statique Next (`app/not-found.tsx`) — nécessaire en export.

## 5. Arborescence de contenu (`content/docs/`) — exhaustive

> En gras les **ajouts** par rapport au brief §5. Chaque page indique sa **source** et ses
> **points clés**. Sources internes citées pour la rédaction ; le contenu publié est réécrit
> en anglais, orienté public, **sans jamais exposer de nom client**.

```
content/docs/
  meta.json                    # ordre racine : index, quickstart, deploy, admin,
                               #   publish-from-claude, how-it-works, troubleshooting
  index.mdx                    # Docs overview : "What is latch", les trois surfaces, plan de lecture par public
  quickstart.mdx               # Chemin doré bout-en-bout (voir détail ci-dessous)
  troubleshooting.mdx          # (NEW) FAQ / modes d'échec concrets

  deploy/                      # public OPÉRATEUR
    meta.json
    docker.mdx                 # conteneur simple (run + env minimal)
    docker-compose.mdx         # compose + volume data/ + .env (recommandé)
    reverse-proxy.mdx          # (NEW) Caddy / Nginx / Traefik / Apache + en-têtes "hide"
    from-source.mdx            # build backend (cargo) + front (pnpm build) — PAS Trunk
    configuration.mdx          # référence env exhaustive (17 clés)
    backup-upgrade.mdx         # volume data/ (sqlite + html), migrate au boot
    releases.mdx               # (NEW) tags GHCR, LATCH_IMAGE_TAG, rollback

  admin/                       # public DESIGNER (pilote l'admin)
    meta.json
    projects.mdx               # créer (side-panel), slug + suffixe, largeur, lecture seule
    access-codes.mdx           # PIN auto-généré 6 chiffres, deux états de /c, rotation = révocation
    versions.mdx               # déployer (upload), prévisualiser, basculer l'active
    co-branding.mdx            # brand_name sur la page de déverrouillage

  publish-from-claude/         # public DESIGNER (publie depuis Claude)
    meta.json
    connect-mcp.mdx            # brancher le connecteur, récupérer mcp_url + deploy_token (Settings)
    tools-reference.mdx        # (NEW, fusion) deploy_prototype (activate) + list_projects
    why-token-not-oauth.mdx    # Modèle 1 expliqué + note sécu

  how-it-works/                # public CONTRIBUTEUR / curieux
    meta.json
    architecture.mdx           # archi en couches (réécrite du contrat, pour public)
    security-model.mdx         # deux cookies, rate-limit load-bearing, CVE/allowed_hosts, invariants §9
    contributing.mdx           # build, tests par couche, CI, gate Sonar
```

### 5.1 Détail des pages — source & points clés

**`index.mdx` (docs overview)** — *Source : README, contrat §1/§4/§5/§6.* Les trois surfaces
en une phrase chacune ; tableau « je suis opérateur / designer / contributeur → commence ici ».

**`quickstart.mdx`** — *Source : README Quickstart, ENVIRONMENT, contrat §5-6.* Chemin doré :
(1) `docker compose up` avec `.env` minimal → (2) login admin → (3) créer un 1er projet (slug +
PIN) → (4) brancher le connecteur MCP (mcp_url + deploy_token depuis Settings) → (5) `deploy_prototype`
depuis Claude → (6) ouvrir le lien client `/c/<slug>` et déverrouiller. Étape Claude illustrée
par **schéma annoté + résultat** (la version qui apparaît côté admin), pas de capture claude.ai.

**`deploy/docker.mdx`** — `docker run` minimal, image GHCR publique (pas de `docker login`),
volume `/data`, les 6 secrets obligatoires.

**`deploy/docker-compose.mdx`** — *Source : `docker-compose.yml`, `deploy.sh`, `.env.example`.*
Compose complet, volume `data/`, `.env`, `deploy.sh` (pull + up + prune), entrypoint migrate→start.

**`deploy/reverse-proxy.mdx` (NEW)** — *Source : BOOTSTRAP §7/§9, contrat §6.* Pour chacun de
**Caddy / Nginx / Traefik / Apache** : snippet TLS + reverse proxy vers le binaire (port `PORT`),
forward des en-têtes (`X-Forwarded-For`/`X-Real-IP` — load-bearing pour le rate-limit et
`SmartIpKeyExtractor`), pose de `X-Robots-Tag: noindex, nofollow`, service de `robots.txt`
(`Disallow: /`). Note `Host` forwardé cohérent avec `LATCH_PUBLIC_BASE_URL` (allowed_hosts MCP).

**`deploy/from-source.mdx`** — *Source : BOOTSTRAP §3, ENVIRONMENT.* `cargo loco start` (backend,
depuis `backend/`), `pnpm build` (frontend, **plus de Trunk/Yew** — React/Vite/pnpm 9.15.9, Node 24),
`LATCH_SPA_DIST`. Mention build Docker local.

**`deploy/configuration.mdx`** — *Source : ENVIRONMENT, `.env.example` (17 clés).* Référence
**exhaustive**, par groupe : **obligatoires prod (fail-secure)** [`ADMIN_USER`, `ADMIN_PASS`,
`DEPLOY_TOKEN`, `LATCH_PUBLIC_BASE_URL`, `SESSION_SECRET`, `UNLOCK_COOKIE_SECRET`] ; **réglages**
[`LATCH_UNLOCK_TTL_DAYS`, `LATCH_UNLOCK_RL_IP_BURST`, `LATCH_UNLOCK_RL_IP_PER_SECOND`,
`LATCH_UNLOCK_RL_SLUG_BURST`, `LATCH_UNLOCK_RL_SLUG_PERIOD_SECS`, `LATCH_BODY_LIMIT`,
`LATCH_STORAGE_ROOT`, `LATCH_SPA_DIST`, `DATABASE_URL`, `PORT`, `LATCH_IMAGE_TAG`]. Chaque clé :
rôle, défaut, « obligatoire prod » oui/non, comment générer (`openssl rand -hex 32`).

**`deploy/backup-upgrade.mdx`** — Volume `data/` (sqlite **et** HTML ensemble) = unité de sauvegarde ;
upgrade = `deploy.sh` (migrate auto au boot) ; pin de version.

**`deploy/releases.mdx` (NEW)** — *Source : BOOTSTRAP §6/§8, ENVIRONMENT (GHCR).* Schéma de tags
GHCR (`vX.Y.Z` → `X.Y.Z`/`X.Y`/`latest`/`sha-…` ; `main` → `main`/`sha-…`), pin via `LATCH_IMAGE_TAG`,
recette de rollback (remettre l'ancien tag + `deploy.sh`), lien CHANGELOG.

**`admin/projects.mdx`** — *Source : contrat §7.* Créer en side-panel, slug lisible + suffixe 8 base62
(lecture seule v1), page détail lecture seule.

**`admin/access-codes.mdx`** — *Source : contrat §3/§6/§9.* PIN auto-généré 6 chiffres, `code_enabled`
vrai par défaut, deux états de `/c`, page de déverrouillage, **rotation du PIN = révocation des cookies**.

**`admin/versions.mdx`** — *Source : contrat §7/§8.* Déployer (upload HTML, case activer),
prévisualiser (admin-only, no-store), basculer l'active (transactionnel), supprimer (refuse si active).

**`admin/co-branding.mdx`** — `brand_name` sur la page de déverrouillage (« Prototype prepared for {brand} »).

**`publish-from-claude/connect-mcp.mdx`** — *Source : README, ENVIRONMENT §Connexion, contrat §5.*
Récupérer `mcp_url` + `deploy_token` dans **Settings** ; renseigner l'URL MCP côté Claude ; pas
d'OAuth ni de header — auth dans l'argument. Tester avec `list_projects`.

**`publish-from-claude/tools-reference.mdx` (NEW, fusion)** — *Source : contrat §5.1, CONVENTIONS MCP.*
Les **deux** tools : `deploy_prototype(slug, html, deploy_token, activate?)` (slug doit préexister,
`activate` défaut `true`, réponse `DeployResult { url, version, code_protected }` — **jamais de PIN/hash**) ;
`list_projects(deploy_token)` (enveloppe objet `{ projects: [...] }`, `ProjectSummary` sans PIN/hash/id).
Comble le manque : le brief n'avait que `deploy_prototype`.

**`publish-from-claude/why-token-not-oauth.mdx`** — *Source : contrat §5, BACKLOG (Modèle 2).* Modèle 1
(token en argument) expliqué, pourquoi suffisant ici, Modèle 2 OAuth en évolution future.

**`how-it-works/architecture.mdx`** — *Source : contrat §1/§2.* Archi en couches / hexagonale légère :
cœur agnostique HTTP, adaptateurs entrants fins, adaptateurs sortants (SeaORM + `Storage`). Réécrite public.

**`how-it-works/security-model.mdx`** — *Source : contrat §9, BOOTSTRAP §9.* Deux cookies (session admin,
déverrouillage), rate-limit **load-bearing** sur unlock + login, CVE Host-header / `allowed_hosts`,
fail-secure secrets, invariants (jamais de hash, PIN cantonné au détail), robots/X-Robots-Tag.

**`how-it-works/contributing.mdx`** — *Source : BOOTSTRAP §3-6, ENVIRONMENT.* Build par couche, tests
(unit/intégration/MCP/Vitest/Playwright), gate SonarCloud `new_coverage ≥ 80%`, commits gitmoji.

**`troubleshooting.mdx` (NEW)** — *Source : ENVIRONMENT, QUIRKS, contrat.* Modes d'échec concrets :
boot refusé *fail-secure* (secret obligatoire manquant) ; **413** (`LATCH_BODY_LIMIT` trop bas pour un gros proto) ;
**MCP host rejeté** (`allowed_hosts` ≠ `LATCH_PUBLIC_BASE_URL`) ; **lockout** rate-limit unlock (compteurs in-memory,
reset au reboot) ; **404 `/c`** (slug inconnu / pas de version active) ; cookie unlock invalidé après rotation PIN.

## 6. Infrastructure build & déploiement

### 6.1 Next/Fumadocs (export statique)

- `next.config.mjs` : `output: 'export'`, `images: { unoptimized: true }`,
  `basePath: process.env.DOCS_BASE_PATH ?? '/latch'`, `assetPrefix` dérivé du même.
  Domaine custom plus tard → `DOCS_BASE_PATH=''` + CNAME. `.nojekyll` à la racine de l'export.
- Scaffold : `pnpm create fumadocs-app` (template Next.js). App **isolée** dans `public_docs/`
  (son `package.json`, son lockfile) — **n'est pas** un membre du workspace Rust, ni lié à `frontend/`.
- Node 24, pnpm (aligné repo). Vérifier la version Fumadocs courante **via Context7** au scaffold
  (l'API `source.config.ts` / loader bouge entre majors).

### 6.2 CI — `deploy-docs.yml` (séparée de la CI Rust)

- Déclencheur : `push` sur `main`, **filtré `paths: public_docs/**`** (+ `workflow_dispatch`).
- Étapes : checkout → setup Node 24 + pnpm → `pnpm install` (dans `public_docs/`) → `pnpm build`
  (export statique + index Orama) → upload artefact → `actions/deploy-pages`.
- **Permissions** `pages: write`, `id-token: write` ; environnement `github-pages`.
- Actions **épinglées par SHA** (cohérent avec la politique supply-chain du repo, BOOTSTRAP §6).
- La CI Rust (`ci.yml`) n'est **pas** touchée — deux pipelines, deux déclencheurs. Pré-requis humain :
  activer **Pages = GitHub Actions** dans les settings du repo (consigné dans ENVIRONMENT à la livraison).

### 6.3 Recherche

Orama statique, index pré-rendu au build (intégration Fumadocs native). Pas de service externe.

### 6.4 Captures

Réutiliser `frontend/e2e/screenshots.capture.ts` (Phase 6, `CAPTURE=1`). Copier/produire les PNG
dans `public_docs/public/img/`. Flux Claude = **schéma annoté** (SVG/diagramme), jamais de capture
de l'interface Anthropic.

## 7. Découpage en unités (pour le plan d'implémentation)

Chaque unité a un périmètre clair et testable indépendamment :

1. **Scaffold & config** — `public_docs/` Fumadocs, `next.config.mjs` (export, basePath env), thème
   stone/oklch + logo, build local OK (`pnpm build` produit `out/`).
2. **Chrome & landing** — nav header, footer, `app/(home)/page.tsx` (les 7 sections §4.2), 404.
3. **Recherche & navigation docs** — Orama, `meta.json`, sidebar.
4. **CI & déploiement Pages** — `deploy-docs.yml`, `.nojekyll`, déploiement vert sur l'URL §2.
5. **Contenu — `how-it-works/` + `deploy/`** (dérivable du contrat/BOOTSTRAP, sans dépendre des écrans).
6. **Contenu — `admin/` + `publish-from-claude/` + `quickstart` + `troubleshooting`** (captures + écrans réels).
7. **Captures** — script de génération + intégration des PNG + schéma flux Claude.
8. **Finitions** — liens internes vérifiés, recherche fonctionnelle, README pointe vers l'URL, mémoire projet à jour.

> L'ordre install→population voulu par l'utilisateur correspond à : unités 1-4 (install) puis 5-7 (population),
> 8 en clôture. Itérable en autonomie une fois cette spec validée.

## 8. Critères de « terminé » (site)

- Build statique OK, **déployé sur GitHub Pages** à l'URL §2, **styles/scripts servis** (aucun 404 d'asset —
  le piège `basePath`/`assetPrefix`/`.nojekyll` est traité).
- **Landing produit** présente (hero + features + CTA Get Started), identité produit (logo, stone/oklch, clair/sombre).
- **Toutes les pages §5 présentes** et rédigées en anglais ; reverse-proxy couvre **Caddy/Nginx/Traefik/Apache** ;
  `configuration.mdx` couvre les **17 clés** ; `tools-reference` couvre les **deux** tools.
- **Recherche** fonctionnelle ; **liens internes** vérifiés ; **404** statique en place.
- Captures à jour (harnais Playwright) ; schéma pour le flux Claude.
- Contenu sourcé **uniquement** de `public_docs/content/` ; **zéro** contenu du `docs/` interne publié ;
  **zéro nom client** (placeholders fictifs uniquement).
- CI `deploy-docs.yml` verte ; CI Rust intacte.
- Mémoire projet mise à jour (INDEX, ROADMAP Phase 8 ✅, ENVIRONMENT §Pages/URL, HANDOFF, QUIRKS si pièges).

## 9. Hors périmètre (v1 du site)

- Doc **multi-version** (versioning Fumadocs) — version unique alignée `main`.
- **i18n** du site (FR/EN) — EN uniquement ; bascule i18n = évolution future si besoin.
- **Analytics**, recherche serveur, commentaires.
- Automatisation de captures de **claude.ai** (interdit — fragilité/ToS).
- Landing **custom poussée** (animations élaborées) — on reste sur l'identité produit turnkey.

## 10. Risques & points d'attention

- **`basePath` GitHub Pages projet** : *la* source classique de « le site se déploie mais
  styles/scripts en 404 ». Traité par `basePath`/`assetPrefix` env + `.nojekyll`. À vérifier au 1er deploy.
- **Fumadocs bouge vite** (loader, `source.config.ts`) : résoudre la version via **Context7** au scaffold,
  épingler dans `public_docs/package.json`.
- **Pré-requis humain** : activer Pages = « GitHub Actions » dans les settings repo (non scriptable).
- **Lien doc dans le produit** : le bouton « ? » de l'admin et le README pointent déjà vers une URL doc —
  l'aligner sur l'URL §2 une fois en ligne.
- **Confidentialité** : règle CLAUDE.md non-négociable — aucun nom client dans le contenu, placeholders fictifs.
