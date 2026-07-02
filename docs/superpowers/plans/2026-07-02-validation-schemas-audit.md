# Consolidation de la validation des entrées (#23) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Faire de chaque DTO d'entrée (web + MCP) une source de vérité de validation, déclarative et centralisée, invoquée à la frontière, avec un invariant testé.

**Architecture:** Crate `validator` + `#[derive(Validate)]` sur les DTOs. Les bornes/règles vivent dans un module central `services/validation.rs` (fonctions `custom`). `.validate()` est invoqué à deux frontières : un extracteur axum `ValidatedJson<T>` (web) et un appel explicite `args.validate()` en tête de chaque tool MCP. La validation *de forme* migre des services vers les DTOs ; les invariants *métier* restent au cœur.

**Tech Stack:** Rust, axum 0.8.9, rmcp 1.8.0, `validator` 0.20.0 (déjà dans le lockfile), schemars 1.2.1, Loco. Front : React/Vite (attribut `maxLength` uniquement).

## Global Constraints

- Versions épinglées (ne pas bumper) : `axum = 0.8.9`, `rmcp = 1.8.0`, `validator = 0.20.0`, `schemars = 1.2.1`.
- `validator` : ajouter en dépendance DIRECTE dans `backend/Cargo.toml` en `0.20` (déjà transitif → pas de churn lockfile).
- Comptage de longueur = **caractères Unicode** (`.chars().count()`), cohérent avec #13. Tailles `html`/`anchor` = **octets** (`.len()`).
- `deploy_token` : JAMAIS de validation de forme (secret, `secure_compare`).
- `token validé EN PREMIER` côté MCP (§9.3) : `check_token()` AVANT `args.validate()`.
- Confidentialité : aucun nom de client réel dans le code/tests/fixtures — placeholders (`Mon Projet`, `ACME`).
- « Terminé » = `cargo fmt` + `cargo clippy -D warnings` + `cargo nextest run` verts + `pnpm lint`/`typecheck`/`test` verts + doc à jour (contrat, `.env.example`, fumadocs, `ENVIRONMENT.md`) + HANDOFF/INDEX.
- Bornes const (registre) : `MAX_NAME_LEN=128`, `MAX_BODY_LEN=2000`, `MAX_AUTHOR_NAME_LEN=80`, `MAX_RELEASE_NOTES_LEN=10_000`. Env : `LATCH_MAX_HTML_BYTES` (défaut `5_242_880`), `LATCH_MAX_ANCHOR_BYTES` (défaut `8_192`).

---

## File Structure

- **Create** `backend/src/services/validation.rs` — registre central : consts + fonctions `custom` (`validate_name`, `validate_optional_name`, `validate_opt_opt_brand`, `validate_body`, `validate_author_name`, `validate_optional_release_notes`, `validate_pin`, `validate_html`, `validate_anchor`) + lecture env (`max_html_bytes`, `max_anchor_bytes`).
- **Create** `backend/src/web/extract.rs` — extracteur `ValidatedJson<T>` (`FromRequest`).
- **Modify** `backend/src/services/mod.rs` — `pub mod validation;`.
- **Modify** `backend/src/web/mod.rs` — `pub mod extract;` (+ re-export).
- **Modify** `backend/src/dto/mod.rs` — `#[derive(Validate)]` + `#[validate(...)]` sur tous les `*Req` + schema cross-field `CreateProjectReq`.
- **Modify** `backend/src/mcp/mod.rs` — `Validate` sur `DeployArgs`/`ListArgs`/`PullArgs` + `map_validation_err` + `args.validate()` dans chaque tool.
- **Modify** `backend/src/controllers/{admin,auth,serve}.rs` — `Json<T>` → `ValidatedJson<T>` sur les handlers d'écriture.
- **Modify** `backend/src/services/{projects,comments,deploy}.rs` — retirer la validation de forme désormais redondante (garder invariants métier).
- **Create** `backend/tests/validation_invariant.rs` — test registre table-driven.
- **Modify** front : `frontend/src/components/notes-editor.tsx`, `frontend/src/comments/ui/compose-popup.tsx`, `frontend/src/comments/ui/thread-popup.tsx` — `maxLength`.
- **Modify** docs : `.env.example`, `docs/ENVIRONMENT.md`, `public_docs/content/docs/deploy/configuration.mdx`, `docs/contrat-deploy.md` (§1, §9).

---

## Task 1 : Module central `validation.rs` (registre + custom fns)

**Files:**
- Modify: `backend/Cargo.toml` (dépendance `validator`)
- Create: `backend/src/services/validation.rs`
- Modify: `backend/src/services/mod.rs` (déclarer le module)

**Interfaces — Produces:** consts `MAX_*` ; fns `validate_name/validate_optional_name/validate_opt_opt_brand/validate_body/validate_author_name/validate_optional_release_notes/validate_pin/validate_html/validate_anchor(&…) -> Result<(), ValidationError>` ; `max_html_bytes()/max_anchor_bytes() -> u64`.

- [ ] **Step 1 : Ajouter la dépendance**

Dans `backend/Cargo.toml`, section `[dependencies]` :
```toml
validator = { version = "0.20", features = ["derive"] }
```
Run: `cargo tree -i validator` — Expected: `validator v0.20.x` avec latch comme dépendant direct.

- [ ] **Step 2 : Écrire les tests d'abord** (`validation.rs`, module `#[cfg(test)]`)

```rust
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn name_rejects_empty_and_too_long() {
        assert!(validate_name("").is_err());
        assert!(validate_name("   ").is_err());
        assert!(validate_name(&"x".repeat(MAX_NAME_LEN + 1)).is_err());
        assert!(validate_name(&"x".repeat(MAX_NAME_LEN)).is_ok());
    }

    #[test]
    fn body_rejects_empty_and_over_max() {
        assert!(validate_body("").is_err());
        assert!(validate_body(&"x".repeat(MAX_BODY_LEN + 1)).is_err());
        assert!(validate_body("ok").is_ok());
    }

    #[test]
    fn author_rejects_over_max() {
        assert!(validate_author_name("").is_err());
        assert!(validate_author_name(&"x".repeat(MAX_AUTHOR_NAME_LEN + 1)).is_err());
        assert!(validate_author_name("Léa").is_ok());
    }

    #[test]
    fn pin_requires_six_digits() {
        assert!(validate_pin("424242").is_ok());
        assert!(validate_pin("42").is_err());
        assert!(validate_pin("abcdef").is_err());
    }

    #[test]
    fn html_rejects_empty_and_respects_env_bound() {
        std::env::set_var("LATCH_MAX_HTML_BYTES", "10");
        // OnceLock : ce test doit tourner isolé (serial) OU être le seul à lire l'env.
        assert!(validate_html("").is_err());
        assert!(validate_html("12345678901").is_err()); // 11 octets > 10
        std::env::remove_var("LATCH_MAX_HTML_BYTES");
    }

    #[test]
    fn anchor_rejects_empty_and_over_default() {
        assert!(validate_anchor("").is_err());
        assert!(validate_anchor(&"x".repeat((DEFAULT_MAX_ANCHOR_BYTES + 1) as usize)).is_err());
        assert!(validate_anchor("{}").is_ok());
    }
}
```
> Note : le test `html_...` fixe l'env AVANT le premier `max_html_bytes()`. Comme `OnceLock` fige la 1re lecture, marquer ce test `#[serial]` (crate `serial_test`, déjà dispo) et ne PAS lire `max_html_bytes()` ailleurs dans la même binaire de test. Alternative plus robuste : rendre la borne injectable (fn interne `html_len_ok(v, max)`) et tester la logique pure sans env — **préférer** cette forme :
```rust
    #[test]
    fn html_len_logic_pure() {
        assert!(super::bytes_within("", 10).is_err());        // vide
        assert!(super::bytes_within("12345678901", 10).is_err()); // 11 > 10
        assert!(super::bytes_within("hello", 10).is_ok());
    }
```

- [ ] **Step 3 : Vérifier l'échec**

Run: `cargo nextest run -p latch validation:: 2>&1 | tail` — Expected: FAIL (module/fonctions inexistants).

- [ ] **Step 4 : Implémenter `validation.rs`**

```rust
//! Registre central de validation de FORME des entrées (contrat §1 : la validation
//! de forme vit à la frontière ; ce module en est la source de vérité). Les invariants
//! métier restent dans les services propriétaires.

use std::sync::OnceLock;
use validator::ValidationError;

use crate::services::pin;

/// Longueurs max en CARACTÈRES (Unicode), cohérent avec #13.
pub const MAX_NAME_LEN: usize = 128;
pub const MAX_BODY_LEN: usize = 2000;
pub const MAX_AUTHOR_NAME_LEN: usize = 80;
pub const MAX_RELEASE_NOTES_LEN: usize = 10_000;

/// Tailles max en OCTETS (env-configurables — limites opérationnelles).
pub const DEFAULT_MAX_HTML_BYTES: u64 = 5_242_880; // 5 Mo
pub const DEFAULT_MAX_ANCHOR_BYTES: u64 = 8_192; // 8 Ko

fn env_bytes(name: &str, default: u64) -> u64 {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

pub fn max_html_bytes() -> u64 {
    static C: OnceLock<u64> = OnceLock::new();
    *C.get_or_init(|| env_bytes("LATCH_MAX_HTML_BYTES", DEFAULT_MAX_HTML_BYTES))
}
pub fn max_anchor_bytes() -> u64 {
    static C: OnceLock<u64> = OnceLock::new();
    *C.get_or_init(|| env_bytes("LATCH_MAX_ANCHOR_BYTES", DEFAULT_MAX_ANCHOR_BYTES))
}

/// Logique pure « non-vide + ≤ max octets » (testable sans env).
pub(crate) fn bytes_within(v: &str, max: u64) -> Result<(), ValidationError> {
    if v.is_empty() {
        return Err(ValidationError::new("required"));
    }
    if v.len() as u64 > max {
        return Err(ValidationError::new("too_large"));
    }
    Ok(())
}

fn chars_within(v: &str, max: usize, code: &'static str) -> Result<(), ValidationError> {
    if v.chars().count() > max {
        return Err(ValidationError::new(code));
    }
    Ok(())
}

pub fn validate_name(v: &str) -> Result<(), ValidationError> {
    if v.trim().is_empty() {
        return Err(ValidationError::new("name_required"));
    }
    chars_within(v, MAX_NAME_LEN, "name_too_long")
}

pub fn validate_optional_name(v: &Option<String>) -> Result<(), ValidationError> {
    match v {
        Some(s) => validate_name(s),
        None => Ok(()),
    }
}

pub fn validate_optional_brand(v: &Option<String>) -> Result<(), ValidationError> {
    match v {
        Some(s) => chars_within(s, MAX_NAME_LEN, "brand_name_too_long"),
        None => Ok(()),
    }
}

/// `Option<Option<String>>` (UpdateProjectReq.brand_name) : valide l'inner si présent.
pub fn validate_opt_opt_brand(v: &Option<Option<String>>) -> Result<(), ValidationError> {
    match v {
        Some(Some(s)) => chars_within(s, MAX_NAME_LEN, "brand_name_too_long"),
        _ => Ok(()),
    }
}

pub fn validate_body(v: &str) -> Result<(), ValidationError> {
    if v.trim().is_empty() {
        return Err(ValidationError::new("body_required"));
    }
    chars_within(v, MAX_BODY_LEN, "body_too_long")
}

pub fn validate_author_name(v: &str) -> Result<(), ValidationError> {
    if v.trim().is_empty() {
        return Err(ValidationError::new("author_required"));
    }
    chars_within(v, MAX_AUTHOR_NAME_LEN, "author_too_long")
}

pub fn validate_optional_release_notes(v: &Option<String>) -> Result<(), ValidationError> {
    match v {
        Some(s) => chars_within(s, MAX_RELEASE_NOTES_LEN, "release_notes_too_long"),
        None => Ok(()),
    }
}

pub fn validate_pin(v: &str) -> Result<(), ValidationError> {
    if pin::is_valid_pin(v) {
        Ok(())
    } else {
        Err(ValidationError::new("pin_must_be_6_digits"))
    }
}

pub fn validate_html(v: &str) -> Result<(), ValidationError> {
    bytes_within(v, max_html_bytes())
}

pub fn validate_anchor(v: &str) -> Result<(), ValidationError> {
    bytes_within(v, max_anchor_bytes())
}
```

Dans `backend/src/services/mod.rs`, ajouter `pub mod validation;` (ordre alphabétique parmi les `pub mod`).

- [ ] **Step 5 : Vérifier le vert**

Run: `cargo nextest run -p latch validation::` — Expected: PASS. Puis `cargo clippy -p latch -- -D warnings` — Expected: clean.

- [ ] **Step 6 : Commit**

```bash
git add backend/Cargo.toml backend/Cargo.lock backend/src/services/validation.rs backend/src/services/mod.rs
git commit -m "✨ feat(#23): module central services/validation.rs (registre + custom fns)"
```

---

## Task 2 : Extracteur `ValidatedJson<T>` (frontière web)

**Files:**
- Create: `backend/src/web/extract.rs`
- Modify: `backend/src/web/mod.rs` (`pub mod extract;`)
- Test: dans `backend/src/web/extract.rs` (`#[cfg(test)]` avec un DTO jouet), ou test d'intégration en Task 4.

**Interfaces — Consumes:** `crate::controllers::error::into_response`, `CoreError::Validation`. **Produces:** `pub struct ValidatedJson<T>(pub T)` avec `impl FromRequest`.

- [ ] **Step 1 : Écrire le test** (DTO jouet dans le module test)

```rust
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use axum::{routing::post, Router};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;
    use validator::Validate;

    #[derive(serde::Deserialize, Validate)]
    struct Toy {
        #[validate(length(min = 1, max = 3))]
        s: String,
    }

    async fn handler(ValidatedJson(_t): ValidatedJson<Toy>) -> &'static str { "ok" }

    #[tokio::test]
    async fn rejects_invalid_with_400() {
        let app = Router::new().route("/t", post(handler));
        let res = app.clone().oneshot(
            Request::post("/t").header("content-type", "application/json")
                .body(Body::from(r#"{"s":"toolong"}"#)).unwrap()).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let ok = app.oneshot(
            Request::post("/t").header("content-type", "application/json")
                .body(Body::from(r#"{"s":"ok"}"#)).unwrap()).await.unwrap();
        assert_eq!(ok.status(), StatusCode::OK);
    }
}
```
> `tower::ServiceExt` (`oneshot`) : vérifier qu'il est en dev-deps ; sinon ajouter `tower = { version = "*", features = ["util"] }` en `[dev-dependencies]` (version alignée sur le lock).

- [ ] **Step 2 : Vérifier l'échec** — Run: `cargo nextest run -p latch web::extract` — Expected: FAIL (type absent).

- [ ] **Step 3 : Implémenter** (`backend/src/web/extract.rs`)

```rust
//! Extracteur JSON validant : désérialise puis appelle `.validate()` (contrat §1,
//! validation de forme à la frontière). Échec de validation → 400.

use axum::extract::{FromRequest, Request};
use axum::response::{IntoResponse, Response};
use axum::Json;
use validator::Validate;

use crate::controllers::error::into_response;
use crate::services::errors::CoreError;

pub struct ValidatedJson<T>(pub T);

impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: serde::de::DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(IntoResponse::into_response)?;
        value
            .validate()
            .map_err(|e| into_response(CoreError::Validation(e.to_string())).into_response())?;
        Ok(ValidatedJson(value))
    }
}
```
Dans `backend/src/web/mod.rs` : `pub mod extract;`.

- [ ] **Step 4 : Vérifier le vert** — Run: `cargo nextest run -p latch web::extract` — Expected: PASS.

- [ ] **Step 5 : Commit**
```bash
git add backend/src/web/extract.rs backend/src/web/mod.rs backend/Cargo.toml
git commit -m "✨ feat(#23): extracteur ValidatedJson (validation à la frontière web)"
```

---

## Task 3 : Annoter les DTOs admin/visiteur + brancher `ValidatedJson`

**Files:**
- Modify: `backend/src/dto/mod.rs` (dérives + attributs + schema cross-field)
- Modify: `backend/src/controllers/admin.rs`, `auth.rs`, `serve.rs` (handlers `Json` → `ValidatedJson`)

**Interfaces — Consumes:** fns de `services::validation` (Task 1), `ValidatedJson` (Task 2).

- [ ] **Step 1 : Écrire les tests** (dans `dto/mod.rs`, module tests existant)

```rust
    use validator::Validate;

    #[test]
    fn create_project_req_validates() {
        assert!(CreateProjectReq { name: "ok".into(), brand_name: None, code_enabled: false, pin: None, comments_enabled: None }.validate().is_ok());
        assert!(CreateProjectReq { name: "".into(), brand_name: None, code_enabled: false, pin: None, comments_enabled: None }.validate().is_err());
        assert!(CreateProjectReq { name: "x".repeat(129), brand_name: None, code_enabled: false, pin: None, comments_enabled: None }.validate().is_err());
        // cross-field : code activé sans PIN valide → err
        assert!(CreateProjectReq { name: "ok".into(), brand_name: None, code_enabled: true, pin: None, comments_enabled: None }.validate().is_err());
        assert!(CreateProjectReq { name: "ok".into(), brand_name: None, code_enabled: true, pin: Some("424242".into()), comments_enabled: None }.validate().is_ok());
    }

    #[test]
    fn create_pin_req_validates_body_author_anchor() {
        let ok = CreatePinReq { anchor: "{}".into(), author_name: "Léa".into(), body: "hi".into() };
        assert!(ok.validate().is_ok());
        assert!(CreatePinReq { anchor: "{}".into(), author_name: "x".repeat(81), body: "hi".into() }.validate().is_err());
        assert!(CreatePinReq { anchor: "{}".into(), author_name: "Léa".into(), body: "x".repeat(2001) }.validate().is_err());
        assert!(CreatePinReq { anchor: "".into(), author_name: "Léa".into(), body: "hi".into() }.validate().is_err());
    }

    #[test]
    fn deploy_req_validates_html_nonempty() {
        assert!(DeployReq { html: "".into(), activate: false, notes: None }.validate().is_err());
        assert!(DeployReq { html: "<h1>x</h1>".into(), activate: false, notes: None }.validate().is_ok());
    }
```

- [ ] **Step 2 : Vérifier l'échec** — Run: `cargo nextest run -p latch dto::` — Expected: FAIL (pas de `.validate()`).

- [ ] **Step 3 : Annoter `dto/mod.rs`**

Ajouter `use validator::Validate;` en tête. Ajouter `Validate` aux dérives des `*Req` et les attributs (les `custom` pointent `crate::services::validation::…`). Exemples exacts :

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, Validate)]
#[validate(schema(function = "validate_create_project_pin"))]
pub struct CreateProjectReq {
    #[validate(custom(function = "crate::services::validation::validate_name"))]
    pub name: String,
    #[serde(default)]
    #[validate(custom(function = "crate::services::validation::validate_optional_brand"))]
    pub brand_name: Option<String>,
    #[serde(default = "default_true")]
    pub code_enabled: bool,
    #[serde(default)]
    pub pin: Option<String>,
    #[serde(default)]
    pub comments_enabled: Option<bool>,
}

/// Cross-field : si `code_enabled`, un PIN 6 chiffres est requis. Vit ici (a besoin
/// du type DTO) et délègue le format à `validation::validate_pin`.
fn validate_create_project_pin(req: &CreateProjectReq) -> Result<(), validator::ValidationError> {
    if req.code_enabled {
        match &req.pin {
            Some(p) if crate::services::validation::validate_pin(p).is_ok() => Ok(()),
            _ => Err(validator::ValidationError::new("pin_required_6_digits")),
        }
    } else {
        Ok(())
    }
}
```

Appliquer le même schéma (dérive `Validate` + attributs) aux autres, selon la carte :
- `UpdateProjectReq` : `name` → `custom(validate_optional_name)` ; `brand_name` (`Option<Option<String>>`) → `custom(validate_opt_opt_brand)`.
- `SetCodeReq` : `pin` → `custom(validate_pin)`.
- `DeployReq` : `html` → `custom(validate_html)` ; `notes` → `custom(validate_optional_release_notes)`.
- `LoginReq` : `user` et `pass` → `#[validate(length(min = 1))]` (littéral, pas de const).
- `CreatePinReq` : `anchor` → `custom(validate_anchor)` ; `author_name` → `custom(validate_author_name)` ; `body` → `custom(validate_body)`.
- `ReplyReq` : `author_name` → `custom(validate_author_name)` ; `body` → `custom(validate_body)`.
- `EditMessageReq` : `body` → `custom(validate_body)`.
- `AdminCreatePinReq` : `anchor` → `custom(validate_anchor)` ; `body` → `custom(validate_body)`.
- `AdminReplyReq` : `body` → `custom(validate_body)`.
- `UnlockReq` : `pin` → `custom(validate_pin)`.

> `custom(function = "chemin::complet")` : le chemin absolu évite les imports. Vérifier au premier `cargo build` que `validator 0.20` accepte la string de chemin (sinon `use` la fn et référencer par nom court).

- [ ] **Step 4 : Vérifier le vert DTO** — Run: `cargo nextest run -p latch dto::` — Expected: PASS.

- [ ] **Step 5 : Brancher l'extracteur dans les handlers**

Dans `admin.rs`, `auth.rs`, `serve.rs` : remplacer chaque `Json(body): Json<XxxReq>` par `ValidatedJson(body): ValidatedJson<XxxReq>` et importer `use crate::web::extract::ValidatedJson;`. Handlers concernés (écritures) : `auth::login` (LoginReq) ; `admin::{create, update, set_code, deploy, admin_create_pin, admin_reply, admin_edit_comment}` ; `serve::{unlock (UnlockReq), create_comment (CreatePinReq), reply_comment (ReplyReq), edit_comment (EditMessageReq)}`. NE PAS toucher les handlers sans body JSON.

- [ ] **Step 6 : Vérifier build + tests** — Run: `cargo nextest run -p latch` — Expected: PASS (ajuster tout test d'intégration attendant l'ancien code d'erreur — un PIN malformé sur `/unlock` renvoie désormais 400 au lieu de 401 ; corriger l'assertion si un tel test existe).

- [ ] **Step 7 : Commit**
```bash
git add backend/src/dto/mod.rs backend/src/controllers/{admin,auth,serve}.rs
git commit -m "✨ feat(#23): #[validate] sur les DTOs + ValidatedJson sur les handlers"
```

---

## Task 4 : Frontière MCP — valider les args des tools

**Files:**
- Modify: `backend/src/mcp/mod.rs`

**Interfaces — Consumes:** `services::validation`. **Produces:** `map_validation_err`.

- [ ] **Step 1 : Écrire le test** (module tests de `mcp/mod.rs`, ou test d'intégration MCP existant)

```rust
    #[test]
    fn deploy_args_reject_empty_html() {
        use validator::Validate;
        let a = DeployArgs { slug: "s".into(), html: "".into(), deploy_token: "t".into(), activate: None, release_notes: None };
        assert!(a.validate().is_err());
        let ok = DeployArgs { slug: "s".into(), html: "<h1>x</h1>".into(), deploy_token: "t".into(), activate: None, release_notes: None };
        assert!(ok.validate().is_ok());
    }
```

- [ ] **Step 2 : Vérifier l'échec** — Run: `cargo nextest run -p latch mcp::` — Expected: FAIL.

- [ ] **Step 3 : Implémenter**

Ajouter `Validate` aux dérives + attributs sur les args :
```rust
#[derive(Debug, Deserialize, schemars::JsonSchema, validator::Validate)]
struct DeployArgs {
    #[validate(length(min = 1))]
    slug: String,
    #[validate(custom(function = "crate::services::validation::validate_html"))]
    html: String,
    deploy_token: String, // exempté (secret)
    #[serde(default)]
    activate: Option<bool>,
    #[serde(default)]
    #[validate(custom(function = "crate::services::validation::validate_optional_release_notes"))]
    release_notes: Option<String>,
}
```
`PullArgs` : `slug` → `#[validate(length(min = 1))]`. `ListArgs` : rien (seul `deploy_token`, exempté). Helper + appel :
```rust
fn map_validation_err(e: validator::ValidationErrors) -> ErrorData {
    ErrorData::invalid_params(format!("arguments invalides: {e}"), None)
}
```
Dans chaque tool, APRÈS `check_token`, AVANT tout appel cœur :
```rust
use validator::Validate;
self.check_token(&args.deploy_token)?;
args.validate().map_err(map_validation_err)?;
```
(`deploy_prototype`, `pull_prototype` ; `list_projects` n'a que le token → validate est un no-op mais l'ajouter garde l'uniformité de l'invariant.)

- [ ] **Step 4 : Vérifier le vert** — Run: `cargo nextest run -p latch mcp::` — Expected: PASS.

- [ ] **Step 5 : Commit**
```bash
git add backend/src/mcp/mod.rs
git commit -m "✨ feat(#23): validation des args MCP (.validate() après check_token)"
```

---

## Task 5 : Alléger le cœur (retirer la validation de forme redondante)

**Files:**
- Modify: `backend/src/services/projects.rs`, `comments.rs`, `deploy.rs`

**Interfaces:** aucune nouvelle. Les invariants métier restent.

- [ ] **Step 1 : Adapter les tests de service**

Les tests unit de service qui vérifiaient la validation via le service (ex. `create_rejects_empty_name`, `create_rejects_name_over_max_len`) : la validation de forme ne vit plus dans `create`. Deux options — **préférée** : déplacer ces assertions vers `dto::` (déjà couvert en Task 3) et RETIRER les tests de service devenus faux. Retirer aussi `validate_project_name`/`validate_brand_name` de `projects.rs` (migrés en `validation.rs`) et leurs appels dans `create`/`update`.

- [ ] **Step 2 : Retirer la validation de forme**

- `projects.rs::create` : retirer les appels `validate_project_name`/`validate_brand_name` (la frontière valide). Garder toute logique métier (génération PIN, slug…). Supprimer les fns `validate_*` + `MAX_PROJECT_NAME_LEN` (désormais dans `validation.rs`).
- `admin.rs::update` : retirer les appels `validate_project_name`/`validate_brand_name` (ValidatedJson valide en amont).
- `comments.rs` : `validate_body` (form) retiré des chemins où le DTO est déjà validé ; **garder** `sanitize_author_name` (transformation : strip control chars) et les invariants (`MAX_PINS_PER_VERSION_PER_OWNER`). `MAX_BODY_LEN`/`MAX_AUTHOR_NAME_LEN` : supprimer si plus référencés (migrés).
- `deploy.rs` : retirer le check `MAX_RELEASE_NOTES_LEN` inline + la const (migrés) — `notes` validé au DTO/args.

> Attention : `deploy()` est appelé par 2 frontières (admin ValidatedJson + MCP args.validate) → les deux valident `notes`/`html` en amont. OK de retirer du service. Vérifier qu'aucun autre appelant non-validé n'existe (`grep deploy(`).

- [ ] **Step 3 : Vérifier le vert** — Run: `cargo nextest run -p latch` — Expected: PASS. `cargo clippy --all-targets -- -D warnings` — clean (pas de const/fn morte).

- [ ] **Step 4 : Commit**
```bash
git add backend/src/services/{projects,comments,deploy}.rs backend/src/controllers/admin.rs
git commit -m "♻️ refactor(#23): migre la validation de forme cœur→frontière"
```

---

## Task 6 : Invariant testé — registre table-driven

**Files:**
- Create: `backend/tests/validation_invariant.rs`

- [ ] **Step 1 : Écrire le test**

```rust
#![allow(clippy::unwrap_used)]
//! Invariant §9 : chaque DTO de frontière rejette une entrée hors-borne. Type-level
//! garanti par ValidatedJson<T: Validate> + args.validate() (compile). Ici : couverture
//! comportementale table-driven — une régression de borne casse le build.

use latch::dto::*;
use validator::Validate;

#[test]
fn every_write_dto_rejects_oversized_input() {
    // (label, closure renvoyant un DTO hors-borne) → doit être Err(validate)
    macro_rules! reject {
        ($label:expr, $dto:expr) => {
            assert!($dto.validate().is_err(), "{} devrait être rejeté", $label);
        };
    }
    reject!("CreateProjectReq.name vide", CreateProjectReq { name: "".into(), brand_name: None, code_enabled: false, pin: None, comments_enabled: None });
    reject!("CreateProjectReq.name >128", CreateProjectReq { name: "x".repeat(129), brand_name: None, code_enabled: false, pin: None, comments_enabled: None });
    reject!("SetCodeReq.pin", SetCodeReq { pin: "42".into() });
    reject!("DeployReq.html vide", DeployReq { html: "".into(), activate: false, notes: None });
    reject!("DeployReq.notes >10000", DeployReq { html: "<h1>x</h1>".into(), activate: false, notes: Some("x".repeat(10_001)) });
    reject!("LoginReq.user vide", LoginReq { user: "".into(), pass: "x".into() });
    reject!("CreatePinReq.body >2000", CreatePinReq { anchor: "{}".into(), author_name: "A".into(), body: "x".repeat(2001) });
    reject!("CreatePinReq.author >80", CreatePinReq { anchor: "{}".into(), author_name: "x".repeat(81), body: "hi".into() });
    reject!("ReplyReq.body >2000", ReplyReq { author_name: "A".into(), body: "x".repeat(2001) });
    reject!("EditMessageReq.body vide", EditMessageReq { body: "".into() });
    reject!("AdminCreatePinReq.body >2000", AdminCreatePinReq { anchor: "{}".into(), body: "x".repeat(2001) });
    reject!("AdminReplyReq.body vide", AdminReplyReq { body: "".into() });
    reject!("UnlockReq.pin", UnlockReq { pin: "abc".into() });
}
```
> Note : `dto` doit être `pub` dans `lib.rs` (`pub mod dto;`) — vérifier ; sinon exposer. Les DTOs sont déjà `pub`.

- [ ] **Step 2 : Vérifier le vert** — Run: `cargo nextest run -p latch --test validation_invariant` — Expected: PASS.

- [ ] **Step 3 : Commit**
```bash
git add backend/tests/validation_invariant.rs
git commit -m "✅ test(#23): invariant table-driven de validation des DTOs"
```

---

## Task 7 : Front (B) — `maxLength` indicatif

**Files:**
- Modify: `frontend/src/comments/ui/compose-popup.tsx` (author_name input, body textarea)
- Modify: `frontend/src/comments/ui/thread-popup.tsx` (reply + edit body)
- Modify: `frontend/src/components/notes-editor.tsx` (si un textarea existe ; sinon ignorer avec note)

- [ ] **Step 1 : Ajouter `maxLength`**

Sur l'`<input>` auteur : `maxLength={80}`. Sur les `<textarea>` de corps (compose, reply, edit) : `maxLength={2000}`. Sur l'éditeur de notes : `maxLength={10000}` si c'est un `<textarea>` natif (si tiptap/contentEditable, laisser un commentaire « borne back = 10000, pas d'attribut natif » et ne rien forcer). Valeurs = commentaire « // indicatif ; borne réelle côté back ».

- [ ] **Step 2 : Vérifier** — Run (depuis `frontend/`): `pnpm lint && pnpm typecheck && pnpm test` — Expected: vert. QA visuelle : taper au-delà de la limite est bloqué dans les champs concernés.

- [ ] **Step 3 : Commit**
```bash
git add frontend/src/comments/ui/{compose-popup,thread-popup}.tsx frontend/src/components/notes-editor.tsx
git commit -m "✨ feat(#23): maxLength indicatif sur les champs texte (UX, front hors invariant)"
```

---

## Task 8 : Config + documentation

**Files:**
- Modify: `.env.example`, `docs/ENVIRONMENT.md`, `public_docs/content/docs/deploy/configuration.mdx`

- [ ] **Step 1 : `.env.example`** — ajouter, dans un bloc cohérent avec les autres `LATCH_*` (commenté, avec défaut) :
```
# Taille max du HTML déployé (octets). Défaut 5 Mo.
# LATCH_MAX_HTML_BYTES=5242880
# Taille max du descripteur d'ancrage d'un commentaire (octets). Défaut 8 Ko.
# LATCH_MAX_ANCHOR_BYTES=8192
```

- [ ] **Step 2 : `docs/ENVIRONMENT.md`** — 2 lignes dans la section vars (rôle + défaut).

- [ ] **Step 3 : fumadocs** — `public_docs/content/docs/deploy/configuration.mdx` : 2 lignes dans la table des clés (`LATCH_MAX_HTML_BYTES` | `5242880` | Max deployed HTML size (bytes). ; `LATCH_MAX_ANCHOR_BYTES` | `8192` | Max comment anchor descriptor size (bytes).). Régénérer si nécessaire ; `public_docs/out/` non-tracké.

- [ ] **Step 4 : Vérifier** — Run (depuis `public_docs/`): `pnpm build` — Expected: vert. `grep LATCH_MAX .env.example docs/ENVIRONMENT.md public_docs/content/docs/deploy/configuration.mdx` — 3 fichiers touchés.

- [ ] **Step 5 : Commit**
```bash
git add .env.example docs/ENVIRONMENT.md public_docs/content/docs/deploy/configuration.mdx
git commit -m "📝 docs(#23): documente LATCH_MAX_HTML_BYTES / LATCH_MAX_ANCHOR_BYTES"
```

---

## Task 9 : Contrat (§1 + §9) + mémoires

**Files:**
- Modify: `docs/contrat-deploy.md` (§1, §9)
- Modify: `docs/INDEX.md`, `docs/HANDOFF.md`, `docs/QUIRKS.md` (si piège), `docs/CONVENTIONS.md` (pattern validation)

- [ ] **Step 1 : Contrat §1** — préciser : « la **validation de forme** des entrées vit à la frontière (extracteur `ValidatedJson` côté web, `args.validate()` côté MCP) ; le cœur suppose l'input déjà validé. Les invariants métier restent au cœur. »

- [ ] **Step 2 : Contrat §9** — ajouter l'invariant : « Toute entrée franchissant une frontière (web, MCP) est validée via `Validate` (source de vérité back), couverte par un test bloquant (`tests/validation_invariant.rs`). Un DTO de frontière sans `impl Validate` ne compile pas. » Au même rang que les autres invariants testés.

- [ ] **Step 3 : Mémoires** — `INDEX.md` (ligne livrable #23), `HANDOFF.md` (entrée datée), `CONVENTIONS.md` (pattern : « nouvel input de frontière → `#[derive(Validate)]` + bornes dans `validation.rs` + `ValidatedJson`/`args.validate()` »), `QUIRKS.md` si piège rencontré (ex. OnceLock env + tests).

- [ ] **Step 4 : Commit**
```bash
git add docs/contrat-deploy.md docs/INDEX.md docs/HANDOFF.md docs/CONVENTIONS.md docs/QUIRKS.md
git commit -m "📝 docs(#23): invariant validation au contrat §1/§9 + mémoires"
```

---

## Task 10 : Gate finale + PR

- [ ] **Step 1 : Backend** — `cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo nextest run` — tout vert. Si l'OpenAPI a bougé (ne devrait pas — `#[validate]` ne feed pas utoipa) : `UPDATE_OPENAPI=1 cargo test --test openapi_drift` puis `git diff openapi.json` (attendu : vide).
- [ ] **Step 2 : Frontend** — depuis `frontend/` : `pnpm lint && pnpm typecheck && pnpm test`. Rebuild `dist/` (`pnpm build`) pour la QA `:5150`.
- [ ] **Step 3 : QA manuelle `:5150`** — payload over-limit (nom >128, body >2000, html vide) → 400 ; MCP `deploy_prototype` html vide → tool error ; `LATCH_MAX_HTML_BYTES=100` → petit HTML rejeté (prouve la lecture env).
- [ ] **Step 4 : Push + PR** `Closes #23`, carte In review. QA humaine, CI/Sonar, merge = gate.

---

## Notes de séquençage

- **Ordre** : Task 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10. Les tâches 1-2 sont l'infra ; 3-4 branchent l'invariant ; 5 nettoie ; 6 verrouille ; 7-9 périphérie ; 10 gate.
- **Point de vigilance récurrent** : à chaque suppression de validation cœur (Task 5), s'assurer que TOUTES les frontières appelant le service valident en amont (`grep` des appelants). Aucun appelant non-validé ne doit subsister.
- **`validator 0.20` const-path** : si `custom(function = "chemin::complet")` en string ne compile pas, `use crate::services::validation::*;` et référencer par nom court. Trancher au 1er build de Task 3 (non bloquant pour l'archi).
