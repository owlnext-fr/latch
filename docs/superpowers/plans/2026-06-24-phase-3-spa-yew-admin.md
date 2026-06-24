# Phase 3 — SPA Yew admin — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Livrer la SPA admin (`latch-ui`, Yew 0.21 CSR) au-dessus de l'API JSON Phase 2 (re-préfixée `/api/*`), servie en statique par le binaire Loco sous `/admin`, avec login, liste, détail et mutations en side-panels.

**Architecture:** Un binaire Loco sert les assets de la SPA (`nest_service("/admin", ServeDir+fallback)`) **et** l'API JSON `/api/*`. Même origin → le cookie de session `HttpOnly` et l'en-tête `Origin` partent automatiquement. La SPA ne stocke aucun token : l'état de connexion est **dérivé** des codes HTTP (sonde `GET /api/projects` ; tout `401` → écran Login). Les types du contrat de fil vivent dans une crate partagée `latch-dto` (serde-only), dépendue par le back **et** le front.

**Tech Stack:** Yew 0.21 (csr) · yew-router **0.18** (compat yew 0.21) · gloo-net **0.6** (`http`,`json`) · gloo-timers 0.3 · gloo-file 0.3 · shadcn-rs 0.1 · wasm-bindgen-futures 0.4 · web-sys · Trunk · côté back : Loco 0.16 (axum 0.8) + tower-http **0.6** (`fs`).

## Global Constraints

- **Le cœur (`backend/src/services/`) ne voit jamais axum/loco.** Aucun `use axum::`/`use loco_rs::` sous `services/`. Vérifié par `backend/tests/architecture.rs`. Cette phase ne touche pas `services/`.
- **Invariants de sécurité (contrat §9), testés, cassent le build si violés :** (1) aucune réponse ne contient de hash ; (2) le **PIN en clair** n'apparaît **que** sur le détail, jamais en liste — garanti structurellement (`ProjectListItem` n'a pas de champ `pin`, désormais dans `latch-dto`) ; (3) l'auth vit dans l'adaptateur ; (4) rate-limit login ; (5) cookie admin `HttpOnly`/`Secure`(prod)/`SameSite=Lax`, vérif `Origin` sur mutations.
- **Confidentialité (NON-NÉGOCIABLE) :** aucun nom de client réel nulle part. Placeholders `Mon Projet`/`mon-projet`, `ACME`, `demo`.
- **Pas d'`unwrap`/`expect`** hors tests et hors `main()` d'init (frontend `main()` toléré pour le bootstrap Yew). Erreurs propagées (`Result`).
- **Commits** conventionnels + gitmoji : `<gitmoji> <type>: <desc>` (`✨ feat:`, `🐛 fix:`, `📝 docs:`, `🧱 chore:`, `✅ test:`, `♻️ refactor:`). Inclure le trailer `Co-Authored-By:` + `Claude-Session:` (cf. autres commits).
- **CWD :** serveur lancé depuis `backend/` (`cd backend && cargo loco start`). `fmt`/`clippy`/`test` backend depuis la racine. Frontend buildé/testé depuis `frontend/`.
- **Versions à épingler (vérifiées, ne pas deviner) :** `yew-router = "0.18"`, `gloo-net = { version = "0.6", default-features = false, features = ["http","json"] }`, `gloo-timers = "0.3"`, `gloo-file = { version = "0.3", features = ["futures"] }`, `wasm-bindgen-futures = "0.4"`, backend `tower-http = { version = "0.6", features = ["fs"] }`.
- **Definition of done par tâche :** `cargo fmt --all` + `cargo clippy --all-targets -- -D warnings` verts (backend), `cd frontend && cargo clippy --target wasm32-unknown-unknown -- -D warnings` vert (frontend), tests de la tâche verts, commit. En fin de phase : mémoire à jour (INDEX, HANDOFF, QUIRKS, CONVENTIONS, ENVIRONMENT) + contrat §4/§7 amendé.
- **Context7 d'abord** avant toute API non triviale (versions du lockfile). Les API clés sont déjà documentées dans ce plan.

### Décisions tranchées (cf. spec, à reporter dans le contrat §4/§7 — Task 14)

- API re-préfixée `/api/*` ; SPA à `/admin/*` (BrowserRouter `basename="/admin"`).
- DTO partagés via crate `latch-dto` ; conversions sea-orm = **fonctions libres** côté back (orphan rule).
- Toutes les mutations en side-panels (`SheetContent` piloté manuellement) ; confirmations destructives en side-panels *danger* (pas de modale).
- Page détail en lecture seule ; slug en lecture seule ; URL publique via `window.location.origin`.
- Pas de système de toast global (shadcn `Toast`/`Sonner` sont déclaratifs, `duration` non implémenté) → feedback **inline** + « Copié ! » éphémère via `gloo-timers`.

---

## File Structure

**Crate `latch-dto/` (nouvelle, membre workspace) :**
- Create `latch-dto/Cargo.toml`, `latch-dto/src/lib.rs` — structs serde du contrat de fil + tests d'invariant de sérialisation.

**Backend (modifiés) :**
- `Cargo.toml` (racine) — `members`/`default-members` += `latch-dto`.
- `backend/Cargo.toml` — deps `latch-dto`, `tower-http` (`fs`).
- `backend/src/controllers/dto.rs` — supprime les défs de struct (→ `latch-dto`), garde les conversions en fonctions libres.
- `backend/src/controllers/admin.rs` — call-sites des conversions ; (préfixe via `auth`/`admin` `routes()`).
- `backend/src/controllers/auth.rs` — `.prefix("/api")`, `LoginReq` depuis `latch-dto`.
- `backend/src/controllers/admin.rs` — `.prefix("/api")`.
- `backend/src/web/mod.rs` — helper `spa_dist_dir()`.
- `backend/src/app.rs` — `after_routes` : `nest_service("/admin", ServeDir+fallback)`.
- `backend/tests/admin_api.rs`, `backend/tests/security_invariants.rs` — chemins `/admin` → `/api`.
- `backend/tests/spa_serving.rs` (nouveau) — serving + fallback SPA.

**Frontend `frontend/` :**
- `Cargo.toml` — deps (router, gloo-*, web-sys, latch-dto, etc.).
- `Trunk.toml` — `public_url = "/admin/"`, proxy dev.
- `index.html` — link CSS vendorisée.
- `styles/` (vendorisée depuis le cache cargo) — 5 fichiers `.css`.
- `src/main.rs` — bootstrap + `<BrowserRouter>` + `<AuthProvider>` + `<Switch>`.
- `src/routes.rs` — `enum Route` + `switch` + `Protected`.
- `src/util/mod.rs`, `src/util/pin.rs`, `src/util/url.rs`, `src/util/clipboard.rs`.
- `src/api/mod.rs`, `src/api/error.rs`, `src/api/client.rs`.
- `src/auth.rs` — `AuthState`, `AuthContext`, `AuthProvider`.
- `src/components/mod.rs`, `copy_button.rs`, `pin_field.rs`.
- `src/pages/mod.rs`, `login.rs`, `list.rs`, `detail.rs`.
- `src/panels/mod.rs`, `project_form.rs`, `deploy.rs`, `delete_project.rs`, `delete_version.rs`.

**Infra :**
- `Dockerfile` — copier `frontend/dist` + `ENV LATCH_SPA_DIST`.
- `.env.example`, `docs/ENVIRONMENT.md` — `LATCH_SPA_DIST`.

---

## Task 1: Crate partagée `latch-dto`

**Files:**
- Create: `latch-dto/Cargo.toml`, `latch-dto/src/lib.rs`
- Modify: `Cargo.toml` (racine — `members`, `default-members`)

**Interfaces:**
- Produces (consommés par back **et** front) : structs `ProjectListItem`, `VersionItem`, `ProjectDetail`, `CreateProjectReq`, `UpdateProjectReq`, `SetCodeReq`, `DeployReq`, `LoginReq` — toutes `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]`. `ProjectListItem` **n'a pas** de champ `pin` (invariant §9.2).

- [ ] **Step 1: Créer le `Cargo.toml` de la crate**

```toml
# latch-dto/Cargo.toml
[package]
name = "latch-dto"
version = "0.1.0"
edition = "2021"
publish = false
license = "MIT OR Apache-2.0"

[dependencies]
serde = { version = "1", features = ["derive"] }

[dev-dependencies]
serde_json = "1"
```

- [ ] **Step 2: Écrire le test d'invariant qui échoue** (`latch-dto/src/lib.rs`, module tests)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn sample_list_item() -> ProjectListItem {
        ProjectListItem {
            id: 1,
            slug: "mon-projet-k7Qp2maZ".into(),
            name: "Mon Projet".into(),
            code_enabled: true,
            brand_name: None,
            active_version_id: None,
        }
    }

    #[test]
    fn list_item_never_serializes_pin() {
        let json = serde_json::to_string(&sample_list_item()).unwrap();
        assert!(!json.contains("\"pin\""), "le champ pin ne doit pas exister en liste");
        assert!(!json.contains("424242"));
    }

    #[test]
    fn detail_roundtrips_with_pin() {
        let detail = ProjectDetail {
            id: 1,
            slug: "mon-projet-k7Qp2maZ".into(),
            name: "Mon Projet".into(),
            code_enabled: true,
            pin: Some("424242".into()),
            brand_name: None,
            active_version_id: Some(3),
            versions: vec![VersionItem { id: 3, n: 3, created_at: "2026-06-24T00:00:00+00:00".into(), is_active: true }],
        };
        let json = serde_json::to_string(&detail).unwrap();
        assert!(json.contains("424242"), "le détail expose le PIN");
        let back: ProjectDetail = serde_json::from_str(&json).unwrap();
        assert_eq!(detail, back, "round-trip stable (contrat de fil)");
    }

    #[test]
    fn create_req_defaults_code_enabled_true() {
        let req: CreateProjectReq = serde_json::from_str(r#"{"name":"X"}"#).unwrap();
        assert!(req.code_enabled, "code_enabled défaut = true (contrat §3)");
        assert_eq!(req.name, "X");
    }
}
```

- [ ] **Step 3: Lancer le test, vérifier qu'il échoue (types absents)**

Run: `cargo test -p latch-dto`
Expected: FAIL compilation — `cannot find type ProjectListItem`.

- [ ] **Step 4: Écrire les structs** (en tête de `latch-dto/src/lib.rs`, avant le module tests)

```rust
//! Contrat de fil partagé entre le backend (`latch`) et la SPA (`latch-ui`).
//! Une seule source de vérité : pas de drift possible. serde uniquement → wasm-safe.
//! Les dates sont des `String` (RFC 3339). Aucune dépendance sea-orm ici.

use serde::{Deserialize, Serialize};

/// Item de liste — **sans PIN** (invariant §9.2 : structurellement absent).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectListItem {
    pub id: i32,
    pub slug: String,
    pub name: String,
    pub code_enabled: bool,
    pub brand_name: Option<String>,
    pub active_version_id: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionItem {
    pub id: i32,
    pub n: i32,
    pub created_at: String,
    pub is_active: bool,
}

/// Détail — expose le PIN (copiable en admin uniquement, invariant §9.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectDetail {
    pub id: i32,
    pub slug: String,
    pub name: String,
    pub code_enabled: bool,
    pub pin: Option<String>,
    pub brand_name: Option<String>,
    pub active_version_id: Option<i32>,
    pub versions: Vec<VersionItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateProjectReq {
    pub name: String,
    #[serde(default)]
    pub brand_name: Option<String>,
    #[serde(default = "default_true")]
    pub code_enabled: bool,
    #[serde(default)]
    pub pin: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateProjectReq {
    #[serde(default)]
    pub name: Option<String>,
    /// `Option<Option<String>>` : champ absent ⇒ pas de changement ; `null` ⇒ effacer.
    #[serde(default)]
    pub brand_name: Option<Option<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetCodeReq {
    pub pin: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeployReq {
    pub html: String,
    #[serde(default)]
    pub activate: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoginReq {
    pub user: String,
    pub pass: String,
}
```

- [ ] **Step 5: Enregistrer la crate dans le workspace** (`Cargo.toml` racine)

Ajouter `"latch-dto"` à `members` ET à `default-members` (crate native-safe ; ses tests tournent avec `cargo test`). Exemple si le bloc est :

```toml
[workspace]
members = ["backend", "backend/migration", "frontend", "latch-dto"]
default-members = ["backend", "backend/migration", "latch-dto"]
```

- [ ] **Step 6: Lancer les tests, vérifier vert**

Run: `cargo test -p latch-dto`
Expected: PASS (3 tests).

- [ ] **Step 7: fmt + clippy + commit**

```bash
cargo fmt --all
cargo clippy -p latch-dto --all-targets -- -D warnings
git add latch-dto Cargo.toml
git commit -m "✨ feat: crate partagée latch-dto (contrat de fil back/front)"
```

---

## Task 2: Backend — re-préfixer l'API sous `/api` + conversions en fonctions libres

**Files:**
- Modify: `backend/Cargo.toml` (dep `latch-dto`)
- Modify: `backend/src/controllers/dto.rs` (suppr. structs, conversions libres)
- Modify: `backend/src/controllers/admin.rs` (call-sites + `.prefix("/api")`)
- Modify: `backend/src/controllers/auth.rs` (`.prefix("/api")` + `LoginReq` de `latch-dto`)
- Modify: `backend/tests/admin_api.rs`, `backend/tests/security_invariants.rs` (chemins `/admin`→`/api`)

**Interfaces:**
- Consumes: types de `latch-dto` (Task 1).
- Produces: API JSON montée sous `/api/*` (`/api/login`, `/api/logout`, `/api/projects`, …). Fonctions `dto::to_list_item(&projects::Model) -> ProjectListItem` et `dto::to_detail(projects::Model, Vec<versions::Model>) -> ProjectDetail`.

> **⚠️ Orphan rule :** `impl From<&projects::Model> for ProjectListItem` est interdit (les deux types sont étrangers au backend). On remplace par des **fonctions libres** dans `dto.rs`.

- [ ] **Step 1: Ajouter la dep `latch-dto` au backend** (`backend/Cargo.toml`, section `[dependencies]`)

```toml
latch-dto = { path = "../latch-dto" }
```

- [ ] **Step 2: Réécrire `backend/src/controllers/dto.rs`** (remplacer tout le fichier)

```rust
//! Adaptateur DTO : ré-exporte le contrat de fil partagé (`latch-dto`) et fournit
//! les conversions depuis les modèles sea-orm. Les conversions sont des FONCTIONS
//! LIBRES (orphan rule : on ne peut pas `impl From<&Model>` pour un type étranger).
//! L'invariant §9.2 reste structurel : `ProjectListItem` (latch-dto) n'a pas de `pin`.

pub use latch_dto::{
    CreateProjectReq, DeployReq, ProjectDetail, ProjectListItem, SetCodeReq, UpdateProjectReq,
    VersionItem,
};

use crate::models::_entities::{projects, versions};

/// Projet → item de liste (sans PIN).
pub fn to_list_item(m: &projects::Model) -> ProjectListItem {
    ProjectListItem {
        id: m.id,
        slug: m.slug.clone(),
        name: m.name.clone(),
        code_enabled: m.code_enabled,
        brand_name: m.brand_name.clone(),
        active_version_id: m.active_version_id,
    }
}

/// Projet + ses versions → détail (avec PIN).
pub fn to_detail(m: projects::Model, vers: Vec<versions::Model>) -> ProjectDetail {
    let active = m.active_version_id;
    let versions = vers
        .into_iter()
        .map(|v| VersionItem {
            id: v.id,
            n: v.n,
            created_at: v.created_at.to_rfc3339(),
            is_active: Some(v.id) == active,
        })
        .collect();
    ProjectDetail {
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
        let json = serde_json::to_string(&to_list_item(&sample_model())).unwrap();
        assert!(!json.contains("424242"));
        assert!(!json.contains("\"pin\""));
    }

    #[test]
    fn detail_does_serialize_pin() {
        let json = serde_json::to_string(&to_detail(sample_model(), vec![])).unwrap();
        assert!(json.contains("424242"));
    }
}
```

- [ ] **Step 3: Mettre à jour les call-sites dans `backend/src/controllers/admin.rs`**

Dans l'import en tête, retirer les types qui n'existent plus comme call et garder l'usage via `dto::`. Remplacer :
- `use crate::controllers::dto::{CreateProjectReq, DeployReq, ProjectDetail, ProjectListItem, SetCodeReq, UpdateProjectReq};` reste valide (ré-exportés).
- Dans `list` : `let items: Vec<ProjectListItem> = projects.iter().map(ProjectListItem::from).collect();` → `let items: Vec<ProjectListItem> = projects.iter().map(crate::controllers::dto::to_list_item).collect();`
- Dans `detail` : `format::json(ProjectDetail::from_model(project, vers))` → `format::json(crate::controllers::dto::to_detail(project, vers))`.

(Chercher tout autre usage de `::from`/`::from_model` sur ces types et remplacer par `dto::to_list_item`/`dto::to_detail`.)

- [ ] **Step 4: Re-préfixer les routes** — dans `backend/src/controllers/admin.rs`, fonction `routes()` : `.prefix("/admin")` → `.prefix("/api")`. Dans `backend/src/controllers/auth.rs`, fonction `routes()` : `.prefix("/admin")` → `.prefix("/api")` (les chemins deviennent `/api/login`, `/api/logout`).

- [ ] **Step 5: Faire pointer `LoginReq` (auth.rs) sur `latch-dto`** — dans `backend/src/controllers/auth.rs`, supprimer la déclaration locale `pub struct LoginReq { ... }` et son `#[derive(Deserialize)]`, et ajouter en tête : `use latch_dto::LoginReq;`. Le handler `login(session, Json(body): Json<LoginReq>)` lit `body.user` / `body.pass` (champs identiques — vérifier que c'est `user`/`pass`, pas `username`/`password`).

- [ ] **Step 6: Mettre à jour les chemins dans les tests** — dans `backend/tests/admin_api.rs` et `backend/tests/security_invariants.rs`, remplacer toutes les occurrences de chemin :
  - `"/admin/login"` → `"/api/login"`, `"/admin/logout"` → `"/api/logout"`
  - `"/admin/projects"` → `"/api/projects"` (et toutes les variantes `/admin/projects/...` → `/api/projects/...`).
  - Laisser les en-têtes `Origin: http://127.0.0.1` inchangés.

- [ ] **Step 7: Lancer les tests backend (runner CI), vérifier vert**

Run: `cargo nextest run -p latch`
Expected: PASS — tous les tests Phase 2 verts sous `/api` (77 attendus, ajustés).

- [ ] **Step 8: fmt + clippy + commit**

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
git add backend Cargo.lock
git commit -m "♻️ refactor: API admin re-préfixée /api/* + DTO via latch-dto"
```

---

## Task 3: Backend — servir la SPA en statique sous `/admin` (ServeDir + fallback)

**Files:**
- Modify: `backend/Cargo.toml` (dep `tower-http` feature `fs`)
- Modify: `backend/src/web/mod.rs` (helper `spa_dist_dir`)
- Modify: `backend/src/app.rs` (`after_routes`)
- Create: `backend/tests/spa_serving.rs`

**Interfaces:**
- Consumes: `after_routes(router, ctx)` existant (monte déjà le SessionLayer).
- Produces: `GET /admin` et `GET /admin/<route-spa>` rendent `index.html` ; `GET /admin/<asset>` rend le fichier ; `/api/*` non masqué.

- [ ] **Step 1: Ajouter `tower-http` (feature `fs`)** (`backend/Cargo.toml`)

```toml
tower-http = { version = "0.6", features = ["fs"] }
```

- [ ] **Step 2: Ajouter le helper `spa_dist_dir` dans `backend/src/web/mod.rs`**

```rust
use std::path::PathBuf;

/// Racine des assets buildés de la SPA (`frontend/dist`). Surclassable par
/// `LATCH_SPA_DIST` (posée dans l'image Docker). Défaut relatif au CWD `backend/`.
pub fn spa_dist_dir() -> PathBuf {
    std::env::var("LATCH_SPA_DIST")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("../frontend/dist"))
}
```

- [ ] **Step 3: Écrire le test de serving qui échoue** (`backend/tests/spa_serving.rs`)

```rust
use latch::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;

/// Prépare un faux dist/ avec un index.html reconnaissable + un asset, pointé par
/// LATCH_SPA_DIST, et garde le tempdir vivant pour toute la durée du test.
fn fake_dist() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("index.html"), "<!doctype html><title>latch-spa</title>")
        .expect("write index");
    std::fs::write(dir.path().join("app.js"), "// spa asset").expect("write asset");
    std::env::set_var("LATCH_SPA_DIST", dir.path());
    dir
}

#[tokio::test]
#[serial]
async fn admin_root_serves_spa_index() {
    let _dist = fake_dist();
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/admin").await;
        res.assert_status_ok();
        assert!(res.text().contains("latch-spa"), "GET /admin rend index.html");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn admin_deep_link_falls_back_to_index() {
    let _dist = fake_dist();
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/admin/projects/5").await;
        res.assert_status_ok();
        assert!(res.text().contains("latch-spa"), "deep-link → index.html");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn api_is_not_shadowed_by_spa() {
    let _dist = fake_dist();
    request::<App, _, _>(|request, _ctx| async move {
        // /api/projects sans session → 401 (pas l'index SPA en 200).
        let res = request.get("/api/projects").await;
        res.assert_status(401);
    })
    .await;
}
```

> Note : `tempfile` et `serial_test` sont déjà des dev-deps du backend (utilisés en Phase 2). Si `tempfile` manque en dev-dep, l'ajouter à `backend/Cargo.toml [dev-dependencies]`.

- [ ] **Step 4: Lancer, vérifier l'échec**

Run: `cargo nextest run -p latch --test spa_serving`
Expected: FAIL — `/admin` renvoie 404 (serving pas encore câblé).

- [ ] **Step 5: Câbler le serving dans `backend/src/app.rs` `after_routes`**

Ajouter les imports en tête du fichier :

```rust
use tower_http::services::{ServeDir, ServeFile};
```

Remplacer le corps de `after_routes` par :

```rust
async fn after_routes(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
    let store = crate::web::build_session_store(ctx).await?;
    let router = router.layer(axum_session::SessionLayer::new(store));

    // SPA servie sous /admin : assets si le fichier existe, sinon index.html
    // (deep-links client). nest_service strip le préfixe /admin ; les routes
    // /api/* et /_health restent prioritaires (non masquées).
    let dist = crate::web::spa_dist_dir();
    let index = dist.join("index.html");
    let spa = ServeDir::new(&dist).fallback(ServeFile::new(index));
    let router = router.nest_service("/admin", spa);

    Ok(router)
}
```

- [ ] **Step 6: Lancer, vérifier vert**

Run: `cargo nextest run -p latch --test spa_serving`
Expected: PASS (3 tests).

- [ ] **Step 7: Suite complète + fmt + clippy + commit**

```bash
cargo nextest run -p latch
cargo fmt --all
cargo clippy --all-targets -- -D warnings
git add backend
git commit -m "✨ feat: serving statique SPA sous /admin (ServeDir + fallback index)"
```

---

## Task 4: Frontend — scaffold (deps, CSS vendorisée, routeur, pages placeholder)

**Files:**
- Modify: `frontend/Cargo.toml`, `frontend/Trunk.toml`, `frontend/index.html`
- Create: `frontend/styles/*.css` (vendorisés), `frontend/src/main.rs` (réécrit), `frontend/src/routes.rs`, modules vides `src/{api,pages,panels,components,util}/mod.rs`, `src/auth.rs`

**Interfaces:**
- Produces: app Yew qui boote sous `/admin` avec `<BrowserRouter basename="/admin">`, `enum Route` + `switch`, pages placeholder. CSS shadcn-rs chargée.

- [ ] **Step 1: Remplacer `frontend/Cargo.toml`**

```toml
[package]
name = "latch-ui"
version = "0.1.0"
edition = "2021"
publish = false
license = "MIT OR Apache-2.0"

[dependencies]
yew = { version = "0.21", features = ["csr"] }
yew-router = "0.18"
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
gloo-net = { version = "0.6", default-features = false, features = ["http", "json"] }
gloo-timers = "0.3"
gloo-file = { version = "0.3", features = ["futures"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
latch-dto = { path = "../latch-dto" }
shadcn-rs = "0.1"

[dependencies.web-sys]
version = "0.3"
features = ["Window", "Location", "Document", "Navigator", "Clipboard", "HtmlInputElement", "File", "FileList"]

[dev-dependencies]
wasm-bindgen-test = "0.3"
```

- [ ] **Step 2: Vendoriser la CSS shadcn-rs depuis le cache cargo**

```bash
mkdir -p frontend/styles
cp /home/pr0xyblu3/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/shadcn-rs-0.1.0/styles/shadcn-rs.css frontend/styles/
cp /home/pr0xyblu3/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/shadcn-rs-0.1.0/styles/base.css frontend/styles/
cp /home/pr0xyblu3/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/shadcn-rs-0.1.0/styles/variables.css frontend/styles/
cp /home/pr0xyblu3/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/shadcn-rs-0.1.0/styles/components.css frontend/styles/
cp /home/pr0xyblu3/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/shadcn-rs-0.1.0/styles/utilities.css frontend/styles/
```

(Si le chemin du cache diffère, le retrouver via `python3 -c "import glob;print(glob.glob('/home/*/.cargo/registry/src/*/shadcn-rs-0.1.0/styles'))"`.)

- [ ] **Step 3: Réécrire `frontend/index.html`**

```html
<!DOCTYPE html>
<html lang="fr">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <meta name="robots" content="noindex, nofollow" />
    <title>latch — admin</title>
    <link data-trunk rel="copy-dir" href="styles" />
    <link rel="stylesheet" href="styles/shadcn-rs.css" />
  </head>
  <body></body>
</html>
```

- [ ] **Step 4: Réécrire `frontend/Trunk.toml`**

```toml
[build]
target = "index.html"
dist = "dist"
# La SPA est servie sous /admin → les assets doivent être référencés sous ce préfixe.
public_url = "/admin/"

[serve]
# Dev : proxy de l'API backend pendant `trunk serve`.
address = "127.0.0.1"
port = 8080
[[proxy]]
backend = "http://127.0.0.1:5150/api"
```

- [ ] **Step 5: Créer les modules vides** (`frontend/src/api/mod.rs`, `pages/mod.rs`, `panels/mod.rs`, `components/mod.rs`, `util/mod.rs`) — chacun avec un commentaire d'en-tête, contenu rempli aux tâches suivantes. Pour l'instant :

```rust
// frontend/src/util/mod.rs
pub mod pin;
pub mod url;
pub mod clipboard;
```

Pour les autres `mod.rs`, déclarer ce qui existera (les `pub mod` seront ajoutés au fil des tâches ; commencer vide est OK tant que `main.rs` ne les référence pas). Créer `pages/mod.rs` avec `pub mod login; pub mod list; pub mod detail;` seulement quand ces fichiers existent (sinon échec de compilation). **Règle : ne déclarer un `pub mod X;` que lorsque `X.rs` existe.** Pour cette tâche, ne créer que ce que `main.rs`/`routes.rs` référencent (pages placeholder inline).

- [ ] **Step 6: Écrire `frontend/src/routes.rs`** (pages placeholder inline pour cette tâche)

```rust
use yew::prelude::*;
use yew_router::prelude::*;

/// Routes client. `basename="/admin"` est posé sur le `<BrowserRouter>` ; les
/// chemins ci-dessous sont ABSOLUS (incluent /admin) — combo robuste en yew-router 0.18.
#[derive(Clone, Routable, PartialEq)]
pub enum Route {
    #[at("/admin")]
    Home,
    #[at("/admin/login")]
    Login,
    #[at("/admin/projects/:id")]
    Project { id: i32 },
    #[not_found]
    #[at("/admin/404")]
    NotFound,
}

pub fn switch(route: Route) -> Html {
    match route {
        Route::Home => html! { <h1>{ "Liste (placeholder)" }</h1> },
        Route::Login => html! { <h1>{ "Login (placeholder)" }</h1> },
        Route::Project { id } => html! { <h1>{ format!("Détail {} (placeholder)", id) }</h1> },
        Route::NotFound => html! { <h1>{ "404" }</h1> },
    }
}
```

- [ ] **Step 7: Réécrire `frontend/src/main.rs`**

```rust
use yew::prelude::*;
use yew_router::prelude::*;

mod routes;
mod util;

use routes::{switch, Route};

#[function_component(App)]
fn app() -> Html {
    html! {
        <BrowserRouter basename="/admin">
            <Switch<Route> render={switch} />
        </BrowserRouter>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
```

> `main.rs` déclare `mod util;` → il faut que `util/mod.rs` + `util/pin.rs` + `util/url.rs` + `util/clipboard.rs` existent. Les créer vides (stubs `pub fn`) dans cette tâche OU déplacer `mod util;` en Task 5. **Choix : retirer `mod util;` de `main.rs` ici** (rien ne l'utilise encore) et l'ajouter en Task 5.

- [ ] **Step 8: Build de vérification**

Run: `cd frontend && trunk build`
Expected: build OK ; `frontend/dist/index.html` + `dist/styles/shadcn-rs.css` présents.

- [ ] **Step 9: clippy wasm + commit**

```bash
cd frontend && cargo clippy --target wasm32-unknown-unknown -- -D warnings
cd .. && git add frontend
git commit -m "🧱 chore: scaffold SPA (router, deps, CSS shadcn-rs vendorisée)"
```

---

## Task 5: Frontend — utilitaires purs (`pin`, `url`, `clipboard`)

**Files:**
- Create: `frontend/src/util/mod.rs`, `util/pin.rs`, `util/url.rs`, `util/clipboard.rs`
- Modify: `frontend/src/main.rs` (ajouter `mod util;`)

**Interfaces:**
- Produces:
  - `util::pin::generate_pin() -> String` (6 chiffres), `util::pin::is_valid_pin(&str) -> bool`.
  - `util::url::public_url(slug: &str) -> String` (= `origin + "/c/" + slug`).
  - `util::clipboard::copy(text: String)` (écrit dans le presse-papier, best-effort).

- [ ] **Step 1: `util/mod.rs`**

```rust
pub mod clipboard;
pub mod pin;
pub mod url;
```

- [ ] **Step 2: Écrire les tests wasm qui échouent** (`frontend/src/util/pin.rs`)

```rust
//! Génération/validation du PIN côté SPA (affichage live dans le panel). Le cœur
//! backend garde sa propre génération pour le chemin MCP (contrat §3/§7, D10).

/// Vrai si `s` fait exactement 6 caractères, tous des chiffres ASCII.
pub fn is_valid_pin(s: &str) -> bool {
    s.len() == 6 && s.bytes().all(|b| b.is_ascii_digit())
}

/// Génère un PIN de 6 chiffres (entropie via crypto.getRandomValues du navigateur).
pub fn generate_pin() -> String {
    let mut buf = [0u8; 6];
    // web-sys Crypto : getRandomValues remplit le buffer.
    if let Some(win) = web_sys::window() {
        if let Ok(crypto) = win.crypto() {
            let _ = crypto.get_random_values_with_u8_array(&mut buf);
        }
    }
    buf.iter().map(|b| char::from(b'0' + (b % 10))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;
    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn generated_pin_is_six_digits() {
        let p = generate_pin();
        assert_eq!(p.len(), 6);
        assert!(is_valid_pin(&p));
    }

    #[wasm_bindgen_test]
    fn rejects_bad_pins() {
        assert!(!is_valid_pin("12345"));
        assert!(!is_valid_pin("1234567"));
        assert!(!is_valid_pin("12a456"));
        assert!(is_valid_pin("000000"));
    }
}
```

> `web_sys::Crypto` requiert le feature `Crypto`. Ajouter `"Crypto"` à la liste `web-sys` de `frontend/Cargo.toml`.

- [ ] **Step 3: `util/url.rs`**

```rust
//! Construit l'URL publique absolue d'un prototype. Admin et serving partagent
//! l'origin (D9) → pas de config nécessaire.

/// `https://latch.owlnext.fr/c/<slug>` (dérivé de l'origin courant).
pub fn public_url(slug: &str) -> String {
    let origin = web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_default();
    format!("{origin}/c/{slug}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;
    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn public_url_ends_with_slug_path() {
        let u = public_url("mon-projet-k7Qp2maZ");
        assert!(u.ends_with("/c/mon-projet-k7Qp2maZ"), "got {u}");
        assert!(u.contains("://"), "doit être absolu : {u}");
    }
}
```

- [ ] **Step 4: `util/clipboard.rs`**

```rust
//! Copie best-effort dans le presse-papier (Clipboard API). Échec silencieux si
//! l'API n'est pas dispo (le composant appelant affiche quand même « Copié ! »).

use wasm_bindgen_futures::JsFuture;

pub fn copy(text: String) {
    wasm_bindgen_futures::spawn_local(async move {
        if let Some(win) = web_sys::window() {
            let clipboard = win.navigator().clipboard();
            let _ = JsFuture::from(clipboard.write_text(&text)).await;
        }
    });
}
```

> `navigator().clipboard()` retourne `web_sys::Clipboard` (non-`Option` en web-sys récent). Si la signature locale diffère (`Option`), adapter avec `if let Some(c) = ...`. Features web-sys déjà incluses : `Navigator`, `Clipboard`, `Window`.

- [ ] **Step 5: Ajouter `mod util;` dans `frontend/src/main.rs`** (sous `mod routes;`).

- [ ] **Step 6: Lancer les tests wasm**

Run: `cd frontend && wasm-pack test --headless --firefox`
Expected: PASS (3 tests). (Si `wasm-pack`/firefox absents : `--chrome`, ou installer via ENVIRONMENT. C'est l'outillage de test wasm du projet.)

- [ ] **Step 7: clippy + commit**

```bash
cd frontend && cargo clippy --target wasm32-unknown-unknown --all-targets -- -D warnings
cd .. && git add frontend
git commit -m "✨ feat: utilitaires SPA (pin, url publique, presse-papier) + tests wasm"
```

---

## Task 6: Frontend — client API typé (`gloo-net`)

**Files:**
- Create: `frontend/src/api/error.rs`, `frontend/src/api/client.rs`
- Modify: `frontend/src/api/mod.rs`, `frontend/src/main.rs` (`mod api;`)

**Interfaces:**
- Produces (consommés par auth + pages + panels) :
  - `api::error::ApiError { Unauthorized, Status(u16), Network(gloo_net::Error) }`.
  - Fonctions `async` (toutes `-> Result<_, ApiError>`) : `login(&LoginReq)`, `logout()`, `list_projects() -> Vec<ProjectListItem>`, `get_project(i32) -> ProjectDetail`, `create_project(&CreateProjectReq) -> ProjectDetail`, `update_project(i32,&UpdateProjectReq)`, `delete_project(i32)`, `set_code(i32,&SetCodeReq)`, `clear_code(i32)`, `deploy(i32,&DeployReq) -> VersionItem`, `activate_version(i32,i32)`, `delete_version(i32,i32)`.
  - `api::client::preview_url(id,n) -> String` (URL d'onglet, pas un fetch).

> **⚠️ gloo-net :** un 401/404 est `Ok(Response)` (pas une `Err`). On lit `resp.status()`. `.json(&body)?` consomme le builder et renvoie `Result<Request,_>` AVANT `.send()`.

- [ ] **Step 1: `api/error.rs`**

```rust
//! Erreurs du client API. Un 401 est distingué pour piloter l'état d'auth global.

#[derive(Debug, Clone, PartialEq)]
pub enum ApiError {
    /// 401 — session absente/expirée. Bascule l'app en Anonymous.
    Unauthorized,
    /// Autre code HTTP non-2xx.
    Status(u16),
    /// Échec réseau / parse JSON.
    Network(String),
}

impl From<gloo_net::Error> for ApiError {
    fn from(e: gloo_net::Error) -> Self {
        ApiError::Network(e.to_string())
    }
}

impl ApiError {
    /// Message court présentable à l'utilisateur (inline).
    pub fn user_message(&self) -> String {
        match self {
            ApiError::Unauthorized => "Session expirée, reconnecte-toi.".into(),
            ApiError::Status(c) => format!("Erreur serveur ({c})."),
            ApiError::Network(_) => "Erreur réseau, réessaie.".into(),
        }
    }
}
```

- [ ] **Step 2: `api/mod.rs`**

```rust
pub mod client;
pub mod error;

pub use error::ApiError;
```

- [ ] **Step 3: `api/client.rs`** (toutes les fonctions)

```rust
//! Client HTTP typé vers l'API /api/*. Même origin → le cookie de session part
//! automatiquement (credentials same-origin par défaut). Les types viennent de
//! `latch-dto` (contrat de fil partagé).

use gloo_net::http::Request;
use latch_dto::{
    CreateProjectReq, DeployReq, LoginReq, ProjectDetail, ProjectListItem, SetCodeReq,
    UpdateProjectReq, VersionItem,
};

use crate::api::error::ApiError;

/// Convertit un statut HTTP en `Result<()>` (401 distingué).
fn check_status(status: u16) -> Result<(), ApiError> {
    match status {
        200..=299 => Ok(()),
        401 => Err(ApiError::Unauthorized),
        other => Err(ApiError::Status(other)),
    }
}

pub async fn login(body: &LoginReq) -> Result<(), ApiError> {
    let resp = Request::post("/api/login").json(body)?.send().await?;
    check_status(resp.status())
}

pub async fn logout() -> Result<(), ApiError> {
    let resp = Request::post("/api/logout").send().await?;
    check_status(resp.status())
}

pub async fn list_projects() -> Result<Vec<ProjectListItem>, ApiError> {
    let resp = Request::get("/api/projects").send().await?;
    check_status(resp.status())?;
    Ok(resp.json::<Vec<ProjectListItem>>().await?)
}

pub async fn get_project(id: i32) -> Result<ProjectDetail, ApiError> {
    let resp = Request::get(&format!("/api/projects/{id}")).send().await?;
    check_status(resp.status())?;
    Ok(resp.json::<ProjectDetail>().await?)
}

pub async fn create_project(body: &CreateProjectReq) -> Result<ProjectDetail, ApiError> {
    let resp = Request::post("/api/projects").json(body)?.send().await?;
    check_status(resp.status())?;
    Ok(resp.json::<ProjectDetail>().await?)
}

pub async fn update_project(id: i32, body: &UpdateProjectReq) -> Result<(), ApiError> {
    let resp = Request::put(&format!("/api/projects/{id}")).json(body)?.send().await?;
    check_status(resp.status())
}

pub async fn delete_project(id: i32) -> Result<(), ApiError> {
    let resp = Request::delete(&format!("/api/projects/{id}")).send().await?;
    check_status(resp.status())
}

pub async fn set_code(id: i32, body: &SetCodeReq) -> Result<(), ApiError> {
    let resp = Request::post(&format!("/api/projects/{id}/code")).json(body)?.send().await?;
    check_status(resp.status())
}

pub async fn clear_code(id: i32) -> Result<(), ApiError> {
    let resp = Request::delete(&format!("/api/projects/{id}/code")).send().await?;
    check_status(resp.status())
}

pub async fn deploy(id: i32, body: &DeployReq) -> Result<VersionItem, ApiError> {
    let resp = Request::post(&format!("/api/projects/{id}/deploy")).json(body)?.send().await?;
    check_status(resp.status())?;
    Ok(resp.json::<VersionItem>().await?)
}

pub async fn activate_version(id: i32, n: i32) -> Result<(), ApiError> {
    let resp = Request::post(&format!("/api/projects/{id}/versions/{n}/activate"))
        .send()
        .await?;
    check_status(resp.status())
}

pub async fn delete_version(id: i32, n: i32) -> Result<(), ApiError> {
    let resp = Request::delete(&format!("/api/projects/{id}/versions/{n}")).send().await?;
    check_status(resp.status())
}

/// URL de prévisualisation (HTML brut no-store) — à ouvrir dans un nouvel onglet.
pub fn preview_url(id: i32, n: i32) -> String {
    format!("/api/projects/{id}/versions/{n}/preview")
}
```

> **Note `deploy` retour :** le handler backend Phase 2 répond `{id, n}` (pas un `VersionItem` complet). **Vérifier** la forme réelle renvoyée par `controllers/admin.rs::deploy` ; si c'est `{"id":..,"n":..}`, soit (a) aligner le handler pour renvoyer un `VersionItem`, soit (b) côté SPA ne pas désérialiser le corps et juste recharger le détail après deploy. **Choix recommandé : (b)** — `deploy` renvoie `Result<(), ApiError>` et la page recharge `get_project`. Ajuster la signature en conséquence (retirer `-> VersionItem`, lire juste le statut). Reporter le choix ici lors de l'implémentation.

- [ ] **Step 4: Ajouter `mod api;` dans `main.rs`** (sous `mod util;`).

- [ ] **Step 5: Build de vérification** (pas de test unitaire : ces fns font du réseau, couvertes par la vérif manuelle e2e et les tests backend)

Run: `cd frontend && cargo clippy --target wasm32-unknown-unknown -- -D warnings`
Expected: compile + 0 warning.

- [ ] **Step 6: Commit**

```bash
git add frontend
git commit -m "✨ feat: client API typé SPA (gloo-net, latch-dto, gestion 401)"
```

---

## Task 7: Frontend — état d'authentification (`AuthContext` + `AuthProvider` + `Protected`)

**Files:**
- Create: `frontend/src/auth.rs`
- Modify: `frontend/src/main.rs` (`mod auth;`, wrap `<AuthProvider>`), `frontend/src/routes.rs` (`Protected` + redirections)

**Interfaces:**
- Produces:
  - `auth::AuthState { Checking, Anonymous, Authenticated }`.
  - `auth::AuthContext { state: AuthState, set_authenticated: Callback<()>, set_anonymous: Callback<()> }` (via `ContextProvider`).
  - `#[function_component(AuthProvider)]` — lance la sonde de boot, fournit le contexte.
  - `auth::use_auth() -> AuthContext` (hook d'accès).
  - `routes::Protected` — wrappe une page ; redirige vers Login si Anonymous, spinner si Checking.

- [ ] **Step 1: `frontend/src/auth.rs`**

```rust
//! État d'authentification dérivé (D4). Aucun token stocké : on déduit l'état des
//! codes HTTP. Sonde de boot = GET /api/projects. Tout 401 → Anonymous.

use yew::prelude::*;
use yew_router::prelude::*;

use crate::api;
use crate::routes::Route;

#[derive(Clone, PartialEq)]
pub enum AuthState {
    Checking,
    Anonymous,
    Authenticated,
}

#[derive(Clone, PartialEq)]
pub struct AuthContext {
    pub state: AuthState,
    pub set_authenticated: Callback<()>,
    pub set_anonymous: Callback<()>,
}

#[hook]
pub fn use_auth() -> AuthContext {
    use_context::<AuthContext>().expect("AuthProvider manquant au-dessus de l'arbre")
}

#[derive(Properties, PartialEq)]
pub struct AuthProviderProps {
    pub children: Html,
}

#[function_component(AuthProvider)]
pub fn auth_provider(props: &AuthProviderProps) -> Html {
    let state = use_state(|| AuthState::Checking);

    let set_authenticated = {
        let state = state.clone();
        Callback::from(move |_| state.set(AuthState::Authenticated))
    };
    let set_anonymous = {
        let state = state.clone();
        Callback::from(move |_| state.set(AuthState::Anonymous))
    };

    // Sonde de boot : une seule fois au montage.
    {
        let state = state.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                match api::client::list_projects().await {
                    Ok(_) => state.set(AuthState::Authenticated),
                    Err(_) => state.set(AuthState::Anonymous),
                }
            });
            || ()
        });
    }

    let ctx = AuthContext {
        state: (*state).clone(),
        set_authenticated,
        set_anonymous,
    };

    html! { <ContextProvider<AuthContext> context={ctx}>{ props.children.clone() }</ContextProvider<AuthContext>> }
}

/// Wrappe une page protégée : Checking → rien ; Anonymous → redirige Login ;
/// Authenticated → enfants.
#[derive(Properties, PartialEq)]
pub struct ProtectedProps {
    pub children: Html,
}

#[function_component(Protected)]
pub fn protected(props: &ProtectedProps) -> Html {
    let auth = use_auth();
    let navigator = use_navigator().expect("router");

    {
        let state = auth.state.clone();
        use_effect_with(state, move |state| {
            if *state == AuthState::Anonymous {
                navigator.push(&Route::Login);
            }
            || ()
        });
    }

    match auth.state {
        AuthState::Authenticated => props.children.clone(),
        _ => html! { <div class="loading">{ "Chargement…" }</div> },
    }
}
```

- [ ] **Step 2: Déplacer `Protected` consommé par `routes.rs`** — `Protected` est défini dans `auth.rs` ; `routes.rs` l'utilise. Mettre à jour `switch` :

```rust
use crate::auth::Protected;
// ...
pub fn switch(route: Route) -> Html {
    match route {
        Route::Home => html! { <Protected>{ html!{ <h1>{ "Liste (placeholder)" }</h1> } }</Protected> },
        Route::Login => html! { <h1>{ "Login (placeholder)" }</h1> },
        Route::Project { id } => html! { <Protected>{ html!{ <h1>{ format!("Détail {id}") }</h1> } }</Protected> },
        Route::NotFound => html! { <h1>{ "404" }</h1> },
    }
}
```

- [ ] **Step 3: Mettre à jour `main.rs`** — déclarer `mod auth;` et wrapper :

```rust
use crate::auth::AuthProvider;
// dans app() :
html! {
    <BrowserRouter basename="/admin">
        <AuthProvider>
            <Switch<Route> render={switch} />
        </AuthProvider>
    </BrowserRouter>
}
```

> Ordre des `mod` : `auth` dépend de `api` et `routes` ; `routes` dépend de `auth`. Le cycle de modules est OK en Rust (modules d'une même crate se voient mutuellement). Déclarer dans `main.rs` : `mod api; mod auth; mod routes; mod util;`.

- [ ] **Step 4: Build + clippy**

Run: `cd frontend && trunk build && cargo clippy --target wasm32-unknown-unknown -- -D warnings`
Expected: OK, 0 warning.

- [ ] **Step 5: Commit**

```bash
git add frontend
git commit -m "✨ feat: état d'auth dérivé SPA (AuthProvider, sonde boot, Protected)"
```

---

## Task 8: Frontend — page Login

**Files:**
- Create: `frontend/src/pages/login.rs`
- Modify: `frontend/src/pages/mod.rs` (`pub mod login;`), `frontend/src/main.rs` (`mod pages;`), `frontend/src/routes.rs` (`Route::Login => <LoginPage/>`)

**Interfaces:**
- Consumes: `api::client::login`, `auth::use_auth`, `util` non requis.
- Produces: `pages::login::LoginPage`.

- [ ] **Step 1: `frontend/src/pages/mod.rs`**

```rust
pub mod login;
```

- [ ] **Step 2: `frontend/src/pages/login.rs`**

```rust
//! Écran de login. Sur succès, bascule l'auth en Authenticated et navigue vers la
//! liste. Erreur 401 → message inline. Rate-limit géré côté serveur.

use shadcn_rs::{Button, Card, CardContent, CardHeader, CardTitle, Input, Label, Variant};
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api;
use crate::auth::use_auth;
use crate::routes::Route;

#[function_component(LoginPage)]
pub fn login_page() -> Html {
    let auth = use_auth();
    let navigator = use_navigator().expect("router");
    let user = use_state(String::new);
    let pass = use_state(String::new);
    let error = use_state(|| Option::<String>::None);
    let busy = use_state(|| false);

    let on_user = {
        let user = user.clone();
        Callback::from(move |e: InputEvent| {
            let v = e.target_unchecked_into::<HtmlInputElement>().value();
            user.set(v);
        })
    };
    let on_pass = {
        let pass = pass.clone();
        Callback::from(move |e: InputEvent| {
            let v = e.target_unchecked_into::<HtmlInputElement>().value();
            pass.set(v);
        })
    };

    let on_submit = {
        let (user, pass, error, busy) = (user.clone(), pass.clone(), error.clone(), busy.clone());
        let set_auth = auth.set_authenticated.clone();
        let navigator = navigator.clone();
        Callback::from(move |_| {
            let body = latch_dto::LoginReq { user: (*user).clone(), pass: (*pass).clone() };
            let (error, busy, set_auth, navigator) =
                (error.clone(), busy.clone(), set_auth.clone(), navigator.clone());
            busy.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match api::client::login(&body).await {
                    Ok(()) => {
                        set_auth.emit(());
                        navigator.push(&Route::Home);
                    }
                    Err(_) => error.set(Some("Identifiants invalides.".into())),
                }
                busy.set(false);
            });
        })
    };

    html! {
        <div class="auth-screen">
            <Card>
                <CardHeader><CardTitle>{ "latch — admin" }</CardTitle></CardHeader>
                <CardContent>
                    <Label html_for="user">{ "Identifiant" }</Label>
                    <Input id="user" value={(*user).clone()} oninput={on_user} />
                    <Label html_for="pass">{ "Mot de passe" }</Label>
                    <Input id="pass" r#type="password" value={(*pass).clone()} oninput={on_pass} />
                    if let Some(msg) = (*error).clone() {
                        <p class="error">{ msg }</p>
                    }
                    <Button variant={Variant::Primary} full_width={true}
                            disabled={*busy} onclick={on_submit}>
                        { if *busy { "Connexion…" } else { "Se connecter" } }
                    </Button>
                </CardContent>
            </Card>
        </div>
    }
}
```

- [ ] **Step 3: Câbler la route** — `main.rs` : ajouter `mod pages;`. `routes.rs` : `Route::Login => html! { <crate::pages::login::LoginPage /> }`.

- [ ] **Step 4: Build + clippy**

Run: `cd frontend && trunk build && cargo clippy --target wasm32-unknown-unknown -- -D warnings`
Expected: OK.

- [ ] **Step 5: Commit**

```bash
git add frontend
git commit -m "✨ feat: page Login SPA (shadcn Card/Input/Button, erreur inline)"
```

---

## Task 9: Frontend — composants réutilisables (`CopyButton`, `PinField`)

**Files:**
- Create: `frontend/src/components/copy_button.rs`, `frontend/src/components/pin_field.rs`
- Modify: `frontend/src/components/mod.rs`, `frontend/src/main.rs` (`mod components;`)

**Interfaces:**
- Produces:
  - `components::copy_button::CopyButton { value: String, label: Option<String> }` — bouton-icône qui copie `value` et affiche « Copié ! » 2 s (gloo-timers).
  - `components::pin_field::PinField { pin: String }` — affiche `••••••` + œil (révéler/masquer) + bouton copier.

- [ ] **Step 1: `frontend/src/components/mod.rs`**

```rust
pub mod copy_button;
pub mod pin_field;
```

- [ ] **Step 2: `frontend/src/components/copy_button.rs`**

```rust
//! Bouton-icône « copier » avec confirmation éphémère (pas de toast global :
//! shadcn Toast/Sonner n'auto-dismiss pas — D : feedback inline + gloo-timers).

use gloo_timers::callback::Timeout;
use shadcn_rs::{Button, Size, Variant};
use yew::prelude::*;

use crate::util::clipboard;

#[derive(Properties, PartialEq)]
pub struct CopyButtonProps {
    pub value: String,
    #[prop_or_default]
    pub aria_label: Option<AttrValue>,
}

#[function_component(CopyButton)]
pub fn copy_button(props: &CopyButtonProps) -> Html {
    let copied = use_state(|| false);

    let onclick = {
        let (value, copied) = (props.value.clone(), copied.clone());
        Callback::from(move |_| {
            clipboard::copy(value.clone());
            copied.set(true);
            let copied = copied.clone();
            // reset après 2 s ; Timeout::forget garde le timer vivant.
            Timeout::new(2000, move || copied.set(false)).forget();
        })
    };

    html! {
        <Button variant={Variant::Ghost} size={Size::Sm} onclick={onclick}
                aria_label={props.aria_label.clone()}>
            { if *copied { "Copié !" } else { "⧉" } }
        </Button>
    }
}
```

- [ ] **Step 3: `frontend/src/components/pin_field.rs`**

```rust
//! Affiche un PIN masqué avec révélation à la demande + bouton copier.

use shadcn_rs::{Button, Size, Variant};
use yew::prelude::*;

use crate::components::copy_button::CopyButton;

#[derive(Properties, PartialEq)]
pub struct PinFieldProps {
    pub pin: String,
}

#[function_component(PinField)]
pub fn pin_field(props: &PinFieldProps) -> Html {
    let revealed = use_state(|| false);
    let toggle = {
        let revealed = revealed.clone();
        Callback::from(move |_| revealed.set(!*revealed))
    };

    html! {
        <span class="pin-field">
            <code>{ if *revealed { props.pin.clone() } else { "••••••".to_string() } }</code>
            <Button variant={Variant::Ghost} size={Size::Sm} onclick={toggle}
                    aria_label={ if *revealed { "Masquer le PIN" } else { "Révéler le PIN" } }>
                { if *revealed { "🙈" } else { "👁" } }
            </Button>
            <CopyButton value={props.pin.clone()} aria_label={AttrValue::from("Copier le PIN")} />
        </span>
    }
}
```

- [ ] **Step 4: `main.rs`** — ajouter `mod components;`.

- [ ] **Step 5: Build + clippy**

Run: `cd frontend && cargo clippy --target wasm32-unknown-unknown -- -D warnings`
Expected: OK.

- [ ] **Step 6: Commit**

```bash
git add frontend
git commit -m "✨ feat: composants SPA CopyButton (Copié! éphémère) + PinField"
```

---

## Task 10: Frontend — page Liste

**Files:**
- Create: `frontend/src/pages/list.rs`
- Modify: `frontend/src/pages/mod.rs`, `frontend/src/routes.rs`

**Interfaces:**
- Consumes: `api::client::list_projects`, `auth::use_auth`, `components::CopyButton`, `util::url::public_url`, panel création (Task 11 — pour cette tâche, le bouton « Nouveau projet » est câblé mais ouvre un panel ajouté en Task 11 ; ici on met un `use_state(open)` et on branchera le panel ensuite).
- Produces: `pages::list::ListPage`.

> **Stratégie d'intégration :** cette tâche livre la liste **lisible** (chargement, table, état vide, navigation, logout) avec un bouton « Nouveau projet » qui togglera un state `creating`. Le `ProjectForm` (Task 11) sera branché sur ce state en fin de Task 11. Ne pas bloquer cette tâche sur le panel.

- [ ] **Step 1: `frontend/src/pages/list.rs`**

```rust
//! Liste des projets : table (nom, URL+copie, code, version active, #versions),
//! clic ligne → détail, bouton Nouveau projet, logout.

use shadcn_rs::{
    Badge, Button, Table, TableBody, TableCell, TableHead, TableHeader, TableRow, Variant,
};
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api::{self, ApiError};
use crate::auth::use_auth;
use crate::components::copy_button::CopyButton;
use crate::routes::Route;
use crate::util::url::public_url;
use latch_dto::ProjectListItem;

#[derive(Clone, PartialEq)]
enum Load {
    Loading,
    Ready(Vec<ProjectListItem>),
    Failed(String),
}

#[function_component(ListPage)]
pub fn list_page() -> Html {
    let auth = use_auth();
    let navigator = use_navigator().expect("router");
    let data = use_state(|| Load::Loading);

    // Chargement au montage.
    {
        let (data, set_anon) = (data.clone(), auth.set_anonymous.clone());
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                match api::client::list_projects().await {
                    Ok(items) => data.set(Load::Ready(items)),
                    Err(ApiError::Unauthorized) => set_anon.emit(()),
                    Err(e) => data.set(Load::Failed(e.user_message())),
                }
            });
            || ()
        });
    }

    let on_logout = {
        let set_anon = auth.set_anonymous.clone();
        Callback::from(move |_| {
            let set_anon = set_anon.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let _ = api::client::logout().await;
                set_anon.emit(());
            });
        })
    };

    let on_new = {
        let navigator = navigator.clone();
        // En attendant le panel (Task 11), on peut naviguer ou ouvrir un state.
        Callback::from(move |_| {
            let _ = &navigator; // remplacé par l'ouverture du ProjectForm en Task 11
        })
    };

    let body = match &*data {
        Load::Loading => html! { <p>{ "Chargement…" }</p> },
        Load::Failed(msg) => html! { <p class="error">{ msg.clone() }</p> },
        Load::Ready(items) if items.is_empty() => html! {
            <div class="empty-state">
                <p>{ "Aucun projet pour l'instant." }</p>
                <Button variant={Variant::Primary} onclick={on_new.clone()}>
                    { "+ Créer le premier projet" }
                </Button>
            </div>
        },
        Load::Ready(items) => {
            let rows = items.iter().map(|p| {
                let id = p.id;
                let nav = navigator.clone();
                let onclick = Callback::from(move |_| nav.push(&Route::Project { id }));
                let url = public_url(&p.slug);
                let badge = if p.code_enabled {
                    html! { <Badge variant={Variant::Secondary}>{ "code activé" }</Badge> }
                } else {
                    html! { <Badge variant={Variant::Outline}>{ "libre" }</Badge> }
                };
                let version = match p.active_version_id {
                    Some(_) => html! { <span>{ "active" }</span> },
                    None => html! { <span>{ "—" }</span> },
                };
                html! {
                    <TableRow>
                        <TableCell><a onclick={onclick.clone()}>{ p.name.clone() }</a></TableCell>
                        <TableCell>
                            <code>{ url.clone() }</code>
                            <CopyButton value={url} aria_label={AttrValue::from("Copier l'URL")} />
                        </TableCell>
                        <TableCell>{ badge }</TableCell>
                        <TableCell>{ version }</TableCell>
                    </TableRow>
                }
            }).collect::<Html>();
            html! {
                <Table>
                    <TableHeader>
                        <TableRow>
                            <TableHead>{ "Nom" }</TableHead>
                            <TableHead>{ "URL publique" }</TableHead>
                            <TableHead>{ "Code" }</TableHead>
                            <TableHead>{ "Version active" }</TableHead>
                        </TableRow>
                    </TableHeader>
                    <TableBody>{ rows }</TableBody>
                </Table>
            }
        }
    };

    html! {
        <div class="admin-page">
            <header class="topbar">
                <span class="brand">{ "latch" }</span>
                <span class="actions">
                    <Button variant={Variant::Primary} onclick={on_new}>{ "+ Nouveau projet" }</Button>
                    <Button variant={Variant::Ghost} onclick={on_logout}>{ "Logout" }</Button>
                </span>
            </header>
            { body }
        </div>
    }
}
```

- [ ] **Step 2: Câbler la route** — `pages/mod.rs` : `pub mod list;`. `routes.rs` : `Route::Home => html! { <Protected>{ html!{ <crate::pages::list::ListPage /> } }</Protected> }`.

- [ ] **Step 3: Build + clippy**

Run: `cd frontend && trunk build && cargo clippy --target wasm32-unknown-unknown -- -D warnings`
Expected: OK.

- [ ] **Step 4: Commit**

```bash
git add frontend
git commit -m "✨ feat: page Liste SPA (table, copie URL, état vide, logout)"
```

---

## Task 11: Frontend — side-panel Créer / Éditer (`ProjectForm`)

**Files:**
- Create: `frontend/src/panels/project_form.rs`
- Modify: `frontend/src/panels/mod.rs`, `frontend/src/main.rs` (`mod panels;`), `frontend/src/pages/list.rs` (brancher l'ouverture en création)

**Interfaces:**
- Consumes: `api::client::{create_project, update_project, set_code, clear_code}`, `util::pin`, shadcn `SheetContent` (piloté manuellement).
- Produces: `panels::project_form::ProjectForm { open: bool, mode: FormMode, on_close: Callback<()>, on_saved: Callback<()> }` où `FormMode::Create` ou `FormMode::Edit(ProjectDetail)`.

> **⚠️ Sheet shadcn-rs :** le wrapper `<Sheet>` ignore ses props. On utilise **`<SheetContent open=… on_close=…>`** directement (le seul qui marche). Pas de `SheetClose`.

- [ ] **Step 1: `frontend/src/panels/mod.rs`**

```rust
pub mod project_form;
```

- [ ] **Step 2: `frontend/src/panels/project_form.rs`**

```rust
//! Side-panel Créer/Éditer un projet (même composant, 2 modes). Pilote SheetContent
//! manuellement (open + on_close). Code = toggle + explication ; PIN généré côté SPA.

use shadcn_rs::{
    Button, Input, Label, Position, SheetContent, SheetFooter, SheetHeader, SheetTitle, Switch,
    Variant,
};
use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::api;
use crate::util::pin;
use latch_dto::{CreateProjectReq, ProjectDetail, SetCodeReq, UpdateProjectReq};

#[derive(Clone, PartialEq)]
pub enum FormMode {
    Create,
    Edit(ProjectDetail),
}

#[derive(Properties, PartialEq)]
pub struct ProjectFormProps {
    pub open: bool,
    pub mode: FormMode,
    pub on_close: Callback<()>,
    pub on_saved: Callback<()>,
}

#[function_component(ProjectForm)]
pub fn project_form(props: &ProjectFormProps) -> Html {
    let is_edit = matches!(props.mode, FormMode::Edit(_));
    let initial = match &props.mode {
        FormMode::Edit(d) => d.clone(),
        FormMode::Create => ProjectDetail {
            id: 0,
            slug: String::new(),
            name: String::new(),
            code_enabled: true,
            pin: Some(pin::generate_pin()),
            brand_name: None,
            active_version_id: None,
            versions: vec![],
        },
    };

    let name = use_state(|| initial.name.clone());
    let brand = use_state(|| initial.brand_name.clone().unwrap_or_default());
    let code_on = use_state(|| initial.code_enabled);
    let pin_val = use_state(|| initial.pin.clone().unwrap_or_else(pin::generate_pin));
    let error = use_state(|| Option::<String>::None);

    let on_name = {
        let name = name.clone();
        Callback::from(move |e: InputEvent| name.set(e.target_unchecked_into::<HtmlInputElement>().value()))
    };
    let on_brand = {
        let brand = brand.clone();
        Callback::from(move |e: InputEvent| brand.set(e.target_unchecked_into::<HtmlInputElement>().value()))
    };
    let on_pin = {
        let pin_val = pin_val.clone();
        Callback::from(move |e: InputEvent| pin_val.set(e.target_unchecked_into::<HtmlInputElement>().value()))
    };
    let on_code_toggle = {
        let code_on = code_on.clone();
        Callback::from(move |_| code_on.set(!*code_on))
    };
    let on_regen = {
        let pin_val = pin_val.clone();
        Callback::from(move |_| pin_val.set(pin::generate_pin()))
    };

    let on_save = {
        let (name, brand, code_on, pin_val, error) =
            (name.clone(), brand.clone(), code_on.clone(), pin_val.clone(), error.clone());
        let (on_saved, on_close, mode) =
            (props.on_saved.clone(), props.on_close.clone(), props.mode.clone());
        Callback::from(move |_| {
            // Validation locale.
            if name.trim().is_empty() {
                error.set(Some("Le nom est requis.".into()));
                return;
            }
            if *code_on && !pin::is_valid_pin(&pin_val) {
                error.set(Some("Le PIN doit faire 6 chiffres.".into()));
                return;
            }
            let brand_opt = if brand.trim().is_empty() { None } else { Some((*brand).clone()) };
            let (name_v, code_v, pin_v) = ((*name).clone(), *code_on, (*pin_val).clone());
            let (on_saved, on_close, error, mode) =
                (on_saved.clone(), on_close.clone(), error.clone(), mode.clone());

            wasm_bindgen_futures::spawn_local(async move {
                let res: Result<(), api::ApiError> = async {
                    match &mode {
                        FormMode::Create => {
                            let req = CreateProjectReq {
                                name: name_v,
                                brand_name: brand_opt,
                                code_enabled: code_v,
                                pin: if code_v { Some(pin_v) } else { None },
                            };
                            api::client::create_project(&req).await.map(|_| ())
                        }
                        FormMode::Edit(d) => {
                            // 1) nom + brand
                            let upd = UpdateProjectReq {
                                name: Some(name_v),
                                brand_name: Some(brand_opt),
                            };
                            api::client::update_project(d.id, &upd).await?;
                            // 2) code : activer/changer le PIN, ou désactiver.
                            if code_v {
                                api::client::set_code(d.id, &SetCodeReq { pin: pin_v }).await?;
                            } else if d.code_enabled {
                                api::client::clear_code(d.id).await?;
                            }
                            Ok(())
                        }
                    }
                }
                .await;

                match res {
                    Ok(()) => {
                        on_saved.emit(());
                        on_close.emit(());
                    }
                    Err(e) => error.set(Some(e.user_message())),
                }
            });
        })
    };

    let close = {
        let on_close = props.on_close.clone();
        Callback::from(move |_| on_close.emit(()))
    };

    html! {
        <SheetContent open={props.open} on_close={props.on_close.clone()} side={Position::Right}>
            <SheetHeader>
                <SheetTitle>{ if is_edit { "Éditer le projet" } else { "Nouveau projet" } }</SheetTitle>
            </SheetHeader>

            <Label html_for="pf-name" required={true}>{ "Nom" }</Label>
            <Input id="pf-name" value={(*name).clone()} oninput={on_name} />

            if is_edit {
                <Label html_for="pf-slug">{ "Slug (auto)" }</Label>
                <Input id="pf-slug" value={initial.slug.clone()} readonly={true} />
            }

            <Label html_for="pf-brand">{ "Nom de marque (optionnel)" }</Label>
            <Input id="pf-brand" value={(*brand).clone()} oninput={on_brand} />

            <Label html_for="pf-code">{ "Code d'accès" }</Label>
            <div class="toggle-row">
                <Switch id="pf-code" checked={*code_on} onchange={on_code_toggle} />
                <span class="hint">
                    { "Quand activé, les visiteurs saisissent un PIN à 6 chiffres avant d'accéder au prototype. Désactivé = accès libre par l'URL." }
                </span>
            </div>

            if *code_on {
                <Label html_for="pf-pin">{ "PIN (6 chiffres)" }</Label>
                <div class="pin-row">
                    <Input id="pf-pin" value={(*pin_val).clone()} oninput={on_pin} />
                    <Button variant={Variant::Outline} onclick={on_regen}>{ "⟳ régénérer" }</Button>
                </div>
            }

            if let Some(msg) = (*error).clone() {
                <p class="error">{ msg }</p>
            }

            <SheetFooter>
                <Button variant={Variant::Ghost} onclick={close}>{ "Annuler" }</Button>
                <Button variant={Variant::Primary} onclick={on_save}>{ "Enregistrer" }</Button>
            </SheetFooter>
        </SheetContent>
    }
}
```

- [ ] **Step 3: Brancher la création depuis la Liste** — dans `pages/list.rs` : `main.rs` ajoute `mod panels;`. Ajouter un `let creating = use_state(|| false);`, faire que `on_new` fasse `creating.set(true)`, ajouter à la fin du `html!` (dans `.admin-page`) :

```rust
<crate::panels::project_form::ProjectForm
    open={*creating}
    mode={crate::panels::project_form::FormMode::Create}
    on_close={{ let c = creating.clone(); Callback::from(move |_| c.set(false)) }}
    on_saved={{
        let data = data.clone();
        Callback::from(move |_| {
            let data = data.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(items) = api::client::list_projects().await { data.set(Load::Ready(items)); }
            });
        })
    }}
/>
```

- [ ] **Step 4: Build + clippy**

Run: `cd frontend && trunk build && cargo clippy --target wasm32-unknown-unknown -- -D warnings`
Expected: OK.

- [ ] **Step 5: Commit**

```bash
git add frontend
git commit -m "✨ feat: side-panel Créer/Éditer projet (Sheet contrôlé, code toggle, PIN)"
```

---

## Task 12: Frontend — side-panel Déployer (`DeployPanel`)

**Files:**
- Create: `frontend/src/panels/deploy.rs`
- Modify: `frontend/src/panels/mod.rs`

**Interfaces:**
- Consumes: `api::client::deploy`, `gloo-file` (lire le HTML uploadé), shadcn `SheetContent`/`Switch`.
- Produces: `panels::deploy::DeployPanel { open, project_id, on_close, on_deployed }`.

- [ ] **Step 1: `panels/mod.rs`** — ajouter `pub mod deploy;`.

- [ ] **Step 2: `frontend/src/panels/deploy.rs`**

```rust
//! Side-panel Déployer une version : lit un fichier HTML (gloo-file) et POST /deploy.

use gloo_file::File;
use shadcn_rs::{
    Button, Label, Position, SheetContent, SheetFooter, SheetHeader, SheetTitle, Switch, Variant,
};
use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::api;
use latch_dto::DeployReq;

#[derive(Properties, PartialEq)]
pub struct DeployPanelProps {
    pub open: bool,
    pub project_id: i32,
    pub on_close: Callback<()>,
    pub on_deployed: Callback<()>,
}

#[function_component(DeployPanel)]
pub fn deploy_panel(props: &DeployPanelProps) -> Html {
    let html_content = use_state(|| Option::<String>::None);
    let filename = use_state(|| Option::<String>::None);
    let activate = use_state(|| true);
    let error = use_state(|| Option::<String>::None);
    let busy = use_state(|| false);

    let on_file = {
        let (html_content, filename, error) = (html_content.clone(), filename.clone(), error.clone());
        Callback::from(move |e: Event| {
            let input = e.target_unchecked_into::<HtmlInputElement>();
            let Some(files) = input.files() else { return };
            let Some(file) = files.get(0) else { return };
            let name = file.name();
            let gfile = File::from(file);
            let (html_content, filename, error) =
                (html_content.clone(), filename.clone(), error.clone());
            filename.set(Some(name));
            wasm_bindgen_futures::spawn_local(async move {
                match gloo_file::futures::read_as_text(&gfile).await {
                    Ok(text) => html_content.set(Some(text)),
                    Err(_) => error.set(Some("Lecture du fichier impossible.".into())),
                }
            });
        })
    };

    let on_toggle = {
        let activate = activate.clone();
        Callback::from(move |_| activate.set(!*activate))
    };

    let on_deploy = {
        let (html_content, activate, error, busy) =
            (html_content.clone(), activate.clone(), error.clone(), busy.clone());
        let (on_close, on_deployed, id) =
            (props.on_close.clone(), props.on_deployed.clone(), props.project_id);
        Callback::from(move |_| {
            let Some(html) = (*html_content).clone() else {
                error.set(Some("Choisis un fichier HTML.".into()));
                return;
            };
            let req = DeployReq { html, activate: *activate };
            let (on_close, on_deployed, error, busy) =
                (on_close.clone(), on_deployed.clone(), error.clone(), busy.clone());
            busy.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match api::client::deploy(id, &req).await {
                    Ok(_) => {
                        on_deployed.emit(());
                        on_close.emit(());
                    }
                    Err(e) => error.set(Some(e.user_message())),
                }
                busy.set(false);
            });
        })
    };

    let close = {
        let on_close = props.on_close.clone();
        Callback::from(move |_| on_close.emit(()))
    };

    html! {
        <SheetContent open={props.open} on_close={props.on_close.clone()} side={Position::Right}>
            <SheetHeader><SheetTitle>{ "Déployer une version" }</SheetTitle></SheetHeader>

            <Label html_for="dp-file">{ "Fichier HTML" }</Label>
            <input id="dp-file" type="file" accept="text/html,.html" onchange={on_file} />
            if let Some(n) = (*filename).clone() { <p class="hint">{ n }</p> }

            <div class="toggle-row">
                <Switch id="dp-activate" checked={*activate} onchange={on_toggle} />
                <span class="hint">{ "Activer immédiatement : la nouvelle version devient l'active servie sur l'URL publique." }</span>
            </div>

            if let Some(msg) = (*error).clone() { <p class="error">{ msg }</p> }

            <SheetFooter>
                <Button variant={Variant::Ghost} onclick={close}>{ "Annuler" }</Button>
                <Button variant={Variant::Primary} disabled={*busy} onclick={on_deploy}>
                    { if *busy { "Déploiement…" } else { "Déployer" } }
                </Button>
            </SheetFooter>
        </SheetContent>
    }
}
```

> Si Task 6 a fixé `deploy() -> Result<(), ApiError>` (option (b)), le `Ok(_)` ci-dessus reste correct.

- [ ] **Step 3: Build + clippy**

Run: `cd frontend && cargo clippy --target wasm32-unknown-unknown -- -D warnings`
Expected: OK.

- [ ] **Step 4: Commit**

```bash
git add frontend
git commit -m "✨ feat: side-panel Déployer (upload HTML via gloo-file, activer)"
```

---

## Task 13: Frontend — page Détail + side-panels danger (supprimer projet / version) + actions versions

**Files:**
- Create: `frontend/src/panels/delete_project.rs`, `frontend/src/panels/delete_version.rs`, `frontend/src/pages/detail.rs`
- Modify: `frontend/src/panels/mod.rs`, `frontend/src/pages/mod.rs`, `frontend/src/routes.rs`

**Interfaces:**
- Consumes: `api::client::{get_project, delete_project, delete_version, activate_version, preview_url}`, `panels::{project_form, deploy}`, `components::{CopyButton, PinField}`.
- Produces: `pages::detail::DetailPage { id }`, `panels::delete_project::DeleteProjectPanel`, `panels::delete_version::DeleteVersionPanel`.

- [ ] **Step 1: `panels/delete_project.rs`**

```rust
//! Side-panel danger : supprimer un projet (confirmation in-panel).

use shadcn_rs::{
    Button, Position, SheetContent, SheetFooter, SheetHeader, SheetTitle, Variant,
};
use yew::prelude::*;

use crate::api;
use latch_dto::ProjectDetail;

#[derive(Properties, PartialEq)]
pub struct DeleteProjectPanelProps {
    pub open: bool,
    pub project: ProjectDetail,
    pub on_close: Callback<()>,
    pub on_deleted: Callback<()>,
}

#[function_component(DeleteProjectPanel)]
pub fn delete_project_panel(props: &DeleteProjectPanelProps) -> Html {
    let error = use_state(|| Option::<String>::None);
    let busy = use_state(|| false);
    let n_versions = props.project.versions.len();

    let on_confirm = {
        let (on_close, on_deleted, error, busy, id) = (
            props.on_close.clone(),
            props.on_deleted.clone(),
            error.clone(),
            busy.clone(),
            props.project.id,
        );
        Callback::from(move |_| {
            let (on_close, on_deleted, error, busy) =
                (on_close.clone(), on_deleted.clone(), error.clone(), busy.clone());
            busy.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match api::client::delete_project(id).await {
                    Ok(()) => {
                        on_deleted.emit(());
                        on_close.emit(());
                    }
                    Err(e) => error.set(Some(e.user_message())),
                }
                busy.set(false);
            });
        })
    };
    let close = {
        let on_close = props.on_close.clone();
        Callback::from(move |_| on_close.emit(()))
    };

    html! {
        <SheetContent open={props.open} on_close={props.on_close.clone()} side={Position::Right}
                      class={classes!("sheet-danger")}>
            <SheetHeader><SheetTitle>{ format!("Supprimer « {} »", props.project.name) }</SheetTitle></SheetHeader>
            <p>{ "Cette action est irréversible. Seront supprimés définitivement :" }</p>
            <ul>
                <li>{ "le projet et sa configuration ;" }</li>
                <li>{ format!("ses {n_versions} version(s) et leurs fichiers HTML ;") }</li>
                <li>{ "l'URL publique (404 ensuite)." }</li>
            </ul>
            if let Some(msg) = (*error).clone() { <p class="error">{ msg }</p> }
            <SheetFooter>
                <Button variant={Variant::Ghost} onclick={close}>{ "Annuler" }</Button>
                <Button variant={Variant::Destructive} disabled={*busy} onclick={on_confirm}>
                    { "Oui, supprimer définitivement" }
                </Button>
            </SheetFooter>
        </SheetContent>
    }
}
```

- [ ] **Step 2: `panels/delete_version.rs`**

```rust
//! Side-panel danger : supprimer une version (inactive).

use shadcn_rs::{
    Button, Position, SheetContent, SheetFooter, SheetHeader, SheetTitle, Variant,
};
use yew::prelude::*;

use crate::api;

#[derive(Properties, PartialEq)]
pub struct DeleteVersionPanelProps {
    pub open: bool,
    pub project_id: i32,
    pub n: i32,
    pub on_close: Callback<()>,
    pub on_deleted: Callback<()>,
}

#[function_component(DeleteVersionPanel)]
pub fn delete_version_panel(props: &DeleteVersionPanelProps) -> Html {
    let error = use_state(|| Option::<String>::None);
    let on_confirm = {
        let (on_close, on_deleted, error, id, n) = (
            props.on_close.clone(),
            props.on_deleted.clone(),
            error.clone(),
            props.project_id,
            props.n,
        );
        Callback::from(move |_| {
            let (on_close, on_deleted, error) = (on_close.clone(), on_deleted.clone(), error.clone());
            wasm_bindgen_futures::spawn_local(async move {
                match api::client::delete_version(id, n).await {
                    Ok(()) => {
                        on_deleted.emit(());
                        on_close.emit(());
                    }
                    Err(e) => error.set(Some(e.user_message())),
                }
            });
        })
    };
    let close = {
        let on_close = props.on_close.clone();
        Callback::from(move |_| on_close.emit(()))
    };
    html! {
        <SheetContent open={props.open} on_close={props.on_close.clone()} side={Position::Right}
                      class={classes!("sheet-danger")}>
            <SheetHeader><SheetTitle>{ format!("Supprimer la version v{}", props.n) }</SheetTitle></SheetHeader>
            <p>{ "Cette version et son fichier HTML seront supprimés. Action irréversible." }</p>
            if let Some(msg) = (*error).clone() { <p class="error">{ msg }</p> }
            <SheetFooter>
                <Button variant={Variant::Ghost} onclick={close}>{ "Annuler" }</Button>
                <Button variant={Variant::Destructive} onclick={on_confirm}>{ "Oui, supprimer" }</Button>
            </SheetFooter>
        </SheetContent>
    }
}
```

- [ ] **Step 3: `panels/mod.rs`** — ajouter `pub mod delete_project;` et `pub mod delete_version;`.

- [ ] **Step 4: `frontend/src/pages/detail.rs`**

```rust
//! Détail projet : lecture seule + actions en haut à droite (Éditer / Déployer /
//! Supprimer) + versions avec actions-icône. Tout passe par des side-panels.

use shadcn_rs::{
    Badge, Button, Card, CardContent, CardHeader, CardTitle, Size, Table, TableBody, TableCell,
    TableHead, TableHeader, TableRow, Variant,
};
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api::{self, ApiError};
use crate::auth::use_auth;
use crate::components::{copy_button::CopyButton, pin_field::PinField};
use crate::panels::deploy::DeployPanel;
use crate::panels::delete_project::DeleteProjectPanel;
use crate::panels::delete_version::DeleteVersionPanel;
use crate::panels::project_form::{FormMode, ProjectForm};
use crate::routes::Route;
use crate::util::url::public_url;
use latch_dto::ProjectDetail;

#[derive(Properties, PartialEq)]
pub struct DetailProps {
    pub id: i32,
}

#[derive(Clone, PartialEq)]
enum Load {
    Loading,
    Ready(ProjectDetail),
    Failed(String),
}

#[function_component(DetailPage)]
pub fn detail_page(props: &DetailProps) -> Html {
    let id = props.id;
    let auth = use_auth();
    let navigator = use_navigator().expect("router");
    let data = use_state(|| Load::Loading);
    let editing = use_state(|| false);
    let deploying = use_state(|| false);
    let deleting = use_state(|| false);
    let deleting_version = use_state(|| Option::<i32>::None);

    // reload helper
    let reload = {
        let (data, set_anon) = (data.clone(), auth.set_anonymous.clone());
        Callback::from(move |_| {
            let (data, set_anon) = (data.clone(), set_anon.clone());
            wasm_bindgen_futures::spawn_local(async move {
                match api::client::get_project(id).await {
                    Ok(d) => data.set(Load::Ready(d)),
                    Err(ApiError::Unauthorized) => set_anon.emit(()),
                    Err(e) => data.set(Load::Failed(e.user_message())),
                }
            });
        })
    };

    {
        let reload = reload.clone();
        use_effect_with((), move |_| {
            reload.emit(());
            || ()
        });
    }

    let body = match &*data {
        Load::Loading => html! { <p>{ "Chargement…" }</p> },
        Load::Failed(msg) => html! { <p class="error">{ msg.clone() }</p> },
        Load::Ready(p) => {
            let url = public_url(&p.slug);
            let on_back = {
                let nav = navigator.clone();
                Callback::from(move |_| nav.push(&Route::Home))
            };
            let open_edit = { let e = editing.clone(); Callback::from(move |_| e.set(true)) };
            let open_deploy = { let d = deploying.clone(); Callback::from(move |_| d.set(true)) };
            let open_delete = { let d = deleting.clone(); Callback::from(move |_| d.set(true)) };

            let access = html! {
                <Card>
                    <CardHeader><CardTitle>{ "Accès public" }</CardTitle></CardHeader>
                    <CardContent>
                        <div class="kv">
                            <span class="k">{ "URL publique" }</span>
                            <span class="v">
                                <code>{ url.clone() }</code>
                                <CopyButton value={url.clone()} aria_label={AttrValue::from("Copier l'URL")} />
                            </span>
                        </div>
                        <div class="kv">
                            <span class="k">{ "Code d'accès" }</span>
                            <span class="v">
                                if p.code_enabled {
                                    if let Some(pin) = p.pin.clone() { <PinField pin={pin} /> }
                                } else {
                                    <Badge variant={Variant::Outline}>{ "Accès libre" }</Badge>
                                }
                            </span>
                        </div>
                    </CardContent>
                </Card>
            };

            let config = html! {
                <Card>
                    <CardHeader><CardTitle>{ "Configuration" }</CardTitle></CardHeader>
                    <CardContent>
                        <div class="kv"><span class="k">{ "Nom de marque" }</span>
                            <span class="v">{ p.brand_name.clone().unwrap_or_else(|| "—".into()) }</span></div>
                        <div class="kv"><span class="k">{ "Code" }</span>
                            <span class="v">{ if p.code_enabled { "activé" } else { "libre" } }</span></div>
                    </CardContent>
                </Card>
            };

            let rows = p.versions.iter().map(|v| {
                let n = v.n;
                let activate = {
                    let reload = reload.clone();
                    Callback::from(move |_| {
                        let reload = reload.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            let _ = api::client::activate_version(id, n).await;
                            reload.emit(());
                        });
                    })
                };
                let preview_href = api::client::preview_url(id, n);
                let on_del = {
                    let dv = deleting_version.clone();
                    Callback::from(move |_| dv.set(Some(n)))
                };
                html! {
                    <TableRow>
                        <TableCell>{ format!("v{}", v.n) }</TableCell>
                        <TableCell>{ v.created_at.clone() }</TableCell>
                        <TableCell>
                            if v.is_active { <Badge variant={Variant::Secondary}>{ "active" }</Badge> }
                        </TableCell>
                        <TableCell>
                            if !v.is_active {
                                <Button variant={Variant::Ghost} size={Size::Sm} onclick={activate}
                                        aria_label={AttrValue::from("Activer")}>{ "↑" }</Button>
                            }
                            <a href={preview_href} target="_blank" rel="noopener" class="icon-link"
                               aria-label="Prévisualiser">{ "↗" }</a>
                            if !v.is_active {
                                <Button variant={Variant::Ghost} size={Size::Sm} onclick={on_del}
                                        aria_label={AttrValue::from("Supprimer")}>{ "🗑" }</Button>
                            }
                        </TableCell>
                    </TableRow>
                }
            }).collect::<Html>();

            html! {
                <>
                    <header class="detail-head">
                        <div>
                            <a class="crumb" onclick={on_back}>{ "‹ Projets" }</a>
                            <h1>{ p.name.clone() }</h1>
                        </div>
                        <div class="head-actions">
                            <Button variant={Variant::Outline} onclick={open_edit}>{ "✎ Éditer" }</Button>
                            <Button variant={Variant::Outline} onclick={open_deploy}>{ "⬆ Déployer" }</Button>
                            <Button variant={Variant::Destructive} onclick={open_delete}>{ "🗑 Supprimer" }</Button>
                        </div>
                    </header>
                    { access }
                    { config }
                    <Card>
                        <CardHeader><CardTitle>{ "Versions" }</CardTitle></CardHeader>
                        <CardContent>
                            <Table>
                                <TableHeader><TableRow>
                                    <TableHead>{ "#" }</TableHead>
                                    <TableHead>{ "Date" }</TableHead>
                                    <TableHead>{ "Statut" }</TableHead>
                                    <TableHead>{ "" }</TableHead>
                                </TableRow></TableHeader>
                                <TableBody>{ rows }</TableBody>
                            </Table>
                        </CardContent>
                    </Card>

                    // Panels
                    <ProjectForm open={*editing} mode={FormMode::Edit(p.clone())}
                        on_close={{ let e = editing.clone(); Callback::from(move |_| e.set(false)) }}
                        on_saved={reload.clone()} />
                    <DeployPanel open={*deploying} project_id={id}
                        on_close={{ let d = deploying.clone(); Callback::from(move |_| d.set(false)) }}
                        on_deployed={reload.clone()} />
                    <DeleteProjectPanel open={*deleting} project={p.clone()}
                        on_close={{ let d = deleting.clone(); Callback::from(move |_| d.set(false)) }}
                        on_deleted={{ let nav = navigator.clone(); Callback::from(move |_| nav.push(&Route::Home)) }} />
                    if let Some(n) = *deleting_version {
                        <DeleteVersionPanel open={true} project_id={id} n={n}
                            on_close={{ let dv = deleting_version.clone(); Callback::from(move |_| dv.set(None)) }}
                            on_deleted={reload.clone()} />
                    }
                </>
            }
        }
    };

    html! { <div class="admin-page">{ body }</div> }
}
```

- [ ] **Step 5: Câbler la route** — `pages/mod.rs` : `pub mod detail;`. `routes.rs` : `Route::Project { id } => html! { <Protected>{ html!{ <crate::pages::detail::DetailPage {id} /> } }</Protected> }`.

- [ ] **Step 6: Build + clippy**

Run: `cd frontend && trunk build && cargo clippy --target wasm32-unknown-unknown -- -D warnings`
Expected: OK.

- [ ] **Step 7: Vérification manuelle du parcours complet** (serveur + SPA)

```bash
# Terminal 1 : backend (sert aussi la SPA buildée si LATCH_SPA_DIST pointe sur frontend/dist)
cd frontend && trunk build && cd ../backend && cargo loco start
# Naviguer http://127.0.0.1:5150/admin :
#  - login → liste → + Nouveau projet → détail → Éditer → Déployer (un .html) →
#    activer une version → prévisualiser (onglet) → supprimer une version →
#    supprimer le projet → logout. Vérifier que 401 ramène au login.
```

Expected: parcours complet fonctionnel (critère de sortie ROADMAP Phase 3).

- [ ] **Step 8: Commit**

```bash
git add frontend
git commit -m "✨ feat: page Détail SPA + side-panels danger (supprimer projet/version)"
```

---

## Task 14: Docker/env, contrat, mémoire (clôture de phase)

**Files:**
- Modify: `Dockerfile`, `.env.example`, `docs/ENVIRONMENT.md`
- Modify: `docs/contrat-deploy.md` (§4, §7), `docs/BACKLOG.md`
- Modify: `docs/INDEX.md`, `docs/HANDOFF.md`, `docs/QUIRKS.md`, `docs/CONVENTIONS.md`

**Interfaces:** aucune (clôture).

- [ ] **Step 1: Dockerfile — copier la SPA + poser `LATCH_SPA_DIST`** — vérifier que l'étape Trunk produit `frontend/dist` et que l'étape runtime la copie à un chemin fixe (ex. `/app/spa`) ; ajouter `ENV LATCH_SPA_DIST=/app/spa` dans l'étape runtime. (Adapter au Dockerfile multi-stage existant : `COPY --from=trunk /app/frontend/dist /app/spa`.)

- [ ] **Step 2: `.env.example` + `docs/ENVIRONMENT.md`** — documenter `LATCH_SPA_DIST` (défaut dev `../frontend/dist`, prod `/app/spa`).

- [ ] **Step 3: Contrat `docs/contrat-deploy.md`**
  - **§4** : ajouter — API JSON sous **`/api/*`** ; SPA servie en statique sous **`/admin/*`** (`nest_service` ServeDir + fallback `index.html`, BrowserRouter `basename="/admin"`) ; DTO partagés via crate **`latch-dto`**.
  - **§7** : éditer — édition/suppression/déploiement = **side-panels dédiés** ; confirmations destructives = **side-panels *danger*** (remplace « modale ») ; page détail en **lecture seule** ; actions principales en haut à droite, actions de ligne/copie en **boutons-icône** ; **slug en lecture seule** (base éditable reportée) ; URL publique via `window.location.origin`.

- [ ] **Step 4: `docs/BACKLOG.md`** — ajouter : base de slug éditable ; override `PUBLIC_BASE_URL` ; couche de toast globale (shadcn `Sonner` n'auto-dismiss pas) ; dark-mode toggle (`.dark`).

- [ ] **Step 5: `docs/QUIRKS.md`** — ajouter les pièges découverts :
  1. `yew-router = 0.18` (PAS 0.21) pour `yew 0.21` — numérotation divergente.
  2. `gloo-net` : un 401/404 est `Ok(Response)`, pas une `Err` → tester `status()`. `.json(&body)?` consomme le builder (retourne `Result<Request>`) avant `.send()`.
  3. `tower-http` doit activer explicitement le feature `fs` (même si transitif).
  4. shadcn-rs : `<Sheet>` wrapper est une **coquille** (ignore ses props) → piloter `<SheetContent open=… on_close=…>` directement ; pas de `SheetClose`. Pas de toast programmatique (`Toast`/`Sonner` déclaratifs, `duration` non implémenté).
  5. shadcn-rs `Switch`/`Dialog` : état « contrôlé » retombe sur l'état interne tant que `checked={false}` → gérer le state soi-même.
  6. shadcn-rs.css livrée sous `styles/` du crate (5 fichiers, `@import` relatifs) — vendoriser tout le dossier ; dark-mode via classe `.dark`.
  7. SPA sous `/admin` : `Trunk.toml public_url = "/admin/"` + `BrowserRouter basename="/admin"` + `#[at("/admin/...")]` absolus.
  8. Orphan rule : conversions DTO en fonctions libres côté back (`dto::to_list_item`/`to_detail`).

- [ ] **Step 6: `docs/CONVENTIONS.md`** — ajouter les squelettes : « Composant Yew (shadcn-rs) type » (page + SheetContent contrôlé), « Client API SPA type » (fn gloo-net + check_status 401), « Tool MCP type » reste à remplir (Phase 5).

- [ ] **Step 7: `docs/INDEX.md`** — cocher les livrables Phase 3 (crate latch-dto, serving SPA, pages login/list/detail, panels, composants) — Phase 3 — 2026-06-24. Marquer `- [x] Phase 3` dans « Phases closes ».

- [ ] **Step 8: `docs/HANDOFF.md`** — entrée datée en haut : Dernière chose faite (Phase 3 livrée, parcours admin complet), Trucs en suspens (e2e Phase 6, deploy retour `{id,n}`), Prochaine chose (Phase 4 serving `/c/<slug>`), Notes pour future Claude (quirks ci-dessus).

- [ ] **Step 9: Vérification finale complète**

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo nextest run -p latch
cargo test -p latch-dto
cd frontend && cargo clippy --target wasm32-unknown-unknown --all-targets -- -D warnings && wasm-pack test --headless --firefox && trunk build
```

Expected: tout vert.

- [ ] **Step 10: Commit de clôture**

```bash
git add -A
git commit -m "📝 docs: clôture Phase 3 (SPA Yew admin) — contrat §4/§7, mémoire, Docker SPA"
```

---

## Self-Review (effectué)

**Couverture spec :** §3 archi → Tasks 3,6,7 ; §4 routes → Tasks 2,3 ; §5 latch-dto → Task 1 ; §6 structure → Tasks 4-13 ; §7 écrans → Tasks 8 (login), 10 (liste), 11 (form), 12 (deploy), 13 (détail+delete) ; §8 styling → Task 4 ; §9 serving → Task 3 ; §10 tests → Tasks 1,2,3,5 + vérif manuelle 13 ; §11 risques → résolus (versions épinglées) ; §12 amendements → Task 14 ; §13 critères de sortie → Task 13 step 7 + Task 14 step 9.

**Placeholders :** aucun « TBD/TODO » ; deux points signalés explicitement à trancher à l'implémentation (forme du retour `deploy` → recommandation (b) ; signature `navigator().clipboard()` selon version web-sys) — assortis d'une décision par défaut.

**Cohérence des types :** `ApiError`, `Load`, `FormMode`, `AuthState`/`AuthContext`, noms de fonctions `api::client::*` cohérents entre Tasks 6/7/8/10/11/12/13. Conversions back `dto::to_list_item`/`to_detail` cohérentes Tasks 2.
