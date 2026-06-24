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

## Page de déverrouillage en 200, pas 401
`/c/<slug>` protégé sans cookie rend la page-code en **HTTP 200** (formulaire
accueillant), pas un 401 (qui déclencherait le popup natif — précisément ce qu'on
fuit en remplaçant le Basic Auth).
