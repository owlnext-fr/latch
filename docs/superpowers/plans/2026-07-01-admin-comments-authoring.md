# Authoring de commentaires côté admin — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Permettre à l'admin de créer ses propres fils de commentaires (notes privées), de répondre aux fils des visiteurs, et d'éditer/supprimer ses propres messages, depuis la page Review.

**Architecture:** On réutilise le modèle de propriété `owner_token` existant via un **jeton sentinelle réservé** (`ADMIN_OWNER_TOKEN`) pour l'unique compte admin — aucune migration DB. La distinction admin est un **booléen dérivé** `is_admin` (`owner_token == sentinelle`), sérialisé sur les deux surfaces (le token lui-même n'est jamais exposé). Le frontend est déjà piloté par `capabilities` ; on active l'authoring admin et on ajoute un seam `fixedAuthorName` + un badge.

**Tech Stack:** Rust (Loco/axum, SeaORM, utoipa) ; React + Vite + TypeScript (TanStack Query, shadcn/ui, react-i18next) ; Vitest + MSW ; Playwright.

## Global Constraints

- **Confidentialité** : aucun nom de client réel nulle part (placeholders `Demo`/`ACME`/`Mon Projet`).
- **Invariant §9** : `owner_token` JAMAIS sérialisé (ni web ni MCP) ; on ne renvoie qu'un booléen dérivé.
- **PIN en clair** : uniquement sur le détail projet (inchangé ici).
- **Gardes mutations** : tout endpoint admin d'écriture porte `AdminAuth` + `require_same_origin` (401 sans session, 403 cross-origin). Pas de `X-Comment-Client` côté admin (c'est la garde visiteur).
- **Identité admin forcée serveur** : le `author_name` envoyé par le client est ignoré sur les endpoints admin ; le serveur pose `ADMIN_OWNER_TOKEN` / `ADMIN_AUTHOR`.
- **Sentinelle** : `ADMIN_OWNER_TOKEN = "__admin__"` — non-collision avec un ULID (26 chars Crockford base32, sans underscore).
- **Définition de « terminé »** : `cargo fmt` + `cargo clippy --all-targets -- -D warnings` clean ; `cargo nextest run` vert (dont `openapi_drift`) ; `pnpm lint` + `pnpm typecheck` + Vitest verts ; e2e Playwright verts ; doc + mémoire à jour.
- **Régénération contrat** : après tout changement DTO/route → `UPDATE_OPENAPI=1 cargo test --test openapi_drift` (backend) puis `pnpm gen:api` (frontend).

---

## File Structure

**Backend**
- `backend/src/services/comments.rs` — consts `ADMIN_OWNER_TOKEN`/`ADMIN_AUTHOR` + méthode `admin_add_reply` (Task 1).
- `backend/src/dto/mod.rs` — champ `is_admin` sur `CommentMessage`/`AdminCommentMessage`, helper `to_admin_comment_message`, DTO requêtes `AdminCreatePinReq`/`AdminReplyReq` (Task 2).
- `backend/src/controllers/serve.rs` — literals `CommentMessage` (reply/edit) ajoutent `is_admin: false` (Task 2).
- `backend/src/controllers/admin.rs` — 4 handlers + routes (Task 3).
- `backend/src/openapi.rs` — enregistrement paths + schémas (Task 3).
- `openapi.json` (racine) — régénéré (Tasks 2, 3).

**Frontend**
- `frontend/src/api/schema.d.ts` — régénéré (Task 5).
- `frontend/src/comments/data/adapter.ts` — `CommentsAdapter.fixedAuthorName` (Task 5).
- `frontend/src/comments/data/admin-adapter.ts` — authoring complet (Task 5).
- `frontend/src/comments/data/visitor-adapter.ts` — `fixedAuthorName: null` (Task 5).
- `frontend/src/routes/review.tsx` — passe le libellé i18n à `createAdminAdapter` (Task 5).
- `frontend/src/comments/comments-app.tsx` — câblage `fixedAuthorName` (compose + reply) (Task 6).
- `frontend/src/comments/ui/compose-popup.tsx` — masque le champ nom si `fixedAuthorName` (Task 6).
- `frontend/src/comments/ui/thread-popup.tsx` + `comments-drawer.tsx` — libellé + badge admin (Task 7).
- `frontend/src/i18n/locales/comments/{en,fr}.json` — nouvelles clés (Tasks 6, 7).

**Tests**
- `backend/src/services/comments.rs` (mod tests) ; `backend/tests/comments_admin.rs` ; `backend/tests/comments_serve.rs`.
- `frontend/src/comments/data/admin-adapter.test.ts` ; `.../ui/compose-popup.test.tsx` ; `.../ui/thread-popup.test.tsx` ; `.../comments-app.test.tsx` + mocks.
- `frontend/e2e/comments-admin.spec.ts`.

**Docs**
- `docs/contrat-deploy.md` ; `public_docs/content/docs/admin/comments.mdx` ; mémoire (`docs/INDEX.md`, `docs/HANDOFF.md`).

---

## Task 1 : Backend — sentinelle + `admin_add_reply`

**Files:**
- Modify: `backend/src/services/comments.rs` (consts en tête d'impl + nouvelle méthode ; tests dans le `mod tests` existant l. 379+)

**Interfaces:**
- Produces:
  - `pub const ADMIN_OWNER_TOKEN: &str = "__admin__";`
  - `pub const ADMIN_AUTHOR: &str = "admin";`
  - `pub async fn admin_add_reply(&self, project_id: i32, pin_id: i32, body: &str) -> Result<comments::Model, CoreError>`

- [ ] **Step 1 : Écrire les tests qui échouent** (dans `mod tests`, après `add_reply_to_foreign_pin_is_not_found`)

```rust
    #[tokio::test]
    async fn admin_add_reply_appends_to_any_pin_with_sentinel_owner() {
        let db = test_db().await;
        let v = version(&db).await;
        let svc = CommentsService::new(db);
        // Un visiteur crée un fil ; l'admin y répond sans le posséder.
        let pwm = svc.create_pin(v.id, OWNER_A, "Léa", "un", "{}").await.unwrap();

        let reply = svc
            .admin_add_reply(v.project_id, pwm.pin.id, "réponse admin")
            .await
            .unwrap();

        assert_eq!(reply.pin_id, pwm.pin.id);
        assert_eq!(reply.body, "réponse admin");
        assert_eq!(reply.owner_token, ADMIN_OWNER_TOKEN);
    }

    #[tokio::test]
    async fn admin_add_reply_wrong_project_is_not_found() {
        let db = test_db().await;
        let v = version(&db).await;
        let svc = CommentsService::new(db);
        let pwm = svc.create_pin(v.id, OWNER_A, "Léa", "un", "{}").await.unwrap();

        // project_id qui ne possède pas ce pin → NotFound (ne révèle pas l'existence).
        let err = svc
            .admin_add_reply(v.project_id + 999, pwm.pin.id, "intrus")
            .await
            .unwrap_err();
        assert!(matches!(err, CoreError::NotFound));
    }
```

- [ ] **Step 2 : Lancer les tests, vérifier l'échec**

Run: `cd backend && cargo test --lib services::comments::tests::admin_add_reply -- --nocapture`
Expected: FAIL — `no method named admin_add_reply` / `ADMIN_OWNER_TOKEN not found`.

- [ ] **Step 3 : Ajouter les consts + la méthode**

En tête de `impl CommentsService` (juste après `pub fn new`), ajouter les consts au niveau module (au-dessus de `impl`, à côté de `MAX_PINS_PER_VERSION_PER_OWNER`) :

```rust
/// Identité de propriété de l'unique compte admin (jamais sérialisée : voir `is_admin`).
/// Non-collision avec un ULID visiteur (26 chars Crockford base32, sans underscore).
pub const ADMIN_OWNER_TOKEN: &str = "__admin__";
/// `author_name` stocké pour les messages admin — jamais affiché (l'UI rend un libellé i18n via `is_admin`).
pub const ADMIN_AUTHOR: &str = "admin";
```

Puis, dans `impl CommentsService`, après `add_reply` :

```rust
    /// Ajoute une réponse admin à **n'importe quel** pin du projet `project_id`
    /// (sans owner-check — l'admin ne possède pas les pins des visiteurs).
    /// Vérifie pin → version → projet avant d'insérer (NotFound sinon).
    pub async fn admin_add_reply(
        &self,
        project_id: i32,
        pin_id: i32,
        body: &str,
    ) -> Result<comments::Model, CoreError> {
        use crate::models::_entities::versions;
        let body = validate_body(body)?;
        let pin = comment_pins::Entity::find_by_id(pin_id)
            .filter(comment_pins::Column::DeletedAt.is_null())
            .one(&self.db)
            .await?
            .ok_or(CoreError::NotFound)?;
        let version = versions::Entity::find_by_id(pin.version_id)
            .one(&self.db)
            .await?
            .ok_or(CoreError::NotFound)?;
        if version.project_id != project_id {
            return Err(CoreError::NotFound);
        }
        Ok(comments::ActiveModel {
            pin_id: Set(pin.id),
            owner_token: Set(ADMIN_OWNER_TOKEN.to_string()),
            author_name: Set(ADMIN_AUTHOR.to_string()),
            body: Set(body),
            ..Default::default()
        }
        .insert(&self.db)
        .await?)
    }
```

- [ ] **Step 4 : Lancer les tests, vérifier le succès**

Run: `cd backend && cargo test --lib services::comments::tests::admin_add_reply`
Expected: PASS (2 tests).

- [ ] **Step 5 : fmt + clippy + commit**

```bash
cd backend && cargo fmt && cargo clippy --all-targets -- -D warnings
git add backend/src/services/comments.rs
git commit -m "✨ feat(comments): admin_add_reply + sentinelle owner_token admin"
```

---

## Task 2 : Backend — DTO `is_admin` + requêtes admin + régénération OpenAPI

**Files:**
- Modify: `backend/src/dto/mod.rs` (structs + helpers + tests l. 540+)
- Modify: `backend/src/controllers/serve.rs:552-559,588-595` (literals `CommentMessage`)
- Modify: `openapi.json` (régénéré)

**Interfaces:**
- Consumes: `ADMIN_OWNER_TOKEN` (Task 1).
- Produces:
  - `CommentMessage { …, is_admin: bool }`, `AdminCommentMessage { …, is_admin: bool }`
  - `pub fn to_admin_comment_message(m: &comments::Model) -> AdminCommentMessage`
  - `AdminCreatePinReq { anchor: String, body: String }`, `AdminReplyReq { body: String }`

- [ ] **Step 1 : Écrire le test DTO qui échoue** (dans `mod tests` de `dto/mod.rs`)

```rust
    #[test]
    fn is_admin_true_only_for_sentinel_owner() {
        use crate::services::comments::ADMIN_OWNER_TOKEN;
        let pin = sample_pin(); // helper existant du module de tests
        let admin_msg = sample_message_with_owner(ADMIN_OWNER_TOKEN);
        let visitor_msg = sample_message_with_owner("01OWNERAAAAAAAAAAAAAAAAAAA");

        let dto_admin = to_comment_pin(&pin, std::slice::from_ref(&admin_msg), "someone");
        assert!(dto_admin.messages[0].is_admin);

        let dto_visitor = to_comment_pin(&pin, std::slice::from_ref(&visitor_msg), "someone");
        assert!(!dto_visitor.messages[0].is_admin);

        let admin_dto = to_admin_comment_pin(&pin, std::slice::from_ref(&admin_msg));
        assert!(admin_dto.messages[0].is_admin);
    }
```

> **Note d'implémentation** : si `sample_pin`/`sample_message_with_owner` n'existent pas dans le module de tests, les créer à partir des modèles `comment_pins::Model`/`comments::Model` en s'inspirant du test existant `comment_pin_hides_owner_token_and_computes_editable` (l. 546). Un `comments::Model` se construit avec tous ses champs (`id`, `pin_id`, `owner_token`, `author_name`, `body`, `created_at`, `updated_at`, `deleted_at`). Réutiliser le style déjà présent dans ce `mod tests`.

- [ ] **Step 2 : Lancer, vérifier l'échec**

Run: `cd backend && cargo test --lib dto::tests::is_admin_true_only_for_sentinel_owner`
Expected: FAIL — `no field is_admin` / `to_admin_comment_message` absent.

- [ ] **Step 3 : Ajouter le champ + la dérivation**

Dans `CommentMessage` (après `pub editable: bool,`) :

```rust
    /// `true` si le message a été écrit par l'admin (identité sentinelle). Booléen dérivé —
    /// l'`owner_token` n'est jamais sérialisé (invariant §9).
    pub is_admin: bool,
```

Dans `AdminCommentMessage` (après `pub updated_at: String,`) :

```rust
    /// `true` si le message est celui de l'admin (ses propres messages, éditables/supprimables).
    pub is_admin: bool,
```

Dans `to_comment_pin`, remplacer la construction du `CommentMessage` par :

```rust
                CommentMessage {
                    id,
                    author_name,
                    body,
                    created_at,
                    updated_at,
                    editable: m.owner_token == caller_owner_token,
                    is_admin: m.owner_token == crate::services::comments::ADMIN_OWNER_TOKEN,
                }
```

Remplacer le corps de `to_admin_comment_pin` (mapping des messages) par un appel au nouveau helper, et ajouter le helper juste après :

```rust
/// `comments::Model` → `AdminCommentMessage` (avec `is_admin` dérivé).
pub fn to_admin_comment_message(m: &comments::Model) -> AdminCommentMessage {
    let (id, author_name, body, created_at, updated_at) = message_base_fields(m);
    AdminCommentMessage {
        id,
        author_name,
        body,
        created_at,
        updated_at,
        is_admin: m.owner_token == crate::services::comments::ADMIN_OWNER_TOKEN,
    }
}
```

Et dans `to_admin_comment_pin`, `messages: messages.iter().map(to_admin_comment_message).collect(),`.

Ajouter les DTO de requête admin (à côté de `CreatePinReq`/`ReplyReq`) :

```rust
/// Corps de `POST /api/projects/{id}/versions/{n}/comments` (fil propre de l'admin).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AdminCreatePinReq {
    pub anchor: String,
    pub body: String,
}

/// Corps de `POST /api/projects/{id}/comments/pins/{pin}/replies` (réponse admin).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AdminReplyReq {
    pub body: String,
}
```

- [ ] **Step 4 : Corriger les literals `CommentMessage` du serving visiteur**

Dans `backend/src/controllers/serve.rs`, aux deux endroits qui construisent un `CommentMessage` en dur (`reply_comment` ~l. 552, `edit_comment` ~l. 588), ajouter le champ après `editable: true,` :

```rust
        is_admin: false,
```

> Ces endpoints sont visiteur-only (owner = cookie signé, jamais la sentinelle) → `false` est correct.

- [ ] **Step 5 : Vérifier que ça compile + tests DTO verts**

Run: `cd backend && cargo test --lib dto::tests`
Expected: PASS (dont le nouveau test + les tests d'invariant existants `owner_token` absent / `editable` absent côté admin restent verts).

- [ ] **Step 6 : Régénérer `openapi.json`**

Run: `cd backend && UPDATE_OPENAPI=1 cargo test --test openapi_drift`
Then: `cd backend && cargo test --test openapi_drift`
Expected: 2ᵉ run PASS (drift résolu). `git diff --stat openapi.json` doit montrer l'ajout de `is_admin`.

- [ ] **Step 7 : fmt + clippy + commit**

```bash
cd backend && cargo fmt && cargo clippy --all-targets -- -D warnings
git add backend/src/dto/mod.rs backend/src/controllers/serve.rs openapi.json
git commit -m "✨ feat(comments): champ derive is_admin + DTO requetes admin"
```

---

## Task 3 : Backend — handlers admin d'écriture + routes + OpenAPI paths

**Files:**
- Modify: `backend/src/controllers/admin.rs` (4 handlers + `routes()`)
- Modify: `backend/src/openapi.rs` (paths + schemas + test `document_contains_all_paths`)
- Modify: `openapi.json` (régénéré)
- Test: `backend/tests/comments_admin.rs`

**Interfaces:**
- Consumes: `admin_add_reply` (Task 1) ; `to_admin_comment_pin`, `to_admin_comment_message`, `AdminCreatePinReq`, `AdminReplyReq`, `EditMessageReq` (Task 2) ; `find_version` (`admin.rs:55`) ; `ADMIN_OWNER_TOKEN`, `ADMIN_AUTHOR` (Task 1).
- Produces: handlers `admin_create_pin`, `admin_reply`, `admin_edit_comment`, `admin_delete_pin` ; routes associées.

- [ ] **Step 1 : Écrire les tests d'intégration qui échouent** (`backend/tests/comments_admin.rs`, nouveaux `#[tokio::test] #[serial]`)

```rust
#[tokio::test]
#[serial]
async fn admin_reply_is_visible_to_the_visitor_thread_with_is_admin() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("LATCH_STORAGE_ROOT", tmp.path());
    let config = RequestConfigBuilder::new().save_cookies(true).build();
    request_with_config::<App, _, _>(config, |request, _ctx| async move {
        request.post("/api/login")
            .json(&serde_json::json!({"user":"admin","pass":"s3cret"})).await;
        let created = request.post("/api/projects")
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"name":"Demo","code_enabled":false,"comments_enabled":true})).await;
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();
        let slug = created.json::<serde_json::Value>()["slug"].as_str().unwrap().to_string();
        request.post(&format!("/api/projects/{id}/deploy"))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"html":"<h1>v1</h1>","activate":true})).await;
        // Visiteur crée un fil (le client garde le cookie latch_comment).
        let pin = request.post(&format!("/c/{slug}/comments"))
            .add_header("origin", "http://127.0.0.1")
            .add_header("x-comment-client", "1")
            .json(&serde_json::json!({"anchor":"{}","author_name":"Léa","body":"coucou"})).await;
        let pin_id = pin.json::<serde_json::Value>()["id"].as_i64().unwrap();

        // Admin répond dans le fil du visiteur.
        let reply = request.post(&format!("/api/projects/{id}/comments/pins/{pin_id}/replies"))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"body":"merci du retour"})).await;
        assert_eq!(reply.status_code(), 200);
        assert_eq!(reply.json::<serde_json::Value>()["is_admin"], true);

        // Le visiteur (même client, cookie latch_comment) voit la réponse admin.
        let vlist = request.get(&format!("/c/{slug}/comments")).await;
        let vv = vlist.json::<serde_json::Value>();
        let msgs = &vv["pins"][0]["messages"];
        assert_eq!(msgs[1]["body"], "merci du retour");
        assert_eq!(msgs[1]["is_admin"], true);
        assert!(!vlist.text().contains("owner_token"));
    }).await;
    drop(tmp);
}

#[tokio::test]
#[serial]
async fn admin_create_own_pin_is_private_and_flagged() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("LATCH_STORAGE_ROOT", tmp.path());
    let config = RequestConfigBuilder::new().save_cookies(true).build();
    request_with_config::<App, _, _>(config, |request, _ctx| async move {
        request.post("/api/login")
            .json(&serde_json::json!({"user":"admin","pass":"s3cret"})).await;
        let created = request.post("/api/projects")
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"name":"Demo","code_enabled":false,"comments_enabled":true})).await;
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();
        request.post(&format!("/api/projects/{id}/deploy"))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"html":"<h1>v1</h1>","activate":true})).await;

        let pin = request.post(&format!("/api/projects/{id}/versions/1/comments"))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"anchor":"{}","body":"note interne"})).await;
        assert_eq!(pin.status_code(), 200);
        assert_eq!(pin.json::<serde_json::Value>()["messages"][0]["is_admin"], true);

        // Visible dans la liste admin.
        let alist = request.get(&format!("/api/projects/{id}/versions/1/comments")).await;
        assert_eq!(alist.json::<serde_json::Value>()["pins"].as_array().unwrap().len(), 1);
    }).await;
    drop(tmp);
}

#[tokio::test]
#[serial]
async fn admin_write_endpoints_require_session() {
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("LATCH_STORAGE_ROOT", tmp.path());
    request::<App, _, _>(|request, _ctx| async move {
        // Sans login : 401.
        let r = request.post("/api/projects/1/versions/1/comments")
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"anchor":"{}","body":"x"})).await;
        assert_eq!(r.status_code(), 401);
    }).await;
    drop(tmp);
}
```

- [ ] **Step 2 : Lancer, vérifier l'échec**

Run: `cd backend && cargo test --test comments_admin admin_reply_is_visible_to_the_visitor_thread_with_is_admin`
Expected: FAIL — 404/405 (route absente).

- [ ] **Step 3 : Écrire les handlers** (`admin.rs`, après `moderate_delete_comment`)

```rust
/// POST /api/projects/{id}/versions/{n}/comments — l'admin démarre son propre fil (note privée).
#[utoipa::path(
    post, path = "/api/projects/{id}/versions/{n}/comments", tag = "versions",
    params(("id" = i32, Path, description = "Identifiant du projet"),
           ("n" = i32, Path, description = "Numéro de version")),
    request_body = crate::dto::AdminCreatePinReq,
    responses((status = 200, description = "Fil créé", body = crate::dto::AdminCommentPin),
              (status = 404, description = "Version inconnue"),
              (status = 401, description = "Non authentifié"),
              (status = 403, description = "Origin invalide (CSRF)"))
)]
#[debug_handler]
async fn admin_create_pin(
    _auth: AdminAuth,
    State(ctx): State<AppContext>,
    Path((id, n)): Path<(i32, i32)>,
    Json(body): Json<crate::dto::AdminCreatePinReq>,
) -> Result<Response> {
    use crate::services::comments::{ADMIN_AUTHOR, ADMIN_OWNER_TOKEN};
    let version = find_version(&ctx, id, n).await?;
    let svc = crate::services::comments::CommentsService::new(ctx.db.clone());
    let pwm = svc
        .create_pin(version.id, ADMIN_OWNER_TOKEN, ADMIN_AUTHOR, &body.body, &body.anchor)
        .await
        .map_err(into_response)?;
    format::json(crate::dto::to_admin_comment_pin(&pwm.pin, &pwm.messages))
}

/// POST /api/projects/{id}/comments/pins/{pin}/replies — l'admin répond à un fil (visiteur ou sien).
#[utoipa::path(
    post, path = "/api/projects/{id}/comments/pins/{pin}/replies", tag = "versions",
    params(("id" = i32, Path, description = "Identifiant du projet"),
           ("pin" = i32, Path, description = "Identifiant du pin")),
    request_body = crate::dto::AdminReplyReq,
    responses((status = 200, description = "Réponse ajoutée", body = crate::dto::AdminCommentMessage),
              (status = 404, description = "Pin hors projet ou inconnu"),
              (status = 401, description = "Non authentifié"),
              (status = 403, description = "Origin invalide (CSRF)"))
)]
#[debug_handler]
async fn admin_reply(
    _auth: AdminAuth,
    State(ctx): State<AppContext>,
    Path((id, pin)): Path<(i32, i32)>,
    Json(body): Json<crate::dto::AdminReplyReq>,
) -> Result<Response> {
    let svc = crate::services::comments::CommentsService::new(ctx.db.clone());
    let msg = svc.admin_add_reply(id, pin, &body.body).await.map_err(into_response)?;
    format::json(crate::dto::to_admin_comment_message(&msg))
}

/// PUT /api/projects/{id}/comments/messages/{cid} — l'admin édite un de SES messages.
#[utoipa::path(
    put, path = "/api/projects/{id}/comments/messages/{cid}", tag = "versions",
    params(("id" = i32, Path, description = "Identifiant du projet"),
           ("cid" = i32, Path, description = "Identifiant du message")),
    request_body = crate::dto::EditMessageReq,
    responses((status = 200, description = "Message modifié", body = crate::dto::AdminCommentMessage),
              (status = 404, description = "Message étranger ou inconnu"),
              (status = 401, description = "Non authentifié"),
              (status = 403, description = "Origin invalide (CSRF)"))
)]
#[debug_handler]
async fn admin_edit_comment(
    _auth: AdminAuth,
    State(ctx): State<AppContext>,
    Path((_id, cid)): Path<(i32, i32)>,
    Json(body): Json<crate::dto::EditMessageReq>,
) -> Result<Response> {
    use crate::services::comments::ADMIN_OWNER_TOKEN;
    let svc = crate::services::comments::CommentsService::new(ctx.db.clone());
    let msg = svc
        .edit_message(cid, ADMIN_OWNER_TOKEN, &body.body)
        .await
        .map_err(into_response)?;
    format::json(crate::dto::to_admin_comment_message(&msg))
}

/// DELETE /api/projects/{id}/comments/pins/{pin} — l'admin supprime un de SES fils.
#[utoipa::path(
    delete, path = "/api/projects/{id}/comments/pins/{pin}", tag = "versions",
    params(("id" = i32, Path, description = "Identifiant du projet"),
           ("pin" = i32, Path, description = "Identifiant du pin")),
    responses((status = 200, description = "Fil supprimé", body = OkResponse),
              (status = 404, description = "Pin étranger ou inconnu"),
              (status = 401, description = "Non authentifié"),
              (status = 403, description = "Origin invalide (CSRF)"))
)]
#[debug_handler]
async fn admin_delete_pin(
    _auth: AdminAuth,
    State(ctx): State<AppContext>,
    Path((_id, pin)): Path<(i32, i32)>,
) -> Result<Response> {
    use crate::services::comments::ADMIN_OWNER_TOKEN;
    let svc = crate::services::comments::CommentsService::new(ctx.db.clone());
    svc.delete_pin(pin, ADMIN_OWNER_TOKEN).await.map_err(into_response)?;
    format::json(crate::dto::OkResponse::ok())
}
```

> **Note owner-check** : `admin_edit_comment`/`admin_delete_pin` passent `ADMIN_OWNER_TOKEN` à `edit_message`/`delete_pin`, dont le `secure_compare` interne restreint l'admin à ses propres messages/pins (message ou pin visiteur → `NotFound`). Le `_id` du path n'est pas re-vérifié ici (l'owner sentinelle suffit à scoper) — cohérent avec le fait que la sentinelle n'appartient qu'à l'admin.

- [ ] **Step 4 : Enregistrer les routes** (dans `routes()`, à côté des routes commentaires existantes)

```rust
        .add(
            "/projects/{id}/versions/{n}/comments",
            post(admin_create_pin).layer(from_fn(require_same_origin)),
        )
        .add(
            "/projects/{id}/comments/pins/{pin}/replies",
            post(admin_reply).layer(from_fn(require_same_origin)),
        )
        .add(
            "/projects/{id}/comments/messages/{cid}",
            put(admin_edit_comment).layer(from_fn(require_same_origin)),
        )
        .add(
            "/projects/{id}/comments/pins/{pin}",
            axum::routing::delete(admin_delete_pin).layer(from_fn(require_same_origin)),
        )
```

> `POST` sur `/projects/{id}/versions/{n}/comments` et `PUT`/`DELETE` sur les chemins de messages/pins sont fusionnés par axum avec les verbes existants (GET liste, DELETE modération). Vérifier que `put` est importé (`use axum::routing::{get, post, put};` en tête — l'ajouter si absent).

- [ ] **Step 5 : Enregistrer paths + schémas OpenAPI** (`backend/src/openapi.rs`)

Dans `paths(...)` ajouter :
```rust
        admin::admin_create_pin,
        admin::admin_reply,
        admin::admin_edit_comment,
        admin::admin_delete_pin,
```
Dans `components(schemas(...))` ajouter :
```rust
        dto::AdminCreatePinReq,
        dto::AdminReplyReq,
```

- [ ] **Step 6 : Régénérer `openapi.json` + relancer les tests**

```bash
cd backend && UPDATE_OPENAPI=1 cargo test --test openapi_drift
cargo test --test openapi_drift
cargo test --test comments_admin
```
Expected: `openapi_drift` PASS ; `comments_admin` PASS (dont les 3 nouveaux tests).

- [ ] **Step 7 : Suite backend complète + fmt/clippy + commit**

```bash
cd backend && cargo fmt && cargo clippy --all-targets -- -D warnings && cargo nextest run
git add backend/src/controllers/admin.rs backend/src/openapi.rs openapi.json
git commit -m "✨ feat(comments): endpoints admin create/reply/edit/delete + OpenAPI"
```
Expected: `cargo nextest run` tout vert.

---

## Task 4 : Régénérer `schema.d.ts` (client typé front)

**Files:**
- Modify: `frontend/src/api/schema.d.ts` (généré)

**Interfaces:**
- Consumes: `openapi.json` (Tasks 2-3).
- Produces: types front `CommentMessage.is_admin`, `AdminCommentMessage.is_admin`, `AdminCreatePinReq`, `AdminReplyReq`, chemins admin.

- [ ] **Step 1 : Régénérer**

Run: `cd frontend && pnpm gen:api`

- [ ] **Step 2 : Vérifier la présence des nouveaux champs/chemins**

Run: `cd frontend && rtk grep -n "is_admin\|AdminCreatePinReq\|AdminReplyReq" src/api/schema.d.ts`
Expected: matches présents.

- [ ] **Step 3 : typecheck (peut déjà signaler l'adaptateur admin — normal, corrigé Task 5)**

Run: `cd frontend && pnpm typecheck`
Expected: 0 erreur *dans schema.d.ts* (les erreurs éventuelles viendront des consommateurs à mettre à jour Task 5+).

- [ ] **Step 4 : Commit**

```bash
git add frontend/src/api/schema.d.ts
git commit -m "🔧 chore(comments): regen schema.d.ts (is_admin + endpoints admin)"
```

---

## Task 5 : Frontend — adaptateur admin en authoring complet + seam `fixedAuthorName`

**Files:**
- Modify: `frontend/src/comments/data/adapter.ts` (interface)
- Modify: `frontend/src/comments/data/admin-adapter.ts`
- Modify: `frontend/src/comments/data/visitor-adapter.ts`
- Modify: `frontend/src/routes/review.tsx:23`
- Test: `frontend/src/comments/data/admin-adapter.test.ts`

**Interfaces:**
- Consumes: `schema.d.ts` (Task 4).
- Produces: `CommentsAdapter.fixedAuthorName: string | null` ; `createAdminAdapter(projectId, n, authorLabel)`.

- [ ] **Step 1 : Écrire/mettre à jour les tests adaptateur admin** (remplacer `admin-adapter.test.ts`)

```ts
import { it, expect, vi } from 'vitest'
import { createAdminAdapter } from './admin-adapter'
import type { Mock } from 'vitest'

vi.mock('@/api/client', () => ({
  api: { GET: vi.fn(), POST: vi.fn(), PUT: vi.fn(), DELETE: vi.fn() },
}))
import { api } from '@/api/client'

it('list() mappe is_admin -> editable', async () => {
  ;(api.GET as Mock).mockResolvedValue({
    data: { version: 2, pins: [
      { id: 7, anchor: '{}', created_at: 'x', messages: [
        { id: 11, author_name: 'admin', body: 'hi', created_at: 'a', updated_at: 'b', is_admin: true },
        { id: 12, author_name: 'Lea', body: 'yo', created_at: 'a', updated_at: 'b', is_admin: false },
      ] },
    ] },
    error: undefined,
  })
  const out = await createAdminAdapter(3, 2, 'Admin').list()
  expect(out.pins[0].messages[0].editable).toBe(true)   // message admin
  expect(out.pins[0].messages[0].is_admin).toBe(true)
  expect(out.pins[0].messages[1].editable).toBe(false)  // message visiteur
})

it('createPin POSTs anchor+body (pas de author_name)', async () => {
  ;(api.POST as Mock).mockResolvedValue({
    data: { id: 9, anchor: '{}', created_at: 'x', messages: [] }, error: undefined,
  })
  await createAdminAdapter(3, 2, 'Admin').createPin({ anchor: '{"v":1}', author_name: 'ignoré', body: 'note' })
  expect(api.POST).toHaveBeenCalledWith('/api/projects/{id}/versions/{n}/comments', {
    params: { path: { id: 3, n: 2 } },
    body: { anchor: '{"v":1}', body: 'note' },
  })
})

it('addReply POSTs body au pin', async () => {
  ;(api.POST as Mock).mockResolvedValue({
    data: { id: 15, author_name: 'admin', body: 'r', created_at: 'a', updated_at: 'b', is_admin: true },
    error: undefined,
  })
  await createAdminAdapter(3, 2, 'Admin').addReply(7, { author_name: 'ignoré', body: 'r' })
  expect(api.POST).toHaveBeenCalledWith('/api/projects/{id}/comments/pins/{pin}/replies', {
    params: { path: { id: 3, pin: 7 } },
    body: { body: 'r' },
  })
})

it('editMessage PUTs body', async () => {
  ;(api.PUT as Mock).mockResolvedValue({
    data: { id: 11, author_name: 'admin', body: 'edited', created_at: 'a', updated_at: 'b', is_admin: true },
    error: undefined,
  })
  await createAdminAdapter(3, 2, 'Admin').editMessage(11, 'edited')
  expect(api.PUT).toHaveBeenCalledWith('/api/projects/{id}/comments/messages/{cid}', {
    params: { path: { id: 3, cid: 11 } },
    body: { body: 'edited' },
  })
})

it('deletePin DELETEs le pin propre', async () => {
  ;(api.DELETE as Mock).mockResolvedValue({ error: undefined })
  await createAdminAdapter(3, 2, 'Admin').deletePin(7)
  expect(api.DELETE).toHaveBeenCalledWith('/api/projects/{id}/comments/pins/{pin}', {
    params: { path: { id: 3, pin: 7 } },
  })
})

it('capabilities = authoring complet + moderation, fixedAuthorName = label', () => {
  const a = createAdminAdapter(1, 1, 'Admin')
  expect(a.capabilities).toEqual({ canAuthor: true, canEditOwn: true, canModerate: true })
  expect(a.fixedAuthorName).toBe('Admin')
})
```

- [ ] **Step 2 : Lancer, vérifier l'échec**

Run: `cd frontend && pnpm vitest run src/comments/data/admin-adapter.test.ts`
Expected: FAIL.

- [ ] **Step 3 : Étendre l'interface `CommentsAdapter`** (`adapter.ts`, après `readonly capabilities: Capabilities`)

```ts
  /** Nom d'auteur imposé (admin) ; `null` = l'appelant saisit son nom (visiteur). */
  readonly fixedAuthorName: string | null
```

- [ ] **Step 4 : Réécrire `admin-adapter.ts`**

```ts
import { api } from '@/api/client'
import type {
  Capabilities,
  CommentList,
  CommentMessage,
  CommentPin,
  CommentsAdapter,
} from './adapter'
import type { components } from '@/api/schema'

type AdminCommentMessage = components['schemas']['AdminCommentMessage']
type AdminCommentPin = components['schemas']['AdminCommentPin']

const ADMIN_CAPS: Readonly<Capabilities> = Object.freeze({
  canAuthor: true,
  canEditOwn: true,
  canModerate: true,
})

function toMessage(m: AdminCommentMessage): CommentMessage {
  return {
    id: m.id,
    author_name: m.author_name,
    body: m.body,
    created_at: m.created_at,
    updated_at: m.updated_at,
    editable: m.is_admin,
    is_admin: m.is_admin,
  }
}

function toPin(p: AdminCommentPin): CommentPin {
  return {
    id: p.id,
    anchor: p.anchor,
    created_at: p.created_at,
    messages: p.messages.map(toMessage),
  }
}

export function createAdminAdapter(
  projectId: number,
  n: number,
  authorLabel: string,
): CommentsAdapter {
  return {
    capabilities: ADMIN_CAPS,
    fixedAuthorName: authorLabel,

    async list(): Promise<CommentList> {
      const { data, error } = await api.GET('/api/projects/{id}/versions/{n}/comments', {
        params: { path: { id: projectId, n } },
      })
      if (error || !data) throw new Error('comments:admin:list')
      return { version: data.version, pins: data.pins.map(toPin) }
    },

    async createPin(input): Promise<CommentPin> {
      const { data, error } = await api.POST('/api/projects/{id}/versions/{n}/comments', {
        params: { path: { id: projectId, n } },
        body: { anchor: input.anchor, body: input.body },
      })
      if (error || !data) throw new Error('comments:admin:createPin')
      return toPin(data)
    },

    async addReply(pinId, input): Promise<CommentMessage> {
      const { data, error } = await api.POST('/api/projects/{id}/comments/pins/{pin}/replies', {
        params: { path: { id: projectId, pin: pinId } },
        body: { body: input.body },
      })
      if (error || !data) throw new Error('comments:admin:addReply')
      return toMessage(data)
    },

    async editMessage(messageId, body): Promise<CommentMessage> {
      const { data, error } = await api.PUT('/api/projects/{id}/comments/messages/{cid}', {
        params: { path: { id: projectId, cid: messageId } },
        body: { body },
      })
      if (error || !data) throw new Error('comments:admin:editMessage')
      return toMessage(data)
    },

    async deleteMessage(messageId: number): Promise<void> {
      const { error } = await api.DELETE('/api/projects/{id}/comments/messages/{cid}', {
        params: { path: { id: projectId, cid: messageId } },
      })
      if (error) throw new Error('comments:admin:deleteMessage')
    },

    async deletePin(pinId: number): Promise<void> {
      const { error } = await api.DELETE('/api/projects/{id}/comments/pins/{pin}', {
        params: { path: { id: projectId, pin: pinId } },
      })
      if (error) throw new Error('comments:admin:deletePin')
    },
  }
}
```

- [ ] **Step 5 : `visitor-adapter.ts` — ajouter `fixedAuthorName: null`**

Dans l'objet retourné par `createVisitorAdapter`, après `capabilities: VISITOR_CAPS,` :
```ts
    fixedAuthorName: null,
```

- [ ] **Step 6 : `review.tsx` — passer le libellé i18n**

Ligne 23, remplacer :
```ts
  const adapter = useMemo(() => createAdminAdapter(Number(id), Number(n), t('comment.admin_author')), [id, n, t])
```

- [ ] **Step 7 : Lancer les tests adaptateur + typecheck**

Run: `cd frontend && pnpm vitest run src/comments/data/admin-adapter.test.ts && pnpm typecheck`
Expected: adaptateur PASS. Le typecheck peut encore signaler les **mocks** de `comments-app.test.tsx`/`use-comments.test.tsx` (adaptateur factice sans `fixedAuthorName`) → corrigés Task 6.

- [ ] **Step 8 : Commit**

```bash
git add frontend/src/comments/data/adapter.ts frontend/src/comments/data/admin-adapter.ts frontend/src/comments/data/visitor-adapter.ts frontend/src/routes/review.tsx frontend/src/comments/data/admin-adapter.test.ts
git commit -m "✨ feat(comments): adaptateur admin authoring + seam fixedAuthorName"
```

---

## Task 6 : Frontend — `ComposePopup` sans champ nom pour l'admin + câblage + i18n

**Files:**
- Modify: `frontend/src/comments/ui/compose-popup.tsx`
- Modify: `frontend/src/comments/comments-app.tsx`
- Modify: `frontend/src/comments/comments-app.test.tsx` + `frontend/src/comments/data/use-comments.test.tsx` (mocks : ajouter `fixedAuthorName`)
- Modify: `frontend/src/i18n/locales/comments/{en,fr}.json`
- Test: `frontend/src/comments/ui/compose-popup.test.tsx`

**Interfaces:**
- Consumes: `CommentsAdapter.fixedAuthorName` (Task 5).
- Produces: `ComposePopup` prop `fixedAuthorName: string | null`.

- [ ] **Step 1 : Ajouter les clés i18n**

`frontend/src/i18n/locales/comments/en.json` (sous l'objet `comment.compose`) :
```json
      "as_label": "Commenting as {{name}}"
```
et sous `comment` : `"admin_author": "Admin"`, `"admin_badge": "Admin"`.

`frontend/src/i18n/locales/comments/fr.json` :
```json
      "as_label": "Vous commentez en tant que {{name}}"
```
et `"admin_author": "Admin"`, `"admin_badge": "Admin"`.

> Respecter la structure existante du fichier (clés imbriquées sous `comment`). Vérifier l'orthographe FR (accents).

- [ ] **Step 2 : Écrire le test compose qui échoue** (`compose-popup.test.tsx`, nouveau fichier — s'inspirer du render existant des tests UI voisins ; wrap `I18nextProvider` via `renderWithProviders` de `@/test/utils` si présent, sinon `render` + `useTranslation` réel)

```tsx
import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { ComposePopup } from './compose-popup'

describe('ComposePopup', () => {
  const base = { point: { x: 0, y: 0 }, submitting: false, onSubmit: vi.fn(), onCancel: vi.fn() }

  it('affiche le champ nom quand fixedAuthorName est null (visiteur)', () => {
    render(<ComposePopup {...base} fixedAuthorName={null} />)
    expect(screen.getByLabelText(/name/i)).toBeInTheDocument()
  })

  it('masque le champ nom quand fixedAuthorName est fourni (admin)', () => {
    render(<ComposePopup {...base} fixedAuthorName="Admin" />)
    expect(screen.queryByLabelText(/name/i)).not.toBeInTheDocument()
  })
})
```

> Si le repo n'a pas encore `@testing-library/jest-dom` importé globalement, remplacer `toBeInTheDocument()` par `screen.queryByLabelText(...) === null` selon le style des tests UI voisins (`thread-popup.test.tsx`). Aligner sur l'existant.

- [ ] **Step 2b : Lancer, vérifier l'échec**

Run: `cd frontend && pnpm vitest run src/comments/ui/compose-popup.test.tsx`
Expected: FAIL — prop `fixedAuthorName` inexistante.

- [ ] **Step 3 : Modifier `ComposePopup`**

Ajouter à `ComposePopupProps` : `fixedAuthorName: string | null`.
Signature : `export function ComposePopup({ point, submitting, onSubmit, onCancel, fixedAuthorName }: Readonly<ComposePopupProps>) {`.
Initialiser le nom : `const [name, setName] = useState(fixedAuthorName ?? getStoredName())`.
Dans `submit()`, ne pas exiger de saisie quand imposé :
```ts
  function submit() {
    const trimmedName = (fixedAuthorName ?? name).trim()
    const trimmedBody = body.trim()
    if (!trimmedName) return setError(t('comment.error.name_required'))
    if (!trimmedBody) return setError(t('comment.error.body_required'))
    if (trimmedBody.length > MAX_BODY) return setError(t('comment.error.body_too_long'))
    if (!fixedAuthorName) setStoredName(trimmedName)
    onSubmit({ author_name: trimmedName, body: trimmedBody })
  }
```
Remplacer le bloc `Label`+`Input` du nom par un rendu conditionnel :
```tsx
        {fixedAuthorName ? (
          <p className="text-muted-foreground text-xs">
            {t('comment.compose.as_label', { name: fixedAuthorName })}
          </p>
        ) : (
          <>
            <Label htmlFor="comment-name">{t('comment.compose.name_label')}</Label>
            <Input
              id="comment-name"
              value={name}
              placeholder={t('comment.compose.name_placeholder')}
              onChange={(e) => { setName(e.target.value); setError(null) }}
            />
          </>
        )}
```

- [ ] **Step 4 : Câbler `comments-app.tsx`**

Passer la prop au `ComposePopup` (dans le bloc `pick.mode === 'compose'`) :
```tsx
        <ComposePopup
          point={anchorPoint(pick.rect, pick.anchor.offset)}
          submitting={createPin.isPending}
          fixedAuthorName={adapter.fixedAuthorName}
          onSubmit={submitNewComment}
          onCancel={() => dispatch({ type: 'CANCEL' })}
        />
```
Corriger le nom d'auteur des réponses (fonction `onReply` du `ThreadPopup`) pour respecter l'identité imposée :
```tsx
          onReply={(body) =>
            addReply.mutate({
              pinId: activePin.id,
              author_name: adapter.fixedAuthorName ?? (getStoredName() || lastAuthor(activePin)),
              body,
            })
          }
```

- [ ] **Step 5 : Mettre à jour les mocks d'adaptateur des tests**

Dans `comments-app.test.tsx` et `use-comments.test.tsx`, ajouter au mock adaptateur (à côté de `capabilities: {...}`) :
```ts
    fixedAuthorName: null,
```

- [ ] **Step 6 : Lancer les tests concernés + typecheck**

Run: `cd frontend && pnpm vitest run src/comments && pnpm typecheck`
Expected: PASS ; typecheck 0 erreur.

- [ ] **Step 7 : Commit**

```bash
git add frontend/src/comments/ui/compose-popup.tsx frontend/src/comments/ui/compose-popup.test.tsx frontend/src/comments/comments-app.tsx frontend/src/comments/comments-app.test.tsx frontend/src/comments/data/use-comments.test.tsx frontend/src/i18n/locales/comments/en.json frontend/src/i18n/locales/comments/fr.json
git commit -m "✨ feat(comments): compose admin sans champ nom (identite imposee) + i18n"
```

---

## Task 7 : Frontend — libellé + badge « Admin » (fil + drawer)

**Files:**
- Modify: `frontend/src/comments/ui/thread-popup.tsx:72`
- Modify: `frontend/src/comments/ui/comments-drawer.tsx:77`
- Test: `frontend/src/comments/ui/thread-popup.test.tsx`

**Interfaces:**
- Consumes: `CommentMessage.is_admin` (Task 4) ; clés `comment.admin_author`/`comment.admin_badge` (Task 6).

- [ ] **Step 1 : Écrire le test badge qui échoue** (`thread-popup.test.tsx`, ajouter un cas)

```tsx
it('affiche le libellé Admin + badge sur un message is_admin', () => {
  const adminPin = {
    id: 1, anchor: '{}', created_at: 'n',
    messages: [
      { id: 9, author_name: 'admin', body: 'note', created_at: '', updated_at: '', editable: true, is_admin: true },
    ],
  }
  renderThread({ pin: adminPin, capabilities: { canAuthor: true, canEditOwn: true, canModerate: true } })
  expect(screen.getByText('Admin')).toBeInTheDocument()
  // Le nom brut 'admin' (stocké) ne doit PAS s'afficher tel quel comme auteur.
  expect(screen.queryByText('admin')).not.toBeInTheDocument()
})
```

> Adapter `renderThread`/le shape des props au harness existant du fichier (mêmes helpers que les autres `it`). Ajouter `is_admin` aux messages des fixtures existantes du fichier (`is_admin: false`) pour satisfaire le type `CommentMessage`.

- [ ] **Step 2 : Lancer, vérifier l'échec**

Run: `cd frontend && pnpm vitest run src/comments/ui/thread-popup.test.tsx`
Expected: FAIL.

- [ ] **Step 3 : Afficher libellé + badge dans `thread-popup.tsx`**

Ajouter l'import (en tête) : `import { Badge } from '@/components/ui/badge'`.
Remplacer la ligne 72 (`<span className="text-xs font-semibold">{m.author_name}</span>`) par :
```tsx
                <span className="flex items-center gap-1 text-xs font-semibold">
                  {m.is_admin ? t('comment.admin_author') : m.author_name}
                  {m.is_admin && (
                    <Badge variant="secondary" className="px-1 py-0 text-[10px] leading-tight">
                      {t('comment.admin_badge')}
                    </Badge>
                  )}
                </span>
```

- [ ] **Step 4 : Idem dans `comments-drawer.tsx`**

Ligne 77 : `const author = pin.messages[0]?.author_name ?? ''`. Remplacer par :
```tsx
  const firstMsg = pin.messages[0]
  const author = firstMsg?.is_admin ? t('comment.admin_author') : (firstMsg?.author_name ?? '')
```
> Vérifier que `t` est disponible dans le scope où `author` est calculé (le composant utilise déjà `useTranslation` — cf. `formatDateTime`/clés existantes). Si `author` est calculé hors composant, déplacer le calcul dans le rendu de l'item. Un badge dans le drawer est optionnel (le libellé « Admin » suffit à la liste) ; ne pas l'ajouter si ça alourdit — l'exigence badge porte sur le fil.

- [ ] **Step 5 : Lancer les tests + lint + typecheck**

Run: `cd frontend && pnpm vitest run src/comments && pnpm lint && pnpm typecheck`
Expected: tout vert, 0 erreur.

- [ ] **Step 6 : Commit**

```bash
git add frontend/src/comments/ui/thread-popup.tsx frontend/src/comments/ui/thread-popup.test.tsx frontend/src/comments/ui/comments-drawer.tsx
git commit -m "💄 ui(comments): libelle + badge Admin sur les messages admin"
```

---

## Task 8 : e2e Playwright — l'admin crée un fil et répond à un visiteur

**Files:**
- Modify: `frontend/e2e/comments-admin.spec.ts`

**Interfaces:**
- Consumes: endpoints admin (Task 3) ; UI authoring (Tasks 5-7). Réutilise les helpers `apiLogin`/`pageLogin` du fichier.

- [ ] **Step 1 : Ajouter un test e2e**

S'appuyer sur le harness existant du fichier (login, création projet, deploy, seed d'un commentaire visiteur via `#cta`, navigation `reviewPath`). Ajouter un test qui :
1. seed un fil visiteur (API directe, comme le test de modération existant) ;
2. ouvre la Review admin (`pageLogin` + navigation) ;
3. clique le pin, tape une réponse dans le composer, soumet ;
4. assert que la réponse apparaît dans le fil avec le libellé « Admin ».

```ts
test('admin répond à un fil visiteur depuis la Review', async ({ page, request, baseURL }) => {
  const { id, slug, n } = await seedProjectWithVisitorComment(page, request, baseURL) // helper local existant/à extraire
  await pageLogin(page)
  await page.goto(`/admin/projects/${id}/versions/${n}/review`)
  // Ouvrir le fil : cliquer le pin dans l'overlay.
  await page.getByTestId('pin-badge').first().click()
  await page.getByRole('textbox').last().fill('Réponse de l’équipe')
  await page.getByRole('button', { name: /reply|répondre/i }).click()
  await expect(page.getByText('Réponse de l’équipe')).toBeVisible()
  await expect(page.getByText('Admin').first()).toBeVisible()
})
```

> Adapter les sélecteurs aux `data-testid`/roles réels (le fichier existant montre les conventions : `getByTestId('pin-badge')`, `pageLogin`, seed API `#cta`). Si un helper de seed n'existe pas encore, l'extraire du test de modération voisin plutôt que dupliquer (DRY).

- [ ] **Step 2 : Lancer le sous-ensemble e2e**

Run: `cd frontend && pnpm exec playwright test comments-admin`
Expected: PASS (nouveaux + existants).

- [ ] **Step 3 : Commit**

```bash
git add frontend/e2e/comments-admin.spec.ts
git commit -m "✅ test(comments): e2e authoring admin (reponse a un fil visiteur)"
```

---

## Task 9 : Documentation + mémoire projet

**Files:**
- Modify: `docs/contrat-deploy.md` (§6.4/§7/§9)
- Modify: `public_docs/content/docs/admin/comments.mdx`
- Modify: `docs/INDEX.md`, `docs/HANDOFF.md`

- [ ] **Step 1 : Contrat** — documenter les 4 endpoints admin d'écriture (§6.4/§7), la sentinelle `ADMIN_OWNER_TOKEN` et le booléen dérivé `is_admin` (§9), en réaffirmant l'invariant `owner_token` jamais sérialisé. Placeholders génériques uniquement.

- [ ] **Step 2 : Doc publique** — dans `admin/comments.mdx`, ajouter une section « Laisser des commentaires en tant qu'admin » : créer un fil (note privée, visible seulement en Review), répondre à un fil visiteur (visible du visiteur), éditer/supprimer ses messages, badge « Admin ».

- [ ] **Step 3 : Mémoire** — `docs/INDEX.md` : ligne livrable « Authoring commentaires admin » (Phase 10). `docs/HANDOFF.md` : entrée datée en tête (Dernière chose faite / Trucs en suspens / Notes pour future Claude — mentionner la sentinelle `ADMIN_OWNER_TOKEN` et le seam `fixedAuthorName`).

- [ ] **Step 4 : Gate finale complète (Définition de « terminé »)**

```bash
cd backend && cargo fmt && cargo clippy --all-targets -- -D warnings && cargo nextest run
cd ../frontend && pnpm lint && pnpm typecheck && pnpm test && pnpm exec playwright test
```
Expected: tout vert.

- [ ] **Step 5 : Commit**

```bash
git add docs/contrat-deploy.md public_docs/content/docs/admin/comments.mdx docs/INDEX.md docs/HANDOFF.md
git commit -m "📝 docs(comments): authoring admin (contrat + doc publique + memoire)"
```

---

## Self-Review (à jour au moment de l'écriture)

- **Couverture spec** : §2 sentinelle → Task 1 ; §3 service (create/reply/edit/delete) → Tasks 1,3 ; §4 DTO `is_admin` → Task 2 ; §5 endpoints → Task 3 ; §6.1 adaptateur → Task 5 ; §6.2 `fixedAuthorName`/compose → Tasks 5,6 ; §6.3 badge → Task 7 ; §6.4 i18n → Tasks 6,7 ; §7 sécurité → Global Constraints + Tasks 1,3 ; §8 tests → Tasks 1,2,3,5,6,7,8 ; §9 doc → Task 9.
- **Type consistency** : `admin_add_reply(project_id, pin_id, body)` (Task 1) appelé identiquement en Task 3 ; `to_admin_comment_message` produit Task 2, consommé Task 3 ; `fixedAuthorName` défini Task 5, consommé Tasks 5-6 ; `createAdminAdapter(projectId, n, authorLabel)` — 3 args cohérents Tasks 5 + review.tsx.
- **Écart assumé vs spec** : `fixedAuthorName` posé sur `CommentsAdapter` (et non `Capabilities`) pour minimiser la churn des littéraux `capabilities` de test — même effet fonctionnel.
