//! Adaptateur entrant "web admin". Chaque handler : auth via `AdminAuth`,
//! appelle un service du cœur, mappe `CoreError` → HTTP (error::into_response),
//! sérialise un DTO. Aucune logique métier ici.

use axum::middleware::from_fn;
use axum::response::IntoResponse;
use loco_rs::prelude::*;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};

use crate::controllers::auth::AdminAuth;
use crate::controllers::error::into_response;
use crate::controllers::middleware::origin::require_same_origin;
use crate::dto::{
    ActivateResponse, AdminCommentList, CreateProjectReq, DeployReq, DeployResponse, OkResponse,
    ProjectDetail, ProjectListItem, SetCodeReq, UpdateProjectReq,
};
use crate::models::_entities::{projects, versions};
use crate::services::deploy::DeployService;
use crate::services::projects::{CreateProject, ProjectsService};
use crate::web::extract::ValidatedJson;

/// Charge les versions d'un projet + compte de commentaires et renvoie le détail JSON complet.
/// Partagé par `detail` et `update` (toute handler retournant `ProjectDetail` avec versions).
async fn project_detail_json(
    ctx: &AppContext,
    project: projects::Model,
    id: i32,
) -> Result<Response> {
    let vers = versions::Entity::find()
        .filter(versions::Column::ProjectId.eq(id))
        .order_by_desc(versions::Column::N)
        .all(&ctx.db)
        .await
        .map_err(|e| into_response(e.into()))?;
    let svc = crate::services::comments::CommentsService::new(ctx.db.clone());
    let version_ids: Vec<i32> = vers.iter().map(|v| v.id).collect();
    let counts = svc
        .count_comments_by_version(&version_ids)
        .await
        .map_err(into_response)?;
    format::json(crate::dto::to_detail(project, vers, &counts))
}

/// Charge un projet par `id` → `loco_rs::Error::NotFound` si absent.
async fn find_project(ctx: &AppContext, id: i32) -> Result<projects::Model> {
    projects::Entity::find_by_id(id)
        .one(&ctx.db)
        .await
        .map_err(|e| into_response(e.into()))?
        .ok_or(loco_rs::Error::NotFound)
}

/// Charge une version par `(project_id, n)` → `loco_rs::Error::NotFound` si absente.
async fn find_version(ctx: &AppContext, id: i32, n: i32) -> Result<versions::Model> {
    versions::Entity::find()
        .filter(versions::Column::ProjectId.eq(id))
        .filter(versions::Column::N.eq(n))
        .one(&ctx.db)
        .await
        .map_err(|e| into_response(e.into()))?
        .ok_or(loco_rs::Error::NotFound)
}

/// GET /api/projects — liste tous les projets (sans PIN, invariant §9.2).
#[utoipa::path(
    get, path = "/api/projects", tag = "projects",
    responses((status = 200, description = "Liste des projets (sans PIN)", body = Vec<ProjectListItem>),
              (status = 401, description = "Non authentifié"))
)]
#[debug_handler]
async fn list(_auth: AdminAuth, State(ctx): State<AppContext>) -> Result<Response> {
    let svc = ProjectsService::new(ctx.db.clone());
    let rows = svc.list_with_versions().await.map_err(into_response)?;
    let items: Vec<ProjectListItem> = rows
        .iter()
        .map(|(p, vers)| crate::dto::to_list_item(p, vers))
        .collect();
    format::json(items)
}

/// GET /api/projects/{id} — détail d'un projet avec ses versions et son PIN.
#[utoipa::path(
    get, path = "/api/projects/{id}", tag = "projects",
    params(("id" = i32, Path, description = "Identifiant du projet")),
    responses((status = 200, description = "Détail du projet (avec PIN)", body = ProjectDetail),
              (status = 404, description = "Projet inconnu"),
              (status = 401, description = "Non authentifié"))
)]
#[debug_handler]
async fn detail(
    _auth: AdminAuth,
    State(ctx): State<AppContext>,
    Path(id): Path<i32>,
) -> Result<Response> {
    let project = find_project(&ctx, id).await?;

    project_detail_json(&ctx, project, id).await
}

/// POST /api/projects — crée un nouveau projet.
// Requiert un Origin same-origin (garde CSRF, contrat §4/§9.6).
#[utoipa::path(
    post, path = "/api/projects", tag = "projects",
    request_body = CreateProjectReq,
    responses((status = 200, description = "Projet créé", body = ProjectDetail),
              (status = 400, description = "Requête invalide"),
              (status = 401, description = "Non authentifié"),
              (status = 403, description = "Origin invalide (CSRF)"))
)]
#[debug_handler]
async fn create(
    _auth: AdminAuth,
    State(ctx): State<AppContext>,
    ValidatedJson(body): ValidatedJson<CreateProjectReq>,
) -> Result<Response> {
    let svc = ProjectsService::new(ctx.db.clone());
    let comments_enabled = body.comments_enabled.unwrap_or(body.code_enabled);
    let project = svc
        .create(CreateProject {
            name: body.name,
            brand_name: body.brand_name,
            code_enabled: body.code_enabled,
            pin: body.pin,
            comments_enabled,
        })
        .await
        .map_err(into_response)?;
    format::json(crate::dto::to_detail(
        project,
        vec![],
        &std::collections::HashMap::new(),
    ))
}

/// PUT /api/projects/{id} — met à jour le nom ou le brand_name d'un projet.
// `UpdateProjectReq.brand_name` est `Option<Option<String>>` :
//   - `None` → ne pas toucher au champ
//   - `Some(None)` → effacer la valeur
//   - `Some(Some(x))` → remplacer par `x`
#[utoipa::path(
    put, path = "/api/projects/{id}", tag = "projects",
    params(("id" = i32, Path, description = "Identifiant du projet")),
    request_body = UpdateProjectReq,
    responses((status = 200, description = "Projet mis à jour", body = ProjectDetail),
              (status = 400, description = "Requête invalide"),
              (status = 404, description = "Projet inconnu"),
              (status = 401, description = "Non authentifié"),
              (status = 403, description = "Origin invalide (CSRF)"))
)]
#[debug_handler]
async fn update(
    _auth: AdminAuth,
    State(ctx): State<AppContext>,
    Path(id): Path<i32>,
    ValidatedJson(body): ValidatedJson<UpdateProjectReq>,
) -> Result<Response> {
    let model = find_project(&ctx, id).await?;

    let mut active: projects::ActiveModel = model.into();

    // Forme (non-vide, bornes de longueur) déjà validée à la frontière
    // (`ValidatedJson<UpdateProjectReq>`, contrat §1).
    if let Some(name) = body.name {
        active.name = Set(name);
    }
    if let Some(brand) = body.brand_name {
        active.brand_name = Set(brand);
    }
    if let Some(ce) = body.comments_enabled {
        active.comments_enabled = Set(ce);
    }
    // updated_at posé manuellement (le hook before_save ne s'applique pas hors tx, cf. QUIRKS).
    active.updated_at = Set(chrono::Utc::now().into());

    let saved = active
        .update(&ctx.db)
        .await
        .map_err(|e| into_response(e.into()))?;

    project_detail_json(&ctx, saved, id).await
}

/// DELETE /api/projects/{id} — supprime un projet et ses versions.
// SQLite n'enforce pas les FK sans PRAGMA → suppression explicite en transaction (QUIRKS).
#[utoipa::path(
    delete, path = "/api/projects/{id}", tag = "projects",
    params(("id" = i32, Path, description = "Identifiant du projet")),
    responses((status = 200, description = "Projet supprimé", body = OkResponse),
              (status = 404, description = "Projet inconnu"),
              (status = 401, description = "Non authentifié"),
              (status = 403, description = "Origin invalide (CSRF)"))
)]
#[debug_handler]
async fn delete(
    _auth: AdminAuth,
    State(ctx): State<AppContext>,
    Path(id): Path<i32>,
) -> Result<Response> {
    let txn = ctx.db.begin().await.map_err(|e| into_response(e.into()))?;

    // Supprimer toutes les versions du projet AVANT le projet lui-même (FK non enforced).
    versions::Entity::delete_many()
        .filter(versions::Column::ProjectId.eq(id))
        .exec(&txn)
        .await
        .map_err(|e| into_response(e.into()))?;

    let res = projects::Entity::delete_by_id(id)
        .exec(&txn)
        .await
        .map_err(|e| into_response(e.into()))?;

    // 404 avant le commit : si le projet n'existait pas, la txn se rollback au drop
    // (les versions supprimées sont annulées — correct, elles n'existaient pas non plus).
    if res.rows_affected == 0 {
        return Err(loco_rs::Error::NotFound);
    }

    txn.commit().await.map_err(|e| into_response(e.into()))?;

    format::json(crate::dto::OkResponse::ok())
}

/// POST /api/projects/{id}/code — active le code d'accès avec le PIN fourni.
#[utoipa::path(
    post, path = "/api/projects/{id}/code", tag = "projects",
    params(("id" = i32, Path, description = "Identifiant du projet")),
    request_body = SetCodeReq,
    responses((status = 200, description = "Code activé", body = ProjectDetail),
              (status = 401, description = "Non authentifié"),
              (status = 403, description = "Origin invalide (CSRF)"))
)]
#[debug_handler]
async fn set_code(
    _auth: AdminAuth,
    State(ctx): State<AppContext>,
    Path(id): Path<i32>,
    ValidatedJson(body): ValidatedJson<SetCodeReq>,
) -> Result<Response> {
    let svc = ProjectsService::new(ctx.db.clone());
    let project = svc.set_code(id, &body.pin).await.map_err(into_response)?;
    format::json(crate::dto::to_detail(
        project,
        vec![],
        &std::collections::HashMap::new(),
    ))
}

/// DELETE /api/projects/{id}/code — désactive le code d'accès (PIN effacé).
#[utoipa::path(
    delete, path = "/api/projects/{id}/code", tag = "projects",
    params(("id" = i32, Path, description = "Identifiant du projet")),
    responses((status = 200, description = "Code désactivé", body = ProjectDetail),
              (status = 401, description = "Non authentifié"),
              (status = 403, description = "Origin invalide (CSRF)"))
)]
#[debug_handler]
async fn clear_code(
    _auth: AdminAuth,
    State(ctx): State<AppContext>,
    Path(id): Path<i32>,
) -> Result<Response> {
    let svc = ProjectsService::new(ctx.db.clone());
    let project = svc.clear_code(id).await.map_err(into_response)?;
    format::json(crate::dto::to_detail(
        project,
        vec![],
        &std::collections::HashMap::new(),
    ))
}

/// POST /api/projects/{id}/deploy — déploie un nouveau HTML, crée une version.
// Si `activate=true`, repointe `active_version_id`. Répond `{id, n}`.
#[utoipa::path(
    post, path = "/api/projects/{id}/deploy", tag = "versions",
    params(("id" = i32, Path, description = "Identifiant du projet")),
    request_body = DeployReq,
    responses((status = 200, description = "Version déployée", body = DeployResponse),
              (status = 401, description = "Non authentifié"),
              (status = 403, description = "Origin invalide (CSRF)"))
)]
#[debug_handler]
async fn deploy(
    _auth: AdminAuth,
    State(ctx): State<AppContext>,
    Path(id): Path<i32>,
    ValidatedJson(body): ValidatedJson<DeployReq>,
) -> Result<Response> {
    let storage = crate::web::storage_from_ctx(&ctx);
    let svc = DeployService::new(ctx.db.clone(), storage);
    let version = svc
        .deploy(id, &body.html, body.activate, body.notes.as_deref())
        .await
        .map_err(into_response)?;
    format::json(crate::dto::DeployResponse {
        id: version.id,
        n: version.n,
    })
}

/// POST /api/projects/{id}/versions/{n}/activate — bascule le pointeur actif.
// Charge la version par (project_id, n) → 404 si absente. Met à jour le projet.
#[utoipa::path(
    post, path = "/api/projects/{id}/versions/{n}/activate", tag = "versions",
    params(("id" = i32, Path, description = "Identifiant du projet"),
           ("n" = i32, Path, description = "Numéro de version")),
    responses((status = 200, description = "Version activée", body = ActivateResponse),
              (status = 404, description = "Version inconnue"),
              (status = 401, description = "Non authentifié"),
              (status = 403, description = "Origin invalide (CSRF)"))
)]
#[debug_handler]
async fn activate_version(
    _auth: AdminAuth,
    State(ctx): State<AppContext>,
    Path((id, n)): Path<(i32, i32)>,
) -> Result<Response> {
    let version = find_version(&ctx, id, n).await?;

    let project = find_project(&ctx, id).await?;

    let mut active: projects::ActiveModel = project.into();
    active.active_version_id = Set(Some(version.id));
    // updated_at posé manuellement (cf. QUIRKS hook before_save).
    active.updated_at = Set(chrono::Utc::now().into());
    active
        .update(&ctx.db)
        .await
        .map_err(|e| into_response(e.into()))?;

    format::json(crate::dto::ActivateResponse {
        ok: true,
        active_version_id: version.id,
    })
}

/// DELETE /api/projects/{id}/versions/{n} — supprime une version NON active.
// Charge la version par (project_id, n) → 404 si absente.
// Refuse la suppression si elle est la version active du projet → 400.
// Nettoie le fichier HTML orphelin sur le storage (best-effort, après le DELETE DB).
#[utoipa::path(
    delete, path = "/api/projects/{id}/versions/{n}", tag = "versions",
    params(("id" = i32, Path, description = "Identifiant du projet"),
           ("n" = i32, Path, description = "Numéro de version")),
    responses((status = 200, description = "Version supprimée", body = OkResponse),
              (status = 400, description = "Version active : suppression refusée"),
              (status = 404, description = "Version inconnue"),
              (status = 401, description = "Non authentifié"),
              (status = 403, description = "Origin invalide (CSRF)"))
)]
#[debug_handler]
async fn delete_version(
    _auth: AdminAuth,
    State(ctx): State<AppContext>,
    Path((id, n)): Path<(i32, i32)>,
) -> Result<Response> {
    let version = find_version(&ctx, id, n).await?;

    let project = find_project(&ctx, id).await?;

    // Invariant : ne jamais supprimer la version active (laisserait un pointeur orphelin).
    if project.active_version_id == Some(version.id) {
        return Err(loco_rs::Error::BadRequest(
            "cannot delete the active version".to_string(),
        ));
    }

    versions::Entity::delete_by_id(version.id)
        .exec(&ctx.db)
        .await
        .map_err(|e| into_response(e.into()))?;

    // Nettoyage best-effort du fichier HTML orphelin (idempotent). La ligne DB est
    // déjà supprimée → aucun pointeur ne peut plus le servir (l'invariant qui compte) ;
    // un échec de suppression ne doit donc PAS renvoyer 500 sur une suppression réussie.
    let storage = crate::web::storage_from_ctx(&ctx);
    if let Err(e) = storage.delete(&version.html_path).await {
        tracing::warn!(
            error = %e,
            html_path = %version.html_path,
            "échec du nettoyage du fichier HTML orphelin après delete_version"
        );
    }

    format::json(crate::dto::OkResponse::ok())
}

/// GET /api/projects/{id}/versions/{n}/preview — sert le HTML brut de la version.
// Répond avec `Cache-Control: no-store` pour éviter tout cache navigateur en admin.
// Protégé par `AdminAuth` (pas de garde Origin : GET est idempotent).
// Confirmé via Context7 : `loco_rs::prelude::Response = axum::response::Response` ;
// le tuple `(headers_array, body_string).into_response()` est idiomatique axum 0.8.
#[utoipa::path(
    get, path = "/api/projects/{id}/versions/{n}/preview", tag = "versions",
    params(("id" = i32, Path, description = "Identifiant du projet"),
           ("n" = i32, Path, description = "Numéro de version")),
    responses((status = 200, description = "HTML brut de la version", content_type = "text/html"),
              (status = 404, description = "Version inconnue"),
              (status = 401, description = "Non authentifié"))
)]
#[debug_handler]
async fn preview_version(
    _auth: AdminAuth,
    State(ctx): State<AppContext>,
    Path((id, n)): Path<(i32, i32)>,
) -> Result<Response> {
    let version = find_version(&ctx, id, n).await?;

    let storage = crate::web::storage_from_ctx(&ctx);
    let html = storage
        .read(&version.html_path)
        .await
        .map_err(into_response)?;

    // Réponse brute axum : (tableau de headers, body) → IntoResponse.
    // `axum::response::IntoResponse` est importé en tête de fichier.
    Ok((
        [
            (
                axum::http::header::CACHE_CONTROL,
                axum::http::HeaderValue::from_static("no-store"),
            ),
            (
                axum::http::header::CONTENT_TYPE,
                axum::http::HeaderValue::from_static("text/html; charset=utf-8"),
            ),
            (
                axum::http::header::CONTENT_SECURITY_POLICY,
                axum::http::HeaderValue::from_static("frame-ancestors 'self'"),
            ),
        ],
        html,
    )
        .into_response())
}

/// GET /api/projects/{id}/versions/{n}/comments — tous les fils de la version (lecture seule).
#[utoipa::path(
    get, path = "/api/projects/{id}/versions/{n}/comments", tag = "versions",
    params(("id" = i32, Path, description = "Identifiant du projet"),
           ("n" = i32, Path, description = "Numéro de version")),
    responses((status = 200, description = "Commentaires de la version", body = AdminCommentList),
              (status = 404, description = "Version inconnue"),
              (status = 401, description = "Non authentifié"))
)]
#[debug_handler]
async fn list_version_comments(
    _auth: AdminAuth,
    State(ctx): State<AppContext>,
    Path((id, n)): Path<(i32, i32)>,
) -> Result<Response> {
    let version = find_version(&ctx, id, n).await?;
    let svc = crate::services::comments::CommentsService::new(ctx.db.clone());
    let rows = svc
        .list_for_version(version.id)
        .await
        .map_err(into_response)?;
    let pins = rows
        .iter()
        .map(|pwm| crate::dto::to_admin_comment_pin(&pwm.pin, &pwm.messages))
        .collect();
    format::json(crate::dto::AdminCommentList { version: n, pins })
}

/// DELETE /api/projects/{id}/comments/messages/{cid} — modération (vérifie l'appartenance au projet).
#[utoipa::path(
    delete, path = "/api/projects/{id}/comments/messages/{cid}", tag = "versions",
    params(("id" = i32, Path, description = "Identifiant du projet"),
           ("cid" = i32, Path, description = "Identifiant du message")),
    responses((status = 200, description = "Message supprimé", body = OkResponse),
              (status = 404, description = "Message hors projet ou inconnu"),
              (status = 401, description = "Non authentifié"),
              (status = 403, description = "Origin invalide (CSRF)"))
)]
#[debug_handler]
async fn moderate_delete_comment(
    _auth: AdminAuth,
    State(ctx): State<AppContext>,
    Path((id, cid)): Path<(i32, i32)>,
) -> Result<Response> {
    let svc = crate::services::comments::CommentsService::new(ctx.db.clone());
    svc.moderate_delete_message(id, cid)
        .await
        .map_err(into_response)?;
    format::json(crate::dto::OkResponse::ok())
}

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
    ValidatedJson(body): ValidatedJson<crate::dto::AdminCreatePinReq>,
) -> Result<Response> {
    use crate::services::comments::{ADMIN_AUTHOR, ADMIN_OWNER_TOKEN};
    let version = find_version(&ctx, id, n).await?;
    let svc = crate::services::comments::CommentsService::new(ctx.db.clone());
    let pwm = svc
        .create_pin(
            version.id,
            ADMIN_OWNER_TOKEN,
            ADMIN_AUTHOR,
            &body.body,
            &body.anchor,
        )
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
    ValidatedJson(body): ValidatedJson<crate::dto::AdminReplyReq>,
) -> Result<Response> {
    let svc = crate::services::comments::CommentsService::new(ctx.db.clone());
    let msg = svc
        .admin_add_reply(id, pin, &body.body)
        .await
        .map_err(into_response)?;
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
    Path((id, cid)): Path<(i32, i32)>,
    ValidatedJson(body): ValidatedJson<crate::dto::EditMessageReq>,
) -> Result<Response> {
    let svc = crate::services::comments::CommentsService::new(ctx.db.clone());
    let msg = svc
        .admin_edit_message(id, cid, &body.body)
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
    Path((id, pin)): Path<(i32, i32)>,
) -> Result<Response> {
    let svc = crate::services::comments::CommentsService::new(ctx.db.clone());
    svc.admin_delete_own_pin(id, pin)
        .await
        .map_err(into_response)?;
    format::json(crate::dto::OkResponse::ok())
}

/// Routes de l'adaptateur admin. Les endpoints de lecture (GET) sont publics après
/// auth ; les mutations (POST/PUT/DELETE) sont également protégées par la garde
/// `require_same_origin` (contrat §4/§9.6 — CSRF complémentaire au SameSite).
///
/// Loco 0.16/axum 0.8 : plusieurs `.add()` sur le même chemin avec des verbes
/// différents sont fusionnés par axum (`Router::route` merge les `MethodRouter`).
/// Le `.layer(...)` par handler s'applique uniquement au verbe concerné.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        // Lecture — pas de garde Origin (GET est idempotent).
        .add("/projects", get(list))
        .add("/projects/{id}", get(detail))
        // Mutations — garde Origin obligatoire (contrat §4/§9.6).
        .add(
            "/projects",
            post(create).layer(from_fn(require_same_origin)),
        )
        .add(
            "/projects/{id}",
            put(update).layer(from_fn(require_same_origin)),
        )
        .add(
            "/projects/{id}",
            axum::routing::delete(delete).layer(from_fn(require_same_origin)),
        )
        .add(
            "/projects/{id}/code",
            post(set_code).layer(from_fn(require_same_origin)),
        )
        .add(
            "/projects/{id}/code",
            axum::routing::delete(clear_code).layer(from_fn(require_same_origin)),
        )
        // Déploiement + gestion de versions (Task 8).
        // Mutations : garde Origin obligatoire.
        .add(
            "/projects/{id}/deploy",
            post(deploy).layer(from_fn(require_same_origin)),
        )
        .add(
            "/projects/{id}/versions/{n}/activate",
            post(activate_version).layer(from_fn(require_same_origin)),
        )
        .add(
            "/projects/{id}/versions/{n}",
            axum::routing::delete(delete_version).layer(from_fn(require_same_origin)),
        )
        .add(
            "/projects/{id}/versions/{n}/comments",
            get(list_version_comments),
        )
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
            axum::routing::delete(moderate_delete_comment).layer(from_fn(require_same_origin)),
        )
        .add(
            "/projects/{id}/comments/messages/{cid}",
            put(admin_edit_comment).layer(from_fn(require_same_origin)),
        )
        .add(
            "/projects/{id}/comments/pins/{pin}",
            axum::routing::delete(admin_delete_pin).layer(from_fn(require_same_origin)),
        )
        // Preview : GET idempotent, pas de garde Origin, mais derrière AdminAuth.
        .add("/projects/{id}/versions/{n}/preview", get(preview_version))
}
