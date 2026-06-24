# Phase 2 — Adaptateur web admin (API JSON + session) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Câbler l'adaptateur web admin de `latch` — auth par cookie de session SQLite, API JSON CRUD projets + déploiement manuel + bascule/preview de versions — au-dessus du cœur `services/` déjà livré (Phase 1), sans jamais polluer le cœur de types HTTP.

**Architecture:** Adaptateurs entrants fins (`controllers/`) qui (1) décident l'auth *avant* d'appeler un service, (2) traduisent `CoreError` → status HTTP + JSON, (3) exposent des DTO sérialisés qui font respecter l'invariant de sécurité « PIN seulement sur le détail ». Session stateful via `axum-session` branchée dans `Hooks::after_routes`, table `sessions` créée par une migration SeaORM dédiée. Rate-limit du login via `tower_governor`. Garde `Origin` sur toutes les mutations.

**Tech Stack:** Loco 0.16 (axum 0.8) · SeaORM 1.1 · `axum-session` + `axum_session_sqlx` (store SQLite) · `tower_governor` (rate-limit) · `subtle` (compare temps constant, déjà dep) · helpers de test Loco (`loco_rs::testing`) + SQLite de test.

## Global Constraints

- **Le cœur (`src/services/`) ne voit jamais axum/loco.** Aucun `use axum::`/`use loco_rs::` sous `services/`. Vérifié par `backend/tests/architecture.rs` (récursif, détecte aussi `pub use`). Tout le code de cette phase vit dans `controllers/`, `models/`, `migration/`, `app.rs` — jamais dans `services/`.
- **Invariants de sécurité (contrat §9), testés, cassent le build si violés :**
  1. Aucune réponse (web) ne contient de hash. *(latch ne hash rien : l'invariant se teste en s'assurant qu'aucun champ secret ne fuit.)*
  2. Le **PIN en clair** n'apparaît **que** sur le détail d'un projet — jamais dans la liste.
  3. L'auth vit dans l'adaptateur, jamais dans le cœur (un service suppose l'appelant autorisé).
  4. Rate-limit *load-bearing* sur `/admin/login`.
  5. Cookie admin `HttpOnly` + `Secure` (prod) + `SameSite=Lax` ; vérif `Origin` sur les mutations.
- **Confidentialité (NON-NÉGOCIABLE) :** aucun nom de client réel où que ce soit. Placeholders `Mon Projet`/`mon-projet`, `ACME`, `demo`.
- **Pas d'`unwrap`/`expect`** hors tests et hors init `main`. Erreurs propagées.
- **Commits** : `<gitmoji> <type>: <desc>` conventionnel (`✨ feat:`, `🐛 fix:`, `📝 docs:`, `🧱 chore:`, `✅ test:`, `♻️ refactor:`).
- **Lancer le serveur** depuis `backend/` (Loco lit `./config` au CWD). `fmt`/`clippy`/`test` depuis la racine. SQLite de test in-memory = `max_connections(1)` (LOAD-BEARING).
- **Definition of done** par tâche : `cargo fmt --all` + `cargo clippy --all-targets -- -D warnings` verts, tests de la tâche verts, commit. En fin de phase : mémoire mise à jour (INDEX, HANDOFF, QUIRKS, CONVENTIONS, ENVIRONMENT).
- **Context7 d'abord** avant toute API loco/sea-orm/axum-session/tower_governor non triviale (versions épinglées du lockfile).

### Décisions Phase 2 (tranchées avec l'humain — à reporter dans le contrat avant code)

- **Table `sessions` = migration SeaORM dédiée** (schéma sous notre contrôle), au schéma exact attendu par axum-session.
- **Rate-limit login = `tower_governor`** (GCRA, `SmartIpKeyExtractor`).
- Storage root de prod = volume `/data` ; dev = `./data`. Résolu via env `LATCH_STORAGE_ROOT`.
- Cookie de session signé via clé HMAC env `SESSION_SECRET` (64 octets) ; `Secure`/`__Host-` activés **uniquement** hors `development`.

---

## File Structure

**Créés :**
- `backend/migration/src/m20260624_000003_create_sessions.rs` — migration table `sessions` (schéma axum-session).
- `backend/src/controllers/error.rs` — mapping `CoreError` → `loco_rs::Error` (status HTTP).
- `backend/src/controllers/dto.rs` — DTO sérialisés : `ProjectListItem` (sans PIN), `ProjectDetail` (avec PIN), `VersionItem`, requêtes.
- `backend/src/controllers/auth.rs` — login / logout, extracteur `AdminAuth`.
- `backend/src/controllers/admin.rs` — API JSON projets + déploiement + versions.
- `backend/src/controllers/middleware/mod.rs` + `backend/src/controllers/middleware/origin.rs` — garde same-origin sur mutations.
- `backend/src/web/mod.rs` — helpers adaptateur : `storage_from_ctx`, `session_store`, résolution config/env.
- `backend/tests/admin_api.rs` — tests d'intégration bout-en-bout (Loco testing harness).
- `backend/tests/security_invariants.rs` — tests d'invariants §9 (PIN jamais en liste, pas de fuite).

**Modifiés :**
- `backend/Cargo.toml` — deps `axum-session`, `axum_session_sqlx`, `tower_governor`, `tower`, `time` (dev: rien de neuf).
- `backend/migration/src/lib.rs` — enregistrer la migration sessions.
- `backend/src/app.rs` — `after_routes` (session layer), montage des routes auth + admin.
- `backend/src/controllers/mod.rs` — déclarer `auth`, `admin`, `dto`, `error`, `middleware`.
- `backend/src/lib.rs` — déclarer `pub mod web;`.
- `.env.example`, `docs/ENVIRONMENT.md` — nouvelles env (`SESSION_SECRET`, `LATCH_STORAGE_ROOT`).

---

## Task 1: Migration SeaORM de la table `sessions`

**Files:**
- Create: `backend/migration/src/m20260624_000003_create_sessions.rs`
- Modify: `backend/migration/src/lib.rs`
- Test: `backend/src/services/mod.rs` (ajouter un `#[tokio::test]` dans le module `migration_tests` existant)

**Interfaces:**
- Consumes: rien (première tâche).
- Produces: table `sessions` migrée, colonnes **exactes** attendues par axum-session SQLite : `id TEXT PRIMARY KEY NOT NULL`, `expires INTEGER NULL`, `session TEXT NOT NULL`. Aucune autre tâche n'en dépend en code, mais Task 2 (câblage axum-session) **suppose ce schéma**.

> **⚠️ Schéma load-bearing.** axum-session exécute `CREATE TABLE IF NOT EXISTS <table>` au boot (`SessionStore::new`). Si notre table préexiste avec des colonnes différentes, le `CREATE` est ignoré silencieusement et les `INSERT` d'axum-session échouent au runtime. Les noms `id` / `expires` / `session` et leur nullabilité doivent matcher.

- [ ] **Step 1: Écrire le test qui échoue** (dans `backend/src/services/mod.rs`, module `migration_tests`)

```rust
    #[tokio::test]
    async fn sessions_table_has_axum_session_schema() {
        use sea_orm::{ConnectionTrait, Statement};
        let db = test_db().await;
        // INSERT au schéma axum-session : id TEXT PK, expires INTEGER NULL, session TEXT NOT NULL.
        let stmt = Statement::from_string(
            db.get_database_backend(),
            "INSERT INTO sessions (id, expires, session) VALUES ('abc', NULL, '{}')".to_string(),
        );
        db.execute(stmt).await.expect("insert dans sessions doit réussir");
    }
```

- [ ] **Step 2: Lancer le test, vérifier qu'il échoue**

Run: `cargo test -p latch --lib services::migration_tests::sessions_table_has_axum_session_schema`
Expected: FAIL — `no such table: sessions` (la migration n'existe pas encore).

- [ ] **Step 3: Écrire la migration** (`backend/migration/src/m20260624_000003_create_sessions.rs`)

```rust
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Sessions {
    Table,
    Id,
    Expires,
    Session,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Sessions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Sessions::Id)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Sessions::Expires).big_integer().null())
                    .col(ColumnDef::new(Sessions::Session).text().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Sessions::Table).to_owned())
            .await
    }
}
```

- [ ] **Step 4: Enregistrer la migration** (`backend/migration/src/lib.rs`)

Ajouter `mod m20260624_000003_create_sessions;` avec les autres `mod`, puis pousser dans le `Vec` de `migrations()` **après** `m20260624_000002_create_versions` :

```rust
            Box::new(m20260624_000003_create_sessions::Migration),
```

- [ ] **Step 5: Lancer le test, vérifier qu'il passe**

Run: `cargo test -p latch --lib services::migration_tests::sessions_table_has_axum_session_schema`
Expected: PASS.

- [ ] **Step 6: Appliquer en dev + commit**

```bash
cd backend && cargo loco db migrate && cd ..
cargo fmt --all && cargo clippy --all-targets -- -D warnings
git add backend/migration backend/src/services/mod.rs
git commit -m "✨ feat: migration table sessions (schéma axum-session)"
```

---

## Task 2: Dépendances + câblage `axum-session` dans `after_routes` + helpers `web`

**Files:**
- Modify: `backend/Cargo.toml`
- Create: `backend/src/web/mod.rs`
- Modify: `backend/src/lib.rs` (ajouter `pub mod web;`)
- Modify: `backend/src/app.rs` (implémenter `after_routes`)
- Test: `backend/tests/admin_api.rs` (smoke : l'app boote avec le layer)

**Interfaces:**
- Consumes: table `sessions` (Task 1).
- Produces:
  - Type alias `pub type SessionPool = axum_session_sqlx::SessionSqlitePool;` et `pub type AdminSession = axum_session::Session<SessionPool>;` (réexportés depuis `web`), consommés par `auth.rs` (Task 4) et `admin.rs`.
  - `pub fn storage_from_ctx(ctx: &AppContext) -> std::sync::Arc<dyn crate::services::storage::Storage>` — résout `LATCH_STORAGE_ROOT` (défaut `data`).
  - `pub async fn build_session_store(ctx: &AppContext) -> loco_rs::Result<axum_session::SessionStore<SessionPool>>`.

> Context7 (avant d'écrire) : `axum_session` `SessionConfig`/`SessionStore`/`SessionLayer`, `axum_session_sqlx::SessionSqlitePool`, et `SeaORM 1.1 DatabaseConnection::get_sqlite_connection_pool`. Vérifier la version d'`axum_session`/`axum_session_sqlx` compatible sqlx 0.8 (Loco). **Spike :** si l'arbre sqlx d'axum_session_sqlx diverge de celui de Loco, épingler la version qui partage sqlx 0.8 ; consigner en QUIRKS.

- [ ] **Step 1: Ajouter les dépendances** (`backend/Cargo.toml`, section `[dependencies]`)

```toml
tower = { version = "0.5" }
tower_governor = { version = "0.7" }
axum_session = { version = "0.16" }
axum_session_sqlx = { version = "0.5", features = ["sqlite", "tls-rustls"] }
time = { version = "0.3" }
```

> Les numéros ci-dessus sont un point de départ : **résoudre via Context7/`cargo update -p` la version réelle compatible sqlx 0.8** et figer dans `Cargo.lock`. Vérifier `cargo deny check licenses advisories` après ajout (cf. QUIRKS cargo-deny : ajouter toute licence permissive nouvellement rencontrée à `allow` dans `deny.toml`).

- [ ] **Step 2: Écrire le helper `web`** (`backend/src/web/mod.rs`)

```rust
//! Helpers de l'adaptateur web (HTTP). Hors du cœur : c'est ici que vivent
//! session, storage concret, résolution d'environnement. Le cœur reste agnostique.

use std::sync::Arc;

use loco_rs::app::AppContext;
use loco_rs::Result;

use crate::services::storage::{FsStorage, Storage};

/// Store de session adossé au pool SQLite de Loco.
pub type SessionPool = axum_session_sqlx::SessionSqlitePool;
/// Extracteur de session injectable dans les handlers.
pub type AdminSession = axum_session::Session<SessionPool>;

/// Racine de stockage des HTML de versions (volume). `LATCH_STORAGE_ROOT`, défaut `data`.
pub fn storage_from_ctx(_ctx: &AppContext) -> Arc<dyn Storage> {
    let root = std::env::var("LATCH_STORAGE_ROOT").unwrap_or_else(|_| "data".to_string());
    Arc::new(FsStorage::new(root.into()))
}

/// Construit le `SessionStore` : pool SQLite dérivé de la connexion Loco, table
/// `sessions` (déjà migrée), cookie signé + flags adaptés à l'environnement.
pub async fn build_session_store(
    ctx: &AppContext,
) -> Result<axum_session::SessionStore<SessionPool>> {
    let pool = ctx
        .db
        .get_sqlite_connection_pool()
        .map_err(|e| loco_rs::Error::Message(format!("no sqlite pool: {e}")))?
        .clone();
    let session_pool = SessionPool::new(pool);

    let is_prod = !matches!(ctx.environment, loco_rs::environment::Environment::Development);

    let secret = std::env::var("SESSION_SECRET").unwrap_or_else(|_| {
        // En dev uniquement : clé déterministe de secours (jamais en prod : SESSION_SECRET requis).
        "dev-only-insecure-session-secret-please-override-0123456789".to_string()
    });
    let key = axum_session::Key::from(secret.as_bytes());

    let config = axum_session::SessionConfig::default()
        .with_table_name("sessions")
        .with_cookie_name("latch_admin")
        .with_http_only(true)
        .with_secure(is_prod)
        .with_cookie_same_site(axum_session::SameSite::Lax)
        .with_prefix_with_host(is_prod) // __Host- exige Secure → prod only
        .with_key(key);

    let store = axum_session::SessionStore::<SessionPool>::new(Some(session_pool), config)
        .await
        .map_err(|e| loco_rs::Error::Message(format!("session store init: {e}")))?;
    Ok(store)
}
```

> Les noms de méthodes builder (`with_cookie_name`, `with_http_only`, `with_secure`, `with_prefix_with_host`, `with_key`, `with_cookie_same_site`) sont à **confirmer via Context7** sur la version épinglée — l'API `SessionConfig` a des variantes selon la version. Ajuster les noms sans changer l'intention (HttpOnly + Secure prod + SameSite=Lax + clé de signature).

- [ ] **Step 3: Déclarer le module** (`backend/src/lib.rs`)

Ajouter, en ordre alpha : `pub mod web;`

- [ ] **Step 4: Implémenter `after_routes`** (`backend/src/app.rs`)

Ajouter dans `impl Hooks for App` (importer `use axum::Router as AxumRouter;` en tête) :

```rust
    async fn after_routes(router: axum::Router, ctx: &AppContext) -> Result<axum::Router> {
        let store = crate::web::build_session_store(ctx).await?;
        let router = router.layer(axum_session::SessionLayer::new(store));
        Ok(router)
    }
```

- [ ] **Step 5: Écrire le smoke test** (`backend/tests/admin_api.rs`)

```rust
use latch::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn boots_with_session_layer_and_serves_health() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/_health").await;
        assert_eq!(res.status_code(), 200);
    })
    .await;
}
```

> Vérifier le chemin du health endpoint réel (`/_health` ou `/_ping`) dans la config/route Loco par défaut ; ajuster.

- [ ] **Step 6: Lancer le smoke test**

Run: `cargo test -p latch --test admin_api boots_with_session_layer_and_serves_health`
Expected: PASS (l'app boote, le layer session est monté sans paniquer).

- [ ] **Step 7: fmt + clippy + commit**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo deny check licenses advisories
git add backend/Cargo.toml backend/Cargo.lock backend/src/web backend/src/lib.rs backend/src/app.rs backend/tests/admin_api.rs deny.toml
git commit -m "✨ feat: câblage axum-session (after_routes) + helpers web (storage/session)"
```

---

## Task 3: Mapping `CoreError` → HTTP + DTO (scoping du PIN)

**Files:**
- Create: `backend/src/controllers/error.rs`
- Create: `backend/src/controllers/dto.rs`
- Modify: `backend/src/controllers/mod.rs`
- Test: tests `#[cfg(test)]` inline dans `error.rs` et `dto.rs`

**Interfaces:**
- Consumes: `crate::services::errors::CoreError`, `crate::models::_entities::{projects, versions}`.
- Produces (consommés par auth.rs + admin.rs) :
  - `pub fn into_response(err: CoreError) -> loco_rs::Error` — `NotFound`→404, `Validation`→400, `Db`/`Io`→500.
  - `pub struct ProjectListItem` (champs publics, **sans `pin`**) + `From<&projects::Model>`.
  - `pub struct ProjectDetail` (**avec `pin: Option<String>`**) + `From<projects::Model>` (et la liste de ses versions injectée par l'appelant).
  - `pub struct VersionItem` + `From<&versions::Model>`.
  - `pub struct CreateProjectReq`, `UpdateProjectReq`, `SetCodeReq`, `DeployReq` (Deserialize).

- [ ] **Step 1: Écrire les tests qui échouent** (`backend/src/controllers/dto.rs`, bas de fichier)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::_entities::projects;

    fn sample_model() -> projects::Model {
        projects::Model {
            id: 1,
            slug: "mon-projet-k7Qp2maZ".to_string(),
            name: "Mon Projet".to_string(),
            code_enabled: true,
            pin: Some("424242".to_string()),
            brand_name: None,
            active_version_id: None,
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
        }
    }

    #[test]
    fn list_item_never_serializes_pin() {
        let item = ProjectListItem::from(&sample_model());
        let json = serde_json::to_string(&item).unwrap();
        assert!(!json.contains("424242"), "le PIN ne doit JAMAIS apparaître en liste");
        assert!(!json.contains("\"pin\""), "le champ pin ne doit pas exister en liste");
    }

    #[test]
    fn detail_does_serialize_pin() {
        let detail = ProjectDetail::from_model(sample_model(), vec![]);
        let json = serde_json::to_string(&detail).unwrap();
        assert!(json.contains("424242"), "le détail doit exposer le PIN (copiable en admin)");
    }
}
```

Et dans `backend/src/controllers/error.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::errors::CoreError;

    #[test]
    fn not_found_maps_to_404() {
        let e = into_response(CoreError::NotFound);
        assert!(matches!(e, loco_rs::Error::NotFound));
    }

    #[test]
    fn validation_maps_to_400() {
        let e = into_response(CoreError::Validation("bad".into()));
        assert!(matches!(e, loco_rs::Error::BadRequest(_)));
    }
}
```

- [ ] **Step 2: Lancer, vérifier l'échec**

Run: `cargo test -p latch --lib controllers::`
Expected: FAIL — modules/types absents.

- [ ] **Step 3: Écrire `error.rs`**

```rust
//! Traduction du `CoreError` (cœur, agnostique HTTP) vers l'erreur Loco/axum.
//! C'est ICI que vit la frontière HTTP — jamais dans `services/` (contrat §1).

use crate::services::errors::CoreError;

/// Mappe une erreur métier vers le type d'erreur Loco (→ status HTTP).
pub fn into_response(err: CoreError) -> loco_rs::Error {
    match err {
        CoreError::NotFound => loco_rs::Error::NotFound,
        CoreError::Validation(msg) => loco_rs::Error::BadRequest(msg),
        CoreError::Db(e) => loco_rs::Error::Message(format!("db error: {e}")),
        CoreError::Io(e) => loco_rs::Error::Message(format!("io error: {e}")),
    }
}
```

> Vérifier via Context7 les variantes réelles de `loco_rs::Error` (0.16) : `NotFound`, `BadRequest(String)`, `Message(String)`/`InternalServerError`. Ajuster les bras sans changer le mapping de status (404/400/500).

- [ ] **Step 4: Écrire `dto.rs`**

```rust
//! DTO de l'API admin. Le découpage liste/détail fait respecter l'invariant §9.2 :
//! le PIN n'est sérialisé QUE par `ProjectDetail`. `ProjectListItem` ne le porte
//! même pas comme champ — impossible de le fuiter par erreur dans une liste.

use serde::{Deserialize, Serialize};

use crate::models::_entities::{projects, versions};

#[derive(Debug, Serialize)]
pub struct ProjectListItem {
    pub id: i32,
    pub slug: String,
    pub name: String,
    pub code_enabled: bool,
    pub brand_name: Option<String>,
    pub active_version_id: Option<i32>,
    // PAS de `pin` ici. Volontaire (invariant §9.2).
}

impl From<&projects::Model> for ProjectListItem {
    fn from(m: &projects::Model) -> Self {
        Self {
            id: m.id,
            slug: m.slug.clone(),
            name: m.name.clone(),
            code_enabled: m.code_enabled,
            brand_name: m.brand_name.clone(),
            active_version_id: m.active_version_id,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct VersionItem {
    pub id: i32,
    pub n: i32,
    pub created_at: String,
    pub is_active: bool,
}

#[derive(Debug, Serialize)]
pub struct ProjectDetail {
    pub id: i32,
    pub slug: String,
    pub name: String,
    pub code_enabled: bool,
    /// Exposé UNIQUEMENT sur le détail (invariant §9.2). Copiable en admin.
    pub pin: Option<String>,
    pub brand_name: Option<String>,
    pub active_version_id: Option<i32>,
    pub versions: Vec<VersionItem>,
}

impl ProjectDetail {
    pub fn from_model(m: projects::Model, versions: Vec<versions::Model>) -> Self {
        let active = m.active_version_id;
        let versions = versions
            .iter()
            .map(|v| VersionItem {
                id: v.id,
                n: v.n,
                created_at: v.created_at.to_rfc3339(),
                is_active: Some(v.id) == active,
            })
            .collect();
        Self {
            id: m.id,
            slug: m.slug,
            name: m.name,
            code_enabled: m.code_enabled,
            pin: m.pin,
            brand_name: m.brand_name,
            active_version_id: m.active_version_id,
            versions,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectReq {
    pub name: String,
    pub brand_name: Option<String>,
    #[serde(default = "default_true")]
    pub code_enabled: bool,
    pub pin: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectReq {
    pub name: Option<String>,
    pub brand_name: Option<Option<String>>,
}

#[derive(Debug, Deserialize)]
pub struct SetCodeReq {
    pub pin: String,
}

#[derive(Debug, Deserialize)]
pub struct DeployReq {
    pub html: String,
    #[serde(default)]
    pub activate: bool,
}
```

- [ ] **Step 5: Déclarer les modules** (`backend/src/controllers/mod.rs`)

```rust
pub mod admin;
pub mod auth;
pub mod dto;
pub mod error;
pub mod home;
pub mod middleware;
```

> `admin`, `auth`, `middleware` seront créés aux tâches suivantes ; pour que cette tâche compile seule, ajouter d'abord seulement `dto`, `error`, `home`, puis compléter aux tâches 4/5. (Alternative : créer des fichiers stub vides maintenant et les remplir ensuite — au choix de l'exécutant, mais le module doit compiler à chaque commit.)

- [ ] **Step 6: Lancer les tests**

Run: `cargo test -p latch --lib controllers::`
Expected: PASS (4 tests : 2 dto + 2 error).

- [ ] **Step 7: fmt + clippy + commit**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings
git add backend/src/controllers
git commit -m "✨ feat: mapping CoreError→HTTP + DTO admin (PIN scopé au détail)"
```

---

## Task 4: Auth — login / logout + extracteur `AdminAuth` + rate-limit `tower_governor`

**Files:**
- Create: `backend/src/controllers/auth.rs`
- Modify: `backend/src/controllers/mod.rs` (déjà déclaré en Task 3)
- Modify: `backend/src/app.rs` (monter les routes auth + GovernorLayer sur login)
- Test: `backend/tests/admin_api.rs`

**Interfaces:**
- Consumes: `web::AdminSession`, `services::security::secure_compare`, env `ADMIN_USER`/`ADMIN_PASS`.
- Produces:
  - `pub fn routes() -> Routes` (login/logout) montées sous `/admin`.
  - `pub struct AdminAuth;` — extracteur axum (`FromRequestParts`) : 401 si la session ne porte pas `admin == true`. Consommé par tous les handlers de `admin.rs`.
  - Constante de clé de session : `pub const ADMIN_FLAG: &str = "admin";`

- [ ] **Step 1: Écrire les tests d'intégration qui échouent** (`backend/tests/admin_api.rs`)

```rust
#[tokio::test]
#[serial]
async fn login_rejects_bad_credentials() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    request::<App, _, _>(|request, _ctx| async move {
        let res = request
            .post("/admin/login")
            .json(&serde_json::json!({"user": "admin", "pass": "wrong"}))
            .await;
        assert_eq!(res.status_code(), 401);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn protected_route_is_401_without_session() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/admin/projects").await;
        assert_eq!(res.status_code(), 401);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn login_then_access_protected_route() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    request::<App, _, _>(|request, _ctx| async move {
        let login = request
            .post("/admin/login")
            .json(&serde_json::json!({"user": "admin", "pass": "s3cret"}))
            .await;
        assert_eq!(login.status_code(), 200);
        // axum-test propage le cookie de session entre requêtes du même `request`.
        let listed = request.get("/admin/projects").await;
        assert_eq!(listed.status_code(), 200);
    })
    .await;
}
```

> `GET /admin/projects` n'existe qu'à la Task 6. Pour exécuter cette tâche seule, l'exécutant ajoute une route admin minimale protégée (ou ordonne Task 4 après Task 6). Recommandation subagent-driven : implémenter Task 4 puis Task 6, et n'activer `login_then_access_protected_route` qu'une fois `/admin/projects` présent (marquer le test `#[ignore]` entre-temps avec un commentaire).

- [ ] **Step 2: Lancer, vérifier l'échec**

Run: `cargo test -p latch --test admin_api login_rejects_bad_credentials`
Expected: FAIL (route `/admin/login` absente → 404).

- [ ] **Step 3: Écrire `auth.rs`**

```rust
//! Adaptateur entrant "auth admin". L'auth est décidée ICI, avant tout service
//! (contrat §1, §9.3). Compte unique env, comparaison à temps constant.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use loco_rs::prelude::*;
use serde::Deserialize;

use crate::services::security::secure_compare;
use crate::web::AdminSession;

pub const ADMIN_FLAG: &str = "admin";

#[derive(Debug, Deserialize)]
pub struct LoginReq {
    pub user: String,
    pub pass: String,
}

/// Extracteur : présent ⇒ session authentifiée. Sinon `401`.
pub struct AdminAuth;

impl<S> FromRequestParts<S> for AdminAuth
where
    S: Send + Sync,
{
    type Rejection = loco_rs::Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> std::result::Result<Self, Self::Rejection> {
        let session = AdminSession::from_request_parts(parts, state)
            .await
            .map_err(|_| loco_rs::Error::Unauthorized("no session".to_string()))?;
        if session.get::<bool>(ADMIN_FLAG).unwrap_or(false) {
            Ok(AdminAuth)
        } else {
            Err(loco_rs::Error::Unauthorized("not authenticated".to_string()))
        }
    }
}

#[debug_handler]
async fn login(session: AdminSession, Json(body): Json<LoginReq>) -> Result<Response> {
    let expected_user = std::env::var("ADMIN_USER").unwrap_or_default();
    let expected_pass = std::env::var("ADMIN_PASS").unwrap_or_default();

    // Toujours comparer les deux (temps constant) pour ne pas révéler quel champ a échoué.
    let user_ok = secure_compare(&expected_user, &body.user);
    let pass_ok = secure_compare(&expected_pass, &body.pass);
    if !(user_ok && pass_ok) || expected_user.is_empty() || expected_pass.is_empty() {
        return Err(loco_rs::Error::Unauthorized("bad credentials".to_string()));
    }

    session.set(super::auth::ADMIN_FLAG, true);
    format::json(serde_json::json!({"ok": true}))
}

#[debug_handler]
async fn logout(session: AdminSession) -> Result<Response> {
    session.clear();
    format::json(serde_json::json!({"ok": true}))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/admin")
        .add("/login", post(login))
        .add("/logout", post(logout))
}
```

> Confirmer via Context7 : signature `FromRequestParts` (axum 0.8 — plus de `#[async_trait]`), API `Session::get`/`set`/`clear` (axum_session), et les variantes `loco_rs::Error::Unauthorized`. `secure_compare(a, b)` existe déjà dans `services/security.rs` (vérifier l'ordre des args / le type `&str`).

- [ ] **Step 4: Monter les routes + rate-limit** (`backend/src/app.rs`, dans `fn routes`)

```rust
    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes()
            .add_route(controllers::home::routes())
            .add_route(controllers::auth::routes())
            .add_route(controllers::admin::routes()) // Task 6+
    }
```

Le rate-limit `tower_governor` doit cibler **uniquement** `/admin/login`. Comme `Routes` Loco enveloppe axum, l'appliquer dans `after_routes` via un `Router` imbriqué, ou via `.layer()` sur la route login. Approche retenue : layer par route dans `auth::routes()` (cf. Context7 « Add Middleware to Handler »). Ajouter dans `auth.rs` :

```rust
use std::sync::Arc;
use tower_governor::{governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor, GovernorLayer};

fn login_rate_limit() -> GovernorLayer<SmartIpKeyExtractor, ...> {
    let conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(2)
            .burst_size(5)
            .key_extractor(SmartIpKeyExtractor) // lit X-Forwarded-For (derrière Caddy)
            .finish()
            .expect("governor config"),
    );
    GovernorLayer::new(conf)
}
```

et dans `routes()` : `.add("/login", post(login).layer(login_rate_limit()))`.

> Le type de retour exact de `GovernorLayer<...>` est verbeux : si l'annotation pose problème, construire le layer inline dans `routes()`. Confirmer via Context7 la signature `tower_governor` 0.7 (`GovernorConfigBuilder`, `SmartIpKeyExtractor`, `GovernorLayer::new`). `.expect` ici est en init au boot (toléré, comme `main`).

- [ ] **Step 5: Lancer les tests d'auth**

Run: `cargo test -p latch --test admin_api login_rejects_bad_credentials protected_route_is_401_without_session`
Expected: PASS.

- [ ] **Step 6: Écrire le test de rate-limit** (`backend/tests/admin_api.rs`)

```rust
#[tokio::test]
#[serial]
async fn login_is_rate_limited() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    request::<App, _, _>(|request, _ctx| async move {
        let mut saw_429 = false;
        for _ in 0..15 {
            let res = request
                .post("/admin/login")
                .json(&serde_json::json!({"user": "admin", "pass": "wrong"}))
                .await;
            if res.status_code() == 429 {
                saw_429 = true;
                break;
            }
        }
        assert!(saw_429, "le login doit finir par renvoyer 429 (rate-limit load-bearing)");
    })
    .await;
}
```

> Le harness de test peut ne pas fournir d'IP de peer → `SmartIpKeyExtractor` retombe sur une clé par défaut, ce qui suffit à déclencher le compteur. Si le test est instable (pas d'IP), injecter un header `X-Forwarded-For: 1.2.3.4` sur chaque requête. Si la rejection key échoue (`UnableToExtractKey`→400), ajuster l'`error_handler` du layer ou le header.

Run: `cargo test -p latch --test admin_api login_is_rate_limited`
Expected: PASS.

- [ ] **Step 7: fmt + clippy + commit**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings
git add backend/src/controllers/auth.rs backend/src/controllers/mod.rs backend/src/app.rs backend/tests/admin_api.rs
git commit -m "✨ feat: auth admin (login/logout, extracteur AdminAuth, rate-limit login)"
```

---

## Task 5: Garde same-origin sur les mutations

**Files:**
- Create: `backend/src/controllers/middleware/mod.rs`
- Create: `backend/src/controllers/middleware/origin.rs`
- Test: `backend/tests/admin_api.rs`

**Interfaces:**
- Consumes: rien de spécifique.
- Produces: `pub async fn require_same_origin(req: Request, next: Next) -> std::result::Result<Response, loco_rs::Error>` — middleware axum (`from_fn`) appliqué aux routes mutantes (`POST`/`PUT`/`DELETE`) d'admin. Compare l'hôte de l'en-tête `Origin` (ou `Referer` en repli) à l'hôte de la requête (`Host`). Absence des deux sur une mutation ⇒ 403.

> Context7 : `axum::middleware::{from_fn, Next}` (axum 0.8), extraction des headers `Origin`/`Referer`/`Host`.

- [ ] **Step 1: Écrire le test qui échoue** (`backend/tests/admin_api.rs`)

```rust
#[tokio::test]
#[serial]
async fn mutation_rejected_on_cross_origin() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    request::<App, _, _>(|request, _ctx| async move {
        // login d'abord (sinon 401 masquerait le 403)
        request
            .post("/admin/login")
            .json(&serde_json::json!({"user": "admin", "pass": "s3cret"}))
            .await;
        let res = request
            .post("/admin/projects")
            .add_header("origin", "https://evil.example")
            .json(&serde_json::json!({"name": "X"}))
            .await;
        assert_eq!(res.status_code(), 403, "Origin étranger sur mutation ⇒ 403");
    })
    .await;
}
```

> `POST /admin/projects` arrive en Task 7 ; activer ce test une fois la route présente (ordre subagent-driven : 5 → 7, ou `#[ignore]` temporaire).

- [ ] **Step 2: Lancer, vérifier l'échec.** Run: `cargo test -p latch --test admin_api mutation_rejected_on_cross_origin` → FAIL.

- [ ] **Step 3: Écrire le middleware** (`backend/src/controllers/middleware/origin.rs`)

```rust
//! Garde CSRF complémentaire au SameSite (contrat §4, §9.6). Toute mutation admin
//! doit présenter un `Origin` (ou `Referer`) same-origin. Sinon 403.

use axum::extract::Request;
use axum::http::header::{HOST, ORIGIN, REFERER};
use axum::middleware::Next;
use axum::response::Response;

pub async fn require_same_origin(
    req: Request,
    next: Next,
) -> std::result::Result<Response, loco_rs::Error> {
    let headers = req.headers();
    let host = headers.get(HOST).and_then(|v| v.to_str().ok()).map(str::to_string);

    let origin_host = headers
        .get(ORIGIN)
        .or_else(|| headers.get(REFERER))
        .and_then(|v| v.to_str().ok())
        .and_then(|raw| url_host(raw));

    match (host, origin_host) {
        (Some(h), Some(o)) if same_host(&h, &o) => Ok(next.run(req).await),
        _ => Err(loco_rs::Error::Unauthorized("cross-origin mutation refused".to_string())),
    }
}

/// Extrait l'hôte (`host[:port]`) d'une URL `scheme://host[:port]/...`.
fn url_host(raw: &str) -> Option<String> {
    let after_scheme = raw.split("://").nth(1)?;
    let host = after_scheme.split('/').next()?;
    Some(host.to_string())
}

/// Compare deux `host[:port]` en ignorant un port absent côté Host.
fn same_host(host_header: &str, origin_host: &str) -> bool {
    host_header == origin_host
        || host_header.split(':').next() == origin_host.split(':').next()
}
```

> Le code renvoie `Unauthorized` ⇒ vérifier que ça mappe bien **403** côté Loco ; si `Unauthorized`→401, utiliser la variante `loco_rs::Error::Forbidden`/`CustomError(StatusCode::FORBIDDEN, ...)` (confirmer via Context7). L'intention contractuelle est **403** sur cross-origin.

- [ ] **Step 4: Déclarer le module** (`backend/src/controllers/middleware/mod.rs`)

```rust
pub mod origin;
```

- [ ] **Step 5: Appliquer le middleware** — il sera câblé sur les routes mutantes en Task 7 via `.layer(axum::middleware::from_fn(middleware::origin::require_same_origin))`. (Aucune route mutante n'existe encore ; le test reste `#[ignore]` jusqu'à Task 7.)

- [ ] **Step 6: fmt + clippy + commit**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings
git add backend/src/controllers/middleware
git commit -m "✨ feat: middleware garde same-origin (CSRF) sur mutations admin"
```

---

## Task 6: API projets — lecture (liste + détail) + invariant PIN

**Files:**
- Create: `backend/src/controllers/admin.rs`
- Modify: `backend/src/app.rs` (route déjà ajoutée en Task 4)
- Test: `backend/tests/admin_api.rs`, `backend/tests/security_invariants.rs`

**Interfaces:**
- Consumes: `AdminAuth`, `ProjectsService`, `dto::{ProjectListItem, ProjectDetail, VersionItem}`, `error::into_response`.
- Produces:
  - `pub fn routes() -> Routes` sous `/admin` : `GET /projects`, `GET /projects/:id`.
  - Handlers `list`, `detail` (consommés tels quels par les tâches suivantes qui ajoutent au même `routes()`).

- [ ] **Step 1: Écrire les tests qui échouent**

`backend/tests/security_invariants.rs` :

```rust
use latch::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;

async fn login(request: &axum_test::TestServer) {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    request
        .post("/admin/login")
        .json(&serde_json::json!({"user": "admin", "pass": "s3cret"}))
        .await;
}

#[tokio::test]
#[serial]
async fn pin_never_appears_in_project_list() {
    request::<App, _, _>(|request, _ctx| async move {
        login(&request).await;
        // crée un projet protégé via l'API (Task 7) ou directement le service via ctx
        request
            .post("/admin/projects")
            .add_header("origin", "http://localhost")
            .json(&serde_json::json!({"name": "Mon Projet", "code_enabled": true, "pin": "424242"}))
            .await;
        let list = request.get("/admin/projects").await;
        let body = list.text();
        assert!(!body.contains("424242"), "PIN fuité dans la liste (viole §9.2)");
        assert!(!body.contains("\"pin\""), "champ pin présent en liste (viole §9.2)");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn pin_appears_on_project_detail() {
    request::<App, _, _>(|request, _ctx| async move {
        login(&request).await;
        let created = request
            .post("/admin/projects")
            .add_header("origin", "http://localhost")
            .json(&serde_json::json!({"name": "Mon Projet", "code_enabled": true, "pin": "424242"}))
            .await;
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();
        let detail = request.get(&format!("/admin/projects/{id}")).await;
        assert!(detail.text().contains("424242"), "le détail doit exposer le PIN");
    })
    .await;
}
```

`backend/tests/admin_api.rs` (lecture nominale) :

```rust
#[tokio::test]
#[serial]
async fn list_projects_returns_empty_array_when_none() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    request::<App, _, _>(|request, _ctx| async move {
        request.post("/admin/login").json(&serde_json::json!({"user":"admin","pass":"s3cret"})).await;
        let res = request.get("/admin/projects").await;
        assert_eq!(res.status_code(), 200);
        assert_eq!(res.json::<serde_json::Value>(), serde_json::json!([]));
    })
    .await;
}
```

- [ ] **Step 2: Lancer, vérifier l'échec.** Run: `cargo test -p latch --test admin_api list_projects_returns_empty_array_when_none` → FAIL (route absente).

- [ ] **Step 3: Écrire `admin.rs` (lecture seule pour cette tâche)**

```rust
//! Adaptateur entrant "web admin". Chaque handler : auth via `AdminAuth`,
//! appelle un service du cœur, mappe `CoreError` → HTTP (error::into_response),
//! sérialise un DTO. Aucune logique métier ici.

use loco_rs::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::controllers::auth::AdminAuth;
use crate::controllers::dto::{ProjectDetail, ProjectListItem};
use crate::controllers::error::into_response;
use crate::models::_entities::versions;
use crate::services::projects::ProjectsService;

#[debug_handler]
async fn list(_auth: AdminAuth, State(ctx): State<AppContext>) -> Result<Response> {
    let svc = ProjectsService::new(ctx.db.clone());
    let projects = svc.list().await.map_err(into_response)?;
    let items: Vec<ProjectListItem> = projects.iter().map(ProjectListItem::from).collect();
    format::json(items)
}

#[debug_handler]
async fn detail(
    _auth: AdminAuth,
    State(ctx): State<AppContext>,
    Path(id): Path<i32>,
) -> Result<Response> {
    use crate::models::_entities::projects;
    let project = projects::Entity::find_by_id(id)
        .one(&ctx.db)
        .await
        .map_err(|e| into_response(e.into()))?
        .ok_or(loco_rs::Error::NotFound)?;
    let versions = versions::Entity::find()
        .filter(versions::Column::ProjectId.eq(id))
        .order_by_desc(versions::Column::N)
        .all(&ctx.db)
        .await
        .map_err(|e| into_response(e.into()))?;
    format::json(ProjectDetail::from_model(project, versions))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/admin")
        .add("/projects", get(list))
        .add("/projects/{id}", get(detail))
}
```

> Loco 0.16/axum 0.8 : la syntaxe de path param est `{id}` (pas `:id`). Confirmer via Context7. `State(ctx): State<AppContext>` est l'injection standard Loco du contexte.

- [ ] **Step 4: Lancer les tests de lecture**

Run: `cargo test -p latch --test admin_api list_projects_returns_empty_array_when_none`
Expected: PASS.

(Les tests `security_invariants` dépendent de `POST /admin/projects` → Task 7. Les laisser `#[ignore]` jusque-là, ou enchaîner 6→7 avant de les exécuter.)

- [ ] **Step 5: fmt + clippy + commit**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings
git add backend/src/controllers/admin.rs backend/tests
git commit -m "✨ feat: API admin lecture projets (liste/détail) + tests invariant PIN"
```

---

## Task 7: API projets — écriture (create / update / delete / code) + garde Origin

**Files:**
- Modify: `backend/src/controllers/admin.rs`
- Test: `backend/tests/admin_api.rs`, `backend/tests/security_invariants.rs` (activer les tests laissés `#[ignore]`)

**Interfaces:**
- Consumes: `ProjectsService::{create, set_code, clear_code}`, `dto::{CreateProjectReq, UpdateProjectReq, SetCodeReq, ProjectDetail}`, `middleware::origin::require_same_origin`.
- Produces (ajoutées au `routes()` existant) : `POST /projects`, `PUT /projects/{id}`, `DELETE /projects/{id}`, `POST /projects/{id}/code`, `DELETE /projects/{id}/code`. Toutes derrière `AdminAuth` + garde Origin.

> **Suppression de projet (QUIRKS) :** SQLite n'enforce pas les FK sans `PRAGMA foreign_keys=ON` → le `ON DELETE CASCADE` est best-effort. Le handler `delete` **doit supprimer explicitement les versions** du projet (et idéalement nettoyer le storage) avant/avec la suppression du projet, dans une transaction. Ne pas se reposer sur la cascade DB.

- [ ] **Step 1: Écrire les tests d'écriture qui échouent** (`backend/tests/admin_api.rs`)

```rust
#[tokio::test]
#[serial]
async fn create_then_get_and_delete_project() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    request::<App, _, _>(|request, _ctx| async move {
        request.post("/admin/login").json(&serde_json::json!({"user":"admin","pass":"s3cret"})).await;

        let created = request
            .post("/admin/projects")
            .add_header("origin", "http://localhost")
            .json(&serde_json::json!({"name": "Mon Projet", "code_enabled": false}))
            .await;
        assert_eq!(created.status_code(), 200);
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();

        let got = request.get(&format!("/admin/projects/{id}")).await;
        assert_eq!(got.status_code(), 200);

        let deleted = request
            .delete(&format!("/admin/projects/{id}"))
            .add_header("origin", "http://localhost")
            .await;
        assert_eq!(deleted.status_code(), 200);

        let gone = request.get(&format!("/admin/projects/{id}")).await;
        assert_eq!(gone.status_code(), 404);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn set_and_clear_code_via_api() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    request::<App, _, _>(|request, _ctx| async move {
        request.post("/admin/login").json(&serde_json::json!({"user":"admin","pass":"s3cret"})).await;
        let created = request.post("/admin/projects").add_header("origin","http://localhost")
            .json(&serde_json::json!({"name":"Mon Projet","code_enabled":false})).await;
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();

        let set = request.post(&format!("/admin/projects/{id}/code")).add_header("origin","http://localhost")
            .json(&serde_json::json!({"pin":"135790"})).await;
        assert_eq!(set.status_code(), 200);
        assert!(set.text().contains("135790"));

        let clear = request.delete(&format!("/admin/projects/{id}/code")).add_header("origin","http://localhost").await;
        assert_eq!(clear.status_code(), 200);
    })
    .await;
}
```

- [ ] **Step 2: Lancer, vérifier l'échec.** Run: `cargo test -p latch --test admin_api create_then_get_and_delete_project` → FAIL.

- [ ] **Step 3: Ajouter les handlers d'écriture** (`backend/src/controllers/admin.rs`)

```rust
use crate::controllers::dto::{CreateProjectReq, SetCodeReq, UpdateProjectReq};
use crate::services::projects::CreateProject;
use sea_orm::{ActiveModelTrait, Set, TransactionTrait};

#[debug_handler]
async fn create(
    _auth: AdminAuth,
    State(ctx): State<AppContext>,
    Json(body): Json<CreateProjectReq>,
) -> Result<Response> {
    let svc = ProjectsService::new(ctx.db.clone());
    let project = svc
        .create(CreateProject {
            name: body.name,
            brand_name: body.brand_name,
            code_enabled: body.code_enabled,
            pin: body.pin,
        })
        .await
        .map_err(into_response)?;
    format::json(ProjectDetail::from_model(project, vec![]))
}

#[debug_handler]
async fn update(
    _auth: AdminAuth,
    State(ctx): State<AppContext>,
    Path(id): Path<i32>,
    Json(body): Json<UpdateProjectReq>,
) -> Result<Response> {
    use crate::models::_entities::projects;
    let model = projects::Entity::find_by_id(id)
        .one(&ctx.db)
        .await
        .map_err(|e| into_response(e.into()))?
        .ok_or(loco_rs::Error::NotFound)?;
    let mut active: projects::ActiveModel = model.into();
    if let Some(name) = body.name {
        if name.trim().is_empty() {
            return Err(loco_rs::Error::BadRequest("name is required".to_string()));
        }
        active.name = Set(name);
    }
    if let Some(brand) = body.brand_name {
        active.brand_name = Set(brand);
    }
    active.updated_at = Set(chrono::Utc::now().into());
    let saved = active.update(&ctx.db).await.map_err(|e| into_response(e.into()))?;
    format::json(ProjectDetail::from_model(saved, vec![]))
}

#[debug_handler]
async fn delete(
    _auth: AdminAuth,
    State(ctx): State<AppContext>,
    Path(id): Path<i32>,
) -> Result<Response> {
    use crate::models::_entities::projects;
    // FK SQLite non enforced (QUIRKS) → supprimer versions + projet explicitement, en tx.
    let txn = ctx.db.begin().await.map_err(|e| into_response(e.into()))?;
    versions::Entity::delete_many()
        .filter(versions::Column::ProjectId.eq(id))
        .exec(&txn)
        .await
        .map_err(|e| into_response(e.into()))?;
    let res = projects::Entity::delete_by_id(id)
        .exec(&txn)
        .await
        .map_err(|e| into_response(e.into()))?;
    txn.commit().await.map_err(|e| into_response(e.into()))?;
    if res.rows_affected == 0 {
        return Err(loco_rs::Error::NotFound);
    }
    format::json(serde_json::json!({"ok": true}))
}

#[debug_handler]
async fn set_code(
    _auth: AdminAuth,
    State(ctx): State<AppContext>,
    Path(id): Path<i32>,
    Json(body): Json<SetCodeReq>,
) -> Result<Response> {
    let svc = ProjectsService::new(ctx.db.clone());
    let project = svc.set_code(id, &body.pin).await.map_err(into_response)?;
    format::json(ProjectDetail::from_model(project, vec![]))
}

#[debug_handler]
async fn clear_code(
    _auth: AdminAuth,
    State(ctx): State<AppContext>,
    Path(id): Path<i32>,
) -> Result<Response> {
    let svc = ProjectsService::new(ctx.db.clone());
    let project = svc.clear_code(id).await.map_err(into_response)?;
    format::json(ProjectDetail::from_model(project, vec![]))
}
```

- [ ] **Step 4: Câbler les routes mutantes + garde Origin** (`routes()` dans `admin.rs`)

```rust
pub fn routes() -> Routes {
    use crate::controllers::middleware::origin::require_same_origin;
    use axum::middleware::from_fn;
    Routes::new()
        .prefix("/admin")
        .add("/projects", get(list))
        .add("/projects", post(create).layer(from_fn(require_same_origin)))
        .add("/projects/{id}", get(detail))
        .add("/projects/{id}", put(update).layer(from_fn(require_same_origin)))
        .add("/projects/{id}", axum::routing::delete(delete).layer(from_fn(require_same_origin)))
        .add("/projects/{id}/code", post(set_code).layer(from_fn(require_same_origin)))
        .add("/projects/{id}/code", axum::routing::delete(clear_code).layer(from_fn(require_same_origin)))
}
```

> Vérifier via Context7 que `Routes::add` Loco accepte plusieurs méthodes sur le même chemin et le `.layer()` par handler (pattern « Add Middleware to Handler »). Si Loco impose un seul verbe par `add`, fusionner via `axum::routing::{get, post}` combinés (`get(list).post(create)`).

- [ ] **Step 5: Lancer tous les tests d'écriture + invariants + Origin**

```bash
cargo test -p latch --test admin_api
cargo test -p latch --test security_invariants
```
Expected: PASS (création/suppression, set/clear code, `mutation_rejected_on_cross_origin`, `pin_never_appears_in_project_list`, `pin_appears_on_project_detail`).

- [ ] **Step 6: fmt + clippy + commit**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings
git add backend/src/controllers/admin.rs backend/tests
git commit -m "✨ feat: API admin écriture projets (CRUD + code) + garde Origin + cascade versions"
```

---

## Task 8: Déploiement manuel + versions (activer / supprimer / preview)

**Files:**
- Modify: `backend/src/controllers/admin.rs`
- Test: `backend/tests/admin_api.rs`

**Interfaces:**
- Consumes: `DeployService::deploy`, `web::storage_from_ctx`, `dto::DeployReq`.
- Produces (ajoutées au `routes()`) :
  - `POST /projects/{id}/deploy` (JSON `{html, activate}`) → nouvelle version.
  - `POST /projects/{id}/versions/{n}/activate` → flip pointeur (transactionnel).
  - `DELETE /projects/{id}/versions/{n}` → supprime une version non active.
  - `GET /projects/{id}/versions/{n}/preview` → sert le HTML, `Cache-Control: no-store`, derrière `AdminAuth`.

- [ ] **Step 1: Écrire les tests qui échouent** (`backend/tests/admin_api.rs`)

```rust
#[tokio::test]
#[serial]
async fn deploy_creates_version_and_preview_serves_html() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("LATCH_STORAGE_ROOT", tmp.path());
    request::<App, _, _>(|request, _ctx| async move {
        request.post("/admin/login").json(&serde_json::json!({"user":"admin","pass":"s3cret"})).await;
        let created = request.post("/admin/projects").add_header("origin","http://localhost")
            .json(&serde_json::json!({"name":"Mon Projet","code_enabled":false})).await;
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();

        let deployed = request.post(&format!("/admin/projects/{id}/deploy")).add_header("origin","http://localhost")
            .json(&serde_json::json!({"html":"<h1>v1</h1>","activate":true})).await;
        assert_eq!(deployed.status_code(), 200);

        let preview = request.get(&format!("/admin/projects/{id}/versions/1/preview")).await;
        assert_eq!(preview.status_code(), 200);
        assert!(preview.text().contains("<h1>v1</h1>"));
        assert_eq!(preview.header("cache-control"), "no-store");
    })
    .await;
}
```

> `tempfile` est déjà en dev-dependency. Le `LATCH_STORAGE_ROOT` pointé sur un tempdir évite d'écrire sur le disque de prod (règle de test : jamais le vrai volume).

- [ ] **Step 2: Lancer, vérifier l'échec.** Run: `cargo test -p latch --test admin_api deploy_creates_version_and_preview_serves_html` → FAIL.

- [ ] **Step 3: Ajouter les handlers** (`backend/src/controllers/admin.rs`)

```rust
use crate::controllers::dto::DeployReq;
use crate::services::deploy::DeployService;

#[debug_handler]
async fn deploy(
    _auth: AdminAuth,
    State(ctx): State<AppContext>,
    Path(id): Path<i32>,
    Json(body): Json<DeployReq>,
) -> Result<Response> {
    let storage = crate::web::storage_from_ctx(&ctx);
    let svc = DeployService::new(ctx.db.clone(), storage);
    let version = svc.deploy(id, &body.html, body.activate).await.map_err(into_response)?;
    format::json(serde_json::json!({"id": version.id, "n": version.n}))
}

#[debug_handler]
async fn activate_version(
    _auth: AdminAuth,
    State(ctx): State<AppContext>,
    Path((id, n)): Path<(i32, i32)>,
) -> Result<Response> {
    use crate::models::_entities::projects;
    let version = versions::Entity::find()
        .filter(versions::Column::ProjectId.eq(id))
        .filter(versions::Column::N.eq(n))
        .one(&ctx.db)
        .await
        .map_err(|e| into_response(e.into()))?
        .ok_or(loco_rs::Error::NotFound)?;
    let project = projects::Entity::find_by_id(id)
        .one(&ctx.db)
        .await
        .map_err(|e| into_response(e.into()))?
        .ok_or(loco_rs::Error::NotFound)?;
    let mut active: projects::ActiveModel = project.into();
    active.active_version_id = Set(Some(version.id));
    active.updated_at = Set(chrono::Utc::now().into());
    active.update(&ctx.db).await.map_err(|e| into_response(e.into()))?;
    format::json(serde_json::json!({"ok": true, "active_version_id": version.id}))
}

#[debug_handler]
async fn delete_version(
    _auth: AdminAuth,
    State(ctx): State<AppContext>,
    Path((id, n)): Path<(i32, i32)>,
) -> Result<Response> {
    use crate::models::_entities::projects;
    let version = versions::Entity::find()
        .filter(versions::Column::ProjectId.eq(id))
        .filter(versions::Column::N.eq(n))
        .one(&ctx.db)
        .await
        .map_err(|e| into_response(e.into()))?
        .ok_or(loco_rs::Error::NotFound)?;
    // Refuser de supprimer la version active (laisserait un pointeur orphelin).
    let project = projects::Entity::find_by_id(id)
        .one(&ctx.db)
        .await
        .map_err(|e| into_response(e.into()))?
        .ok_or(loco_rs::Error::NotFound)?;
    if project.active_version_id == Some(version.id) {
        return Err(loco_rs::Error::BadRequest("cannot delete the active version".to_string()));
    }
    versions::Entity::delete_by_id(version.id)
        .exec(&ctx.db)
        .await
        .map_err(|e| into_response(e.into()))?;
    format::json(serde_json::json!({"ok": true}))
}

#[debug_handler]
async fn preview_version(
    _auth: AdminAuth,
    State(ctx): State<AppContext>,
    Path((id, n)): Path<(i32, i32)>,
) -> Result<Response> {
    let version = versions::Entity::find()
        .filter(versions::Column::ProjectId.eq(id))
        .filter(versions::Column::N.eq(n))
        .one(&ctx.db)
        .await
        .map_err(|e| into_response(e.into()))?
        .ok_or(loco_rs::Error::NotFound)?;
    let storage = crate::web::storage_from_ctx(&ctx);
    let html = storage.read(&version.html_path).await.map_err(into_response)?;
    Ok((
        [
            (axum::http::header::CACHE_CONTROL, "no-store"),
            (axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8"),
        ],
        html,
    )
        .into_response())
}
```

> `preview_version` renvoie une réponse axum brute (`(headers, body).into_response()`) au lieu d'un JSON. Confirmer via Context7 le type `Response` attendu par Loco (`loco_rs::prelude::Response` = `axum::response::Response`) et l'import de `IntoResponse`.

- [ ] **Step 4: Câbler les routes** (`routes()` dans `admin.rs`, ajouter aux existantes)

```rust
        .add("/projects/{id}/deploy", post(deploy).layer(from_fn(require_same_origin)))
        .add("/projects/{id}/versions/{n}/activate", post(activate_version).layer(from_fn(require_same_origin)))
        .add("/projects/{id}/versions/{n}", axum::routing::delete(delete_version).layer(from_fn(require_same_origin)))
        .add("/projects/{id}/versions/{n}/preview", get(preview_version))
```

- [ ] **Step 5: Écrire le test de bascule** (`backend/tests/admin_api.rs`)

```rust
#[tokio::test]
#[serial]
async fn activate_switches_active_version() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("LATCH_STORAGE_ROOT", tmp.path());
    request::<App, _, _>(|request, _ctx| async move {
        request.post("/admin/login").json(&serde_json::json!({"user":"admin","pass":"s3cret"})).await;
        let created = request.post("/admin/projects").add_header("origin","http://localhost")
            .json(&serde_json::json!({"name":"Mon Projet","code_enabled":false})).await;
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();
        request.post(&format!("/admin/projects/{id}/deploy")).add_header("origin","http://localhost")
            .json(&serde_json::json!({"html":"a","activate":true})).await;
        request.post(&format!("/admin/projects/{id}/deploy")).add_header("origin","http://localhost")
            .json(&serde_json::json!({"html":"b","activate":false})).await;
        let act = request.post(&format!("/admin/projects/{id}/versions/2/activate")).add_header("origin","http://localhost").await;
        assert_eq!(act.status_code(), 200);
        let detail = request.get(&format!("/admin/projects/{id}")).await;
        let v = detail.json::<serde_json::Value>();
        // la version n=2 doit être active
        let active_id = v["active_version_id"].as_i64().unwrap();
        let v2 = v["versions"].as_array().unwrap().iter().find(|x| x["n"]==2).unwrap();
        assert_eq!(v2["id"].as_i64().unwrap(), active_id);
        assert_eq!(v2["is_active"], true);
    })
    .await;
}
```

- [ ] **Step 6: Lancer toute la suite d'intégration**

```bash
cargo test -p latch --test admin_api
```
Expected: PASS (deploy+preview, activate switch, + tests précédents).

- [ ] **Step 7: fmt + clippy + commit**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings
git add backend/src/controllers/admin.rs backend/tests/admin_api.rs
git commit -m "✨ feat: déploiement manuel + versions (activate/delete/preview no-store)"
```

---

## Task 9: Finalisation — suite complète verte, env, clôture mémoire

**Files:**
- Modify: `.env.example`
- Modify: `docs/ENVIRONMENT.md`, `docs/INDEX.md`, `docs/HANDOFF.md`, `docs/QUIRKS.md`, `docs/CONVENTIONS.md`, `docs/contrat-deploy.md` (reporter les décisions tranchées)
- Test: toute la suite + garde d'archi

- [ ] **Step 1: Vérifier que le cœur est resté propre**

Run: `cargo test -p latch --test architecture`
Expected: PASS — aucun `use axum`/`use loco_rs` n'a fuité dans `services/`. (Si rouge : un import HTTP a glissé dans le cœur → le déplacer dans `controllers/` ou `web/`.)

- [ ] **Step 2: Suite complète + qualité**

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo nextest run    # ou cargo test
cargo deny check licenses advisories
```
Expected: tout vert. Noter le compte de tests (doit dépasser les 33 de la Phase 1).

- [ ] **Step 3: Mettre à jour `.env.example`** — ajouter :

```
# Clé HMAC de signature du cookie de session admin (64+ octets aléatoires). REQUIS en prod.
SESSION_SECRET=
# Racine de stockage des HTML de versions. Dev: ./data — Prod (image): /data
LATCH_STORAGE_ROOT=./data
```

- [ ] **Step 4: Reporter les décisions dans `docs/contrat-deploy.md`** (§4 / §5 / §9) : session `axum-session` + table SeaORM dédiée ; rate-limit login `tower_governor` (`SmartIpKeyExtractor`) ; garde Origin via middleware ; cookie `Secure`/`__Host-` prod-only.

- [ ] **Step 5: Clôture mémoire** (règle non-négociable du `CLAUDE.md`) :
  - `docs/INDEX.md` : cocher Phase 2 + lignes par livrable (auth, CRUD projets, deploy/versions, invariants testés).
  - `docs/HANDOFF.md` : entrée datée en tête (Dernière chose faite / En suspens / Prochaine chose à creuser = Phase 3 SPA Yew / Notes future Claude).
  - `docs/QUIRKS.md` : (a) axum-session crée toujours sa table → migration SeaORM doit matcher le schéma ; (b) cookie `Secure`/`__Host-` cassé en HTTP dev → piloté par `ctx.environment` ; (c) FK SQLite non enforced → delete projet cascade explicite ; (d) tout réglage sqlx/axum_session_sqlx épinglé.
  - `docs/CONVENTIONS.md` : remplir « Endpoint admin (adaptateur web) type » (handler : `AdminAuth` → service → `into_response` → DTO ; garde Origin via `from_fn`) et « Test d'intégration type » (harness Loco `request::<App,_,_>`, login, `#[serial]`).
  - `docs/ENVIRONMENT.md` : `SESSION_SECRET`, `LATCH_STORAGE_ROOT`.

- [ ] **Step 6: Commit de clôture**

```bash
git add .env.example docs
git commit -m "📝 docs: clôture Phase 2 (adaptateur web admin) + report décisions au contrat"
```

- [ ] **Step 7: Critères de sortie ROADMAP Phase 2 — vérifier que tout est vrai**
  - Tests intégration verts sur chaque endpoint, 401 sans session ✅
  - Deploy transactionnel + switch de version ✅
  - Test-invariant de sécu (pas de hash en réponse, pas de PIN en liste) ✅
  - Rate-limit login effectif (429) ✅
  - Garde Origin sur mutations (403 cross-origin) ✅
  - Cœur toujours sans axum/loco (garde d'archi verte) ✅

---

## Self-Review (couverture du contrat §4/§7/§9 + ROADMAP Phase 2)

- **§4 cookie session same-origin, HttpOnly/Secure/SameSite=Lax** → Task 2 (config) + Task 4 (login/logout). ✅
- **§4 store table SQLite** → Task 1 (migration) + Task 2 (store). ✅
- **§4 compte unique env, compare temps constant** → Task 4 (`secure_compare`). ✅
- **§4 CSRF Origin/Referer sur mutations** → Task 5 + câblage Task 7/8. ✅
- **§4 login rate-limité** → Task 4 (`tower_governor`). ✅
- **§7 liste (sans PIN) / détail (avec PIN) / CRUD / code / versions / preview / deploy** → Tasks 6/7/8. ✅ *(Rendu SPA Yew = Phase 3, hors périmètre ici.)*
- **§9.1 pas de hash** → garde structurelle (DTO) + test invariants Task 6/9. ✅
- **§9.2 PIN seulement au détail** → DTO Task 3 + tests Task 6. ✅
- **§9.3 auth dans l'adaptateur** → `AdminAuth` Task 4, garde d'archi Task 9. ✅
- **§9.5 rate-limit login** → Task 4. ✅
- **§9.6 cookie signé + Origin** → Task 2 (clé) + Task 5. ✅
- **ROADMAP : migration `sessions` créée ici** → Task 1. ✅
- **`deploy()` réutilise le même service que MCP** → Task 8 appelle `DeployService::deploy`. ✅

**Risques connus à valider en cours d'exécution (spikes) :**
1. Compatibilité sqlx 0.8 entre Loco et `axum_session_sqlx` (Task 2). Si conflit → épingler la version partageant sqlx 0.8, consigner QUIRKS.
2. Noms exacts des builders `SessionConfig` et variantes `loco_rs::Error` (0.16) — confirmer Context7, ajuster sans changer l'intention.
3. `Routes::add` Loco : plusieurs verbes sur un même path + `.layer()` par handler — sinon combiner `get(..).post(..)`.
4. Extraction d'IP dans le harness de test pour le 429 — injecter `X-Forwarded-For` si instable.
5. `FromRequestParts` axum 0.8 (plus de `#[async_trait]`) — adapter la signature.

---

## Execution Handoff

Deux modes d'exécution possibles (au choix de l'humain) :
1. **Subagent-Driven (recommandé, comme Phase 1)** — un subagent frais par tâche + revue à deux étages entre tâches.
2. **Inline** — exécution par lots dans cette session avec checkpoints.

> Ordre de dépendance recommandé : 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9. Les tests d'intégration de 4/5 référencent des routes créées en 6/7 : soit enchaîner avant de les dégeler, soit les marquer `#[ignore]` temporairement (noté dans chaque tâche).
