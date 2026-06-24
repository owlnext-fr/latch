# Phase 3 — SPA Yew admin — Design

> Spec de conception (le « quoi » et le « pourquoi »). Le plan d'implémentation
> détaillé (le « comment », tâche par tâche) vivra dans
> `docs/superpowers/plans/2026-06-24-phase-3-spa-yew-admin.md`.
>
> Issu d'un brainstorming visuel avec l'humain (2026-06-24). Les décisions ci-dessous
> sont **tranchées** ; certaines **amendent le contrat** (§4, §7) — voir la section
> dédiée, à reporter dans `docs/contrat-deploy.md` avant de coder.

## 1. Objectif & périmètre

Livrer la SPA admin (crate `latch-ui`, Yew 0.21 CSR) par-dessus l'API JSON déjà
livrée en Phase 2, et la faire servir en statique par le binaire Loco. À la sortie :
parcours admin manuel complet (login → liste → détail → créer/éditer/déployer/
activer/supprimer → logout), `wasm-bindgen-test` verts à dose mesurée.

**Hors périmètre** (autres phases) : serving `/c/<slug>` (Phase 4), endpoint MCP
(Phase 5), e2e Playwright + durcissement final (Phase 6).

## 2. Décisions tranchées (avec l'humain)

| # | Décision | Raison |
|---|---|---|
| D1 | **API déplacée sous `/api/*`** (plus `/admin/*`). | Libère tout `/admin/*` pour la navigation SPA (BrowserRouter, URLs propres, deep-links). `admin` dans l'URL était redondant (seul client = la SPA ; `/c` et `/mcp` sont d'autres surfaces) et n'apporte aucune sécurité (l'auth est dans `AdminAuth`/session). |
| D2 | **Crate partagée `latch-dto`** (serde-only, wasm-safe), dépendue par back **et** front. | Une seule source de vérité du contrat de fil → drift impossible (vérifié par le compilateur). Les DTO n'utilisent que des primitifs (dates en `String`) → wasm-safe sans effort. Le couplage sea-orm (`From<&Model>`) **reste dans le backend** (adaptateur, esprit hexagonal §1). |
| D3 | **`shadcn-rs` retenu** comme prévu au contrat §7. | Vérifié : c'est bien une lib Yew 0.21 complète (`Sheet`, `Table`, `Button`, `Input`, `Switch`, `Card`, `Toast/Sonner`, etc.). L'entrée egui de Context7 (`/ferrismind/shadcn-rs`) était un homonyme. |
| D4 | **Auth = état dérivé des codes HTTP**, pas de token en mémoire. | Le cookie de session est `HttpOnly` → invisible au JS. L'app déduit l'état de connexion d'une sonde `GET /api/projects` (401 ⇒ Login, 200 ⇒ app) ; un intercepteur unique sur tout `401` rebascule globalement en *Anonymous*. |
| D5 | **Toutes les mutations passent par un side-panel dédié** : Créer/Éditer (même composant), Déployer, Supprimer projet, Supprimer version. | Grammaire d'interaction unifiée. **Amende §7** (qui prévoyait l'édition inline + des modales de confirmation). |
| D6 | **Confirmations destructives = side-panel *danger* dédié** (plus de modale `AlertDialog`). | Cohérence avec D5. Explication d'irréversibilité + bouton d'action en bas. **Amende §7.** |
| D7 | **Page détail = lecture seule**, blocs distinctifs façon shadcn (`Card`). Actions principales (Éditer / Déployer / Supprimer) **en haut à droite**. Actions de ligne et copie = **boutons-icône** alignés à droite. | Esthétique shadcn, pas de champs éditables inline. **Amende §7.** |
| D8 | **Slug en lecture seule** en Phase 3 (dérivé du nom server-side). | L'API Phase 2 ne prend pas de base de slug éditable (`generate_slug(&name)`). La base éditable part au BACKLOG (rouvrirait le cœur). **Écart §7 assumé.** |
| D9 | **URL publique absolue via `window.location.origin`** (`origin + "/c/" + slug`). | Admin et serving partagent l'origin (`latch.owlnext.fr`). Zéro config. Un override `PUBLIC_BASE_URL` → BACKLOG (inutile tant que même host). |
| D10 | **PIN affiché généré côté SPA** dans le panel Créer/Éditer (champ éditable + régénérer). | Le panel doit afficher le PIN en live. Le cœur garde sa génération (`pin::generate_pin`) pour le chemin MCP. Le PIN saisi part dans `CreateProjectReq.pin` / `SetCodeReq.pin`. |

## 3. Architecture front-back

Un seul binaire Loco sert **tout** : les assets statiques de la SPA **et** l'API
JSON. Conséquence centrale : **même origin** → le cookie de session et l'en-tête
`Origin` partent automatiquement à chaque `fetch`, donc toute la garde
session/CSRF de Phase 2 fonctionne sans effort côté front.

```
Navigateur — SPA latch-ui (Yew → wasm, CSR)
  · routing client : yew-router (BrowserRouter)
  · client HTTP    : gloo-net (fetch, same-origin → cookies auto)
  · composants     : shadcn-rs
  · n'a AUCUN token : l'état "connecté" est dérivé des réponses HTTP
        ▲  fetch GET (liste/détail) / POST·PUT·DELETE (mutations)
        ▼  cookie de session HttpOnly + Origin envoyés automatiquement
Binaire Loco (un process)
  ├─ assets statiques : dist/ de Trunk (index.html + .wasm + .js + shadcn-rs.css)
  │     servis par tower-http ServeDir dans after_routes + fallback SPA
  └─ API JSON /api/*  : (Phase 2, re-préfixée) — AdminAuth + require_same_origin
Cœur services/ + SQLite + Storage — inchangé (la SPA ne parle qu'à l'API JSON)
```

### Machine à états d'authentification

`AuthState = Checking | Anonymous | Authenticated`, porté par un **contexte Yew**
(`AuthContext`) en haut de l'arbre.

1. **Checking** (boot) : sonde `GET /api/projects`.
2. **Anonymous** : sonde (ou tout appel) renvoie `401` → écran Login.
3. **Authenticated** : `200` → l'app. `POST /api/login` réussi fait ②→③.
4. **Logout** : `POST /api/logout` → retour ②.

**Règle transversale :** un **intercepteur unique** dans le client `gloo-net` ;
tout `401` en cours d'usage (session expirée) bascule globalement l'app en
*Anonymous* et réaffiche le Login. Pas de garde dispersée par page.

- **État global** : minimal — `AuthContext` seulement. Pas de store type Redux (~3
  écrans, YAGNI).
- **État par page** : données chargées localement (`use_state` + `use_effect` au
  montage), états `Loading / Loaded / Error`.
- **Feedback** : toasts shadcn-rs (`Sonner`) — « Copié ! », « Projet créé »,
  erreurs réseau.

## 4. Carte des routes

**API JSON (`/api/*`)** — re-préfixage des routes Phase 2 (changer `.prefix("/admin")`
→ `.prefix("/api")` dans `controllers/auth.rs` et `controllers/admin.rs`) :

| Méthode | Chemin | Rôle |
|---|---|---|
| POST | `/api/login` | login (couple env, temps constant) → pose session |
| POST | `/api/logout` | `session.destroy()` |
| GET | `/api/projects` | liste (sans PIN) — sert aussi de **sonde d'auth** |
| POST | `/api/projects` | créer |
| GET | `/api/projects/{id}` | détail (avec PIN + versions) |
| PUT | `/api/projects/{id}` | éditer (nom, brand_name) |
| DELETE | `/api/projects/{id}` | supprimer |
| POST | `/api/projects/{id}/code` | définir/changer le PIN (active le code) |
| DELETE | `/api/projects/{id}/code` | désactiver le code |
| POST | `/api/projects/{id}/deploy` | déployer une version |
| POST | `/api/projects/{id}/versions/{n}/activate` | activer une version |
| DELETE | `/api/projects/{id}/versions/{n}` | supprimer une version |
| GET | `/api/projects/{id}/versions/{n}/preview` | HTML brut `no-store`, admin-only |

**SPA (BrowserRouter, servie par ServeDir + fallback)** :

| Route client | Écran |
|---|---|
| `/admin` | Liste des projets |
| `/admin/login` | Login |
| `/admin/projects/{id}` | Détail projet |

Les mutations restent gardées par `require_same_origin` ; le navigateur envoie
`Origin` automatiquement (même origin) → OK. Le login reste sans garde Origin
(contrat §4, login-CSRF accepté).

## 5. Partage des types — crate `latch-dto`

Nouveau membre du workspace `latch-dto/` (serde uniquement). Y vivent les structs
**pures** du contrat de fil, dérivant `Serialize + Deserialize + Debug` (le back
sérialise les réponses / dé-sérialise les requêtes ; le front fait l'inverse) :

- Réponses : `ProjectListItem` (sans champ `pin`), `ProjectDetail` (avec `pin`),
  `VersionItem`. Dates en `String` (RFC 3339).
- Requêtes : `CreateProjectReq`, `UpdateProjectReq`, `SetCodeReq`, `DeployReq`,
  `LoginReq`.

Le backend (`controllers/dto.rs`) **garde** les `impl From<&projects::Model>` et
`ProjectDetail::from_model` (adaptateurs sea-orm) en important les types depuis
`latch-dto`. L'invariant §9.2 reste **structurel** : `ProjectListItem` n'a toujours
pas de champ `pin` (donc impossible à fuiter en liste), et c'est désormais vrai des
deux côtés.

`latch-dto` est buildé en natif (par le backend) **et** en wasm (par le front) —
membre normal du workspace ; le frontend reste hors `default-members`.

## 6. Structure de la crate `latch-ui`

```
frontend/src/
  main.rs            # Renderer + <BrowserRouter> + <AuthProvider>
  routes.rs          # enum Route (yew-router) + switch
  api/
    mod.rs           # client gloo-net : une fn async par endpoint, intercepteur 401
    error.rs         # ApiError (réseau / 4xx / 5xx)
  auth.rs            # AuthContext + AuthProvider (Checking|Anonymous|Authenticated)
  pages/
    login.rs
    list.rs          # liste projets
    detail.rs        # détail projet (lecture seule + actions)
  panels/
    project_form.rs  # side-panel Créer/Éditer (même composant, 2 modes)
    deploy.rs        # side-panel Déployer
    delete_project.rs# side-panel danger
    delete_version.rs# side-panel danger
  components/
    copy_button.rs   # bouton-icône copier (+ toast "Copié !")
    pin_field.rs     # PIN masqué + œil + copier ; génération/validation 6 chiffres
    ...
  util/
    pin.rs           # génération 6 chiffres + validation côté SPA
    url.rs           # window.location.origin + "/c/" + slug
```

Composants `shadcn-rs` utilisés : `Sheet` (side-panels), `Table`, `Card`,
`Button`, `Input`, `Switch`, `Label`, `Badge`, `Sonner`/`Toast`. (À confirmer via
inspection de l'API exacte de chaque composant — cf. §11.)

## 7. Écrans (contenu + comportement)

### 7.1 Login — `/admin/login` · `POST /api/login`
Carte centrée : identifiant + mot de passe + bouton « Se connecter ». État
d'erreur sur `401` (« Identifiants invalides »). Rate-limit déjà côté serveur
(`tower_governor`). Sur succès → `Authenticated` → redirection `/admin`.

### 7.2 Liste — `/admin` · `GET /api/projects`
Barre du haut : `latch` (lien `/admin`) · `+ Nouveau projet` (ouvre le side-panel
Créer) · `Logout`. Tableau : **Nom**, **URL publique** (+ bouton-icône copier),
**Code** (badge *activé* / *libre*), **Version active** (n° + date), **# versions**.
Clic sur une ligne → détail. **État vide** soigné (« Aucun projet » → CTA créer).

### 7.3 Détail — `/admin/projects/{id}` · `GET /api/projects/{id}`
En-tête : fil d'Ariane « ‹ Projets » + nom ; **en haut à droite** : `✎ Éditer`,
`⬆ Déployer`, `🗑 Supprimer`. Puis des `Card` distinctes :

- **Accès public** : URL publique absolue (lecture seule) + bouton-icône copier ;
  si code activé : PIN `••••••` + bouton-icône œil (révéler) + copier. Si libre :
  indicateur « Accès libre ».
- **Configuration** (lecture seule) : nom de marque, état du code. Modifiable via
  « Éditer ».
- **Versions** : table (n°, date, badge *active*). Actions de ligne en
  **boutons-icône à droite** : `↑` activer · `↗` prévisualiser (nouvel onglet,
  `/api/.../preview`) · `🗑` supprimer (→ side-panel danger). L'icône supprimer est
  **masquée sur la version active** (le serveur refuse en 400 de toute façon).

### 7.4 Side-panel Créer / Éditer — `Sheet` · `POST`/`PUT /api/projects[/{id}]`
Champs : **Nom** (requis) ; **Slug** (auto, lecture seule ; en création : aperçu
`mon-projet-‹suffixe›`) ; **Nom de marque** (optionnel) ; **Code d'accès** =
**toggle avec explication** (« Quand activé, les visiteurs saisissent un PIN à 6
chiffres… ») ; **PIN** (6 chiffres, éditable, bouton régénérer) visible si code
activé. Validation : nom requis, PIN à 6 chiffres si code activé. Pied : Annuler /
Enregistrer. En création, code activé par défaut (contrat §3/§7).

> Édition du code : `PUT /api/projects/{id}` porte nom + brand_name ; l'activation/
> désactivation et le PIN passent par `POST`/`DELETE /api/projects/{id}/code`. Le
> panel orchestre les appels nécessaires selon les changements.

### 7.5 Side-panel Déployer — `Sheet` · `POST /api/projects/{id}/deploy`
Sélecteur de fichier HTML + toggle « Activer immédiatement » (avec explication).
Pied : Annuler / Déployer. Le HTML lu est envoyé dans `DeployReq.html` (le panel
lit le fichier en texte côté navigateur).

### 7.6 Side-panels danger — Supprimer projet / version
Variante *danger* : titre + explication d'irréversibilité (projet : config + N
versions + URL → 404 ensuite). Pied : Annuler / « Oui, supprimer définitivement ».
`DELETE /api/projects/{id}` ou `…/versions/{n}`.

## 8. Styling

- **`shadcn-rs.css` à fournir** : la lib référence `<link rel="stylesheet"
  href="shadcn-rs.css">` mais ne livre pas la feuille comme asset Rust. Il faut la
  **vendoriser** depuis le dépôt `github.com/hughdbrown/shadcn-rs` dans
  `frontend/` et la faire copier dans `dist/` par Trunk
  (`<link data-trunk rel="copy-file" href="shadcn-rs.css">` ou `rel="css"`).
- Dark mode via variables CSS (supporté par la lib). Choix clair/sombre : défaut
  système, non bloquant.
- `index.html` : lier la CSS + `<meta robots noindex,nofollow>` déjà présent.

## 9. Serving statique + fallback (backend)

Dans `Hooks::after_routes` (déjà utilisé pour le `SessionLayer`) : monter un
`tower-http ServeDir` pointant sur le `dist/` de la SPA, avec **fallback**
`index.html` pour les routes SPA inconnues (`/admin`, `/admin/projects/{id}`…),
**sans** masquer `/api/*` (ni, plus tard, `/mcp`, `/c/*`). Ordre de montage : les
routes applicatives d'abord, le service statique + fallback en dernier.

> Détail load-bearing à valider via Context7 (loco/axum/tower-http) : comment
> composer `ServeDir` + fallback dans `after_routes` sans court-circuiter les
> routes Loco déjà enregistrées. Probable : `Router::fallback_service(ServeDir::new(dist).fallback(ServeFile::new(dist/index.html)))`.
> En Docker, `dist/` est déjà copié par l'étape Trunk (BOOTSTRAP §7) ; en dev,
> `trunk build` alimente `frontend/dist` — chemin résolu par env/config.

## 10. Stratégie de test

- **`wasm-bindgen-test` (dose mesurée, ROADMAP)** : logique pure d'abord —
  génération/validation PIN 6 chiffres (`util/pin`), construction d'URL publique
  (`util/url`), réducteur d'`AuthState`. Plus 1–2 smokes de rendu (liste vide,
  détail) si le coût reste raisonnable. L'e2e (Phase 6) porte la confiance réelle.
- **Backend (intégration Loco)** : **mettre à jour tous les tests Phase 2** pour le
  préfixe `/api` (chemins + `Origin`). Ajouter un test de serving : `GET /admin`
  rend `index.html`, `GET /admin/projects/5` (deep-link) rend aussi `index.html`,
  `GET /api/projects` reste du JSON, `GET /api/...` non masqué par le fallback.
- **`latch-dto`** : un test que back et front sérialisent/dé-sérialisent le même
  JSON (round-trip), garantissant le contrat de fil.
- Invariants §9 (sécurité) déjà couverts par `security_invariants.rs` — vérifier
  qu'ils passent sous `/api`.

## 11. Risques & à valider via Context7 (avant code)

- **`yew-router`** : version compatible Yew 0.21 (probable `0.21.x`) — épingler via
  lockfile, pas le `0.20` vu en tête de `cargo search`.
- **`gloo-net`** : API `fetch` + envoi du cookie same-origin (credentials), lecture
  du status pour l'intercepteur 401.
- **`shadcn-rs` 0.1** : API exacte (props) de `Sheet`, `Table`, `Card`, `Button`,
  `Input`, `Switch`, `Badge`, `Sonner`. Lib 0.1 instable (QUIRKS) — inspecter la
  source du crate (déjà en cache) composant par composant avant usage.
- **`shadcn-rs.css`** : récupérer la feuille réelle depuis le dépôt amont +
  intégration Trunk.
- **`tower-http ServeDir` + Loco `after_routes`** : composition fallback sans
  masquer l'API.

## 12. Amendements au contrat / BACKLOG (à acter avant code)

**`docs/contrat-deploy.md` :**
- **§4** : préfixe API = `/api/*` ; SPA servie à `/admin/*` (BrowserRouter,
  ServeDir + fallback). DTO partagés via crate `latch-dto`.
- **§7** : édition + suppression + déploiement = **side-panels dédiés** ;
  confirmations destructives = **side-panels *danger*** (remplace « modale ») ;
  page détail en **lecture seule** (édition via panel, pas inline) ; actions
  principales en haut à droite, actions de ligne/copie en **boutons-icône** ; slug
  **lecture seule** en v1 ; URL publique via `window.location.origin`.

**`docs/BACKLOG.md` :**
- Base de slug éditable (rouvrirait le cœur `slug` + l'API create).
- Override `PUBLIC_BASE_URL` (si admin et serving un jour sur hosts distincts).

## 13. Critères de sortie (ROADMAP Phase 3)

- Parcours admin manuel complet vert : login → liste → créer → détail → éditer →
  déployer → activer une version → prévisualiser → supprimer version → supprimer
  projet → logout, avec gating session (401 → Login).
- `wasm-bindgen-test` verts (dose mesurée).
- Tests backend (intégration) verts sous `/api` + test de serving/fallback.
- `cargo fmt --all` + `cargo clippy --all-targets -- -D warnings` verts (backend
  ET wasm). SPA buildée par Trunk, servie par Loco.
- Contrat (§4/§7) mis à jour ; INDEX + HANDOFF + QUIRKS/CONVENTIONS/ENVIRONMENT à
  jour.
```
