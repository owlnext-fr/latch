# Quirks — pièges connus & contournements

> Ce qui a mordu (ou mordra) si on l'oublie. Une entrée = un piège + son contournement.
> Seedé avec les points identifiés au cadrage, avant tout code.

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

## Crate wasm (frontend) dans un workspace → `default-members` (2026-06-24)
**Symptôme** : `cargo build`/`clippy --workspace` tente de compiler `latch-ui` (Yew) pour
la cible hôte native → échoue (web-sys/wasm-only). **Cause** : un membre wasm dans un
workspace mixte. **Workaround** : `default-members = ["backend", "backend/migration"]`
dans le `Cargo.toml` racine → les commandes sans `--workspace` ignorent le frontend.
Le frontend se build via `trunk` ou `cargo … -p latch-ui --target wasm32-unknown-unknown`.

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
