# Quirks — pièges connus & contournements

> Ce qui a mordu (ou mordra) si on l'oublie. Une entrée = un piège + son contournement.
> Seedé avec les points identifiés au cadrage, avant tout code.

## Session `axum_session` non partagée entre `request` et `page` en e2e (2026-06-30)

Dans les tests Playwright, le cookie de session admin posé par `apiLogin(request)` (via `APIRequestContext`) **n'est pas automatiquement visible** dans le contexte navigateur (`page`). Les routes admin protégées par `AdminAuth` renvoient 401 quand navigué via `page.goto('/admin/...')` même après `apiLogin(request)`.

**Contournement** : utiliser `pageLogin(page)` (formulaire `/admin/login` navigué dans le browser) pour tout test qui accède à l'admin SPA via `page`. `apiLogin(request)` reste utile pour les opérations API headless (créer projet, déployer, seed commentaire).

```ts
// ✅ pattern correct pour tests e2e admin
await apiLogin(request)       // pour les API headless
const project = await createProject(request, baseURL!, {...})
await pageLogin(page)         // pour la navigation browser admin
await page.goto(`/admin/projects/${project.id}/...`)
```

La cause profonde : `axum_session` utilise un mécanisme de session côté serveur avec deux cookies (`latch_admin` + `store`). Le cookie `store` contenant les données de session chiffrées semble ne pas être retransmis correctement entre les deux contextes Playwright dans ce setup.

## ~~Clés i18n `comment.*` absentes du bundle admin~~ — **RÉSOLU par L2 (`mergeFragmentGlob`, 2026-06-30)**

> **Entrée L1 (maintenant FAUSSE) :** le bundle admin ne chargeait que `src/i18n/locales/admin/*.json` ; les clés `comment.*` s'affichaient en texte littéral dans la page Review.

**Corrigé en L2 (commit `49dc0f2`)** : le module partagé `src/comments/` possède ses clés i18n dans `src/i18n/locales/comments/{en,fr}.json`. Ces fichiers sont fusionnés :
- dans l'**instance admin** (singleton `src/i18n/index.ts`) via `mergeFragmentGlob(resources, glob)` ;
- dans les **bundles `createBundleI18n`** (shell) via le paramètre optionnel `fragmentGlob?`.

Règle : **tout nouveau consommateur du module partagé doit fusionner ce glob** (`import.meta.glob('.../locales/comments/*.json')`), sinon il affiche les clés brutes. Les tests e2e admin assertent désormais le texte traduit réel.

## e2e : rate-limit `/api/login` (429) — résolu par cran env (2026-07-01)

`backend/src/controllers/auth.rs` applique `burst_size=5/per_second=2` par IP sur `POST /api/login` (défauts load-bearing : le test `login_is_rate_limited` sous `cargo nextest` en dépend — NE PAS changer les défauts). La suite e2e enchaîne les logins depuis `127.0.0.1` → 429 quand plusieurs specs se suivent dans la même session Playwright.

**Fix** : le rate-limit est réglable via `LATCH_LOGIN_RL_BURST` / `LATCH_LOGIN_RL_PER_SECOND`. Le `webServer.command` de `frontend/playwright.config.ts` pose `LATCH_LOGIN_RL_BURST=100000`, ce qui rend le throttle login inopérant en e2e sans toucher le défaut de prod. Les helpers `apiLogin`/`pageLogin` ont été simplifiés (plus de retry-on-429). **Ne PAS remettre le retry** : si un 429 réapparaît en e2e, vérifier que la var est bien transmise au processus serveur (logs webServer).

## Shims jsdom requis pour la couche commentaire (2026-06-30)

Plusieurs APIs DOM manquent en jsdom et font planter les tests si non stubées :

- **`IntersectionObserver`** : pas implémenté en jsdom. Shim global ajouté dans `vitest.setup.ts` (`window.IntersectionObserver = class { ... }`).
- **`getBoundingClientRect`** renvoie toujours `{0,0,0,0}` en jsdom. Stubber par test (ex. `vi.spyOn(el, 'getBoundingClientRect').mockReturnValue({...})`) quand la logique de positionnement en dépend.
- **`@floating-ui/dom computePosition`** retombe en `(0,0)` sans layout réel (aucun impact sur les tests de formulaire — pas de piège de valeur inattendue, mais les snapshots de position ne sont pas significatifs en jsdom).

## `erasableSyntaxOnly` dans tsconfig — interdit les parameter properties (2026-06-30)

Le `tsconfig.json` frontend a `"erasableSyntaxOnly": true`. Cela **interdit** les *parameter properties* TypeScript (`constructor(private x: T)`). Utiliser des champs explicites à la place :

```ts
// ❌ interdit
constructor(private slug: string) {}

// ✅ ok
private slug: string;
constructor(slug: string) { this.slug = slug; }
```

## `eslint-plugin-react-hooks` v7 STRICT — deux nouvelles règles (2026-06-30)

La v7 est plus stricte :

- **`react-hooks/refs`** : interdit d'écrire dans une ref pendant le render (même via `useRef` + assignation directe dans le corps du composant).
- **`react-hooks/set-state-in-effect`** : interdit d'appeler `setState` directement dans le corps d'un `useEffect` (sans `if` ou condition) — favorise la dérivation d'état.

Tout dispatch React doit lancer `pnpm lint` pour vérifier la conformité.

## i18next v26 — formes plurielles CLDR (`_one`/`_other`, PAS `_plural`) (2026-06-30)

Depuis i18next v21+ / i18next-resources-for-ts v26, les clés plurielles suivent la convention **CLDR** :
- `_one` pour le singulier, `_other` pour le pluriel (et non plus `_plural` qui était l'ancienne convention).
- Les clés `_plural` **mortes** ne produisent pas d'erreur au runtime mais sont ignorées silencieusement → texte pluriel absent.

## Transposition iframe→shell pour le picker de commentaires (2026-06-30)

Le prototype est servi dans un `<iframe>` ; le shell hôte monte le module de commentaires. Le picker calcule la position du pin en deux étapes :

1. **`e.clientX / e.clientY`** : coordonnées en espace shell (viewport navigateur).
2. On ajoute le **rect de l'iframe** (`iframe.getBoundingClientRect()`) pour convertir vers l'espace du document hôte.

Erreur classique : utiliser `el.getBoundingClientRect()` côté proto (espace iframe — coordonnées relatives à l'origine de l'iframe, pas du shell). Le résultat est faux dès que l'iframe n'est pas positionnée à l'origine. Toujours calculer en espace shell.

## `resolve()` byTextQuote — TreeWalker peut renvoyer un conteneur (2026-06-30)

Dans `src/comments/anchor/resolve.ts`, la branche `byTextQuote` utilise un `TreeWalker` en mode `SHOW_ELEMENT`. Ce mode visite les nœuds ancêtres **avant** les feuilles → il peut renvoyer un élément conteneur (ex. `<p>`, `<div>`) au lieu de l'élément feuille le plus spécifique. C'est le **dernier palier de la cascade** (`approximate`), donc l'impact est limité ; durcissement possible : préférer le candidat au `textContent` le plus court. La branche multi-match (`direct.length > 1`, gate STRONG ≥ 0.9) n'a pas de test dédié — à ajouter si elle dérive.

## `LATCH_STORAGE_ROOT` relatif → HTML écrits sur la couche éphémère du conteneur (2026-06-29)
**Symptôme** : en prod, après un redéploiement (`docker compose up -d`, hotfix, restart), **toutes les
versions** tombent en erreur : `GET /c/<slug>/raw` → **500** (page d'erreur serving) et
`GET /api/projects/{id}/versions/{n}/preview` → **404** JSON. La base est intacte (projets/versions
présents, login OK), seuls les **fichiers HTML** manquent. Log serveur :
`"raw: storage read failed","error":"resource not found"`.
**Cause** : `storage_from_ctx` (`web/mod.rs`) lit `LATCH_STORAGE_ROOT` avec un **défaut relatif** (`"data"`),
et `.env.example` livrait `./data`. Le `WORKDIR` de l'image runtime est **`/app`** → un chemin relatif
résout vers **`/app/data`**, soit la **couche d'écriture éphémère** du conteneur, PAS le volume monté
(`./data:/data`). La base SQLite, elle, a un chemin **absolu** (`sqlite:///data/latch.sqlite`) → elle vit
sur le volume et **persiste**. Résultat : DB et storage sur deux persistances distinctes. À la première
recréation du conteneur, `/app/data` est effacé → la DB pointe vers des fichiers HTML disparus.
Invisible jusqu'au premier `up -d`/restart (le fichier est là, au mauvais endroit, tant que le conteneur
vit). L'ordre storage-first de `deploy()` ne protège PAS contre ce cas (il garde la DB cohérente *dans une
même couche*, pas entre deux couches de persistance).
**Fix** : `LATCH_STORAGE_ROOT=/data` (chemin **absolu** sur le volume) dans le `.env` de prod.
**Données déjà perdues** : les HTML écrits dans `/app/data` avant le redémarrage sont **irrécupérables** ;
après correction de l'env, **re-déployer chaque proto** (MCP `deploy_prototype` ou upload admin) pour
réécrire le fichier sur le volume. **Vérif** sur la box : `ls -laR ./data` doit montrer les HTML de versions
À CÔTÉ de `latch.sqlite` ; s'il n'y a que la base, le storage part sur la couche éphémère.
`.env.example` corrigé (`/data` + commentaire « chemin absolu obligatoire »).

## Tous les protos en iframe via le shell — impacts (2026-06-29)

Depuis la Phase 9 (notes de version), `GET /c/<slug>` sert **toujours** un shell HTML qui
charge le prototype dans un `<iframe src="/c/<slug>/raw">`. Impacts à garder en tête :

- **`window.top`** : depuis le proto, `window.top` pointe vers le shell (pas vers le navigateur).
  Un proto qui teste `window === window.top` (anti-framing) verra `false` — il s'affichera vide ou
  en erreur. À documenter aux clients si pertinent.
- **Fullscreen API** : `requestFullscreen()` est bloqué par défaut dans un iframe sans
  `allow="fullscreen"`. Le shell devrait ajouter l'attribut si des protos l'utilisent.
- **CSP `frame-ancestors 'self'`** sur `/raw` : empêche d'embarquer le proto dans un contexte
  tiers, mais **ne bloque pas** le shell lui-même (même origine). C'est intentionnel.
- **Contournement pour tester** : l'admin peut utiliser la route de prévisualisation
  (`/api/projects/<id>/versions/<n>/preview`) qui sert l'HTML brut **sans** iframe, derrière la
  session admin — pratique pour vérifier un proto sans context iframe.

## `release_notes` rendu côté client uniquement — jamais HTML serveur (2026-06-29)

Le champ `versions.release_notes` est stocké **brut** (Markdown texte) en base. Le serveur ne
convertit jamais ce Markdown en HTML : ni dans les réponses JSON (`/c/<slug>/notes` renvoie
`notes_md` en chaîne brute), ni dans les DTOs admin. Le rendu Markdown se fait **exclusivement
côté client** via le composant `MarkdownView` restreint (`react-markdown` + `skipHtml +
allowedElements`). Raison : barrière XSS par construction — impossible d'injecter du HTML via
les notes si le HTML n'est jamais produit côté serveur. Ne jamais ajouter de rendu serveur.

## Site doc (public_docs/, Fumadocs) — pièges Phase 8 (2026-06-26)

- **Scaffold `create-fumadocs-app` interactif malgré les flags** : un prompt « Use `/src` directory? »
  reste non couvert par un flag. `</dev/null` ne valide PAS le défaut (EOF → exit sans rien créer).
  Le piloter via un **PTY** : `python3 -c "import pty; pty.spawn([...])"` en alimentant des `\r`.
  Template à viser : **`+next+fuma-docs-mdx+static`** (`--search orama --pm pnpm --no-git --install`) →
  câble déjà l'export statique ET la recherche statique. (Il génère un layout **`src/`**.)
- **basePath sous-chemin GitHub Pages** (repo projet → site sous `/latch`) : dans `next.config.mjs`,
  `basePath` + `assetPrefix` (= `/latch`) **explicites** + **`public/.nojekyll`** (sinon Jekyll mange
  `_next/` → 404 d'assets). Liens internes **root-relative** (`/docs/...`) → **jamais** `/latch` en dur.
- **Images sous basePath** : un `<img src="/img/x.png">` brut **casse** (404 sous `/latch`). Les
  référencer via **import statique** → URL `/latch/_next/static/media/...` préfixée : MDX `![](/img/x.png)`
  (fumadocs-mdx les transforme en import) ; TSX `import img from '...'; <img src={img.src}>`. Corollaire :
  fumadocs-mdx **résout `![](/img/x.png)` comme un module** → le fichier **doit exister au build**.
- **Recherche statique** (export) : `app/api/search/route.ts` = `export const revalidate = false; export const { staticGET: GET } = createFromSource(source)` + client `oramaStaticClient` (posé par le template `+static`).
- **MDX = JSX** : `{…}` = expression JS, `<mot>` = balise → en prose/frontmatter, `{brand name}` et
  `<slug>` **cassent le build**. Les mettre en backticks (inline code) les protège.
- **Shiki** : pas de grammaire `caddy` → bloc Caddyfile en ` ```text ` (sinon `Language 'caddy' not found`).
- **lucide-react sans `Github`** (déjà connu côté frontend) → composant `GithubIcon` maison inline.

## Storage dev = `backend/data`, pas `/data` racine — gitignore (2026-06-26)
`LATCH_STORAGE_ROOT` défaut `"data"` (relatif au CWD). En dev on lance `cargo loco start` **depuis `backend/`** → le storage des HTML de versions vit dans `backend/data`. Or `.gitignore` n'avait que `/data` (ancré racine = volume Docker prod) → `backend/data` n'était PAS ignoré (risque de commit accidentel des protos déployés). **Fix** : ajouter `backend/data/` à `.gitignore`. Prod (image) = volume monté `/data` (toujours couvert par `/data`).

## Logo qui suit le thème : SVG inline `currentColor`, jamais `<img src>` (2026-06-26)
`currentColor` n'est résolu que pour un SVG **inline dans le DOM** ; un `<img src="logo.svg">` ne peut pas hériter de la couleur du texte. Pour un logo qui bascule clair/sombre, inliner le SVG (composant `Logo`) avec `fill="currentColor"` → suit `text-foreground`. Le favicon (forcément un fichier, pas de contexte DOM) s'adapte autrement : `<style> @media (prefers-color-scheme: dark) { path { fill: … } }</style>` à l'intérieur du SVG (suit le thème navigateur/OS, indépendant du toggle in-app).

## Claude Code : `claude mcp add` en cours de session ne charge pas les tools (2026-06-26)
Les serveurs MCP sont chargés **au démarrage de session**. `claude mcp add <srv>` l'enregistre (et `claude mcp list` le montre connecté ✔) mais ses tools n'apparaissent PAS dans la session courante ; `/mcp` ne **reconnecte que les serveurs déjà chargés**, il n'en charge pas de nouveaux. Pour les avoir en natif → redémarrer la session. Contournement sans perdre le contexte : appeler le même endpoint `/mcp` via un client HTTP (transport Streamable HTTP identique). NB transport rmcp : `Accept: application/json, text/event-stream`, session via header `Mcp-Session-Id`, **`Host` doit matcher `allowed_hosts`** (dérivé de `LATCH_PUBLIC_BASE_URL`), résultat des tools dans `result.structuredContent`.

## Playwright : `testMatch` par défaut = `*.spec.ts` seulement (2026-06-25)
Sans configuration explicite, Playwright ne découvre que les fichiers `*.spec.ts` (et `*.spec.js`). Un fichier nommé `*.capture.ts` n'est pas trouvé → `No tests found` silencieux. **Fix** : ajouter `testMatch: /.*\.(spec|capture)\.ts$/` dans `playwright.config.ts`. Cette option étend la découverte sans perturber les specs CI existantes.

## Playwright captures : `CAPTURE=1` ≠ `CI=1` — rôles distincts (2026-06-25)
Les tests de capture (`e2e/screenshots.capture.ts`) utilisent deux variables d'env aux rôles indépendants :
- **`CAPTURE=1`** : contrôle le **skip** du test (`test.skip(!process.env.CAPTURE, "...")`). Sans cette variable, les tests sont découverts (grâce à `testMatch`) mais skippés immédiatement — zéro temps de build.
- **`CI=1`** : active `reuseExistingServer: true` dans `playwright.config.ts`. Permet de réutiliser un serveur déjà lancé (évite un rebuild complet). Orthogonal au skip.
**Commande de capture** : `CAPTURE=1 pnpm exec playwright test screenshots.capture` (depuis `frontend/`). En CI on peut combiner `CAPTURE=1 CI=1 …` pour réutiliser le serveur existant, mais seul `CAPTURE=1` est obligatoire pour déclencher les captures. Ne pas documenter `CAPTURE=1 CI=1` comme indissociables — le skip est contrôlé par `CAPTURE` seul.

## SonarCloud : Automatic Analysis EXCLUSIVE du scanner CI (2026-06-25)
SonarCloud propose deux modes d'analyse : **Automatic Analysis** (déclenché par SonarCloud lui-même sur chaque push, sans configuration) et **scanner CI** (job GitHub Actions qui pilote `sonar-scanner`). Les deux sont **mutuellement exclusifs** : activer les deux produit une erreur `You are running CI analysis while Automatic Analysis is enabled`. **Procédure** : désactiver l'Automatic Analysis dans les settings SonarCloud (`Administration > Analysis Method > Automatic Analysis = OFF`) AVANT de créer le job CI. Une fois désactivé, le job CI devient l'unique source de scan.

## SonarCloud : `sonar.rust.clippy.enabled=false` obligatoire (2026-06-25)
Sans `sonar.rust.clippy.enabled=false` dans `sonar-project.properties`, le scanner `sonar-scanner-cli` tente de lancer `cargo clippy` **depuis le container sonar-scanner** (qui ne contient pas `cargo`). Résultat : erreur `cargo: command not found` et scan avorté. Clippy reste bloquant dans le job `fmt-clippy` — la couverture lint n'est pas perdue, simplement dissociée du scan Sonar. Règle : **toujours poser ce flag** dans les projets Rust.

## Couverture Rust → SonarCloud : `cargo llvm-cov` + `sonar.rust.lcov.reportPaths` (2026-06-25)
SonarCloud consomme la couverture Rust via le format **lcov** (`sonar.rust.lcov.reportPaths`). Workflow CI : `cargo llvm-cov nextest --lcov --output-path backend-lcov.info` (job `test-backend`) → `actions/upload-artifact` → `actions/download-artifact` dans le job `sonar` → `pnpm test:cov` produit `coverage/lcov.info` (front) → le scanner consolide les deux. Prérequis toolchain : component **`llvm-tools-preview`** ajouté à `rust-toolchain` + `taiki-e/install-action@v2` (SHA `ace6ebe`) avec `tool: cargo-llvm-cov,nextest` (virgule-séparé — v1 ne supporte qu'un seul outil).

## Gate Sonar new-code 80% ≠ `cargo-llvm-cov --fail-under` (2026-06-25)
La gate SonarQube configurée est `new_coverage >= 80%` — elle porte sur le **new-code uniquement** (lignes modifiées depuis la référence de branche). Ce n'est PAS équivalent à `--fail-under=80` de `cargo-llvm-cov` (qui porte sur la couverture totale). Ne pas mélanger les deux mécanismes. Le `--fail-under` n'est PAS utilisé dans ce projet (la gate Sonar est l'autorité).

## `void` (S3735) supprimable sans risque si `no-floating-promises` inactif (2026-06-25)
La règle ESLint `@typescript-eslint/no-floating-promises` est inactive dans la config `recommended` non type-checked (`eslint:recommended` + `tseslint.configs.recommended` sans `strictTypeChecked`). Les `void fn()` ajoutés pour satisfaire cette règle deviennent donc des dead-weight que Sonar signale en S3735. Ils peuvent être retirés sans risque. **Si `no-floating-promises` est activé** (config type-checked), les `void` redeviennent obligatoires — vérifier la config ESLint avant de les retirer.

## `typescript:S1874` `FormEvent` déprécié @types/react 19 — résistant au fix (2026-06-25)
Depuis `@types/react 19`, `FormEvent` est marqué déprécié (renommé en `React.FormEvent`). La remédiation Sonar recommande l'import nommé `import { FormEvent } from 'react'` au lieu de `import React from 'react'; React.FormEvent`. Ce fix **ne supprime pas** l'issue Sonar S1874 car la dépréciation vient du type lui-même dans `@types/react 19`, pas de la façon de l'importer. Clôturer en **won't-fix** dans l'UI SonarCloud.

## `[lints] workspace=true` à répliquer dans chaque `Cargo.toml` de crate (2026-06-25)
La table `[workspace.lints]` dans le `Cargo.toml` racine définit les lints workspace, mais elle n'est **pas héritée automatiquement**. Chaque crate membre (`backend/Cargo.toml`, `backend/migration/Cargo.toml`) doit explicitement opter via `[lints] workspace = true`. Oublier ce flag dans une crate fait silencieusement ignorer tous les lints workspace pour cette crate — clippy passe, mais les règles `unwrap_used`/`expect_used` ne s'appliquent pas.

## `input-otp` exige `document.elementFromPoint` → absent de jsdom (2026-06-25)
**Symptôme** : les tests Vitest avec `<InputOTP>` lancent des `Uncaught Exception: TypeError: document.elementFromPoint is not a function` à la fin des tests (sans les faire échouer, mais le process se termine avec exit 1).
**Cause** : `input-otp@1.4.x` appelle `document.elementFromPoint` pour le positionnement du caret dans un timer interne (`setTimeout`). jsdom ne l'implémente pas.
**Workaround** : ajouter dans `vitest.setup.ts` :
```ts
if (!document.elementFromPoint) {
  document.elementFromPoint = () => null
}
```
Pattern identique au mock `ResizeObserver` déjà présent.

## `input-otp` : timer post-teardown → `window is not defined` (flaky CI) (2026-06-30)
**Symptôme** : le job CI `front (lint/typecheck/test/build)` échoue **par intermittence** sur le
**même commit** (un run vert, l'autre rouge). Tous les tests passent (`100 passed`) mais Vitest
reporte `Errors 1 error` → exit 1. La trace : `ReferenceError: window is not defined` dans
`react-dom` (`resolveUpdatePriority` → `dispatchSetState`), déclenchée par
`Timeout._onTimeout` dans `input-otp/dist/index.mjs`, « originated in `src/unlock/unlock-page.test.tsx` ».
**Cause** : `input-otp` planifie des `setTimeout` **longue durée** (`0, 10, 50` ms **et** `0, 2000, 5000` ms,
cf. `index.mjs`) pour synchroniser le caret. Les timers à 2 s / 5 s survivent largement à la fin du
test ; s'ils se déclenchent **après** que Vitest a démonté l'environnement jsdom, react-dom tente une
mise à jour d'état et touche `window` (disparu). Le flakiness vient du timing : selon que le timer
tire avant ou après le teardown, le run passe ou casse. Distinct du quirk `elementFromPoint` ci-dessus
(qui, lui, ne fait pas échouer la CI de façon non-déterministe).
**Workaround** (`vitest.setup.ts`) : **tracer les `setTimeout` et annuler ceux encore pendants dans
`afterEach`** — wrapper global de `globalThis.setTimeout` qui enregistre les ids dans un `Set`, les
retire à l'exécution du callback, et `clearTimeout` le reliquat après chaque test. Aucun timer ne
survit ainsi à l'environnement. Sûr car **aucun test n'utilise de fake timers** (vérifié) — sinon le
patch global entrerait en conflit. Un simple flush (`await setTimeout(0)`) **ne suffit pas** : il ne
purge pas les timers à 2 s / 5 s.

## Loco `limit_payload` plafonne le body à **2 Mo par défaut** → 413 sur un gros proto (2026-06-25)
**Symptôme** : le deploy d'un HTML mono-fichier > 2 Mo échoue en **413** (`Failed to buffer the request
body: length limit exceeded`, `JsonRejection(... LengthLimitError)`). Le petit HTML passe, le gros non.
**Cause** : Loco active par défaut le middleware `limit_payload` avec `DefaultBodyLimitKind::Limit(2_000_000)`
(2 Mo) **même si `server.middlewares:` est vide**. Le deploy envoie tout le HTML en JSON dans le body.
**Workaround** : configurer `server.middlewares.limit_payload.body_limit` dans `config/*.yaml`. La valeur est
parsée par `byte_unit` (`5mb`, `32mb`) ou `"disable"`. Rendu réglable par env via Tera :
`body_limit: '{{ get_env(name="LATCH_BODY_LIMIT", default="5mb") }}'`. **La config est lue au boot** → un
changement exige un **redémarrage** du serveur. Cf. `docs/ENVIRONMENT.md` (`LATCH_BODY_LIMIT`).

## Tests d'intégration Loco : DB de test **in-memory**, sinon course sous nextest (2026-06-24)
**Symptôme** : `cargo test -p latch` vert en local, mais `cargo nextest run` (CI) rouge sur
les tests qui bootent l'app (`request::<App>`) avec `UNIQUE constraint failed:
seaql_migrations.version` ou `no such table: seaql_migrations` (panic `loco-rs .../testing/request.rs:360`).
**Cause** : `cargo test` exécute tous les tests **dans un seul process** (threads), donc `#[serial]`
les sérialise. `cargo nextest` lance **un process par test** : `#[serial]` (lock intra-process)
**ne sérialise PAS** entre process. Avec une DB de test sur **fichier partagé** (`latch_test.sqlite`)
et `auto_migrate + dangerously_recreate/truncate`, plusieurs process bootent en parallèle et
drop/recréent le schéma en même temps → course sur `seaql_migrations`.
**Workaround** : `config/test.yaml` → `uri: sqlite::memory:` (chaque process a sa base isolée ;
`max_connections=1` reste load-bearing). **La valeur DOIT être quotée** (`'{{ ... }}'`) car
`sqlite::memory:` finit par `:` que YAML lirait comme un mapping → `mapping values are not allowed`.
**Règle de vérif** : valider en local avec **`cargo nextest run`** (même runner que la CI),
pas `cargo test` — sinon ce type de course inter-process passe inaperçu.

## E2E Playwright flaky : `binding: localhost` → bind IPv6 `::1`, poll IPv4 timeout (2026-06-25)
**Symptôme** : le job CI `e2e Playwright (smoke admin)` échoue **par intermittence** (runs FAIL/ok alternés)
avec `Error: Timed out waiting 180000ms from config.webServer.` — alors que le log montre le serveur
**bien démarré** (`listening on http://localhost:5150`, migrations OK) ~75 s **avant** le timeout. Donc
ni crash, ni compilation trop lente.
**Cause** : `development.yaml` avait `binding: localhost`. Sur les runners GitHub, `/etc/hosts` mappe
`localhost` vers `127.0.0.1` **et** `::1` ; `to_socket_addrs("localhost:5150")` peut renvoyer `::1` en
premier → le serveur n'écoute qu'en **IPv6**. Or `playwright.config.ts` poll `http://127.0.0.1:5150/_health`
(**IPv4**) → `ECONNREFUSED` en boucle → timeout. Le flakiness vient de l'ordre de résolution non déterministe.
**Workaround** : forcer une famille d'adresse cohérente des deux côtés. `binding` rendu réglable par env via
Tera (`binding: '{{ get_env(name="LATCH_BINDING", default="localhost") }}'`, défaut inchangé pour le dev
local), et la commande `webServer` de Playwright exporte **`LATCH_BINDING=127.0.0.1`** — cohérent avec le poll
`127.0.0.1/_health`. Vérifié : le serveur loge alors `listening on http://127.0.0.1:5150` et `/_health` → 200.
Cf. `docs/ENVIRONMENT.md` (`LATCH_BINDING`).

## Loco tests — Host header `127.0.0.1:PORT`, pas `localhost` (2026-06-24)
Le harness Loco 0.16 utilise `routes.into_make_service_with_connect_info::<SocketAddr>()`, ce qui force axum-test à utiliser un vrai serveur TCP (pas mock). Dans ce mode, hyper injecte `Host: 127.0.0.1:PORT` (port aléatoire, ex. 8000). Les tests qui envoient `Origin: http://localhost` reçoivent 403 car `127.0.0.1 != localhost` dans `same_host`. **Workaround** : envoyer `Origin: http://127.0.0.1` dans les tests de mutation. `same_host("127.0.0.1:PORT", "127.0.0.1")` passe car hôtes égaux et l'Origin n'a pas de port explicite. Cf. contrat §4/§9.6 et le test `mutation_rejected_on_cross_origin` qui envoie délibérément `Origin: https://evil.example` pour valider le 403.

## cargo-deny = liste blanche stricte (licences) + scope « unmaintained » (2026-06-24)
**Symptôme** : job CI `cargo-deny` rouge sur des licences pourtant permissives (`0BSD`,
`CDLA-Permissive-2.0`) et sur des crates « unmaintained » (bincode, fxhash, proc-macro-
error). **Cause** : cargo-deny **rejette toute licence absente de `allow = [...]`** (modèle
liste blanche, pas liste noire) ; et par défaut il signale les `unmaintained` même
transitifs. **Workaround** (`deny.toml`) : ajouter toute licence permissive *réellement
rencontrée* à `allow` (ex. `0BSD` ← adler, `CDLA-Permissive-2.0` ← webpki-roots) ;
`unmaintained = "workspace"` pour ne contrôler que nos deps directes. **Aussi** : tout
crate du workspace doit déclarer `license = "MIT OR Apache-2.0"` (sinon « unlicensed ») —
piège classique sur le sous-crate `migration`. Vérif locale : binaire cargo-deny prébuilt
(même version que l'action) → `cargo-deny check licenses advisories`.

## Loco lit `config/` depuis le CWD → lancer le serveur depuis `backend/` (2026-06-24)
**Symptôme** : `cargo loco start` depuis la racine du repo → `Error: no configuration
file found in folder: config`. **Cause** : Loco résout `./config/<env>.yaml` relativement
au répertoire courant, et le `config/` vit dans `backend/` (workspace 2 membres).
**Workaround** : lancer les commandes serveur depuis `backend/` (`cd backend && cargo
loco start`). L'alias `cargo loco` est à la racine (`.cargo/config.toml`, `run -p latch --`)
et reste trouvé depuis `backend/` par recherche ascendante. Les commandes `fmt`/`clippy`/
`test` n'ont pas ce souci (pas de config) et tournent depuis la racine.

## [Yew] Crate wasm (frontend) dans un workspace → `default-members` (2026-06-24)
> **Archivé** — la crate Yew est retirée du workspace (migration React).

**Symptôme** : `cargo build`/`clippy --workspace` tente de compiler `latch-ui` (Yew) pour
la cible hôte native → échoue (web-sys/wasm-only). **Cause** : un membre wasm dans un
workspace mixte. **Workaround** : `default-members = ["backend", "backend/migration"]`
dans le `Cargo.toml` racine → les commandes sans `--workspace` ignorent le frontend.
Le frontend se build via `trunk` ou `cargo … -p latch-ui --target wasm32-unknown-unknown`.
_(Aujourd'hui le `default-members` reste `["backend", "backend/migration"]` — le frontend React
n'est pas un crate Cargo, donc `--workspace` n'en a jamais été affecté.)_

## Docker runtime non-root : volume `/data` préexistant possédé par root (2026-06-25)
**Symptôme** : après migration vers `distroless:nonroot` (uid 65532), le boot échoue si le répertoire ou volume `/data` a été créé par une ancienne image tournant en root — `Permission denied` lors de la création du SQLite ou d'un fichier HTML de version.
**Cause** : le stage `dataprep` chown `/data` à `65532` seulement au moment de la construction de l'image. Les données existantes (bind-mount `./data` ou named volume) ne sont pas retouchées au runtime.
**Workaround** : une fois, depuis l'hôte : `chown -R 65532:65532 ./data` (bind-mount) ou via un container helper `docker run --rm -v latch-data:/mnt alpine chown -R 65532:65532 /mnt` (named volume). La prochaine fois, le container écrit nativement en uid 65532.

## Scan local Sonar : chemin absolu lcov ≠ `/usr/src` → couverture Rust silencieusement ignorée (2026-06-25)
**Symptôme** : scan local via `sonarsource/sonar-scanner-cli` (Docker) affiche une couverture Rust à 0% (ou très basse) et la gate `new_coverage ≥ 80%` échoue, alors que la couverture CI est correcte (~94%).
**Cause** : `cargo-llvm-cov` génère `backend-lcov.info` avec des **chemins absolus** locaux (ex. `SF:/srv/owlnext/latch/backend/src/…`). Le container `sonar-scanner-cli` monte le repo sous `/usr/src` → le sensor LCOV Rust ne retrouve pas les fichiers (chemin différent) → **il ignore silencieusement tout le backend** sans erreur explicite. En CI, le chemin du runner (`/home/runner/work/…`) correspond au chemin injecté → pas de problème.
**Fix** : avant le scan local, remappe les chemins dans le fichier lcov :
```bash
sed -i "s#$(pwd)/#/usr/src/#g" backend-lcov.info
```
Cette commande réécrit `SF:/srv/owlnext/latch/` en `SF:/usr/src/` dans toutes les lignes `SF:` du fichier. CI n'a pas besoin de ce fix (les chemins correspondent). Cf. `docs/ENVIRONMENT.md §Scan local`.

## fake_dist écrit unlock.html ET error.html (Phase 7 Lot 4)

Les tests d'intégration `serve` posent un faux `dist/` via `fake_dist()` : il écrit MAINTENANT
`unlock.html` ET `error.html` (marqueur `id="error-root"`). Un test dédié vérifie le fallback inline
quand `error.html` manque. Toute réponse `/c` (page d'erreur comprise) reste `no-store`.

## Favicon servi via /assets (Phase 7 Lot 3)

Le backend ne sert que `/assets` (mount ServeDir), pas la racine du dist. Un favicon à la racine
(`/favicon.ico`, `/vite.svg`) fait 404 sous `/admin` (bug Phase 4). Solution : référencer le SVG
via `/src/assets/latch-logo.svg` dans le HTML → Vite le bundle sous `/assets/<hash>.svg`, servi.
Stratégie SVG-only assumée (pas de bundle multi-tailles : outil interne noindex).

lucide-react 1.21.0 ne fournit PAS d'icônes de marque (`Github` = undefined) → utiliser un SVG inline (`components/github-icon.tsx`).

## rmcp 1.8 — `ServerInfo` est `#[non_exhaustive]` (2026-06-25)
`ServerInfo` dans rmcp 1.8 est marquée `#[non_exhaustive]` → impossible à construire avec un struct literal `ServerInfo { name: "...", ... }` (erreur de compilation « cannot create non-exhaustive struct using struct expression »). **Fix** : construire via `ServerInfo::default()`, puis assigner les champs (`name`, `version`), puis appeler `.with_instructions("...")`. Pattern retenu dans `LatchMcp::get_info()`.

## rmcp 1.8 — tool schema de type `array` à la racine → panic au boot (2026-06-25)
Le `#[tool_router]` de rmcp 1.8 construit le schéma JSON de chaque tool au démarrage. Si le type de retour d'un tool produit un schéma JSON dont le type racine est `"array"` (ex. `Vec<ProjectSummary>` renvoyé directement), rmcp **panique au boot** avec un message sur un schéma invalide (le protocole MCP exige `object` à la racine). **Fix** : toujours envelopper les listes dans un struct :
```rust
// ❌ Panique au boot
async fn list_projects(...) -> Vec<ProjectSummary> { ... }

// ✅ OK — enveloppe objet
pub struct ProjectListResult { pub projects: Vec<ProjectSummary> }
async fn list_projects(...) -> ProjectListResult { ... }
```
Même idiome que `DeployResult` (pas de tableau racine). Cf. contrat §5.1.

## rmcp 1.8 — `#[tool]` macro → `Pin<Box<dyn Future>>`, directement `await`-able (2026-06-25)
La macro `#[tool]` de rmcp 1.8 réécrit les `async fn` en fonctions retournant `Pin<Box<dyn Future<Output=_>>>`. En pratique, ces fonctions sont directement `.await`-ables depuis les tests (pas de transport HTTP requis) :
```rust
// Test inline — level handler, sans transport HTTP
let m = LatchMcp::new(db, storage, token.into(), base_url.into());
let result = m.deploy_prototype(DeployParams { slug: "mon-projet-abc".into(), ... }).await;
```
Ce pattern permet des **tests unitaires de handler** (gate token, logique) sans monter un serveur HTTP. Les tests d'intégration complets (transport streamable HTTP) restent reportés Phase 6. Cf. `docs/CONVENTIONS.md §Test de handler MCP`.

## Tests e2e MCP (transport HTTP) : Host header à fixer explicitement (2026-06-25)
Le harness `axum_test` (utilisé par loco_rs 0.16 `request::<App>`) envoie les requêtes avec `Host: localhost` (sans port). Or `allowed_hosts` rmcp est dérivé de `LATCH_PUBLIC_BASE_URL = "http://localhost:5150"` via `host_authority()` → valeur `localhost:5150`. La validation `Host` rmcp rejette `localhost ≠ localhost:5150` avec `403 Forbidden: Host header is not allowed`. **Fix dans les tests** : ajouter explicitement le header `host: localhost:5150` dans chaque requête MCP `.add_header("host", "localhost:5150")`. Alternativement, `LATCH_PUBLIC_BASE_URL = "http://localhost"` (sans port) + `host: localhost`.

## Tests e2e MCP (transport HTTP) : SSE rmcp 1.8 — première ligne `data:` vide (2026-06-25)
Le transport Streamable HTTP de rmcp 1.8 débute la réponse SSE par un event de keepalive (`data: \nid: 0\nretry: 3000\n\n`) avant l'event JSON-RPC réel. Un parseur SSE qui prend la **première** ligne `data:` obtient une chaîne vide → `serde_json::from_str("")` → erreur. **Fix** : ignorer les lignes `data:` vides et prendre la première avec payload non vide.

## rmcp 1.8 — `serverInfo.name` : utiliser `with_server_info` explicitement (2026-06-25, corrigé 2026-06-25)
`ServerInfo::default()` appelle `Implementation::from_build_env()` qui capture `env!("CARGO_CRATE_NAME")` **au moment de la compilation de la crate rmcp** (pas de la crate `latch`). Résultat brut : `serverInfo.name = "rmcp"`, `version = "1.8.0"`. **Fix** : appeler `info.with_server_info(Implementation::new("latch", env!("CARGO_PKG_VERSION")))` dans `get_info()` avant `with_instructions`. Le nom annoncé est désormais `"latch"` et le test `mcp_initialize_handshake` l'asserte directement.

## Tests e2e MCP : `axum_test::TestServer` non réexporté par `loco_rs::testing::prelude` (2026-06-25)
La fonction `request::<App>` du harness loco prend une closure avec `(TestServer, AppContext)`, mais `TestServer` vient de `axum_test` — non réexporté par `loco_rs::testing::prelude`. Pour typer un helper `async fn mcp_post(request: &???, ...)`, il faut soit ajouter `axum-test = { version = "17.x" }` en dev-dependency directe (`version = "17.3"` épinglée sur la version du lockfile transitif), soit éviter de déclarer explicitement le type (le compilateur infère).

## rmcp < 1.4.0 — DNS rebinding (CVE-2026-42559)
Le transport Streamable HTTP ne validait pas le `Host` avant la 1.4.0. **Épingler
≥ 1.4.0** et configurer `allowed_hosts` (inclure `latch.owlnext.fr`). Caddy valide
aussi le `Host` en amont. Ne jamais désactiver l'allowlist sans proxy qui valide le Host.

## SQLite dans l'image — feature `bundled`
Compiler `libsqlite3-sys` en **`bundled`**, sinon l'image runtime (distroless/alpine)
devra fournir la lib système et ça casse en silence au démarrage. Avec `bundled`, le
binaire est autonome.

## Migrations au démarrage du conteneur
Entrypoint = `migrate` **puis** `start`. Premier boot sur volume vierge sans migration
= pas de schéma → l'app tombe. Ne pas compter sur un `cargo loco` dans l'image
distroless (pas de cargo) : la migration doit être lançable depuis le binaire.

## Le cœur ne doit jamais voir axum/loco
Si `use axum::` ou `use loco_rs::` apparaît dans `src/services/`, l'archi est violée
(contrat §1). Le cœur suppose l'appelant déjà autorisé et rend un `CoreError`.

## Suffixe de slug — 8 chars base62 (≈ 47 bits) — FIGÉ (2026-06-24)
Décision actée : **suffixe = 8 caractères base62** (`[A-Za-z0-9]`), ≈ 47 bits, quasi
non-énumérable. Choix motivé par les protos **sans code**, où l'URL est la *seule*
barrière (un proto avec code a PIN + rate-limit comme vraie barrière, mais on ne veut
pas deux régimes de slug). Gratuit en UX : le suffixe vit dans le lien copié-collé,
jamais tapé. Exemple : `mon-projet-k7Qp2maZ`. _(Antérieurement « non figé, défaut
court 4 hex » — tranché à l'implémentation du service `slug`, Phase 1.)_

## PIN 6 chiffres — la sécurité est dans le rate-limit, pas l'entropie
10⁶ combinaisons = brute-forçable en secondes. Le rate-limit sur `/unlock` est
*load-bearing*, pas optionnel. Hasher le PIN serait surtout théâtral (et de toute
façon on le stocke récupérable, choix (b), pour pouvoir le copier en admin).

## Playwright = Node en CI/dev
Le « pas de Node » ne vaut que pour le **runtime**. L'e2e tire un toolchain Node ;
c'est assumé.

## `cargo loco db entities` requiert `sea-orm-cli` installé séparément (2026-06-24)
**Symptôme** : `cargo loco db entities` → `Error: Message("SeaORM CLI was not found To fix, run: $ cargo install sea-orm-cli")`.
**Cause** : Loco délègue la génération d'entités à `sea-orm-cli` (binaire externe), non inclus dans les dépendances Cargo.
**Workaround** : `cargo install sea-orm-cli` (une seule fois par machine). Vérifier que la version correspond à celle de `sea-orm` du workspace (1.1.x → `sea-orm-cli 1.1.20` installé automatiquement).

## SQLite in-memory — `max_connections(1)` LOAD-BEARING dans les tests (2026-06-24)
**Symptôme** : pool > 1 en SQLite `:memory:` → chaque connexion est une base distincte → tables vides pour la 2e connexion. **Cause** : `sqlite::memory:` crée une nouvelle base par connexion (comportement SQLite). **Workaround** : `ConnectOptions::max_connections(1)` dans `test_db()` — obligatoire, ne jamais l'augmenter pour les in-memory.

## `active_version_id` = FK logique non contrainte (référence circulaire) (2026-06-24)
`projects.active_version_id` pointe vers `versions.id`, mais `versions` a une FK vers `projects.id`. Cette référence circulaire (`projects ⇄ versions`) empêche de déclarer une vraie contrainte `FOREIGN KEY` en SQLite : la table cible doit pré-exister au moment de la création de la table source. **Conséquence** : la colonne est un entier nullable sans contrainte DB ; l'intégrité référentielle est assurée au niveau applicatif (`deploy.rs` vérifie que le projet existe avant d'insérer). Ne pas ajouter de contrainte DB sans revoir l'ordre de création des tables.

## FK SQLite non enforced sans `PRAGMA foreign_keys=ON` (2026-06-24)
SQLite **n'enforce pas** les contraintes `FOREIGN KEY` par défaut. Le `ON DELETE CASCADE` déclaré sur `versions.project_id → projects.id` est purement déclaratif et **best-effort** à l'exécution (fonctionne si la pragma est activée par la session, mais Loco/SeaORM ne l'active pas nécessairement). En pratique, la suppression d'un projet ne cascade pas automatiquement les versions en production sans activation explicite. À prendre en compte pour tout code de suppression de projet dans les adaptateurs (Phase 2).

## axum_session 0.16.0 — `with_prefix_with_host(true)` CASSE la session en prod (2026-06-26)
**Symptôme** : en **prod uniquement** (HTTPS, `is_prod=true`), le login renvoie **200** et pose le
cookie, mais toute requête suivante vers une route protégée renvoie **401** → la SPA rebondit vers
`/admin/login` **sans message d'erreur** (le login a réussi, c'est l'accès protégé qui 401). Au curl :
l'UUID du cookie `__Host-latch_admin` **change à chaque requête** → le serveur crée une **session neuve**
à chaque fois, il ne relit jamais la session entrante. Invisible en dev/test (CI verte) car `is_prod=false`.
**Cause** : bug d'asymétrie écriture/lecture dans `axum_session 0.16.0` (`src/headers.rs`). Avec
`with_prefix_with_host(true)`, le chemin d'**écriture** (`create_cookie` → `NameType::get_name`) préfixe
bien `__Host-` (pose `__Host-latch_admin`), mais le chemin de **lecture** (`get_headers_and_key`) lit le
champ **brut** `config.cookie_and_header.session_name` (= `latch_admin`) **sans** repasser par `get_name`
→ il cherche `latch_admin`, ne trouve jamais `__Host-latch_admin`. (Le `session_mode` par défaut est
`Persistent`, donc le cookie `__Host-store` supprimé à chaque réponse est un faux indice, sans rapport.)
**Fix (contournement, `web/mod.rs::build_session_store`)** : ne **pas** utiliser `with_prefix_with_host` ;
poser nous-mêmes le nom `__Host-latch_admin` / `__Host-store` via `with_session_name`/`with_store_name`
**en prod uniquement** (sur HTTP en dev, un cookie `__Host-` serait rejeté par le navigateur), et laisser
`prefix_with_host=false` → lecture et écriture utilisent le même nom. Le durcissement `__Host-` est
préservé : c'est une convention de nom policée par le **navigateur** (exige Secure + `Path=/` + pas de
`Domain`, déjà fournis par la config). **Ne jamais réactiver `with_prefix_with_host`** tant que la lib
n'est pas patchée en amont.

## axum_session 0.16 — `with_session_name` (pas `with_cookie_name`) (2026-06-24)
`SessionConfig` 0.16 expose `with_session_name` pour nommer le cookie/header de session. Le brief mentionnait `with_cookie_name` (qui n'existe pas). `SameSite` est réexporté par `axum_session` depuis le crate `cookie` (pas besoin d'importer `cookie` séparément). `Key::derive_from` n'existe pas en `cookie` 0.18 — utiliser `Key::from` (exige ≥ 64 bytes) ou `Key::generate`. La clé dev de secours dans `web/mod.rs` fait exactement 64 chars.

## axum_session_sqlx 0.5 — `SessionSqlitePool::from(pool)` (pas `::new`) (2026-06-24)
`SessionSqlitePool` n'a pas de constructeur `::new`. Il implémente `From<Pool<Sqlite>>` → utiliser `SessionSqlitePool::from(pool.clone())`. `get_sqlite_connection_pool()` dans sea-orm 1.1 retourne `&sqlx::SqlitePool` directement (pas un `Result`) — pas de `.map_err` nécessaire.

## `SESSION_SECRET` — minimum 64 bytes en prod (2026-06-24)
`Key::from(bytes)` exige ≥ 64 bytes (signing 32 + encryption 32). En dessous, panique au démarrage. En dev, une clé de 64 chars est codée en dur dans `build_session_store`. En prod, `SESSION_SECRET` doit faire ≥ 64 bytes d'entropie (clé aléatoire, pas un mot de passe).

## tower_governor — GovernorLayer construit avec struct literal, pas ::new() (2026-06-24)
`GovernorLayer` expose un champ public `config: Arc<GovernorConfig<K, M>>` et se construit
avec `GovernorLayer { config: Arc::new(config) }`. Il n'y a pas de méthode `::new()` sur
`GovernorLayer`. De plus, l'annotation explicite du type de retour est verbeuse car
`NoOpMiddleware` vient de la sous-dépendance `governor` (non réexportée dans la crate root
de `tower_governor`) — construire inline dans `routes()` pour éviter ce problème.

## tower_governor — finish() retourne Option, pas Result (2026-06-24)
`GovernorConfigBuilder::finish()` retourne `Option<GovernorConfig<K, M>>` (None si burst_size=0
ou period=0). Utiliser `.expect("governor config valide")` (acceptable en init de boot).

## tower_governor — Session::from_request_parts rejection type (2026-06-24)
`axum_session::Session<T>` implémente `FromRequestParts` avec `Rejection = (http::StatusCode, &'static str)`.
Pour l'utiliser dans un extracteur custom dont le `Rejection = loco_rs::Error`, mapper avec
`.map_err(|_| loco_rs::Error::Unauthorized("..."))`.

## axum_session 0.16 — clear() vs destroy() au logout (2026-06-24)
`session.clear()` vide les clés en mémoire mais laisse la ligne en DB (session valide côté
serveur jusqu'à expiration). `session.destroy()` marque la session pour suppression en DB à
la phase de réponse : révocation immédiate côté serveur + cookie invalidé. Pour un logout
admin, utiliser **`session.destroy()`** (contrat §4). `session.purge()` n'existe pas en 0.16.

## loco_rs::Error::Unauthorized → 401, pas 403 (confirmé 0.16.4) (2026-06-24)
`loco_rs::Error::Unauthorized(msg)` mappe sur **401 UNAUTHORIZED** dans `controller/mod.rs` ligne ~209. Il n'existe pas de variant `Forbidden` dans `loco_rs::Error` 0.16.4. Pour produire un **403** dans un middleware axum, utiliser directement `Ok((StatusCode::FORBIDDEN, "msg").into_response())` — c'est idiomatique (le middleware court-circuite la chaîne en produisant sa propre réponse) et ne dépend pas de `ErrorDetail`. Alternative : `loco_rs::Error::CustomError(StatusCode::FORBIDDEN, ErrorDetail::with_reason(...))` — fonctionne mais couple le middleware à `ErrorDetail`.

## same_host() — ports différents sur même hôte sont des origines distinctes (2026-06-24)
`same_host("example.com:8080", "example.com:9090")` doit retourner `false` (RFC 6454 : l'origine inclut le port). La première implémentation utilisait `host.split(':').next()` — ce qui comparait seulement les noms d'hôtes et acceptait à tort des ports différents. Correction : utiliser `rsplit_once(':')` pour extraire nom et port séparément, et ne comparer les ports que si les deux en ont un. Caveat : IPv6 (`[::1]:port`) non géré.

## `is_prod` dans `web/mod.rs` — fail-secure : exclure Dev/Test, pas inclure Production (2026-06-24)
**Symptôme** : tests d'intégration qui font login + accès protégé échouent en 401 même avec `save_cookies(true)`. **Cause** : `is_prod = !matches!(env, Development)` était `true` en environnement `Test`, activant `cookie_secure = true` (attribut `Secure` sur le cookie de session). En HTTP (transport mock ou localhost), un cookie `Secure` n'est jamais renvoyé. **Workaround** : utiliser la forme fail-secure `is_prod = !matches!(env, Development | Test)` — tout environnement inconnu futur reçoit `Secure=true` par défaut. Ne pas écrire `matches!(..., Production)` (fail-open : un nouvel env hypothétique « staging » serait insécurisé par défaut).

## `request_with_config` avec `save_cookies(true)` requis pour les tests avec session (2026-06-24)
**Symptôme** : tests utilisant `request(...)` (défaut : `save_cookies: false`) ne propagent pas le cookie de session entre requêtes → 401 sur les routes protégées après login. **Cause** : `axum_test::TestServer` ne sauvegarde les `Set-Cookie` que si `save_cookies: true`. **Workaround** : utiliser `request_with_config(RequestConfigBuilder::new().save_cookies(true).build(), ...)` pour tous les tests qui enchaînent login + accès protégé.

## Page de déverrouillage en 200, pas 401
`/c/<slug>` protégé sans cookie rend la page-code en **HTTP 200** (formulaire
accueillant), pas un 401 (qui déclencherait le popup natif — précisément ce qu'on
fuit en remplaçant le Basic Auth).

## Fail-secure secrets — `UNLOCK_COOKIE_SECRET` et `SESSION_SECRET` (2026-06-25)
Les deux secrets de cookie sont résolus via un helper pur (`resolve_cookie_secret` dans
`web/mod.rs`) qui **refuse le boot en prod** si la variable d'env est absente ou vide
(tout environnement hors `Development`/`Test`). La garde de longueur est en **octets** (pas
chars) : `Key::from()` panique si `bytes.len() < 64`. Un secret de 63 octets fait échouer le
boot — c'est le comportement voulu, pas un bug. En dev/test, un fallback déterministe de
64 chars est utilisé. **Ne jamais baisser la garde à ≥ 32 octets** : l'exigence `axum-extra
SignedCookieJar` (signing 32 + encryption 32 = 64 minimum) est non-négociable.

## Cookie unlock = `SignedCookieJar` + empreinte HMAC du PIN (2026-06-25)
Le cookie de déverrouillage utilise `SignedCookieJar` (feature **`cookie-signed`** d'`axum-extra`,
PAS le feature `cookie` seul — l'import est `axum_extra::extract::cookie::SignedCookieJar`) et
stocke dans sa valeur une **empreinte HMAC du PIN** (pas le PIN en clair) : changement de PIN →
rotation implicite des cookies existants → révocation sans liste de révocation. `Key::from()`
exige ≥ 64 bytes (signing key 32 B + encryption key 32 B). `SignedCookieJar::from_headers(&headers, key)` —
construire manuellement depuis `HeaderMap`, pas comme extracteur axum classique dans les handlers
qui combinent plusieurs extracteurs.

## Rate-limit `/unlock` = in-memory (governor, 2 layers via `ServiceBuilder`) (2026-06-25)
Le rate-limit de `/unlock` est **100 % in-memory** (governor, `tower_governor`) : les compteurs
sont **perdus au reboot** du serveur (limite assumée et documentée §9.5 du contrat). Architecture :
deux layers indépendants (par-IP + slug-global) montés via `tower::ServiceBuilder` (`.layer()` ×2)
car `.layer().layer()` chaîné directement sur un `MethodRouter` axum 0.8.9 casse l'inférence de
type → erreur de compilation obscure. `ServiceBuilder` résout le problème car il compose les layers
avant de les passer à axum.

## 2ᵉ entrée Vite `unlock.html` + assets servis sous `/assets` (base `/`) (2026-06-25)
`unlock.html` est la 2ᵉ entrée du build Vite (Phase 4) — déclarée dans `vite.config.ts`
(`build.rollupOptions.input = { main, unlock }`). **Base Vite = `/`** (pas `/admin/`) : les deux
bundles (`main` admin, `unlock` public) référencent leurs assets en `/assets/...` (JS, CSS **et**
URLs `@font-face` des polices Inter incluses). Côté backend, `after_routes` monte
`nest_service("/assets", ServeDir::new(dist.join("assets")))` — assets publics, hors `/admin`.
**Pourquoi pas `/admin/assets` (état initial)** : la page de déverrouillage est une surface
**publique** (`/c/<slug>`) ; si elle tirait ses assets de `/admin/...`, un futur durcissement
« /admin restreint en IP » (BACKLOG) casserait la page pour les clients. Le découplage vers
`/assets` neutralise ce couplage. L'admin reste servi par `nest_service("/admin", ServeDir.fallback(index))`
(son routeur TanStack a `basepath: '/admin'`, orthogonal à la base Vite). Conséquence cosmétique
réglée : le favicon `/vite.svg` (placeholder scaffold) a été retiré d'`index.html` (sinon 404 en `/`).
Pas de collision de route : `/assets` ne préfixe ni `/api`, `/c`, `/admin`, `/mcp`.

---

## Historique Yew — obsolète depuis migration React (2026-06-25)

> Ces quirks concernaient la crate Yew (`latch-ui`, `shadcn-rs`, Trunk, wasm32) retirée du
> workspace lors de la migration React (Plans 1-3, feat/admin-react). Conservés pour référence
> en cas de consultation de l'historique git.

## `yew-router = 0.18` (PAS 0.21) pour `yew 0.21` — numérotation divergente (2026-06-24)
La numérotation de `yew-router` **diverge** de `yew` : `yew-router 0.18` correspond à `yew 0.21`, `yew-router 0.19` à `yew 0.22`, `yew-router 0.20` à `yew 0.23`. Piège classique : chercher `yew-router = "0.21"` → introuvable ou mauvaise version. Épingler `yew-router = "0.18"` avec `yew = "0.21"`.

## `gloo-net` 0.6 : un HTTP 401/404 est `Ok(Response)`, pas une `Err` (2026-06-24)
Avec `gloo-net 0.6`, une réponse HTTP avec status 401 ou 404 est **`Ok(Response)`**, pas une `Err`. Il faut **toujours** inspecter `resp.status()` après `.send().await?`. De plus, `.json(&body)?` sur le `RequestBuilder` **consomme** le builder (retourne `Result<Request>`) **avant** le `.send().await?` — ne pas appeler `.json()` après avoir déjà enchaîné `.send()`.

## `tower-http` : activer explicitement le feature `fs` même si transitif (2026-06-24)
`ServeDir` et `ServeFile` de `tower-http` requièrent le feature `fs`. Même si `tower-http` est une dépendance transitive, il faut l'ajouter **explicitement** au `Cargo.toml` du backend avec `features = ["fs"]` — sinon les types `ServeDir`/`ServeFile` ne sont pas disponibles.

## shadcn-rs 0.1 : `<Sheet>` est une coquille, piloter `<SheetContent>` directement (2026-06-24)
`<Sheet>` (wrapper) est une **coquille qui ignore toutes ses props** — ne pas s'y fier pour passer `open` ou `on_close`. Piloter `<SheetContent open=.. on_close=..>` directement. Il n'existe pas de `SheetClose`. Pas de toast programmatique : `Toast`/`Sonner` sont déclaratifs et `duration` (auto-dismiss) n'est pas implémenté en 0.1. `Switch`/`Dialog` : l'état « contrôlé » retombe sur l'état interne tant que `checked={false}` → gérer le state soi-même. `Switch::onchange` est `Callback<Event>`. `TableRow` n'a pas d'`onclick` → naviguer via `<a onclick>` dans les cellules.

## shadcn-rs.css : variables `--color-card*`/`--color-popover*` manquantes (2026-06-24)
La lib `shadcn-rs` oublie `--color-card*` et `--color-popover*` dans `variables.css` alors que `components.css` les utilise → patcher la CSS vendorisée (ajoutés en `:root` et `.dark`). La CSS vendorisée est composée de **5 fichiers** sous `frontend/styles/` (imports relatifs) ; dark-mode via classe `.dark`.

## SPA sous `/admin` : configuration Trunk + BrowserRouter + backend (2026-06-24, corrigé au test live)
Pour servir la SPA Yew sous `/admin` : (1) `Trunk.toml public_url = "/admin/"` ; (2) **PAS de `basename`** sur `<BrowserRouter>` ; (3) `#[at("/admin/...")]` **absolus** dans les routes Yew ; (4) `nest_service("/admin", ServeDir::new(dir).fallback(ServeFile::new(index)))` côté backend (**PAS** `fallback_service`, qui masquerait les 404 sur `/api`).
**⚠️ Ne PAS utiliser `BrowserRouter basename="/admin"`** : yew-router 0.18 a un bug dans `Navigator::strip_basename` — pour l'URL racine **exacte** `/admin`, `strip_prefix("/admin")` donne `""`, puis comme `""` ne commence pas par `/` le code refait `format!("/{m}")` = **`//admin`** (jamais matchée) → **404 sur toute l'app**. Le combo qui marche est donc **sans basename + routes absolues**. (Trunk avec `public_url` réécrit les assets en absolu et **n'injecte pas** de `<base>`, donc `base_url()` reste `None` → pas de basename implicite.) Diagnostiqué uniquement au test navigateur (Playwright) : ni les tests SDD ni le smoke curl n'exercent le routing wasm.

## shadcn-rs 0.1 : l'animation `slide-in-*` du Sheet casse l'affichage du drawer (2026-06-24, test live)
Les `@keyframes slide-in-*` de `components.css` laissent un `transform` résiduel (~`translateY(-50%)`) sur `.sheet-content` → le drawer est décalé hors écran (haut), **le contenu du side-panel devient invisible** (panneau blanc vide) alors qu'il est bien dans le DOM. Il y a en plus **deux `@keyframes slide-in-right` dupliqués** (components.css + utilities.css). Workaround dans `frontend/styles/app.css` : `.sheet-content { animation: none !important; transform: none !important; display:flex; flex-direction:column; gap:.75rem; overflow-y:auto }` + `.sheet-footer { margin-top:auto }` → drawer statique, plein hauteur, footer en bas. (Là encore : invisible aux tests non-navigateur.)

## Classes de layout de l'app = CSS à écrire soi-même (2026-06-24)
La CSS vendorisée de `shadcn-rs` ne style QUE les **composants** (`.btn`/`.card`/`.input`…). Toutes les classes de **mise en page** propres à l'app (`.admin-page`, `.topbar`, `.kv`, `.toggle-row`, `.auth-screen`, `.detail-head`, `.pin-row`, `.empty-state`, `.head-actions`…) doivent être stylées à la main dans `frontend/styles/app.css` (liée après `shadcn-rs.css`, copiée par Trunk via `copy-dir`). Sans elle : login non centré, cartes pleine largeur, topbar non alignée — l'UI paraît « cassée ».

## shadcn-rs 0.1 `Switch` : l'état contrôlé ne bascule pas visuellement (2026-06-24, test live — À CORRIGER)
Confirmé au test : `<Switch checked={*state} onchange={..}>` **ne reflète pas** visuellement le changement d'état (le composant garde son état interne — cf. le quirk « contrôlé retombe sur interne »). L'action applicative se fait bien, mais le toggle reste coché à l'écran. À corriger prochaine session (forcer le rendu via `key`, piloter autrement, ou switch maison). Cf. punch-list `docs/superpowers/specs/2026-06-24-phase-3-punchlist-ux.md`.

## Orphan rule : conversions DTO en fonctions libres côté backend (2026-06-24)
`From<&Model>` pour un type de `latch-dto` est interdit par la règle d'orphelin (le type `Model` est dans `latch` backend, le type DTO dans `latch-dto` — ni l'un ni l'autre n'est local au site de l'impl). Solution : conversions en **fonctions libres** côté backend (`dto::to_list_item(model)` / `dto::to_detail(model, versions)`), pas de trait impl.

## Side-panels Yew montés en permanence — réinitialiser à la (ré)ouverture (2026-06-24)
Les side-panels Yew sont montés en permanence dans le DOM (prop `open` contrôle la visibilité). Les `use_state` internes **persistent** entre ouvertures : si l'utilisateur ouvre un panel, le ferme sans soumettre, puis le rouvre, les champs peuvent contenir des valeurs périmées. Solution : `use_effect_with(props.open, |open| { if *open { /* reset fields */ } })` à l'ouverture du panel. S'applique aussi au re-déploiement (évite qu'un fichier obsolète soit re-soumis).

## Badges colorés shadcn-rs : doubler la classe pour battre `.badge.variant-*` (2026-06-25, test live)
`.badge.variant-secondary` (et `variant-default`/`variant-destructive`) de `components.css` posent
`background-color` avec une **spécificité (0,2,0)**. Une classe utilitaire simple `.badge--success`
(0,1,0) est donc **écrasée** → le badge reste gris au lieu de vert. (Le `variant-outline` ne pose
PAS de fond, donc `.badge--warning` orange passait, masquant le problème.) **Workaround** : doubler
la classe — `.badge.badge--success` / `.badge.badge--warning` (0,2,0) ; `app.css` étant chargé
**après** `shadcn-rs.css`, à spécificité égale il gagne. **Invisible aux reviews unitaires** :
diagnostiqué uniquement en validant la couleur calculée au navigateur (`getComputedStyle`).

## i18n rust-i18n + Yew : réactivité = abonnement `use_locale()` obligatoire (2026-06-25)
`rust_i18n::set_locale(...)` change un **état global** qui **ne notifie pas Yew**. La macro `t!`
lit cette locale globale au render. Pour que l'UI se re-render au changement de langue, le
`LocaleProvider` bump un `Context` (`LocaleContext`) ET tout composant affichant du texte traduit
DOIT s'y abonner via `use_locale()` en tête (même `let _loc = use_locale();` inutilisé) — sinon ce
composant ne se re-render pas et garde l'ancienne langue. `set_locale` est appelé **synchroniquement**
dans l'initialiseur `use_state` du provider au boot (détection localStorage→navigator→EN) pour éviter
un flash. La macro `i18n!("locales")` embarque les YAML **à la compilation** (pur Rust → wasm OK).

## rust-i18n locale files `_version: 1` : pas de `"` ASCII nu dans une string double-quote (2026-06-25)
Les YAML de locale (`frontend/locales/{en,fr}.yml`, format `_version: 1`, clés plates pointées)
sont parsés par `serde_yaml` dans le proc-macro `i18n!`. Une string en double-quote contenant un `"`
ASCII nu casse le parse (panic à la compilation). **Workaround** : passer ces lignes en single-quote
YAML (ex. `key: 'texte avec "guillemets"'`) ou utiliser des guillemets typographiques « … ».

## `Switch` shadcn-rs vendorisé en `Toggle` — classe `size-md` LOAD-BEARING (2026-06-25)
Le `<Switch>` shadcn-rs 0.1 a un état contrôlé cassé (`is_checked = if checked {..} else {*internal}`
→ ne revient jamais à off, cf. quirk précédent). Vendorisé en `components/toggle.rs` avec
`is_checked = checked` (contrôlé pur, zéro état interne). **Piège** : les dimensions du switch
vivent sur la classe `.switch.size-md` de `components.css` (le `.switch` seul n'a NI hauteur NI
largeur). Le `Toggle` doit émettre `class="switch size-md"` (+ `switch-checked`/`switch-disabled`),
sinon le contrôle est invisible (taille nulle).

## utoipa-swagger-ui ≥ 9 obligatoire avec axum 0.8 (2026-06-25)
`utoipa-swagger-ui` v8 tire `axum 0.7` (via sa dep `utoipa-axum 0.1`). Cela crée un conflit
de types avec l'`axum 0.8` du projet (`axum::Router` de v7 ≠ `axum::Router` de v8) →
erreurs de trait obscures à la compilation. **Épingler `utoipa-swagger-ui = "9"`** (axum 0.8
natif), aligné sur `utoipa = "5"`. Ne jamais downgrader vers v8 pour "essayer".

## utoipa `paths(module::handler)` ne requiert PAS `pub` sur le handler (2026-06-25)
La macro `#[utoipa::path]` génère un struct de chemin `__path_<fn>` résolu par chemin de
module. Les handlers `async fn` privés ou `pub(crate)` restent référençables depuis
`openapi.rs` via `paths(controllers::admin::handler)` — aucune fuite de visibilité public
n'est nécessaire. (Le plan suggérait prudemment `pub(crate)` — finalement inutile dans les
cas standards ; utile uniquement si le compilateur le réclame explicitement pour un module
non-réexporté.)

## utoipa : feature `chrono` inutile si les dates sont sérialisées en `String` (2026-06-25)
Nos DTO portent les dates sous forme `String` (`.to_rfc3339()` côté service), donc `utoipa`
n'a pas besoin de connaître `chrono::DateTime`. `utoipa = "5"` sans `features = ["chrono"]`
suffit. Ne pas ré-ajouter `features = ["chrono"]` par réflexe si une date apparaît dans un
DTO : vérifier d'abord que c'est réellement un type `chrono::*` (pas un `String`).

## utoipa : le doc-comment du handler devient la `description` OpenAPI (2026-06-25)
Un `///` au-dessus d'un handler annoté `#[utoipa::path]` est capturé comme `description`
dans `openapi.json`. Un doc-comment verbeux (notes QUIRKS, contexte Context7, TODOs internes)
**fuite dans le contrat API** et sera ensuite déversé dans le client TypeScript généré (Plan 2).
**Règle** : garder les doc-comments des handlers courts et orientés API publique. Les notes
internes vont dans les commentaires `//` (pas `///`) ou dans les docs mémoire.

## import.meta.glob sous Vitest (Phase 7 Lot 1)
`import.meta.glob` est une primitive Vite — disponible sous Vitest (qui passe par Vite),
mais la logique de transformation a été isolée en fonction pure `parseLocales(glob)` pour
être testée avec des maps factices, sans dépendre du glob réel. Les modules JSON eager
exposent l'objet parsé sous `.default`.

## Thème : anti-FOUC en SPA CSR (Phase 7 Lot 1)
next-themes n'injecte son script anti-flash qu'en environnement Next.js. En SPA Vite pure
(CSR), `<html>` n'a pas `.dark` avant le montage React → flash possible. Mitigation : script
inline bloquant dans `index.html` (lit `localStorage['latch.theme']` / `prefers-color-scheme`
et pose `.dark` avant le 1er paint). `unlock.html` n'a PAS ce script (clair-only assumé).

## Quirks React (stack courante)

## Vitest + `@testing-library/jest-dom` : matchers manquants si `types` absent du tsconfig (2026-06-25)
**Symptôme** : `pnpm typecheck` échoue sur `Property 'toBeInTheDocument' does not exist on type 'Assertion<HTMLElement>'` — même si `vitest.setup.ts` importe `@testing-library/jest-dom/vitest`. **Cause** : l'augmentation de module (`declare module 'vitest' { interface Assertion ... }`) doit être visible lors de la vérification des fichiers `src/**/*.test.tsx`. Elle n'est chargée que si `@testing-library/jest-dom/vitest` est dans `types[]` de `tsconfig.app.json` (qui inclut `src/`). L'import dans `vitest.setup.ts` suffit pour le **runtime** Vitest, pas pour le **typecheck** `tsc`. **Fix** : ajouter `"@testing-library/jest-dom/vitest"` dans `compilerOptions.types` de `tsconfig.app.json`.

## jsdom n'a pas `navigator.clipboard` — stub obligatoire dans les tests CopyButton (2026-06-25)
`navigator.clipboard` est `undefined` dans l'environnement jsdom de Vitest. Tout test qui invoque `navigator.clipboard.writeText(...)` échoue avec `TypeError: Cannot read properties of undefined (reading 'writeText')`. **Fix** : dans `beforeEach`, `Object.assign(navigator, { clipboard: { writeText: vi.fn().mockResolvedValue(undefined) } })`. Ce stub écrase le descripteur pour la durée du test.

## MSW `jsonOnce` body : typer en `JsonBodyType`, pas `unknown` (2026-06-25)
`HttpResponse.json(body)` accepte `JsonBodyType` (exporté de `msw`). Passer `unknown` produit une erreur TS `Argument of type 'unknown' is not assignable to parameter of type 'JsonBodyType'`. **Fix** : importer `type JsonBodyType from 'msw'` et typer le paramètre `body` en conséquence.

## openapi-fetch capture `globalThis.fetch` au module load → wrapper MSW requis (2026-06-25)
`createClient()` de `openapi-fetch` capture `globalThis.fetch` **à l'import du module**, avant
que MSW n'installe son service worker ou son intercepteur Node. Dans les tests Vitest (jsdom),
le mock MSW n'intercède donc pas si le client est créé normalement. **Workaround** : passer un
wrapper dans `frontend/src/api/client.ts` :
```ts
const client = createClient<paths>({
  fetch: (input) => globalThis.fetch(input),   // évalué à l'appel, pas à l'import
  credentials: 'include',
});
```
Ainsi `globalThis.fetch` est résolu à l'appel, après que MSW a remplacé la référence.

## ResizeObserver polyfill requis pour Radix en jsdom (Vitest) (2026-06-25)
Les composants Radix (Popover, Sheet, Select…) appellent `ResizeObserver` en interne.
`jsdom` ne l'implémente pas → `ReferenceError: ResizeObserver is not defined` dans les tests.
**Fix** : dans `vitest.setup.ts`, avant les imports Radix :
```ts
global.ResizeObserver = class ResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
};
```

## pnpm version épinglée obligatoire — corepack sinon tire pnpm 11 (2026-06-25)
Sans `"packageManager": "pnpm@9.15.9"` dans `frontend/package.json`, corepack active
la dernière version de pnpm (actuellement 11). pnpm 11 a une politique `minimumReleaseAge`
qui rejette les paquets récemment publiés dans le lockfile (`ERR_PNPM_OUTDATED_LOCKFILE`).
**Règle** : toujours épingler `pnpm@9.15.9` (ou la version stabilisée retenue) dans
`packageManager`. Vérifier au premier `docker build` ou CI (les étapes `pnpm install`
peuvent diverger silencieusement entre local/CI si `corepack enable` est présent sans pin).

## `shadcn init --preset bJfDPe2y` : `npm_config_ignore_workspace_root_check=true` obligatoire (2026-06-25)
Le template Vite de `create-next-app` / `create vite` pose un `pnpm-workspace.yaml` dans
`frontend/`. Lors de l'exécution de `shadcn init`, pnpm détecte un « workspace root » et
refuse d'installer des paquets directement (`ERR_PNPM_ADDING_TO_ROOT`). **Workaround** :
```bash
npm_config_ignore_workspace_root_check=true pnpm dlx shadcn init --preset bJfDPe2y
```
Cette variable d'env est passée à pnpm dlx et contourne la garde de workspace root.
Le `pnpm-workspace.yaml` peut être retiré si le frontend est un package autonome (pas un
workspace pnpm multi-packages).

## radix Select sous jsdom (Phase 7 Lot 2)

Le Select radix appelle `scrollIntoView`, `hasPointerCapture`, `releasePointerCapture`,
absents de jsdom. Shims ajoutés dans `vitest.setup.ts` (à côté de `ResizeObserver`/
`elementFromPoint`). Les tests ciblent le câblage (option courante, `onValueChange` →
`changeLanguage`) plutôt que le cycle pointer interne de radix.

## Thème de marque : export générateur shadcn (oklch) → triplets HSL (2026-06-25)
La CSS vendorisée de `shadcn-rs` consomme les couleurs en **`hsl(var(--color-X))`** avec des
**triplets HSL** `H S% L%` (y compris des compositions alpha `hsl(var(--color-X) / 0.2)`). Les
générateurs de thème shadcn récents exportent en **`oklch()`** avec des noms `--background`
(sans préfixe `--color-`). On **ne peut pas** coller l'oklch tel quel (casse tous les `hsl(...)`).
**Méthode** : convertir oklch → sRGB → HSL et remplacer **uniquement les valeurs** dans
`variables.css` (`:root` + `.dark`), en gardant les noms `--color-*` et l'usage `hsl(...)` — blast
radius minimal, aucun call-site à toucher dans les 5 fichiers vendorisés, compositions alpha
préservées. Script de conversion : `scratchpad/oklch2hsl.mjs` (implémente oklab→linear-sRGB→HSL ;
gère l'alpha en compositant sur le fond). Cas particuliers : `destructive-foreground` absent de
l'export shadcn récent → quasi-blanc ; bordures/inputs dark = blanc 10%/15% **compositionné** sur
le fond sombre (l'usage `hsl(var(--color-border))` est opaque, pas d'alpha possible dans le triplet).

## Sonar analyse les `#[cfg(test)]` inline des `src/` pour la duplication, pas `backend/tests/` (2026-06-30)
SonarCloud calcule `new_duplicated_lines_density` sur tout fichier `src/`, **modules `#[cfg(test)]` inline
compris**. Des fixtures de test répétées (construire le même `Model { … }` dans 3 tests) comptent comme
duplication et peuvent faire échouer la gate (< 3 %). Les tests d'intégration sous `backend/tests/` ne sont
**pas** analysés en duplication (l'API `duplications/show` renvoie 404 dessus). **Fix** : factoriser les
fixtures inline en helpers (`fn sample_pin(owner) -> …`). La branche commentaires partait à **7,0 %** de
duplication ; deux passes de helpers (helpers de prod + fixtures de test) → **2,1 %**. Le résidu (blocs
`#[utoipa::path]` des handlers DELETE) est de la duplication déclarative irréductible, laissée sous le seuil.

## Loco : un module entité a besoin du wrapper `models/<x>.rs` en plus de `_entities/<x>.rs` (2026-06-30)
Les structs sous `models/_entities/<x>.rs` (générées par sea-orm-cli) **n'implémentent pas**
`ActiveModelBehavior`. Il faut un wrapper `backend/src/models/<x>.rs` (`impl ActiveModelBehavior for
ActiveModel {}`, calqué sur `models/versions.rs`) + le déclarer dans `models/mod.rs`. `cargo check` le
signale immédiatement si oublié (erreur sur l'`ActiveModel`).

## utoipa dérive l'`operationId` du nom de fonction → collision silencieuse (2026-06-30)
Deux handlers de même nom dans des modules différents (`serve::list_comments` + `admin::list_comments`)
produisent **le même `operationId`** dans `openapi.json` (violation OpenAPI 3 : doit être unique). Pire,
`openapi-typescript` génère deux clés identiques dans l'interface `operations` → TS fusionne **sans erreur**
et le client typé résout le **mauvais type** pour l'un des endpoints. **`pnpm typecheck` reste vert (faux-vert).**
**Fix** : noms de fonctions distincts (ici `admin::list_version_comments`) ou `operation_id = "…"` dans
`#[utoipa::path]`, puis regen. Vérif : `grep -c '"operationId": "x"' openapi.json` doit valoir 1.

## Gate de statut HTTP : `Result<_, Response>`, pas `loco_rs::Error` (sinon 403→401) (2026-06-30)
`loco_rs::Error::Unauthorized` mappe vers **401**. Pour renvoyer un **403** exact (projet verrouillé) ou un
404 précis depuis un handler, utiliser le pattern `Result<Model, Response>` (comme `resolve_project_html`)
qui renvoie `StatusCode::FORBIDDEN.into_response()` directement, consommé par `match { Ok=>_,
Err(resp)=>return Ok(resp) }`. C'est ce que fait `comments_gate`.

## axum 0.8 : chaîner `.layer()` sur un MethodRouter avec `GovernorLayer` casse l'inférence (2026-06-30)
Empiler plusieurs `.layer()` (governor + `from_fn`) directement sur un `MethodRouter` échoue à l'inférence
de type. **Fix** : bundler via `ServiceBuilder::new().layer(...).layer(...)` puis appliquer le stack. Le
middleware `require_comment_client` doit aussi renvoyer `Result<Response>` (loco, `Ok(FORBIDDEN.into_response())`),
pas `Result<_, StatusCode>`, pour s'aligner sur `require_same_origin` dans la même stack.

## SQLite n'enforce pas les FK sans PRAGMA → orphelins sur hard-delete (mais bénins) (2026-06-30)
Les FK `ON DELETE CASCADE` déclarées au schéma **ne cascadent pas** en SQLite (pas de `PRAGMA
foreign_keys=ON`). Supprimer un projet/version laisse des `comment_pins`/`comments` orphelins. **Bénin** :
sea-query émet `AUTOINCREMENT` pour les PK SQLite → un `versions.id` n'est jamais réutilisé → l'orphelin ne
refait jamais surface sous une future version (confirmé en revue finale opus). Posture identique aux fichiers
HTML orphelins sur `delete_version`. Nettoyage explicite en tx = BACKLOG.
