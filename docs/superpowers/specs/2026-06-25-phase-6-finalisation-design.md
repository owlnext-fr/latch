# Spec — Phase 6 : E2E, durcissement, packaging publiable

> Design validé le 2026-06-25. Dernière phase **métier** du périmètre v1 (la Phase 7
> « peaufinage graphique » et la Phase 8 « Fumadocs » restent indépendantes et hors
> périmètre ici). Le contrat (`docs/contrat-deploy.md`) reste la loi : ce spec
> *implémente et vérifie*, il ne crée pas de nouvelle décision d'archi ou de sécurité.

## 0. Contexte & objectif

Phase 5 (endpoint MCP + Settings) est mergée sur `main` et poussée. Cette phase
**clôt la v1 publiable** : couverture e2e de bout en bout (navigateur **et** transport
MCP réel), durcissement « hide » porté par l'application elle-même, et packaging FOSS
présentable (README complet avec captures + badges, CHANGELOG, sonar.tests finalisé).

Travail réalisé sur la branche `feat/phase-6-finalisation` (partie de `main`).

### Critères de sortie (definition of done de la phase)

- e2e **navigateur** (`/c` + unlock + bascule de version) **vert en CI** ;
- e2e **MCP transport HTTP réel** (`backend/tests/mcp_http.rs`) vert (suite backend) ;
- `robots.txt` servi par l'app + en-tête `X-Robots-Tag: noindex, nofollow` sur toutes
  les réponses, couverts par des tests d'intégration ;
- README refondu (badges CI + Quality Gate + Coverage + License, captures, quickstart,
  archi, sécurité) committé ; captures PNG sous `docs/assets/` ;
- `CHANGELOG.md` généré par git-cliff (`cliff.toml`), entrée `[0.1.0]` synthétisant
  Phases 0-6 ;
- `sonar-project.properties` : `sonar.tests=frontend/src,backend/tests` ;
- `cargo deny`/`audit` verts (vérification) ; gate SonarCloud `new_coverage ≥ 80%` verte ;
- mémoire mise à jour (INDEX, HANDOFF, ENVIRONMENT, QUIRKS si pièges, CONVENTIONS si
  nouveaux patterns ; stub Phase 8 ajouté à la ROADMAP).

### Hors périmètre (décidé)

- **Caddyfile d'exemple** : non livré. Le « hide » est porté par l'app (robots + en-tête),
  donc l'app est autonome ; le proxy reste à la charge de l'opérateur.
- **`deploy.sh` testé sur la box** : tâche **humaine** (la box est gérée hors-repo). On se
  limite à une relecture de fiabilité du script.
- **Image GHCR publiée** : conséquence automatique du merge sur `main` (job docker CI) ;
  pas une action manuelle de cette phase.
- **Page d'erreur stylée `/c`** et **Settings en side-panel** : reportés Phase 7 (déjà
  consignés dans `docs/ROADMAP.md`).

---

## 1. E2E navigateur — `/c` + unlock + bascule de version

**Fichier** : `frontend/e2e/serve-unlock.spec.ts` (nouveau ; réutilise le `webServer` et la
DB e2e `LATCH_E2E_DB` déjà configurés dans `frontend/playwright.config.ts`).

**Stratégie de setup** : chaque test prépare son état via l'**API admin réelle** (login
session → create project → deploy), pas de fixtures DB manuelles. Le HTML déployé est la
fixture existante `frontend/e2e/fixtures/proto.html`. Slugs et PIN sont lus depuis les
réponses API (le slug est généré, le PIN auto-généré est lu au détail).

**Scénarios** (contrat §6) :

1. **Projet libre** : créer un projet, désactiver le code (`clear_code`), déployer + activer →
   `GET /c/<slug>` répond **200**, `Cache-Control: no-store`, corps = contenu de `proto.html`.
2. **Projet protégé, sans cookie** : créer (code activé par défaut), déployer → `GET /c/<slug>`
   répond **200** et rend la **page de déverrouillage** (présence d'un marqueur DOM du formulaire
   OTP, pas le proto).
3. **Mauvais PIN** : soumettre un PIN erroné → la page reste sur l'unlock / signale l'erreur (cases
   OTP en erreur), le proto n'est **pas** servi.
4. **Bon PIN** : soumettre le PIN correct → cookie d'unlock posé → après reload, `/c/<slug>` sert le
   proto. (Vérifier via le rendu de la page, le flux auto-submit OTP est déjà en place.)
5. **Bascule de version** : déployer une **v2** (HTML distinct, ex. marqueur unique), l'activer via
   l'admin, recharger `/c/<slug>` (cookie valide) → le proto **v2** est servi (le marqueur v2 est
   présent, celui de v1 absent).

**Notes d'implémentation** :
- Utiliser le `request`/`APIRequestContext` de Playwright pour les appels admin de setup (login →
  cookie de session propagé), et `page.goto` pour les assertions navigateur sur `/c`.
- Garde Origin : les mutations admin exigent `Origin` same-origin ; le contexte navigateur Playwright
  l'émet naturellement, mais pour les appels `request` de setup ajouter l'en-tête `Origin` cohérent.
- Le rate-limit `/unlock` est in-memory et tolérant aux quelques essais d'un test ; ne pas marteler.

---

## 2. E2E MCP — transport Streamable HTTP réel (Approche A)

**Fichier** : `backend/tests/mcp_http.rs` (nouveau test d'intégration, suite `nextest`).

**Principe** : booter l'app Loco via le harness de test existant (même mécanisme que
`backend/tests/admin_api.rs` — `request::<App, _, _>` ou un serveur réel selon ce qui expose le
mount `/mcp`), puis dialoguer avec `POST /mcp` sur le **vrai transport Streamable HTTP** (JSON-RPC,
`LocalSessionManager`). Cela couvre exactement le chemin emprunté par Claude web.

**Client** : `reqwest` (déjà au lockfile en transitif ; à confirmer comme dev-dependency explicite
si nécessaire) émettant des requêtes JSON-RPC, avec les en-têtes requis par rmcp 1.8 :
`Accept: application/json, text/event-stream`, `Content-Type: application/json`. La réponse peut
être du JSON direct ou un flux SSE (`event: message\ndata: {...}`) — prévoir un petit parseur qui
extrait le payload JSON dans les deux cas. Le header de session (`Mcp-Session-Id` ou équivalent
exposé par rmcp 1.8 à l'`initialize`) est capturé et rejoué sur les requêtes suivantes.

> **Risque connu / à valider tôt** : le harness Loco standard (`request::<App>`) passe par un
> service axum ; il faut confirmer qu'il route bien le `nest_service("/mcp", …)` monté en
> `after_routes`. Si le harness court-circuite `after_routes`, replier sur un **vrai serveur HTTP**
> lié sur un port éphémère (spawn de l'app via `loco_rs::boot` + `tokio::spawn`) puis `reqwest` sur
> `http://127.0.0.1:<port>/mcp`. Cette bifurcation est à trancher à la première tâche d'implémentation
> (spike de 30 min) et à consigner dans QUIRKS.

**Scénarios** :

1. **`initialize`** : handshake réussi, capture de l'identifiant de session, `serverInfo.name == "latch"`.
2. **`tools/list`** : expose exactement `deploy_prototype` et `list_projects`.
3. **`tools/call deploy_prototype`** (token valide, slug préexistant créé en setup) : réponse
   `DeployResult { url, version, code_protected }` ; `url` = `http://<base>/c/<slug>` ; **jamais de PIN
   ni de hash** dans la réponse.
4. **`tools/call list_projects`** (token valide) : enveloppe objet `{ projects: [...] }` ; chaque entrée
   `ProjectSummary` sans PIN, sans hash, sans `id` DB.
5. **Gate token rejeté** : `deploy_prototype` et `list_projects` avec un `deploy_token` erroné → erreur
   JSON-RPC (pas d'effet de bord, pas de déploiement).

**Invariants de sécurité re-testés ici** (contrat §9.1/§9.2/§9.3) : token sur tous les tools, pas de
hash/PIN en sortie MCP.

---

## 3. Durcissement « hide » porté par l'app

### 3.1 `robots.txt`

Servi par le binaire Loco à `GET /robots.txt`, `Content-Type: text/plain`, corps :

```
User-agent: *
Disallow: /
```

Implémentation : route dédiée (handler renvoyant la chaîne statique) **ou** `ServeFile` sur un asset.
Préférer un **handler statique** (zéro dépendance au filesystem, robuste en distroless). Le placer
dans `after_routes` ou un petit controller `controllers/robots.rs` enregistré dans `app.rs`.

### 3.2 En-tête `X-Robots-Tag`

Un **layer** axum (via `tower_http::set_header::SetResponseHeaderLayer` ou `map_response`) appliqué
dans `after_routes` sur **tout** le routeur, posant `X-Robots-Tag: noindex, nofollow` sur chaque
réponse (admin, `/c`, `/api`, `/mcp`, statiques). À monter de manière à englober toutes les surfaces.

### 3.3 Tests

`backend/tests/hardening.rs` (nouveau) :
- `GET /robots.txt` → 200, `Content-Type: text/plain`, corps contient `Disallow: /` ;
- `X-Robots-Tag: noindex, nofollow` présent sur une réponse `/admin/`, une réponse `/api/*` (même 401),
  et une réponse `/c/<slug>`.

---

## 4. Packaging

### 4.1 README.md (refonte)

Tout en **français**, **aucun nom client**, docs **succinctes** renvoyant vers la future doc détaillée
(Fumadocs, Phase 8) via un lien marqué TBD — **placeholder explicite assumé** (`https://latch.owlnext.fr/docs`
*(documentation détaillée — à venir, Phase 8)*).

**Bandeau de badges** (en-tête) :

```md
[![CI](https://github.com/owlnext-fr/latch/actions/workflows/ci.yml/badge.svg)](https://github.com/owlnext-fr/latch/actions/workflows/ci.yml)
[![Quality Gate](https://sonarcloud.io/api/project_badges/measure?project=owlnext-fr_latch&metric=alert_status)](https://sonarcloud.io/summary/new_code?id=owlnext-fr_latch)
[![Coverage](https://sonarcloud.io/api/project_badges/measure?project=owlnext-fr_latch&metric=coverage)](https://sonarcloud.io/summary/new_code?id=owlnext-fr_latch)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#licence)
```

> **À vérifier à l'implémentation** : les badges SonarCloud ne s'affichent sans token que si le
> projet SonarCloud `owlnext-fr_latch` est **public**. Sinon, le rendre public (settings SonarCloud)
> ou n'embarquer que le badge CI. Ne jamais committer une URL de badge contenant un token.

**Sections** :

1. **En-tête** : titre `latch`, badges, accroche 2 lignes.
2. **Captures** : 1-2 PNG (`docs/assets/`) — liste admin + page d'unlock, données factices.
3. **Les trois surfaces** : `/c/<slug>`, `/admin`, `/mcp` (condensé).
4. **Quickstart** :
   - **Docker** : `cp .env.example .env` → renseigner les secrets → `docker compose up -d` → `/admin`.
     Tableau des **variables obligatoires en prod** (`ADMIN_USER`, `ADMIN_PASS`, `DEPLOY_TOKEN`,
     `LATCH_PUBLIC_BASE_URL`, `SESSION_SECRET`, `UNLOCK_COOKIE_SECRET`) + `openssl rand -hex 32`.
   - **Dev local** : `cd backend && cargo loco start` + `cd frontend && pnpm dev` → renvoi
     `docs/ENVIRONMENT.md`.
5. **Connecter Claude (MCP)** : 3 étapes (récupérer `mcp_url`/`deploy_token` dans `/admin/settings`,
   ajouter le connecteur, tester `list_projects`). 2-3 lignes + « → doc détaillée (TBD) ».
6. **Architecture** : schéma en couches (cœur agnostique HTTP + adaptateurs), 3 invariants de sécurité
   en puces, renvoi `docs/contrat-deploy.md` (source de vérité). Succinct.
7. **Stack** : backend (Loco/SeaORM/rmcp), front (React/Vite/shadcn). Condensé.
8. **Développement & Qualité** : commandes clés + gate Sonar `new_coverage ≥ 80%`, renvoi BOOTSTRAP/doc TBD.
9. **Déploiement** : GHCR public + `deploy.sh`, renvoi `docs/BOOTSTRAP.md §7-8`.
10. **Sécurité & confidentialité** (court) : `robots.txt` + `X-Robots-Tag` servis par l'app, le vrai
    gating reste l'auth.
11. **Licence / Changelog** : pointeurs `CHANGELOG.md`, dual-license MIT/Apache.

### 4.2 Captures (Playwright)

Script **hors suite e2e CI** (ex. `frontend/scripts/screenshots.spec.ts` lancé manuellement, ou une
recette documentée) : boote la stack, peuple des projets de **démo factices** (`Mon Projet`, `ACME`,
`demo`), capture :
- la **liste admin** (badges accès colorés) → `docs/assets/admin-list.png` ;
- la **page d'unlock** stylisée (OTP + `brand_name` générique) → `docs/assets/unlock.png`.

Données manifestement fictives, **jamais de nom client**. Les PNG sont committés (petits ; binaires
assumés). La génération n'est **pas** dans le job CI (pas de dépendance build sur des captures).

### 4.3 CHANGELOG.md (git-cliff)

- `cliff.toml` à la racine, configuration **Keep a Changelog** + parsing **commits conventionnels**.
- **Piège gitmoji** : les commits du repo sont `<gitmoji> <type>: <desc>` (ex. `✨ feat:`). git-cliff
  parse le conventional commit en tête de message ; le gitmoji en préfixe casse le parsing. **Fix** :
  un `commit_preprocessors` qui strippe l'emoji/espace de tête par regex **avant** les `commit_parsers`.
  À valider sur l'historique réel (`git-cliff --unreleased` / sur tag).
- Entrée `[0.1.0]` synthétisant Phases 0-6, regroupée par sections (Added / Changed / Security…),
  sans nom client. `[Unreleased]` en tête pour la suite.
- git-cliff est un **outil de génération** (binaire), pas une dépendance runtime ; documenté dans
  ENVIRONMENT (toolchain). Pas branché en CI obligatoire en v1 (génération à la release).

### 4.4 `.env.example` & `deploy.sh`

- `.env.example` : relecture, s'assurer que **toutes** les variables obligatoires prod sont présentes
  et commentées (déjà à jour Phase 5 — vérification).
- `deploy.sh` : relecture de fiabilité (pull/up/prune + garde `chown` idempotente déjà en place). Pas
  de test box (humain).

---

## 5. Qualité Sonar & supply-chain

- `sonar-project.properties` : `sonar.tests=frontend/src,backend/tests` (classe les tests d'intégration
  Rust comme tests ; aucun impact couverture — canal lcov séparé ; ne corrige pas les tests inline
  `#[cfg(test)]`, limite de granularité fichier connue).
- `cargo deny check` / `cargo audit` : **vérification** verte. Surveiller le backlog `Zlib`
  (transitives `utoipa-swagger-ui 9`) — si `cargo deny` rouge, ajouter la licence réellement rencontrée
  à l'`allow` de `deny.toml` (modèle liste blanche, cf. QUIRKS).
- Avant push : scan Sonar local possible (recette `docs/ENVIRONMENT.md §Scan local`, dont le remap des
  chemins lcov `/usr/src`).

---

## 6. Découpage en tâches (séquencement)

Un seul spec, exécuté en tâches indépendantes ; barrière finale = « tout vert en CI ».

1. **Durcissement app** : `robots.txt` + layer `X-Robots-Tag` + `backend/tests/hardening.rs`.
2. **E2E MCP** : spike transport (harness vs serveur réel) → `backend/tests/mcp_http.rs`.
3. **E2E navigateur** : `frontend/e2e/serve-unlock.spec.ts`.
4. **sonar.tests** : `sonar-project.properties` + vérif `cargo deny`/`audit`.
5. **Captures** : script Playwright → `docs/assets/*.png`.
6. **CHANGELOG** : `cliff.toml` + génération `[0.1.0]`.
7. **README** : refonte complète (badges, captures, sections, liens TBD Phase 8).
8. **Vérif finale + mémoire** : suite complète verte (fmt/clippy/nextest/vitest/playwright/deny),
   gate Sonar, mise à jour INDEX/HANDOFF/ENVIRONMENT/QUIRKS/CONVENTIONS + stub Phase 8 ROADMAP.
   Revue de branche avant merge.

Chaque tâche substantielle : implémentation + revue (Subagent-Driven ou revue ciblée selon l'ampleur).

## 7. Risques & points de vigilance

- **Transport MCP en test** (§2) : le point d'incertitude principal — spike en tout début.
- **git-cliff + gitmoji** (§4.3) : le preprocessor doit être validé sur l'historique réel.
- **Badges Sonar** (§4.1) : visibilité publique du projet SonarCloud requise.
- **Captures** (§4.2) : générées hors CI (sinon dépendance build fragile) ; ne jamais de nom client.
- **e2e local** : nécessite un backend buildé à jour (les mounts `/c`, `/mcp`, `/robots.txt` doivent
  être actifs ; `reuseExistingServer: false` en CI garantit un serveur neuf).
