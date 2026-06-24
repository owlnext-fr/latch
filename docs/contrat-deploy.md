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
dépendances façon Symfony). Le frontend est une crate Yew distincte (cible wasm32).

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
      models/
        _entities/          # SeaORM généré : projects, versions, sessions
        projects.rs …       # finders / helpers
    migration/              # migrations SeaORM
  frontend/                 # crate Yew (latch-ui), buildée par Trunk
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
- `created_at`

**`sessions`** — store de session admin (via `axum-session`).

> Insertion : à chaque deploy on insère d'abord la ligne `versions`, **puis** on
> repointe `projects.active_version_id`, le tout dans une transaction. Jamais un
> projet qui pointe vers une version à moitié écrite.

## 4. Surface `/admin` — session cookie, API JSON + SPA Yew

- **Auth = cookie de session same-origin** (l'équivalent du cookie Symfony), montée
  via `axum-session` dans `after_routes`, store table SQLite. **Pas** le système
  users/JWT natif de Loco. Compte **unique** validé contre `ADMIN_USER` / `ADMIN_PASS`
  (env), comparaison à temps constant. Pas de table `users`.
- **Pourquoi cookie et pas JWT :** client unique, same-origin. Cookie `HttpOnly`
  (non lisible en JS → pas de vol par XSS), part seul, révocation immédiate côté
  serveur. JWT n'apporterait que de la plomberie ici.
- **Cookie admin** : `HttpOnly` + `Secure` + `SameSite=Lax`.
- **CSRF** : tout endpoint **mutant** vérifie l'en-tête `Origin`/`Referer` (same-origin),
  en complément du `SameSite`. Le login est rate-limité (porte publique).
- **Rendu** : la SPA Yew est buildée par Trunk et servie en **statique** par Loco,
  avec **fallback SPA** (toute route admin inconnue → `index.html`). Les opérations
  passent par l'**API JSON** de `controllers/admin.rs`.

## 5. Surface `/mcp` — Modèle 1

- Montée dans `after_routes` via `nest_service("/mcp", StreamableHttpService)`.
- **`rmcp ≥ 1.4.0`** (la < 1.4.0 ne validait pas le `Host` → DNS rebinding,
  CVE-2026-42559). Configurer `allowed_hosts` pour inclure `latch.owlnext.fr` ;
  Caddy valide aussi le `Host` en amont (défense en profondeur).
- **Modèle 1** : l'endpoint ne réclame **rien au niveau HTTP** (Claude web s'y
  connecte sans OAuth). L'auth est **dans l'argument** : chaque tool exige un
  `deploy_token` validé contre l'env (`DEPLOY_TOKEN`).
- **Surface minimale** : `deploy_prototype(slug, html, deploy_token, [activate])`
  et `list_projects(deploy_token)`. La config des codes, la bascule de version, la
  suppression → **uniquement sur l'admin**, jamais exposées en MCP.
- **Token sur TOUS les tools, lecture comprise** : un tool MCP est public tant qu'il
  ne valide pas le token ; gater `list_projects` évite de fuiter la liste des clients.

## 6. Surface `/c/<slug>` — deux états, page de déverrouillage stylée

Pas de Basic Auth (le popup gris du navigateur casse l'expérience d'un livrable
client soigné). À la place, deux états sur la même URL :

- projet **sans code** → sert la version active.
- projet **avec code** + **cookie de déverrouillage valide** → sert la version active.
- projet **avec code** + **pas de cookie** → rend la **page de déverrouillage**
  (HTTP **200**, pas 401 — plus accueillant pour un formulaire), portant `brand_name`
  si présent (« Prototype préparé pour {brand_name} »).
- `POST /c/<slug>/unlock` : vérifie le code (`services::projects::verify_code`,
  comparaison à temps constant), pose le **cookie signé**, redirige vers le GET.

**Cookie de déverrouillage** : signé HMAC (slug + expiration), `HttpOnly` + `Secure`
+ `SameSite=Lax`, `Path=/c/<slug>`. Sans état serveur (pas de table qui gonfle à
chaque visite client). **Révocation = rotation du code du projet** (invalide les
cookies émis). La *vérification* vit dans le cœur ; rendu, pose du cookie et
rate-limit dans l'adaptateur `serve.rs`.

**Slug** : base lisible dérivée du nom + suffixe aléatoire de **8 chars base62**
(ex. `mon-projet-k7Qp2maZ`, ≈ 47 bits — quasi non-énumérable, cf. QUIRKS).
Présentable dans un mail, et noindex par-dessus.

Toutes les réponses de cette surface (page de déverrouillage **et** HTML actif) sont
en **`Cache-Control: no-store`** : le client garde un lien stable qui montre toujours
la dernière version active.

## 7. Admin — rails par page (contenu + comportement, pas layout)

Le rendu fin est laissé à `shadcn-rs`. Grammaire d'interaction : **création/édition
en side-panel** (scrim + Escape), **confirmations destructives en modale**.

- **Login** `/admin/login` — identifiant + mot de passe (couple env), erreur sur
  mauvais credentials, rate-limit. → pose le cookie de session.
- **Liste** `/admin` — tableau : nom, URL publique, badge code activé/libre, version
  active (n° + date), nb de versions. État vide conçu. Actions : « Nouveau projet »
  (side-panel), clic ligne → détail, copie rapide de l'URL par ligne.
- **Créer / éditer** — **side-panel**. Champs : nom (requis) ; slug (base éditable +
  suffixe montré) ; **code activé par défaut**, PIN **auto-généré** (6 chiffres,
  bouton régénérer, éditable) ; `brand_name` (optionnel, texte). Validation : nom
  requis, PIN à 6 chiffres si code activé.
- **Détail** `/admin/projects/<id>` — dans cet ordre :
  - *Accès public* : URL publique en lecture seule + **bouton copier** (confirmation
    « Copié ! »). Si code activé : **PIN masqué `••••••`, œil de révélation + bouton
    copier**. Si libre : indicateur « accès libre ».
  - *Config* : nom (éditable), toggle code on/off + définir/changer le PIN,
    `brand_name`. *Danger zone* : supprimer le projet (modale de confirmation,
    vocabulaire d'irréversibilité).
  - *Versions* : liste (n°, date, badge « active »). Par ligne : activer (UPDATE
    transactionnel du pointeur), **prévisualiser** (route admin-only
    `/admin/projects/<id>/versions/<n>/preview`, `no-store`, derrière la session),
    supprimer une ancienne (confirmation).
  - *Déploiement* : upload manuel d'un HTML → nouvelle version, case « activer
    immédiatement ». Même `services::deploy()` que le tool MCP. État vide : ce bloc
    passe au premier plan.
- **Retour racine** : le nom de l'app en tête est un lien vers `/admin`. Nav minimale :
  titre cliquable + logout. Compte unique → pas de menu utilisateur.
- **Logout** — action : détruit la session → redirige vers le login.

## 8. `deploy()` — ordre imposé

1. Écrire le HTML d'abord (nom temporaire **puis rename atomique** en place, via `Storage`).
2. **Ensuite** la transaction : insérer la ligne `versions`, et flipper
   `active_version_id` si `activate`.

Si la DB échoue après l'écriture : fichier orphelin (inoffensif, ramassable). L'ordre
inverse donnerait un pointeur actif vers un fichier absent — le pire état côté client.

## 9. Invariants de sécurité (non négociables, testés)

1. **Aucune réponse ne renvoie de hash**, jamais (ni web, ni MCP).
2. Le **PIN en clair** n'apparaît **que sur le détail d'un projet** — jamais dans une
   liste, jamais via MCP.
3. **`deploy_token` validé sur TOUS les tools MCP**, lecture comprise.
4. **L'auth vit dans l'adaptateur, jamais dans le cœur.** Un service suppose
   l'appelant autorisé.
5. **Rate-limit *load-bearing*** sur le déverrouillage `/c/<slug>/unlock` (un PIN à
   6 chiffres = 10⁶ combinaisons, brute-forçable en secondes sans garde-fou) et sur
   le login admin. Backoff par `IP+slug`, plafond global par slug (au prix d'un petit
   risque de DoS sur un client légitime — accepté à cet enjeu).
6. Cookie de déverrouillage **signé** et **scopé par projet** ; cookie admin
   `HttpOnly`/`Secure`/`SameSite`. Vérif `Origin` sur les mutations admin.
