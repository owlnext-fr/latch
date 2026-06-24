# Quirks — pièges connus & contournements

> Ce qui a mordu (ou mordra) si on l'oublie. Une entrée = un piège + son contournement.
> Seedé avec les points identifiés au cadrage, avant tout code.

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
