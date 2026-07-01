# Contrat — latch

> **Le contrat fait loi.** Archi, modèle de données, comportement des trois
> surfaces, invariants de sécurité. Toute divergence du code se résout en faveur
> de ce document — ou le document est modifié *en premier*, par décision explicite.

## 1. Principe d'architecture : couches sur squelette Loco

Loco est MVC (façon Rails), mais on ne le suit pas naïvement : on a **deux points
d'entrée qui exécutent la même opération métier** (l'API admin et l'outil MCP
déploient et listent tous les deux). On insère donc une **couche service** entre
contrôleurs et modèles. C'est une archi en couches / hexagonale légère.

- **Le cœur** (`src/services/`) — agnostique HTTP. C'est la seule couche qui porte
  de la logique métier. Il ne connaît ni `Request`, ni cookie, ni token, ni axum.
- **Adaptateurs entrants** (fins) — traduisent une requête en appel de service :
  contrôleurs web (`controllers/`), tools MCP (`mcp/`), serving client (`serve.rs`).
- **Adaptateurs sortants** — SeaORM (projects/versions/pointeur, en direct) et le
  trait `Storage` (lecture/écriture des fichiers HTML, injectable).

**Règle qui tient l'ensemble : le cœur suppose son appelant déjà autorisé.** Toute
décision d'auth est prise *avant* d'entrer dans un service — session sur l'admin,
`deploy_token` dans le tool MCP, code de déverrouillage dans `serve.rs`. Un service
comme `deploy()` ne voit jamais de secret de transport. C'est ce qui rend les
adaptateurs interchangeables au-dessus du même cœur, et le cœur testable sans HTTP.

**Erreurs :** le cœur rend un `CoreError` (thiserror), sans aucun type axum/loco.
Chaque adaptateur le mappe : web → status + JSON ; MCP → tool error. Loco garde son
`Error` pour sa plomberie ; le cœur a le sien et ignore Loco.

## 2. Structure du repo (workspace 2 crates)

Workspace à deux membres. Le **cœur vit comme module dans l'app backend**, pas en
crate séparée (choix assumé : friction minimale, cohérent avec l'injection de
dépendances façon Symfony). Le frontend est une **app React** (`frontend/`, Vite + pnpm).

```
latch/                 # workspace
  backend/                  # app Loco (cible native)
    src/
      app.rs                # Hooks::after_routes : monte MCP, session layer,
                            #   service statique SPA + fallback, robots.txt
      controllers/          # adaptateur entrant "web" — fin
        auth.rs             #   login/logout session (PAS le JWT users de Loco)
        admin.rs            #   API JSON : projets CRUD, deploy manuel, switch, config
        serve.rs            #   GET /c/<slug> + POST /c/<slug>/unlock
        health.rs
      mcp/                  # adaptateur entrant "MCP" — fin
        mod.rs              #   deploy_prototype / list_projects : valident token → services
      services/             # LE CŒUR — agnostique HTTP
        projects.rs         #   create / list / get_by_slug / set_code / clear_code / verify_code
        deploy.rs           #   deploy() : tx (insert version + flip pointeur)
        slug.rs             #   slug lisible + suffixe aléatoire
        storage.rs          #   trait Storage + FsStorage (volume)
        errors.rs           #   CoreError (thiserror)
      dto/                  # DTOs admin (inlinés, ex-latch-dto)
        mod.rs              #   ProjectListItem, ProjectDetail, VersionItem, *Req, ToSchema
      models/
        _entities/          # SeaORM généré : projects, versions, sessions
        projects.rs …       # finders / helpers
    migration/              # migrations SeaORM
  frontend/                 # app React (Vite + pnpm), buildée par `pnpm build`
    src/
      api/
        schema.d.ts         # types TS générés par openapi-typescript depuis openapi.json
        client.ts           # client openapi-fetch typé (credentials: 'include')
      hooks/                # un hook TanStack Query par endpoint
      routes/               # TanStack Router (code-based, basepath /admin)
      components/           # shadcn/ui + composants maison
      test/                 # utils renderWithProviders / renderWithRouter + MSW server
```

## 3. Modèle de données (SQLite)

**`projects`**
- `id` (PK)
- `slug` (unique) — lisible + suffixe aléatoire (voir §6)
- `name`
- `code_enabled` (bool) — **vrai par défaut à la création**
- `pin` — **récupérable** (clair ; chiffrement-au-repos = durcissement reporté, cf. BACKLOG).
  6 chiffres. Récupérable parce que l'admin doit pouvoir le copier (voir §7).
- `brand_name` (nullable) — nom de marque affiché sur la page de déverrouillage. Texte seul, pas de logo.
- `active_version_id` (FK → `versions.id`, **nullable** tant qu'aucun déploiement)
- `created_at`, `updated_at`

**`versions`**
- `id` (PK)
- `project_id` (FK → `projects.id`)
- `n` (numéro de version, incrémental par projet)
- `html_path` (chemin relatif dans le volume, géré par `Storage`)
- `release_notes` (TEXT, nullable) — notes de version en markdown léger, stockées **brutes** (jamais converties en HTML côté serveur). Longueur max **10 000 caractères** (comptage `chars()`). Au-delà : erreur 400 `invalid_params` (admin et MCP). Périmètre autorisé au rendu : paragraphes, titres, gras, italique, listes (puces + numérotées), citation. **Interdits au rendu** : liens, images, code, HTML brut (rendu `react-markdown` restreint : `skipHtml + allowedElements`).
- `created_at`

**`projects`** gains `comments_enabled` (bool, NOT NULL). Défaut sécurité-aware posé à la
création (code activé → `true`, libre → `false`) ; modifiable indépendamment ensuite.

**`comment_pins`** — point d'ancrage d'un fil de commentaires, lié à une version.
- `id` (PK) · `version_id` (FK → `versions.id`, ON DELETE CASCADE)
- `owner_token` (opaque, **jamais sérialisé** en réponse) · `anchor` (TEXT, descripteur JSON)
- `status` (TEXT, `open`/`resolved`, défaut `open`, réservé — pas d'UI v1)
- `created_at` · `updated_at` · `deleted_at` (NULL, soft-delete)

**`comments`** — message d'un fil.
- `id` (PK) · `pin_id` (FK → `comment_pins.id`, ON DELETE CASCADE)
- `owner_token` (opaque, jamais sérialisé) · `author_name` (≤ 80, auto-déclaré)
- `body` (TEXT, texte brut, ≤ 2000 caractères) · `created_at` · `updated_at` · `deleted_at` (NULL)

Le `anchor` est un descripteur JSON opaque côté serveur (jamais interprété) : `{ v, selector,
fingerprint, textQuote, offset, fallbackPoint }`. Suppression = soft-delete (`deleted_at`).

**`sessions`** — store de session admin (via `axum-session`).

> Insertion : à chaque deploy on insère d'abord la ligne `versions`, **puis** on
> repointe `projects.active_version_id`, le tout dans une transaction. Jamais un
> projet qui pointe vers une version à moitié écrite.

## 4. Surface `/admin` — session cookie, API JSON + SPA

> **Migration React livrée (2026-06-25)** : la SPA admin est désormais en **React/Vite/shadcn-ui**
> (Plans 1-3, branch `feat/admin-react`). L'ancien frontend Yew (`latch-ui`) subsiste dans
> l'historique git mais est retiré du workspace. Tout ce qui suit (auth cookie, API `/api/*`,
> serving statique sous `/admin`, garde Origin, invariants sécu) est **agnostique du framework
> front** et reste inchangé. Détail du choix : `docs/superpowers/specs/2026-06-25-admin-react-migration-decision.md`.

- **Auth = cookie de session same-origin** (l'équivalent du cookie Symfony), montée
  via `axum-session` dans `after_routes`, store table SQLite. **Pas** le système
  users/JWT natif de Loco. Compte **unique** validé contre `ADMIN_USER` / `ADMIN_PASS`
  (env), comparaison à temps constant. Pas de table `users`.
- **Pourquoi cookie et pas JWT :** client unique, same-origin. Cookie `HttpOnly`
  (non lisible en JS → pas de vol par XSS), part seul, révocation immédiate côté
  serveur. JWT n'apporterait que de la plomberie ici.
- **Cookie admin** : `HttpOnly` toujours. `Secure` + préfixe `__Host-` activés en
  production uniquement (désactivés en `Development` et `Test` pour les tests HTTP
  locaux — fail-secure : tout env inconnu futur reçoit `Secure=true` par défaut).
  `SameSite=Lax`. Cookie signé via `SESSION_SECRET` (≥ 64 bytes aléatoires, panique
  au démarrage si absent en prod).
- **Store de session** : table `sessions` dédiée, créée via une **migration SeaORM**
  (`m20260101_create_sessions_table`) pour rester cohérent avec le versioning du
  schéma. Schéma : `id TEXT PK`, `expires INTEGER NULL`, `session TEXT`.
- **Logout** : appelle `session.destroy()` (révocation immédiate côté serveur +
  invalidation du cookie — pas `session.clear()` qui laisse la ligne DB active).
- **CSRF** : tout endpoint **mutant** vérifie l'en-tête `Origin` (same-origin) via
  le middleware `require_same_origin` (`axum::from_fn`), producant un 403 sur
  cross-origin. En complément du `SameSite`. Le login est rate-limité (porte publique).
- **Login rate-limit** : `tower_governor 0.7` avec `SmartIpKeyExtractor` (lit
  `X-Forwarded-For` / `X-Real-IP` derrière Caddy), appliqué uniquement sur
  `POST /admin/login` via un layer par route.
- **Login-CSRF accepté** : `POST /admin/login` n'a intentionnellement pas de garde
  `require_same_origin` (le login doit rester publiquement accessible) ; le risque de
  login-CSRF est jugé sans impact significatif car le compte admin est unique et partagé
  — connecter un visiteur dans la session de l'attaquant ne lui confère aucun privilège
  supplémentaire.
- **Rendu** : la SPA React est buildée par Vite (`pnpm build`) et servie en **statique** par
  Loco, avec **fallback SPA** (toute route admin inconnue → `index.html`). Les opérations
  passent par l'**API JSON** de `controllers/admin.rs`.
- **Préfixage des routes** : l'API JSON est servie sous le préfixe **`/api/*`** (re-préfixée
  depuis `/admin/*`) ; la SPA React est servie en statique sous **`/admin/*`** via
  `nest_service("/admin", ServeDir + fallback index.html)` câblé dans `after_routes`.
  **`/admin`** et **`/api`** coexistent sur le même binaire Loco sans conflit.
  Le routeur React est TanStack Router (code-based, `basepath: '/admin'`).
- **Contrat de fil (types TS)** : les types sérialisés échangés entre backend et frontend
  sont générés par **`openapi-typescript`** depuis `openapi.json` (commité, testé par drift)
  vers `frontend/src/api/schema.d.ts`. Le client HTTP est **`openapi-fetch`** typé, avec
  `credentials: 'include'` (cookie de session) ; le wrapper `fetch` est configuré pour
  permettre l'interception MSW en test (cf. QUIRKS). Les conversions `Model → DTO` côté backend
  restent des fonctions libres (`dto::to_list_item` / `dto::to_detail`).

## 5. Surface `/mcp` — Modèle 1

- Montée dans `after_routes` via `nest_service("/mcp", StreamableHttpService)` avec
  `LocalSessionManager` (une session MCP par requête HTTP, sans état côté serveur).
- **`rmcp` épinglé `"1.4"` (floor), résout en 1.8.0.** La < 1.4.0 ne validait pas le
  `Host` → DNS rebinding (CVE-2026-42559). Features : `server, macros, transport-streamable-http-server`.
  `schemars` via `rmcp::schemars` (re-export). `allowed_hosts` dérivé de `LATCH_PUBLIC_BASE_URL`
  via `web::host_authority(public_base_url)` — source unique, non bypassable.
- **`LATCH_PUBLIC_BASE_URL`** (runtime, requis en prod, fail-secure) : URL publique racine de
  l'instance (ex. `https://latch.owlnext.fr`). Slash terminal normalisé. Utilisée par :
  (a) `allowed_hosts` rmcp — dérivé via `web::host_authority()` ; (b) valeur `url` retournée
  par `deploy_prototype`. Le boot refuse de démarrer si absente hors Dev/Test.
- **`DEPLOY_TOKEN`** fail-secure : résolu au démarrage, boot refusé si absent hors Dev/Test.
- **Modèle 1** : l'endpoint ne réclame **rien au niveau HTTP** (Claude web s'y
  connecte sans OAuth). L'auth est **dans l'argument** : chaque tool exige un
  `deploy_token` validé contre l'env (`DEPLOY_TOKEN`) via comparaison à temps constant
  (`secure_compare`) — **premier geste, avant tout appel service**.
- **Surface minimale** : `deploy_prototype(slug, html, deploy_token, activate?)`
  et `list_projects(deploy_token)`. La config des codes, la bascule de version, la
  suppression → **uniquement sur l'admin**, jamais exposées en MCP.
- **Token sur TOUS les tools, lecture comprise** : un tool MCP est public tant qu'il
  ne valide pas le token ; gater `list_projects` évite de fuiter la liste des clients.

### 5.1 Réponses des tools

**`deploy_prototype(slug, html, deploy_token, activate?, release_notes?)`**
- `slug` doit **préexister** en base : aucune auto-création. Slug inconnu → erreur.
- `activate` : défaut **`true`** (la version déployée devient immédiatement active).
- `release_notes` : optionnel. Markdown léger (max 10 000 caractères), **stocké brut en tant que
  texte Markdown** (jamais convertis en HTML au stockage). Au-delà : erreur `invalid_params`.
  Les liens, images, code et HTML brut sont **ignorés au rendu** côté client (barrière de rendu
  réalisée par le composant `MarkdownView` restreint, jamais filtrés à l'écriture).
- Token validé EN PREMIER (`secure_compare`) — avant toute lecture en base.
- Réponse (succès) : `DeployResult { url: "<LATCH_PUBLIC_BASE_URL>/c/<slug>", version: <n>, code_protected: <bool> }`.
  Le champ `url` utilise `LATCH_PUBLIC_BASE_URL` comme source de vérité.
  **Jamais de hash, jamais de PIN.**

**`list_projects(deploy_token)`**
- Token validé EN PREMIER.
- Réponse : enveloppe objet **`{ projects: [...] }`** (`ProjectListResult`), **PAS** un tableau racine.
  Pourquoi : rmcp 1.8 panique à la construction du `tool_router` si le type de sortie d'un tool
  a un schéma JSON de type `array` à la racine (MCP exige `object`) → enveloppe obligatoire.
- Chaque entrée : `ProjectSummary { slug, name, code_protected, active_version: Option<i32> }`.
  **Jamais de hash, jamais de PIN, jamais de `id` DB.**

## 6. Surface `/c/<slug>` — shell + iframe, unlock, notes de version

Pas de Basic Auth (le popup gris du navigateur casse l'expérience d'un livrable
client soigné). La surface `/c/<slug>` est désormais architecturée en **shell + iframe** :

### 6.1 Structure shell / iframe

`GET /c/<slug>` sert **toujours** une **page-coquille** (shell HTML), qui charge le
prototype réel dans un `<iframe src="/c/<slug>/raw">`. Cela permet d'injecter
l'overlay de notes de version sans modifier l'HTML du proto.

**Nouveaux endpoints de la surface `/c` :**

- **`GET /c/<slug>/raw`** : HTML brut du prototype actif (cible de l'iframe).
  En-têtes : `Cache-Control: no-store` + `Content-Security-Policy: frame-ancestors 'self'`
  (empêche l'embarquement du proto dans un contexte tiers). Gardé par le même gate
  d'unlock que le shell : projet protégé sans cookie valide → `403`.

- **`GET /c/<slug>/notes`** : JSON `{ n: <numéro_version>, notes_md: "<markdown>" }` si
  la version active a des notes, ou `204` si aucune note. `Cache-Control: no-store`.
  **Gardé par le même gate unlock** (403 si proto protégé non déverrouillé) → pas de
  fuite des notes avant authentification.

**Arbre de décision (shell) :**

- projet **sans code** → sert le shell (200).
- projet **avec code** + **cookie de déverrouillage valide** → sert le shell (200).
- projet **avec code** + **pas de cookie** → rend la **page de déverrouillage**
  (HTTP **200**, pas 401), portant `brand_name` si présent.
- `POST /c/<slug>/unlock` : vérifie le code (`services::projects::verify_code`,
  comparaison à temps constant), pose le **cookie signé**, redirige vers le GET.

**Distinction des endpoints `/c/<slug>`, `/c/<slug>/raw`, `/c/<slug>/notes` :**

- **`GET /c/<slug>` (shell)** — répond **200** (page de déverrouillage) quand le projet
  est protégé sans cookie. L'utilisateur y entre le code et se déverrouille.
- **`GET /c/<slug>/raw` (iframe, HTML brut du proto)** — répond **403 Forbidden** si le
  projet est protégé et le cookie de déverrouillage est absent ou invalide. Le gate est
  strict : aucune page de déverrouillage, accès refusé.
- **`GET /c/<slug>/notes` (JSON des notes)** — répond **403 Forbidden** si le projet est
  protégé et le cookie est absent ou invalide. Pas de contenu utile servi avant déverrouillage.

Résumé : le shell (200 + formulaire) guide vers le déverrouillage, tandis que `/raw` et
`/notes` refusent sèchement (403) tout accès non authentifié.

**Compromis assumé** : tous les prototypes tournent désormais en iframe. Impacts
potentiels : `window.top` (accessible depuis le proto = le shell), fullscreen API,
et toute API qui se comporte différemment en contexte iframe. À mentionner dans la
documentation publique.

### 6.2 Cookie de déverrouillage

Signé HMAC (slug + expiration), `HttpOnly` + `Secure` + `SameSite=Lax`,
`Path=/c/<slug>`. Sans état serveur. **Révocation = rotation du code du projet**
(invalide les cookies émis). La *vérification* vit dans le cœur ; rendu, pose du
cookie et rate-limit dans l'adaptateur `serve.rs`.

**Slug** : base lisible dérivée du nom + suffixe aléatoire de **8 chars base62**
(ex. `mon-projet-k7Qp2maZ`, ≈ 47 bits — quasi non-énumérable, cf. QUIRKS).
Présentable dans un mail, et noindex par-dessus.

Toutes les réponses de cette surface sont en **`Cache-Control: no-store`** : le
client garde un lien stable qui montre toujours la dernière version active.

### 6.3 Overlay de notes de version (côté visiteur)

Le shell consomme `GET /c/<slug>/notes` après avoir obtenu l'accès (unlock ou libre).
Si la réponse contient des notes (`notes_md`) et que `localStorage['latch:seen:<slug>']`
est différent du numéro de version `n` courant, le shell affiche un **overlay** par-
dessus l'iframe avec les notes rendues en markdown restreint. Au dismiss, la clé
`latch:seen:<slug>` est mise à jour avec `n` → l'overlay ne réapparaît plus pour
cette version.

**Rendu markdown** : composant `MarkdownView` restreint (`react-markdown` avec
`skipHtml + allowedElements`) — même périmètre que l'aperçu admin (§7). Jamais de
HTML serveur pour les notes.

### 6.4 Commentaires ancrés (`/c/<slug>/comments`)

Toutes en `Cache-Control: no-store`, **gardées par `unlock_ok` + `comments_enabled`**
(projet à code non déverrouillé → 403 ; commentaires désactivés → 404). Les **écritures**
portent en plus : garde **Origin** same-origin, header **`X-Comment-Client`** exigé,
cookie d'identité `latch_comment` (ULID opaque, signé, `HttpOnly`/`Secure`/`SameSite=Lax`,
`Path=/c/<slug>`, secret = `UNLOCK_COOKIE_SECRET` réutilisé), et un rate-limit dédié
(`LATCH_COMMENT_RL_*`).

- `GET    /c/<slug>/comments` — mes pins+fils de la version active (filtré `owner_token`).
- `POST   /c/<slug>/comments` — crée un pin + 1ᵉʳ message ; pose le cookie d'identité si absent.
- `POST   /c/<slug>/comments/pins/<pin>/replies` — ajoute un message à mon pin.
- `PUT    /c/<slug>/comments/messages/<id>` — édite mon message.
- `DELETE /c/<slug>/comments/messages/<id>` — supprime mon message (soft ; si dernier → pin soft-deleted).
- `DELETE /c/<slug>/comments/pins/<pin>` — supprime mon pin entier.

Réponses : `owner_token` **jamais** présent ; chaque message porte `editable: bool` (calculé
par appelant). MCP `deploy_prototype` inchangé (ne touche pas aux commentaires).

**Écriture admin — 4 endpoints (authoring)**, en plus des endpoints de lecture/modération déjà
listés en §7. L'unique compte admin possède un **jeton sentinelle** `ADMIN_OWNER_TOKEN` (constant,
non issu de `mint_owner_token()` — voir §9) : ces routes réutilisent le même cœur `CommentsService`
que la surface visiteur, avec cette sentinelle comme `owner_token`. Toutes sous `AdminAuth` +
`require_same_origin` ; **pas** de garde `X-Comment-Client` (spécifique à la surface visiteur), pas
de cookie `latch_comment`.

- `POST   /api/projects/{id}/versions/{n}/comments` — l'admin démarre **son propre fil** (note
  privée de relecture, cf. §7 : visible **seulement** en Review admin, jamais diffusée aux
  visiteurs). Réutilise `create_pin(version_id, ADMIN_OWNER_TOKEN, ADMIN_AUTHOR, …)`.
- `POST   /api/projects/{id}/comments/pins/{pin}/replies` — l'admin **répond à n'importe quel pin
  du projet** (fil d'un visiteur ou le sien). Nouvelle méthode de service `admin_add_reply(project_id,
  pin_id, body)` : résout pin→version→projet (comme `moderate_delete_message`), **sans owner-check**
  (l'admin peut répondre à tout fil de son projet), 404 si le pin n'appartient pas au projet. La
  réponse redevient visible du visiteur propriétaire du fil via son `GET /c/<slug>/comments` habituel.
- `PUT    /api/projects/{id}/comments/messages/{cid}` — l'admin édite un de **ses** messages.
  Réutilise `edit_message(cid, ADMIN_OWNER_TOKEN, body)` : l'owner-check interne restreint
  naturellement à ses propres messages (message visiteur → 404, pas d'escalade).
- `DELETE /api/projects/{id}/comments/pins/{pin}` — l'admin supprime un de **ses** fils entiers.
  Réutilise `delete_pin(pin, ADMIN_OWNER_TOKEN)` : owner-check interne restreint aux pins propres
  de l'admin (pin visiteur → 404 ; la suppression d'un message visiteur individuel reste
  `moderate_delete_message`, déjà couverte).

Identité non usurpable : le `author_name` envoyé par le client sur ces 4 endpoints est **ignoré** —
le serveur pose toujours `ADMIN_AUTHOR` (« admin », valeur brute jamais affichée) ; l'UI rend le
libellé i18n « Admin » via le booléen dérivé `is_admin` (§9), jamais le `author_name` brut.

## 7. Admin — rails par page (contenu + comportement, pas layout)

Le rendu fin est assuré par **shadcn/ui** (Radix, base stone, thème oklch `bJfDPe2y`).
Grammaire d'interaction : **création/édition en side-panel** (`<Sheet>` Radix, scrim + Escape),
**confirmations destructives en side-panels *danger*** (les confirmations irréversibles sont des
`<Sheet>` danger, pas des Dialog/Modal). La page détail est en **lecture seule** (toute édition
passe par un side-panel dédié, jamais inline). Actions principales en haut à droite, actions de
ligne et copie en **boutons-icône**. **Le slug est en lecture seule** en v1 (base éditable
reportée au BACKLOG). L'URL publique est construite côté SPA via `window.location.origin`
(pas de variable d'env `PUBLIC_BASE_URL` en v1 — admin et serving `/c` partagent la même
origine). L'UI est **internationalisée FR + EN** via `react-i18next`, défaut EN, langue
persistée en localStorage.

- **Login** `/admin/login` — identifiant + mot de passe (couple env), erreur sur
  mauvais credentials, rate-limit. → pose le cookie de session.
- **Liste** `/admin` — tableau : nom, URL publique, badge code activé/libre, version
  active (n° + date), nb de versions. État vide conçu. Actions : « Nouveau projet »
  (side-panel), clic ligne → détail, copie rapide de l'URL par ligne en **bouton-icône**.
- **Créer / éditer** — **side-panel dédié** (`ProjectForm`). Champs : nom (requis) ;
  slug (affiché en lecture seule, suffixe aléatoire — base non éditable en v1) ;
  **code activé par défaut**, PIN **auto-généré** (6 chiffres, bouton régénérer,
  éditable via `PinField`) ; `brand_name` (optionnel, texte). Validation : nom requis,
  PIN à 6 chiffres si code activé.
- **Détail** `/admin/projects/<id>` — page en **lecture seule** ; dans cet ordre :
  - *Accès public* : URL publique en lecture seule + **bouton copier** (confirmation
    « Copié ! »). Si code activé : **PIN masqué `••••••`** (`PinField`), œil de
    révélation + bouton copier. Si libre : indicateur « accès libre ».
  - *Actions* (haut à droite) : « Modifier » (ouvre le side-panel `ProjectForm` en mode
    édition), « Déployer » (ouvre `DeployPanel`), « Supprimer » (ouvre le
    **side-panel danger** de suppression projet).
  - *Versions* : liste (n°, date, badge « active »). Par ligne : activer (UPDATE
    transactionnel du pointeur), **prévisualiser** (route admin-only
    `/api/projects/<id>/versions/<n>/preview`, `no-store`, derrière la session),
    supprimer via **side-panel danger** (refuse si version active → 400).
  - *Déploiement* : upload manuel d'un HTML → `DeployPanel` (side-panel) → nouvelle
    version, case « activer immédiatement ». Même `services::deploy()` que le tool MCP.
    État vide : ce bloc passe au premier plan.
- **Commentaires** : toggle `comments_enabled` par projet dans `ProjectForm` (défaut sécurité-aware) ;
  **page Review** `/admin/projects/<id>/versions/<n>/review` : iframe plein-écran (`previewUrl`, `frame-ancestors 'self'`)
  avec overlay lazy `CommentsApp` en mode admin (`createAdminAdapter` : lecture + **authoring**
  (créer son propre fil, répondre à un fil visiteur, éditer ses messages) + modération) ;
  `VersionCommentsPanel` accessible depuis le détail projet (liste lecture seule
  `GET /api/projects/<id>/versions/<n>/comments` + modération `DELETE /api/projects/<id>/comments/messages/<id>`,
  vérifie l'appartenance au projet). **Écriture admin (4 endpoints, sous `AdminAuth` +
  `require_same_origin`, PAS de `X-Comment-Client` — c'est la garde visiteur)** : voir §6.4.
  Identité forcée serveur (libellé i18n « Admin », badge discret à côté du nom sur `/c` **et**
  Review) — le client ne choisit ni le nom, ni l'`owner_token`.
- **Retour racine** : le nom de l'app en tête est un lien vers `/admin`. Nav minimale :
  titre cliquable + **sélecteur de langue FR/EN** + logout. Compte unique → pas de menu utilisateur.
  L'UI est **internationalisée (FR + EN, défaut EN)** via `react-i18next` ; la langue est persistée
  (localStorage) et détectée du navigateur au premier accès. Détails d'implémentation : CONVENTIONS.
- **Logout** — action : détruit la session → redirige vers le login.

## 8. `deploy()` — ordre imposé

1. Écrire le HTML d'abord (nom temporaire **puis rename atomique** en place, via `Storage`).
2. **Ensuite** la transaction : insérer la ligne `versions`, et flipper
   `active_version_id` si `activate`.

Si la DB échoue après l'écriture : fichier orphelin (inoffensif, ramassable). L'ordre
inverse donnerait un pointeur actif vers un fichier absent — le pire état côté client.

## 9. Invariants de sécurité (non négociables, testés)

1. **Aucune réponse ne renvoie de hash**, jamais (ni web, ni MCP). Garanti
   structurellement : aucun DTO admin n'expose de champ hash.
2. Le **PIN en clair** n'apparaît **que sur le détail d'un projet** — jamais dans une
   liste, jamais via MCP. Garanti structurellement : `ProjectListItem` n'a pas de
   champ `pin` (pas de `#[serde(skip)]` — le champ est absent du type).
3. **`deploy_token` validé sur TOUS les tools MCP**, lecture comprise.
4. **L'auth vit dans l'adaptateur, jamais dans le cœur.** Un service suppose
   l'appelant autorisé. Vérifiée par la garde d'architecture `backend/tests/architecture.rs`
   (échoue si `use axum::` ou `use loco_rs::` apparaît dans `src/services/`).
5. **Rate-limit *load-bearing*** sur le déverrouillage `/c/<slug>/unlock` (un PIN à
   6 chiffres = 10⁶ combinaisons, brute-forçable en secondes sans garde-fou) et sur
   le login admin (`tower_governor` + `SmartIpKeyExtractor`, lit `X-Forwarded-For`
   derrière Caddy). Backoff par `IP+slug`, plafond global par slug (au prix d'un petit
   risque de DoS sur un client légitime — accepté à cet enjeu).
6. Cookie de déverrouillage **signé** et **scopé par projet** ; cookie admin
   `HttpOnly`/`Secure`/`SameSite`. Vérif `Origin` sur les mutations admin via
   `require_same_origin` (middleware `axum::from_fn`, 403 cross-origin).
7. **`owner_token` jamais sérialisé** (réponse publique ou admin) — `editable: bool` à la place.
   Le gate `unlock_ok` + `comments_enabled` couvre toutes les routes commentaires. **L'unique
   compte admin est modélisé par un jeton sentinelle constant `ADMIN_OWNER_TOKEN = "__admin__"`**
   (`backend/src/services/comments.rs`, aucune migration DB, aucune colonne de rôle) — non
   collisionnable avec un `owner_token` visiteur (ULID Crockford base32, jamais des underscores).
   Sur les deux surfaces DTO (`CommentMessage` public et `AdminCommentMessage` admin), un booléen
   **dérivé** `is_admin = (owner_token == ADMIN_OWNER_TOKEN)` est sérialisé — **jamais** le token
   lui-même. Anti-usurpation : les écritures visiteur lisent l'owner uniquement depuis le cookie
   signé `latch_comment` (un client ne peut pas se forger `owner_token = "__admin__"`) ; les
   écritures admin sont derrière `AdminAuth` et pose la sentinelle côté serveur (§6.4/§7).

### Note (hors-invariants) — `GET /api/settings` et le `deploy_token`

`GET /api/settings` (protégé par `AdminAuth`, donc 401 sans session) expose :
`{ deploy_token, mcp_url, public_base_url }`. Le `deploy_token` est bien exposé ici,
à l'admin **authentifié**, pour lui permettre de configurer le connecteur Claude.

Pourquoi cela ne viole PAS les invariants §9.1 et §9.2 :
- Le `deploy_token` n'est **pas un hash** (§9.1) — c'est un secret applicatif, pas un
  dérivé cryptographique de données utilisateur.
- Le `deploy_token` n'est **pas un PIN projet** (§9.2) — les invariants §9.2 portent
  sur le PIN de déverrouillage client ; `deploy_token` est le secret de l'adaptateur MCP.
- L'endpoint est derrière `AdminAuth` : un visiteur non authentifié reçoit 401.

`mcp_url = public_base_url + "/mcp"` (valeur informative pour la configuration du
connecteur Claude). `public_base_url` = valeur de `LATCH_PUBLIC_BASE_URL`.
