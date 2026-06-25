# Phase 4 — Serving `/c/<slug>` + déverrouillage — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Servir les prototypes sous `/c/<slug>` avec deux états (libre / protégé par code), une page de déverrouillage React+shadcn dédiée, un cookie signé qui lie le PIN courant, et un rate-limit `governor` *load-bearing* sur l'unlock.

**Architecture:** Adaptateur entrant `controllers/serve.rs` (fin) au-dessus du cœur existant (`ProjectsService::verify_code`, `Storage::read`). La crypto qui lie le PIN au cookie est une **fonction pure du cœur** (`services/unlock_cookie.rs`) ; le jar signé + les attributs cookie vivent dans l'adaptateur. La page de déverrouillage est une **2ᵉ entrée Vite** isolée. Aucun nouveau modèle DB.

**Tech Stack:** Rust / Loco 0.16 / axum 0.8 / SeaORM 1.1 ; `axum-extra` (SignedCookieJar) ; `hmac`+`sha2`+`hex` (empreinte PIN) ; `tower_governor` 0.7 (rate-limit) ; React 19 / Vite / shadcn / react-i18next.

## Global Constraints

- **Le contrat `docs/contrat-deploy.md` fait loi** (§6 serving, §9 invariants). Spec de référence : `docs/superpowers/specs/2026-06-25-phase-4-serving-design.md`.
- **Cœur agnostique HTTP** : aucun `use axum::` / `use loco_rs::` dans `backend/src/services/` (garde `backend/tests/architecture.rs`).
- **Pas d'`unwrap`/`expect`** hors tests et hors init de boot. Erreurs propagées (`CoreError` côté cœur, `loco_rs::Error` côté adaptateur).
- **Invariants §9** : aucune réponse ne contient de hash ; le PIN n'apparaît jamais sur cette surface (le DTO `PublicMeta` n'a pas de champ `pin`).
- **Cache** : `Cache-Control: no-store` sur **toute** réponse de la surface `/c` (proto servi ET page unlock).
- **Confidentialité** : aucun nom de client réel ; placeholders génériques (`Mon Projet`, `ACME`, `demo`).
- **Tests CI** : valider en local avec **`cargo nextest run`** (pas `cargo test` — cf. QUIRKS course inter-process), `#[serial]` sur tout test qui boote l'app.
- **Commits** : conventionnels + gitmoji (`✨ feat:`, `🐛 fix:`, `🧱 chore:`, `📝 docs:`).
- **Après tout changement DTO/handler** : `UPDATE_OPENAPI=1 cargo test --test openapi_drift` (backend) **et** `cd frontend && pnpm gen:api` (front).

---

### Task 1 : Cœur — token de cookie unlock (empreinte PIN + vérif)

**Files:**
- Modify: `backend/Cargo.toml` (deps `hmac`, `sha2`, `hex`)
- Create: `backend/src/services/unlock_cookie.rs`
- Modify: `backend/src/services/mod.rs` (déclarer `pub mod unlock_cookie;`)

**Interfaces:**
- Produces:
  - `pub fn issue_token(secret: &[u8], slug: &str, pin: &str, exp_unix: i64) -> String` — renvoie `"<exp>:<fp_hex>"`.
  - `pub fn verify_token(secret: &[u8], slug: &str, pin: &str, token: &str, now_unix: i64) -> bool` — `true` ssi intégrité + PIN courant + non expiré.
- Consumes : `crate::services::security::secure_compare` (existant).

- [ ] **Step 1: Ajouter les dépendances**

Dans `backend/Cargo.toml`, sous `subtle = { version = "2" }` :

```toml
hmac = { version = "0.12" }
sha2 = { version = "0.10" }
hex = { version = "0.4" }
```

- [ ] **Step 2: Écrire le module avec ses tests (RED)**

Créer `backend/src/services/unlock_cookie.rs` :

```rust
//! Cœur (contrat §1, agnostique HTTP) : jeton de déverrouillage client.
//! Le jeton porté par le cookie signé lie le **PIN courant** du projet :
//! roter le PIN invalide les jetons déjà émis (révocation §6), et l'expiration
//! borne leur durée de vie. La signature du *transport* (anti-falsification) est
//! assurée par le `SignedCookieJar` côté adaptateur ; ici on ne gère que le lien
//! au PIN + l'expiration, en valeurs pures et testables.

use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::services::security::secure_compare;

type HmacSha256 = Hmac<Sha256>;

/// Empreinte one-way du PIN, scopée au slug. Sûre à exposer dans la valeur du
/// cookie (un cookie signé n'est pas chiffré — sa valeur est lisible).
fn fingerprint(secret: &[u8], slug: &str, pin: &str) -> String {
    // `new_from_slice` n'échoue jamais pour HMAC (accepte toute longueur de clé).
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC accepte toute clé");
    mac.update(slug.as_bytes());
    mac.update(b":");
    mac.update(pin.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Valeur du cookie : `"<exp_unix>:<fp_hex>"`.
pub fn issue_token(secret: &[u8], slug: &str, pin: &str, exp_unix: i64) -> String {
    format!("{exp_unix}:{}", fingerprint(secret, slug, pin))
}

/// `true` ssi le jeton est bien formé, non expiré (`now <= exp`), et son empreinte
/// correspond au PIN **courant** (comparaison à temps constant).
pub fn verify_token(secret: &[u8], slug: &str, pin: &str, token: &str, now_unix: i64) -> bool {
    let Some((exp_str, fp)) = token.split_once(':') else {
        return false;
    };
    let Ok(exp) = exp_str.parse::<i64>() else {
        return false;
    };
    if now_unix > exp {
        return false;
    }
    secure_compare(fp, &fingerprint(secret, slug, pin))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &[u8] = b"unit-test-secret-key-please-override-0123456789abcdef0123456789";

    #[test]
    fn valid_token_roundtrips() {
        let t = issue_token(SECRET, "demo-abc", "123456", 1000);
        assert!(verify_token(SECRET, "demo-abc", "123456", &t, 999));
    }

    #[test]
    fn rotated_pin_invalidates_token() {
        // Jeton émis sous l'ancien PIN ; le projet a roté vers un nouveau PIN.
        let t = issue_token(SECRET, "demo-abc", "123456", 1000);
        assert!(!verify_token(SECRET, "demo-abc", "654321", &t, 999));
    }

    #[test]
    fn expired_token_rejected() {
        let t = issue_token(SECRET, "demo-abc", "123456", 1000);
        assert!(!verify_token(SECRET, "demo-abc", "123456", &t, 1001));
    }

    #[test]
    fn tampered_fingerprint_rejected() {
        let t = issue_token(SECRET, "demo-abc", "123456", 1000);
        let tampered = format!("{}0", t); // fp altéré
        assert!(!verify_token(SECRET, "demo-abc", "123456", &tampered, 999));
    }

    #[test]
    fn malformed_token_rejected() {
        assert!(!verify_token(SECRET, "demo-abc", "123456", "garbage", 999));
        assert!(!verify_token(SECRET, "demo-abc", "123456", "notanint:abc", 999));
    }

    #[test]
    fn fingerprint_is_slug_scoped() {
        // Même PIN, slug différent → empreinte différente (un cookie ne vaut que pour son slug).
        let t = issue_token(SECRET, "demo-abc", "123456", 1000);
        assert!(!verify_token(SECRET, "autre-slug", "123456", &t, 999));
    }
}
```

- [ ] **Step 3: Déclarer le module**

Dans `backend/src/services/mod.rs`, ajouter à la liste des `pub mod` (ordre alphabétique) :

```rust
pub mod unlock_cookie;
```

- [ ] **Step 4: Lancer les tests (RED → GREEN)**

Run: `cd backend && cargo nextest run -E 'test(unlock_cookie)'`
Expected: 6 tests PASS.

- [ ] **Step 5: Garde d'archi + qualité**

Run: `cargo nextest run --test architecture && cargo fmt --all && cargo clippy --all-targets -- -D warnings`
Expected: architecture PASS (le cœur n'importe ni axum ni loco), 0 warning.

- [ ] **Step 6: Commit**

```bash
rtk git add backend/Cargo.toml backend/Cargo.lock backend/src/services/unlock_cookie.rs backend/src/services/mod.rs
rtk git commit -m "✨ feat(core): jeton de cookie unlock liant le PIN (HMAC + expiration)"
```

---

### Task 2 : DTO — `PublicMeta` + `UnlockReq`

**Files:**
- Modify: `backend/src/dto/mod.rs`

**Interfaces:**
- Produces:
  - `pub struct PublicMeta { pub brand_name: Option<String>, pub code_enabled: bool }` (Serialize, ToSchema) — **pas de champ `pin`**.
  - `pub struct UnlockReq { pub pin: String }` (Deserialize, ToSchema).
  - `pub fn to_public_meta(m: &projects::Model) -> PublicMeta`.

- [ ] **Step 1: Écrire les types + test (RED)**

Dans `backend/src/dto/mod.rs`, après `ProjectDetail` (avant les `*Req`) ajouter :

```rust
/// Meta publique servie à la page de déverrouillage (`GET /api/public/{slug}`).
/// **Sans PIN** (invariant §9.2 : structurellement absent) — `brand_name` est fait
/// pour être affiché publiquement sur la page de code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct PublicMeta {
    pub brand_name: Option<String>,
    pub code_enabled: bool,
}

/// Corps de `POST /c/{slug}/unlock`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct UnlockReq {
    pub pin: String,
}
```

Dans la fonction de conversion (après `to_detail`) ajouter :

```rust
/// Projet → meta publique (sans PIN, sans version).
pub fn to_public_meta(m: &projects::Model) -> PublicMeta {
    PublicMeta {
        brand_name: m.brand_name.clone(),
        code_enabled: m.code_enabled,
    }
}
```

Dans le `mod tests` de `dto/mod.rs`, ajouter :

```rust
    #[test]
    fn public_meta_never_serializes_pin() {
        let json = serde_json::to_string(&to_public_meta(&sample_model())).unwrap();
        assert!(
            !json.contains("424242") && !json.contains("\"pin\""),
            "PublicMeta ne doit jamais exposer le PIN (§9.2)"
        );
        assert!(json.contains("code_enabled"));
    }
```

- [ ] **Step 2: Lancer le test**

Run: `cd backend && cargo nextest run -E 'test(dto)'`
Expected: PASS (dont `public_meta_never_serializes_pin`).

- [ ] **Step 3: Commit**

```bash
rtk git add backend/src/dto/mod.rs
rtk git commit -m "✨ feat(dto): PublicMeta (sans PIN) + UnlockReq pour la surface /c"
```

---

### Task 3 : Helpers web (secret/clé unlock, chemin unlock.html) + dep axum-extra

**Files:**
- Modify: `backend/Cargo.toml` (dep `axum-extra`)
- Modify: `backend/src/web/mod.rs`

**Interfaces:**
- Produces (dans `crate::web`) :
  - `pub fn unlock_secret() -> loco_rs::Result<String>` — lit `UNLOCK_COOKIE_SECRET` (≥ 64 bytes), fallback dev déterministe, erreur si trop court.
  - `pub fn unlock_key() -> loco_rs::Result<axum_extra::extract::cookie::Key>` — `Key` du jar signé, dérivée du secret.
  - `pub fn cookie_secure(ctx: &AppContext) -> bool` — `true` sauf en Development/Test (fail-secure, même critère que la session).
  - `pub fn unlock_index() -> std::path::PathBuf` — `spa_dist_dir().join("unlock.html")`.

- [ ] **Step 1: Ajouter axum-extra**

Dans `backend/Cargo.toml`, sous `axum = { version = "0.8" }` :

```toml
axum-extra = { version = "0.10", features = ["cookie"] }
```

(`axum-extra` 0.10 cible axum 0.8. `cookie` active `SignedCookieJar`/`Key`/`Cookie`.)

- [ ] **Step 2: Écrire les helpers**

Dans `backend/src/web/mod.rs`, ajouter (après `spa_dist_dir`) :

```rust
/// Chemin du `unlock.html` buildé (2ᵉ entrée Vite), sous la même racine que la SPA.
pub fn unlock_index() -> PathBuf {
    spa_dist_dir().join("unlock.html")
}

/// Secret HMAC du cookie de déverrouillage client. Doit faire ≥ 64 bytes
/// (exigence de `cookie::Key`). En dev, clé de secours déterministe (insécurisée).
/// En prod, `UNLOCK_COOKIE_SECRET` doit être défini avec ≥ 64 bytes d'entropie.
pub fn unlock_secret() -> Result<String> {
    let secret = std::env::var("UNLOCK_COOKIE_SECRET").unwrap_or_else(|_| {
        "dev-only-insecure-unlock-cookie-secret-please-override-in-production!!".to_string()
    });
    if secret.len() < 64 {
        return Err(loco_rs::Error::Message(format!(
            "UNLOCK_COOKIE_SECRET trop court : {} octets (minimum 64)",
            secret.len()
        )));
    }
    Ok(secret)
}

/// `Key` du `SignedCookieJar` (signature anti-falsification du cookie unlock).
pub fn unlock_key() -> Result<axum_extra::extract::cookie::Key> {
    Ok(axum_extra::extract::cookie::Key::from(
        unlock_secret()?.as_bytes(),
    ))
}

/// `true` si l'on doit poser `Secure` sur le cookie (fail-secure : tout env hors
/// Development/Test). Aligné sur `build_session_store`.
pub fn cookie_secure(ctx: &AppContext) -> bool {
    !matches!(
        ctx.environment,
        loco_rs::environment::Environment::Development | loco_rs::environment::Environment::Test
    )
}
```

- [ ] **Step 3: Valider le secret au boot (fail-fast en prod)**

Dans `backend/src/app.rs`, dans `after_routes`, juste après `let store = crate::web::build_session_store(ctx).await?;` ajouter :

```rust
        // Fail-fast : un UNLOCK_COOKIE_SECRET trop court en prod doit casser le boot,
        // pas produire un 500 à la première requête /c protégée.
        crate::web::unlock_secret()?;
```

- [ ] **Step 4: Compiler**

Run: `cd backend && cargo build`
Expected: compile sans erreur (les helpers sont encore inutilisés ailleurs → pas de warning bloquant car `pub`).

- [ ] **Step 5: Commit**

```bash
rtk git add backend/Cargo.toml backend/Cargo.lock backend/src/web/mod.rs backend/src/app.rs
rtk git commit -m "🧱 chore(web): helpers cookie unlock (secret≥64, Key signée, no-store path) + axum-extra"
```

---

### Task 4 : `GET /api/public/{slug}` (meta publique) + câblage + OpenAPI

**Files:**
- Create: `backend/src/controllers/serve.rs`
- Modify: `backend/src/controllers/mod.rs` (`pub mod serve;`)
- Modify: `backend/src/app.rs` (monter `serve::routes()`)
- Modify: `backend/src/openapi.rs` (path + schema)
- Create: `backend/tests/serve.rs`
- Modify: `frontend/src/api/schema.d.ts` (régénéré)
- Modify: `openapi.json` (régénéré)

**Interfaces:**
- Produces : `crate::controllers::serve::routes() -> Routes`, handler `pub(crate) async fn public_meta(...)`.
- Consumes : `ProjectsService::get_by_slug` (existant), `dto::to_public_meta` (Task 2).

- [ ] **Step 1: Écrire le test d'intégration (RED)**

Créer `backend/tests/serve.rs` :

```rust
use latch::app::App;
use latch::models::_entities::projects;
use loco_rs::testing::prelude::*;
use sea_orm::{ActiveModelTrait, Set};
use serial_test::serial;

/// Insère un projet et renvoie son modèle.
async fn make_project(
    db: &sea_orm::DatabaseConnection,
    slug: &str,
    code_enabled: bool,
    pin: Option<&str>,
    brand: Option<&str>,
) -> projects::Model {
    projects::ActiveModel {
        slug: Set(slug.to_string()),
        name: Set("Mon Projet".to_string()),
        code_enabled: Set(code_enabled),
        pin: Set(pin.map(str::to_string)),
        brand_name: Set(brand.map(str::to_string)),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("insert project")
}

#[tokio::test]
#[serial]
async fn public_meta_returns_brand_and_code_without_pin() {
    request::<App, _, _>(|request, ctx| async move {
        make_project(&ctx.db, "demo-aaaaaaaa", true, Some("424242"), Some("ACME")).await;
        let res = request.get("/api/public/demo-aaaaaaaa").await;
        res.assert_status_ok();
        let body = res.text();
        assert!(body.contains("ACME"), "brand_name attendu");
        assert!(body.contains("code_enabled"));
        assert!(!body.contains("424242"), "le PIN ne doit JAMAIS fuiter (§9.2)");
        assert!(!body.contains("\"pin\""), "pas de champ pin (§9.2)");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn public_meta_unknown_slug_is_404() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/api/public/nope-xxxxxxxx").await;
        assert_eq!(res.status_code(), 404);
    })
    .await;
}
```

- [ ] **Step 2: Créer le contrôleur avec le handler meta**

Créer `backend/src/controllers/serve.rs` :

```rust
//! Adaptateur entrant "serving client" (`/c/<slug>`) + meta publique. Surface
//! publique (pas de session admin). L'auth = code projet + cookie signé ;
//! la barrière = rate-limit (contrat §6, §9.5). Aucune réponse ne porte le PIN.

use loco_rs::prelude::*;

use crate::controllers::error::into_response;
use crate::services::projects::ProjectsService;

/// GET /api/public/{slug} — meta publique pour la page de déverrouillage.
/// Renvoie `brand_name` + `code_enabled`, jamais le PIN (DTO sans champ pin).
#[utoipa::path(
    get, path = "/api/public/{slug}", tag = "serving",
    params(("slug" = String, Path, description = "Slug public du projet")),
    responses(
        (status = 200, description = "Meta publique (sans PIN)", body = crate::dto::PublicMeta),
        (status = 404, description = "Slug inconnu")
    )
)]
#[debug_handler]
pub(crate) async fn public_meta(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
) -> Result<Response> {
    let svc = ProjectsService::new(ctx.db.clone());
    let project = svc.get_by_slug(&slug).await.map_err(into_response)?;
    format::json(crate::dto::to_public_meta(&project))
}

pub fn routes() -> Routes {
    Routes::new().add("/api/public/{slug}", get(public_meta))
}
```

- [ ] **Step 3: Déclarer + monter**

Dans `backend/src/controllers/mod.rs`, ajouter `pub mod serve;` (ordre alpha, après `pub mod middleware;`).

Dans `backend/src/app.rs`, dans `fn routes`, ajouter après `.add_route(controllers::admin::routes())` :

```rust
            .add_route(controllers::serve::routes())
```

- [ ] **Step 4: Enregistrer dans l'OpenAPI**

Dans `backend/src/openapi.rs` :
- `use crate::controllers::{admin, auth, serve};`
- dans `paths(...)`, ajouter `serve::public_meta,`
- dans `components(schemas(...))`, ajouter `dto::PublicMeta,`
- dans `tags(...)`, ajouter `(name = "serving", description = "Serving client /c et meta publique"),`

- [ ] **Step 5: Lancer le test + régénérer le contrat**

Run:
```bash
cd backend && cargo nextest run --test serve
UPDATE_OPENAPI=1 cargo test --test openapi_drift
cd ../frontend && pnpm gen:api
```
Expected: 2 tests serve PASS ; `openapi.json` mis à jour avec `/api/public/{slug}` ; `schema.d.ts` régénéré (drift remis à zéro).

- [ ] **Step 6: Qualité + commit**

```bash
cd backend && cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo nextest run --test openapi_drift
cd .. && rtk git add backend/src/controllers/serve.rs backend/src/controllers/mod.rs backend/src/app.rs backend/src/openapi.rs backend/tests/serve.rs openapi.json frontend/src/api/schema.d.ts
rtk git commit -m "✨ feat(serve): GET /api/public/{slug} (meta sans PIN) + OpenAPI"
```

---

### Task 5 : `GET /c/{slug}` — arbre de décision (serve proto / page unlock)

**Files:**
- Modify: `backend/src/controllers/serve.rs`
- Modify: `backend/tests/serve.rs`

**Interfaces:**
- Produces : handler `pub(crate) async fn serve(...)`, const `UNLOCK_COOKIE_NAME`.
- Consumes : `web::storage_from_ctx`, `web::unlock_index`, `web::unlock_key`, `services::unlock_cookie::verify_token`, `axum_extra::extract::cookie::SignedCookieJar`.

- [ ] **Step 1: Écrire les tests des branches sans cookie (RED)**

Dans `backend/tests/serve.rs`, ajouter en tête le helper de fake dist + une version déployée, et les tests :

```rust
use latch::models::_entities::versions;

/// Prépare un faux `dist/` avec un unlock.html reconnaissable + pointe LATCH_SPA_DIST.
fn fake_dist() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("unlock.html"),
        "<!doctype html><title>latch-unlock</title>",
    )
    .expect("write unlock.html");
    std::env::set_var("LATCH_SPA_DIST", dir.path());
    dir
}

/// Crée une version + écrit son HTML dans un storage temporaire (LATCH_STORAGE_ROOT),
/// active la version sur le projet. Renvoie le tempdir storage (à garder vivant).
async fn deploy_active(
    db: &sea_orm::DatabaseConnection,
    project: &projects::Model,
    html: &str,
) -> tempfile::TempDir {
    let storage = tempfile::tempdir().expect("storage tempdir");
    std::env::set_var("LATCH_STORAGE_ROOT", storage.path());
    let html_path = format!("{}/1.html", project.id);
    std::fs::create_dir_all(storage.path().join(project.id.to_string())).unwrap();
    std::fs::write(storage.path().join(&html_path), html).unwrap();
    let v = versions::ActiveModel {
        project_id: Set(project.id),
        n: Set(1),
        html_path: Set(html_path),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("insert version");
    let mut p: projects::ActiveModel = project.clone().into();
    p.active_version_id = Set(Some(v.id));
    p.update(db).await.expect("activate");
    storage
}

#[tokio::test]
#[serial]
async fn open_project_serves_active_html_no_store() {
    let _dist = fake_dist();
    request::<App, _, _>(|request, ctx| async move {
        let p = make_project(&ctx.db, "libre-aaaaaaaa", false, None, None).await;
        let _storage = deploy_active(&ctx.db, &p, "<h1>PROTO-LIBRE</h1>").await;
        let res = request.get("/c/libre-aaaaaaaa").await;
        res.assert_status_ok();
        assert!(res.text().contains("PROTO-LIBRE"));
        assert_eq!(
            res.headers().get("cache-control").unwrap(),
            "no-store",
            "tout /c doit être no-store (§6)"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn protected_project_without_cookie_serves_unlock_page() {
    let _dist = fake_dist();
    request::<App, _, _>(|request, ctx| async move {
        let p = make_project(&ctx.db, "prot-aaaaaaaa", true, Some("123456"), None).await;
        let _storage = deploy_active(&ctx.db, &p, "<h1>SECRET</h1>").await;
        let res = request.get("/c/prot-aaaaaaaa").await;
        res.assert_status_ok(); // 200, PAS 401 (contrat §6 / QUIRKS)
        assert!(res.text().contains("latch-unlock"), "rend unlock.html");
        assert!(!res.text().contains("SECRET"), "le proto ne fuit pas sans déverrouillage");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn unknown_slug_is_404() {
    let _dist = fake_dist();
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/c/nope-xxxxxxxx").await;
        assert_eq!(res.status_code(), 404);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn project_without_active_version_is_404() {
    let _dist = fake_dist();
    request::<App, _, _>(|request, ctx| async move {
        make_project(&ctx.db, "vide-aaaaaaaa", false, None, None).await; // aucune version
        let res = request.get("/c/vide-aaaaaaaa").await;
        assert_eq!(res.status_code(), 404);
    })
    .await;
}
```

- [ ] **Step 2: Implémenter le handler `serve`**

Dans `backend/src/controllers/serve.rs`, ajouter les imports en tête :

```rust
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::HeaderMap;
use axum::http::HeaderValue;
use axum::response::IntoResponse;
use axum_extra::extract::cookie::SignedCookieJar;

use crate::services::unlock_cookie::verify_token;
```

Ajouter la constante et le handler :

```rust
/// Nom du cookie de déverrouillage (scopé par `Path=/c/{slug}` → nom constant OK).
pub(crate) const UNLOCK_COOKIE_NAME: &str = "latch_unlock";

/// Construit la réponse HTML brute du proto actif, `no-store`.
fn html_response(html: String) -> Response {
    (
        [
            (CACHE_CONTROL, HeaderValue::from_static("no-store")),
            (CONTENT_TYPE, HeaderValue::from_static("text/html; charset=utf-8")),
        ],
        html,
    )
        .into_response()
}

/// Rend la page de déverrouillage (`unlock.html` buildé), HTTP 200, `no-store`.
async fn unlock_page_response() -> Result<Response> {
    let path = crate::web::unlock_index();
    let html = tokio::fs::read_to_string(&path).await.map_err(|e| {
        loco_rs::Error::Message(format!("unlock.html introuvable ({}): {e}", path.display()))
    })?;
    Ok(html_response(html))
}

/// GET /c/{slug} — décision serveur (cf. spec §2 / contrat §6).
#[debug_handler]
pub(crate) async fn serve(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Result<Response> {
    let svc = ProjectsService::new(ctx.db.clone());
    // Slug inconnu → 404 (NotFound mappé par into_response).
    let project = svc.get_by_slug(&slug).await.map_err(into_response)?;

    // Pas de version active → rien à servir.
    let Some(active_id) = project.active_version_id else {
        return Err(loco_rs::Error::NotFound);
    };

    // Projet protégé sans cookie valide → page de déverrouillage (avant de lire le HTML).
    if project.code_enabled {
        let pin = project.pin.clone().unwrap_or_default();
        let key = crate::web::unlock_key()?;
        let jar = SignedCookieJar::from_headers(&headers, key);
        let now = chrono::Utc::now().timestamp();
        let ok = jar
            .get(UNLOCK_COOKIE_NAME)
            .map(|c| verify_token(crate::web::unlock_secret()?.as_bytes(), &slug, &pin, c.value(), now))
            .unwrap_or(false);
        if !ok {
            return unlock_page_response().await;
        }
    }

    // Libre, ou protégé + cookie valide → servir le HTML de la version active.
    use crate::models::_entities::versions;
    let version = versions::Entity::find_by_id(active_id)
        .one(&ctx.db)
        .await
        .map_err(|e| into_response(e.into()))?
        .ok_or(loco_rs::Error::NotFound)?;
    let storage = crate::web::storage_from_ctx(&ctx);
    let html = storage.read(&version.html_path).await.map_err(into_response)?;
    Ok(html_response(html))
}
```

> Note : `jar.get(...).map(|c| verify_token(... web::unlock_secret()?...))` — le `?` à
> l'intérieur de la closure ne compile pas. Écrire explicitement :
> ```rust
> let secret = crate::web::unlock_secret()?;
> let ok = match jar.get(UNLOCK_COOKIE_NAME) {
>     Some(c) => verify_token(secret.as_bytes(), &slug, &pin, c.value(), now),
>     None => false,
> };
> ```

Remplacer le bloc `let ok = ...` par cette forme explicite.

Ajouter la route dans `routes()` :

```rust
pub fn routes() -> Routes {
    Routes::new()
        .add("/api/public/{slug}", get(public_meta))
        .add("/c/{slug}", get(serve))
}
```

Ajouter en tête du fichier l'import du finder versions si besoin (déjà importé inline via `use` dans le handler).

- [ ] **Step 3: Lancer les tests**

Run: `cd backend && cargo nextest run --test serve`
Expected: les 4 nouveaux tests + les 2 de Task 4 PASS (6 total). `#[serial]` requis (env partagé LATCH_SPA_DIST/STORAGE_ROOT).

- [ ] **Step 4: Qualité + commit**

```bash
cd backend && cargo fmt --all && cargo clippy --all-targets -- -D warnings
cd .. && rtk git add backend/src/controllers/serve.rs backend/tests/serve.rs
rtk git commit -m "✨ feat(serve): GET /c/{slug} — proto actif no-store ou page de déverrouillage"
```

---

### Task 6 : `POST /c/{slug}/unlock` — vérif PIN + cookie signé

**Files:**
- Modify: `backend/src/controllers/serve.rs`
- Modify: `backend/tests/serve.rs`

**Interfaces:**
- Produces : handler `pub(crate) async fn unlock(...)`.
- Consumes : `ProjectsService::verify_code`, `unlock_cookie::issue_token`, `web::{unlock_key, unlock_secret, cookie_secure}`, `axum_extra::extract::cookie::{Cookie, SameSite}`, `dto::UnlockReq`.

- [ ] **Step 1: Écrire les tests (RED)**

Dans `backend/tests/serve.rs`, ajouter :

```rust
#[tokio::test]
#[serial]
async fn unlock_wrong_pin_is_401_no_cookie() {
    let _dist = fake_dist();
    request::<App, _, _>(|request, ctx| async move {
        let p = make_project(&ctx.db, "prot-bbbbbbbb", true, Some("123456"), None).await;
        let _storage = deploy_active(&ctx.db, &p, "<h1>SECRET</h1>").await;
        let res = request
            .post("/c/prot-bbbbbbbb/unlock")
            .json(&serde_json::json!({ "pin": "000000" }))
            .await;
        assert_eq!(res.status_code(), 401);
        assert!(res.headers().get("set-cookie").is_none(), "pas de cookie sur échec");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn unlock_good_pin_sets_cookie_then_serves_proto() {
    let _dist = fake_dist();
    let config = RequestConfigBuilder::new().save_cookies(true).build();
    request_with_config::<App, _, _>(config, |request, ctx| async move {
        let p = make_project(&ctx.db, "prot-cccccccc", true, Some("123456"), None).await;
        let _storage = deploy_active(&ctx.db, &p, "<h1>SECRET-OK</h1>").await;

        let unlocked = request
            .post("/c/prot-cccccccc/unlock")
            .json(&serde_json::json!({ "pin": "123456" }))
            .await;
        assert_eq!(unlocked.status_code(), 204);
        assert!(unlocked.headers().get("set-cookie").is_some(), "cookie posé");

        // save_cookies(true) renvoie le cookie → le GET sert maintenant le proto.
        let served = request.get("/c/prot-cccccccc").await;
        served.assert_status_ok();
        assert!(served.text().contains("SECRET-OK"));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn rotating_pin_invalidates_cookie() {
    let _dist = fake_dist();
    let config = RequestConfigBuilder::new().save_cookies(true).build();
    request_with_config::<App, _, _>(config, |request, ctx| async move {
        let p = make_project(&ctx.db, "prot-dddddddd", true, Some("123456"), None).await;
        let _storage = deploy_active(&ctx.db, &p, "<h1>SECRET-ROT</h1>").await;

        // Déverrouille → cookie valide → proto servi.
        request
            .post("/c/prot-dddddddd/unlock")
            .json(&serde_json::json!({ "pin": "123456" }))
            .await;
        assert!(request.get("/c/prot-dddddddd").await.text().contains("SECRET-ROT"));

        // Rotation du PIN (set_code) → le cookie émis sous l'ancien PIN doit être rejeté.
        latch::services::projects::ProjectsService::new(ctx.db.clone())
            .set_code(p.id, "654321")
            .await
            .unwrap();
        let after = request.get("/c/prot-dddddddd").await;
        after.assert_status_ok();
        assert!(after.text().contains("latch-unlock"), "rotation → re-déverrouillage exigé (§6)");
        assert!(!after.text().contains("SECRET-ROT"));
    })
    .await;
}
```

- [ ] **Step 2: Implémenter le handler `unlock`**

Dans `backend/src/controllers/serve.rs`, étendre les imports cookie :

```rust
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum_extra::extract::cookie::{Cookie, SameSite, SignedCookieJar};

use crate::dto::UnlockReq;
use crate::services::unlock_cookie::issue_token;
```

Ajouter le handler :

```rust
/// Durée de vie du cookie unlock (jours). Configurable via `LATCH_UNLOCK_TTL_DAYS`.
fn unlock_ttl_days() -> i64 {
    std::env::var("LATCH_UNLOCK_TTL_DAYS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30)
}

/// POST /c/{slug}/unlock — vérifie le PIN (temps constant), pose le cookie signé.
/// Surface publique : pas de garde Origin (le PIN + le rate-limit sont la barrière).
#[debug_handler]
pub(crate) async fn unlock(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    headers: HeaderMap,
    Json(body): Json<UnlockReq>,
) -> Result<Response> {
    let svc = ProjectsService::new(ctx.db.clone());
    // Slug inconnu → 404 ; PIN faux → 401.
    let ok = svc.verify_code(&slug, &body.pin).await.map_err(into_response)?;
    if !ok {
        return Err(loco_rs::Error::Unauthorized("bad code".to_string()));
    }

    // PIN correct (ou projet libre) → poser le cookie signé liant le PIN courant.
    let secret = crate::web::unlock_secret()?;
    let ttl = unlock_ttl_days();
    let exp = chrono::Utc::now().timestamp() + ttl * 86_400;
    let token = issue_token(secret.as_bytes(), &slug, &body.pin, exp);

    let cookie = Cookie::build((UNLOCK_COOKIE_NAME, token))
        .path(format!("/c/{slug}"))
        .http_only(true)
        .secure(crate::web::cookie_secure(&ctx))
        .same_site(SameSite::Lax)
        .max_age(time::Duration::days(ttl))
        .build();

    let key = crate::web::unlock_key()?;
    let jar = SignedCookieJar::from_headers(&headers, key).add(cookie);
    Ok((jar, StatusCode::NO_CONTENT).into_response())
}
```

> `loco_rs::Error::Unauthorized` mappe sur **401** (confirmé QUIRKS) — c'est le code voulu pour un PIN faux.

Câbler la route (la garde Origin n'est PAS posée — surface publique) :

```rust
pub fn routes() -> Routes {
    Routes::new()
        .add("/api/public/{slug}", get(public_meta))
        .add("/c/{slug}", get(serve))
        .add("/c/{slug}/unlock", post(unlock))
}
```

- [ ] **Step 3: Lancer les tests**

Run: `cd backend && cargo nextest run --test serve`
Expected: 9 tests PASS (dont rotation + roundtrip cookie).

- [ ] **Step 4: Qualité + invariants + commit**

```bash
cd backend && cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo nextest run --test security_invariants
cd .. && rtk git add backend/src/controllers/serve.rs backend/tests/serve.rs
rtk git commit -m "✨ feat(serve): POST /c/{slug}/unlock — cookie signé liant le PIN (révocation par rotation §6)"
```

---

### Task 7 : Rate-limit `governor` sur `/unlock` (IP+slug & slug global)

**Files:**
- Create: `backend/src/controllers/serve_ratelimit.rs`
- Modify: `backend/src/controllers/serve.rs` (poser les layers)
- Modify: `backend/tests/serve.rs`

**Interfaces:**
- Produces : `IpSlugKeyExtractor`, `SlugKeyExtractor` (impl `tower_governor::key_extractor::KeyExtractor`), `fn slug_from_path(path: &str) -> Option<String>`.

> **Vérification d'API au démarrage de la tâche** : confirmer via Context7 (`tower_governor` 0.7)
> la signature exacte de `KeyExtractor` (`fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError>`)
> et le variant `GovernorError::UnableToExtractKey`. Ajuster les `use` si la version diffère.

- [ ] **Step 1: Écrire le test du parseur de slug + le test de rate-limit (RED)**

Dans `backend/tests/serve.rs`, ajouter :

```rust
#[tokio::test]
#[serial]
async fn unlock_rate_limited_after_burst() {
    let _dist = fake_dist();
    request::<App, _, _>(|request, ctx| async move {
        let p = make_project(&ctx.db, "prot-eeeeeeee", true, Some("123456"), None).await;
        let _storage = deploy_active(&ctx.db, &p, "<h1>x</h1>").await;
        // Burst IP+slug = 5 ; au-delà → 429. Clé IP fixée via X-Forwarded-For.
        let mut got_429 = false;
        for _ in 0..12 {
            let res = request
                .post("/c/prot-eeeeeeee/unlock")
                .add_header(
                    axum::http::HeaderName::from_static("x-forwarded-for"),
                    axum::http::HeaderValue::from_static("9.9.9.9"),
                )
                .json(&serde_json::json!({ "pin": "000000" }))
                .await;
            if res.status_code() == 429 {
                got_429 = true;
                break;
            }
        }
        assert!(got_429, "le burst dépassé doit déclencher un 429 (§9.5)");
    })
    .await;
}
```

Le parseur `slug_from_path` est testé unitairement dans le module (Step 2).

- [ ] **Step 2: Écrire les extracteurs de clé**

Créer `backend/src/controllers/serve_ratelimit.rs` :

```rust
//! Extracteurs de clé `governor` pour `POST /c/{slug}/unlock` (contrat §9.5).
//! Deux couches in-memory : `IP+slug` (backoff par client) et `slug` seul
//! (plafond global, rattrape la rotation d'IP). Compteurs en RAM (reset au reboot).

use axum::http::Request;
use tower_governor::key_extractor::{KeyExtractor, SmartIpKeyExtractor};
use tower_governor::GovernorError;

/// Extrait le slug du chemin `/c/{slug}/unlock`.
pub(crate) fn slug_from_path(path: &str) -> Option<String> {
    let mut segs = path.split('/').filter(|s| !s.is_empty());
    match (segs.next(), segs.next()) {
        (Some("c"), Some(slug)) => Some(slug.to_string()),
        _ => None,
    }
}

/// Clé = `IP|slug` (backoff par client sur un projet donné).
#[derive(Clone)]
pub struct IpSlugKeyExtractor;

impl KeyExtractor for IpSlugKeyExtractor {
    type Key = String;

    fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError> {
        let ip = SmartIpKeyExtractor.extract(req)?;
        let slug = slug_from_path(req.uri().path()).ok_or(GovernorError::UnableToExtractKey)?;
        Ok(format!("{ip}|{slug}"))
    }
}

/// Clé = `slug` seul (plafond global par projet).
#[derive(Clone)]
pub struct SlugKeyExtractor;

impl KeyExtractor for SlugKeyExtractor {
    type Key = String;

    fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError> {
        slug_from_path(req.uri().path()).ok_or(GovernorError::UnableToExtractKey)
    }
}

#[cfg(test)]
mod tests {
    use super::slug_from_path;

    #[test]
    fn extracts_slug_from_unlock_path() {
        assert_eq!(slug_from_path("/c/demo-abc/unlock").as_deref(), Some("demo-abc"));
        assert_eq!(slug_from_path("/c/demo-abc").as_deref(), Some("demo-abc"));
    }

    #[test]
    fn rejects_non_c_paths() {
        assert_eq!(slug_from_path("/api/public/demo"), None);
        assert_eq!(slug_from_path("/"), None);
    }
}
```

Déclarer le module dans `backend/src/controllers/mod.rs` : `pub mod serve_ratelimit;`.

- [ ] **Step 3: Poser les deux layers sur la route unlock**

Dans `backend/src/controllers/serve.rs`, ajouter les imports :

```rust
use std::sync::Arc;
use std::time::Duration;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};

use crate::controllers::serve_ratelimit::{IpSlugKeyExtractor, SlugKeyExtractor};
```

Remplacer `routes()` par :

```rust
pub fn routes() -> Routes {
    // Burst & période réglables par env (défauts : IP+slug 5/1s, slug global 20/3s).
    let ip_burst: u32 = env_u32("LATCH_UNLOCK_RL_IP_BURST", 5);
    let ip_per_sec: u64 = env_u64("LATCH_UNLOCK_RL_IP_PER_SECOND", 1);
    let slug_burst: u32 = env_u32("LATCH_UNLOCK_RL_SLUG_BURST", 20);
    let slug_period: u64 = env_u64("LATCH_UNLOCK_RL_SLUG_PERIOD_SECS", 3);

    let ip_layer = {
        let config = Arc::new(
            GovernorConfigBuilder::default()
                .per_second(ip_per_sec)
                .burst_size(ip_burst)
                .key_extractor(IpSlugKeyExtractor)
                .finish()
                .expect("governor IP+slug config valide"),
        );
        GovernorLayer { config }
    };
    let slug_layer = {
        let config = Arc::new(
            GovernorConfigBuilder::default()
                .period(Duration::from_secs(slug_period))
                .burst_size(slug_burst)
                .key_extractor(SlugKeyExtractor)
                .finish()
                .expect("governor slug config valide"),
        );
        GovernorLayer { config }
    };

    Routes::new()
        .add("/api/public/{slug}", get(public_meta))
        .add("/c/{slug}", get(serve))
        .add(
            "/c/{slug}/unlock",
            post(unlock).layer(ip_layer).layer(slug_layer),
        )
}

fn env_u32(name: &str, default: u32) -> u32 {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}
fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}
```

- [ ] **Step 4: Lancer les tests**

Run: `cd backend && cargo nextest run --test serve && cargo nextest run -E 'test(serve_ratelimit)'`
Expected: le test `unlock_rate_limited_after_burst` PASS + les 2 tests unitaires `slug_from_path` PASS.

> Si `GovernorError::UnableToExtractKey` ou la signature de `extract` diffère en
> `tower_governor` 0.7, corriger d'après la vérification Context7 du début de tâche.

- [ ] **Step 5: Qualité + commit**

```bash
cd backend && cargo fmt --all && cargo clippy --all-targets -- -D warnings
cd .. && rtk git add backend/src/controllers/serve_ratelimit.rs backend/src/controllers/serve.rs backend/src/controllers/mod.rs backend/tests/serve.rs
rtk git commit -m "✨ feat(serve): rate-limit governor /unlock (IP+slug + plafond global slug)"
```

---

### Task 8 : Frontend — entrée Vite dédiée `unlock` (React + shadcn)

**Files:**
- Modify: `frontend/vite.config.ts` (input multi-page)
- Create: `frontend/unlock.html`
- Create: `frontend/src/unlock/main.tsx`
- Create: `frontend/src/unlock/i18n.ts`
- Create: `frontend/src/unlock/reload.ts`
- Create: `frontend/src/unlock/unlock-page.tsx`
- Create: `frontend/src/unlock/unlock-page.test.tsx`

**Interfaces:**
- Produces : page autonome montée sur `#unlock-root`, lisant le slug depuis `window.location.pathname`, appelant `GET /api/public/<slug>` puis `POST /c/<slug>/unlock`.
- Consumes : `@/components/ui/{card,input,button,label}`, `@/index.css` (thème partagé).

- [ ] **Step 1: Déclarer la 2ᵉ entrée Vite**

Dans `frontend/vite.config.ts`, remplacer le bloc `build` :

```ts
import { fileURLToPath } from 'node:url'
// ...
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    rollupOptions: {
      input: {
        main: fileURLToPath(new URL('./index.html', import.meta.url)),
        unlock: fileURLToPath(new URL('./unlock.html', import.meta.url)),
      },
    },
  },
```

- [ ] **Step 2: Créer l'entrée HTML**

Créer `frontend/unlock.html` :

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <meta name="robots" content="noindex, nofollow" />
    <title>latch</title>
  </head>
  <body>
    <div id="unlock-root"></div>
    <script type="module" src="/src/unlock/main.tsx"></script>
  </body>
</html>
```

- [ ] **Step 3: i18n minimal + helper reload**

Créer `frontend/src/unlock/i18n.ts` :

```ts
import i18next from 'i18next'
import { initReactI18next } from 'react-i18next'
import LanguageDetector from 'i18next-browser-languagedetector'

const en = {
  'unlock.title_brand': 'Prototype prepared for {{brand}}',
  'unlock.title_neutral': 'Protected prototype',
  'unlock.pin_label': 'Access code',
  'unlock.submit': 'Unlock',
  'unlock.error_wrong': 'Incorrect code.',
  'unlock.error_throttled': 'Too many attempts. Please try again in a moment.',
  'unlock.error_generic': 'Something went wrong. Please try again.',
}
const fr = {
  'unlock.title_brand': 'Prototype préparé pour {{brand}}',
  'unlock.title_neutral': 'Prototype protégé',
  'unlock.pin_label': "Code d'accès",
  'unlock.submit': 'Déverrouiller',
  'unlock.error_wrong': 'Code incorrect.',
  'unlock.error_throttled': 'Trop de tentatives. Réessaie dans un moment.',
  'unlock.error_generic': "Une erreur s'est produite. Réessaie.",
}

const instance = i18next.createInstance()
instance
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources: { en: { translation: en }, fr: { translation: fr } },
    fallbackLng: 'en',
    supportedLngs: ['en', 'fr'],
    keySeparator: false,
    nsSeparator: false,
    interpolation: { escapeValue: false },
    detection: { order: ['localStorage', 'navigator'], lookupLocalStorage: 'latch.locale' },
  })

export default instance
```

Créer `frontend/src/unlock/reload.ts` (indirection testable) :

```ts
export const reloadPage = () => window.location.reload()
```

- [ ] **Step 4: La page (RED — test d'abord)**

Créer `frontend/src/unlock/unlock-page.test.tsx` :

```tsx
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { I18nextProvider } from 'react-i18next'
import { http, HttpResponse } from 'msw'
import { server } from '@/test/msw'
import i18n from './i18n'
import { UnlockPage } from './unlock-page'

vi.mock('./reload', () => ({ reloadPage: vi.fn() }))
import { reloadPage } from './reload'

function renderUnlock() {
  return render(
    <I18nextProvider i18n={i18n}>
      <UnlockPage />
    </I18nextProvider>,
  )
}

beforeEach(() => {
  window.history.replaceState({}, '', '/c/demo-abc')
  vi.mocked(reloadPage).mockClear()
})

describe('UnlockPage', () => {
  it('affiche le brand_name récupéré', async () => {
    server.use(
      http.get('*/api/public/demo-abc', () =>
        HttpResponse.json({ brand_name: 'ACME', code_enabled: true }),
      ),
    )
    renderUnlock()
    await waitFor(() => expect(screen.getByText(/ACME/)).toBeInTheDocument())
  })

  it('recharge la page sur PIN correct (204)', async () => {
    server.use(
      http.get('*/api/public/demo-abc', () =>
        HttpResponse.json({ brand_name: null, code_enabled: true }),
      ),
      http.post('*/c/demo-abc/unlock', () => new HttpResponse(null, { status: 204 })),
    )
    renderUnlock()
    await userEvent.type(screen.getByLabelText(/access code|code/i), '123456')
    await userEvent.click(screen.getByRole('button', { name: /unlock|déverrouiller/i }))
    await waitFor(() => expect(reloadPage).toHaveBeenCalledOnce())
  })

  it('affiche une erreur sur PIN faux (401)', async () => {
    server.use(
      http.get('*/api/public/demo-abc', () =>
        HttpResponse.json({ brand_name: null, code_enabled: true }),
      ),
      http.post('*/c/demo-abc/unlock', () => new HttpResponse(null, { status: 401 })),
    )
    renderUnlock()
    await userEvent.type(screen.getByLabelText(/access code|code/i), '000000')
    await userEvent.click(screen.getByRole('button', { name: /unlock|déverrouiller/i }))
    await waitFor(() => expect(screen.getByText(/incorrect/i)).toBeInTheDocument())
    expect(reloadPage).not.toHaveBeenCalled()
  })

  it('affiche un message de throttle sur 429', async () => {
    server.use(
      http.get('*/api/public/demo-abc', () =>
        HttpResponse.json({ brand_name: null, code_enabled: true }),
      ),
      http.post('*/c/demo-abc/unlock', () => new HttpResponse(null, { status: 429 })),
    )
    renderUnlock()
    await userEvent.type(screen.getByLabelText(/access code|code/i), '111111')
    await userEvent.click(screen.getByRole('button', { name: /unlock|déverrouiller/i }))
    await waitFor(() =>
      expect(screen.getByText(/too many attempts|trop de tentatives/i)).toBeInTheDocument(),
    )
  })
})
```

- [ ] **Step 5: Implémenter la page**

Créer `frontend/src/unlock/unlock-page.tsx` :

```tsx
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { Label } from '@/components/ui/label'
import { reloadPage } from './reload'

function slugFromPath(): string {
  // /c/<slug> → segment d'indice 1
  return window.location.pathname.split('/').filter(Boolean)[1] ?? ''
}

export function UnlockPage() {
  const { t } = useTranslation()
  const slug = slugFromPath()
  const [brand, setBrand] = useState<string | null>(null)
  const [pin, setPin] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  useEffect(() => {
    fetch(`/api/public/${slug}`)
      .then((r) => (r.ok ? r.json() : null))
      .then((meta) => meta && setBrand(meta.brand_name ?? null))
      .catch(() => {})
  }, [slug])

  async function submit(e: React.FormEvent) {
    e.preventDefault()
    setError(null)
    setBusy(true)
    try {
      const res = await fetch(`/c/${slug}/unlock`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ pin }),
      })
      if (res.status === 204) {
        reloadPage()
        return
      }
      if (res.status === 429) setError(t('unlock.error_throttled'))
      else if (res.status === 401) setError(t('unlock.error_wrong'))
      else setError(t('unlock.error_generic'))
    } catch {
      setError(t('unlock.error_generic'))
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="flex min-h-svh items-center justify-center bg-background p-4">
      <Card className="w-full max-w-sm">
        <CardHeader>
          <CardTitle>
            {brand ? t('unlock.title_brand', { brand }) : t('unlock.title_neutral')}
          </CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={submit} className="flex flex-col gap-4">
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="pin">{t('unlock.pin_label')}</Label>
              <Input
                id="pin"
                inputMode="numeric"
                autoComplete="off"
                autoFocus
                maxLength={6}
                value={pin}
                onChange={(e) => setPin(e.target.value.replace(/\D/g, ''))}
                aria-invalid={error ? true : undefined}
              />
            </div>
            {error && (
              <p role="alert" className="text-sm text-destructive">
                {error}
              </p>
            )}
            <Button type="submit" disabled={busy || pin.length === 0}>
              {t('unlock.submit')}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  )
}
```

Créer `frontend/src/unlock/main.tsx` :

```tsx
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { I18nextProvider } from 'react-i18next'
import i18n from './i18n'
import { UnlockPage } from './unlock-page'
import '@/index.css'

createRoot(document.getElementById('unlock-root')!).render(
  <StrictMode>
    <I18nextProvider i18n={i18n}>
      <UnlockPage />
    </I18nextProvider>
  </StrictMode>,
)
```

- [ ] **Step 6: Lancer tests + typecheck + lint + build**

Run:
```bash
cd frontend && pnpm test -- src/unlock && pnpm typecheck && pnpm lint && pnpm build
```
Expected: 4 tests unlock PASS ; typecheck/lint propres ; `pnpm build` produit `dist/unlock.html` **et** `dist/index.html` (vérifier : `ls dist/*.html`).

- [ ] **Step 7: Commit**

```bash
cd .. && rtk git add frontend/vite.config.ts frontend/unlock.html frontend/src/unlock/
rtk git commit -m "✨ feat(front): page de déverrouillage /c (entrée Vite dédiée, React+shadcn)"
```

---

### Task 9 : Vérif e2e manuelle (navigateur) du flux serving

**Files:** aucun (vérification fonctionnelle réelle, cf. QUIRKS « toujours valider au navigateur »)

- [ ] **Step 1: Build front + lancer le backend en dev**

```bash
cd frontend && pnpm build
cd ../backend && LATCH_SPA_DIST=../frontend/dist ADMIN_USER=admin ADMIN_PASS=secret \
  UNLOCK_COOKIE_SECRET=dev-only-insecure-unlock-cookie-secret-please-override-in-production!! \
  LATCH_STORAGE_ROOT=/tmp/latch-dev-data DATABASE_URL='sqlite:///tmp/latch-dev.sqlite?mode=rwc' \
  cargo loco start
```

- [ ] **Step 2: Parcours (admin pour préparer, puis /c)**

Via l'admin (`/admin`, login admin/secret) : créer un projet **protégé** (PIN), déployer un HTML, l'activer. Noter le slug + PIN.

Vérifier au navigateur :
- `GET /c/<slug>` (sans cookie) → **page de déverrouillage** stylée (shadcn), 200, titre avec `brand_name` si défini.
- PIN faux → message « code incorrect », pas de redirection.
- PIN correct → la page se recharge et **sert le proto**.
- Recharger `/c/<slug>` → le proto est servi directement (cookie présent).
- Faire un 2ᵉ projet **libre** (sans code) → `GET /c/<slug>` sert directement le proto.
- DevTools → Network : `Cache-Control: no-store` sur toutes les réponses `/c`.

- [ ] **Step 3: Consigner le résultat** (capture éventuelle) — pas de commit (vérification seule).

---

### Task 10 : Config, docs mémoire, clôture de phase

**Files:**
- Modify: `.env.example`
- Modify: `docs/ENVIRONMENT.md`
- Modify: `docs/QUIRKS.md`
- Modify: `docs/INDEX.md`
- Modify: `docs/HANDOFF.md`
- Modify: `docs/BACKLOG.md`
- Modify: `docs/ROADMAP.md`

- [ ] **Step 1: `.env.example`**

Corriger la ligne `UNLOCK_COOKIE_SECRET` (le jar exige ≥ 64 bytes) et ajouter les knobs :

```bash
# --- Cookie de déverrouillage client : clé HMAC de signature (≥ 64 octets, comme SESSION_SECRET) ---
UNLOCK_COOKIE_SECRET=change-me-64-bytes-min-random-0123456789abcdef0123456789abcdef
# Durée de vie du cookie unlock (jours). Défaut 30.
LATCH_UNLOCK_TTL_DAYS=30
# Rate-limit /unlock (gouvernor in-memory). Défauts : IP+slug 5 req / 1s ; slug global 20 / 3s.
LATCH_UNLOCK_RL_IP_BURST=5
LATCH_UNLOCK_RL_IP_PER_SECOND=1
LATCH_UNLOCK_RL_SLUG_BURST=20
LATCH_UNLOCK_RL_SLUG_PERIOD_SECS=3
```

- [ ] **Step 2: `docs/ENVIRONMENT.md`** — table des env : `UNLOCK_COOKIE_SECRET` (≥ 64 bytes, fallback dev), `LATCH_UNLOCK_TTL_DAYS`, `LATCH_UNLOCK_RL_*`. Mentionner que `unlock.html` est servi depuis `LATCH_SPA_DIST` (même racine que la SPA).

- [ ] **Step 3: `docs/QUIRKS.md`** — ajouter les entrées :
  - Rate-limit `/unlock` **in-memory** (compteurs perdus au reboot — limite assumée §9.5).
  - Cookie unlock = `SignedCookieJar` (intégrité) **+ empreinte HMAC du PIN** dans la valeur (révocation par rotation). `Key` exige ≥ 64 bytes.
  - 2ᵉ entrée Vite : `unlock.html` tire ses assets depuis `/admin/assets/*` (base Vite `/admin/`) — fonctionnel car le `ServeDir` admin sert le dist complet, assets publics sans auth.
  - Page de déverrouillage = **HTTP 200** (pas 401, cf. contrat §6).

- [ ] **Step 4: `docs/INDEX.md`** — section « Backend » : lignes Phase 4 (`controllers/serve.rs` : GET /c, POST /unlock, GET /api/public ; `services/unlock_cookie.rs` ; rate-limit). Section « Frontend React » : entrée unlock. Cocher Phase 4 dans « Phases closes » (`- [x] Phase 4 — serving /c/<slug>`).

- [ ] **Step 5: `docs/BACKLOG.md`** — consigner les écartés (table `unlock_attempts` + stat admin + scheduler/purge + backoff durable au reboot) comme choix de périmètre.

- [ ] **Step 6: `docs/ROADMAP.md`** — marquer Phase 4 ✅ LIVRÉE (date, renvoi spec/plan).

- [ ] **Step 7: `docs/HANDOFF.md`** — entrée datée en haut : Dernière chose faite / Trucs en suspens (e2e complet Playwright = Phase 6) / Prochaine chose (Phase 5 MCP) / Notes pour future Claude (cookie unlock = jar+empreinte ; rate-limit in-memory).

- [ ] **Step 8: Vérification finale complète**

Run:
```bash
cd backend && cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo nextest run && cargo deny check
cd ../frontend && pnpm lint && pnpm typecheck && pnpm test && pnpm build
```
Expected: tout vert. `cargo deny` : licences des nouvelles deps (`axum-extra`, `hmac`, `sha2`, `hex`) MIT/Apache — si une licence inattendue apparaît, l'ajouter à `deny.toml` (cf. QUIRKS cargo-deny).

- [ ] **Step 9: Commit de clôture**

```bash
cd .. && rtk git add .env.example docs/
rtk git commit -m "📝 docs(phase-4): config unlock + mémoire (ENV, QUIRKS, INDEX, ROADMAP, HANDOFF, BACKLOG)"
```

---

## Self-Review

**1. Spec coverage** (chaque section de la spec → tâche) :
- §1 routes (GET /c, POST /unlock, GET /api/public) → Tasks 4, 5, 6. ✓
- §2 arbre de décision GET → Task 5. ✓
- §3 entrée Vite React+shadcn + flux fetch/meta/reload → Task 8. ✓
- §4 PublicMeta + OpenAPI → Tasks 2, 4. ✓
- §5 unlock + cookie SignedCookieJar + empreinte PIN + 204 → Tasks 1, 6. ✓
- §6 rate-limit deux clés governor in-memory → Task 7. ✓
- §7 cœur `unlock_cookie` pur → Task 1 ; garde d'archi vérifiée Task 1 Step 5. ✓
- §8 headers no-store + pas de garde Origin → Tasks 5, 6. ✓
- §9 écartés → BACKLOG Task 10. ✓
- §10 config env → Tasks 3 (secret), 6 (TTL), 7 (RL), 10 (.env.example). ✓
- §11 critères de sortie (unit cœur, intégration, front Vitest, qualité) → Tasks 1, 4-8, 10. ✓

**2. Placeholder scan** : aucun « TBD/TODO ». Deux notes de vérification d'API explicites (axum-extra cookie `from_headers`/`Cookie::build` implicites ; `tower_governor` KeyExtractor en Task 7 Step intro) — ce sont des étapes concrètes (confirmer une signature), pas des trous de code.

**3. Type consistency** : `issue_token`/`verify_token` (Task 1) ↔ utilisés Tasks 5/6 avec les mêmes signatures (`&[u8]`, `&str`, `i64`). `PublicMeta`/`UnlockReq`/`to_public_meta` (Task 2) ↔ Tasks 4/6. `UNLOCK_COOKIE_NAME` défini Task 5, réutilisé Task 6. `web::{unlock_secret,unlock_key,cookie_secure,unlock_index}` (Task 3) ↔ Tasks 5/6. `slug_from_path`/extracteurs (Task 7). Cohérent.

> **Point d'attention pour l'implémenteur (Task 5)** : le `?` ne peut pas vivre dans la
> closure `.map(|c| ... unlock_secret()? ...)`. Utiliser la forme `match jar.get(...)` donnée
> dans la note de la Task 5 Step 2.
