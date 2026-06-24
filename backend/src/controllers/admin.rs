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
    ActivateResponse, CreateProjectReq, DeployReq, DeployResponse, OkResponse, ProjectDetail,
    ProjectListItem, SetCodeReq, UpdateProjectReq,
};
use crate::models::_entities::versions;
use crate::services::deploy::DeployService;
use crate::services::projects::{CreateProject, ProjectsService};

/// GET /api/projects — liste tous les projets (sans PIN, invariant §9.2).
#[utoipa::path(
    get, path = "/api/projects", tag = "projects",
    responses((status = 200, description = "Liste des projets (sans PIN)", body = Vec<ProjectListItem>),
              (status = 401, description = "Non authentifié"))
)]
#[debug_handler]
async fn list(_auth: AdminAuth, State(ctx): State<AppContext>) -> Result<Response> {
    let svc = ProjectsService::new(ctx.db.clone());
    let projects = svc.list().await.map_err(into_response)?;
    let items: Vec<ProjectListItem> = projects.iter().map(crate::dto::to_list_item).collect();
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
    use crate::models::_entities::projects;

    let project = projects::Entity::find_by_id(id)
        .one(&ctx.db)
        .await
        .map_err(|e| into_response(e.into()))?
        .ok_or(loco_rs::Error::NotFound)?;

    let vers = versions::Entity::find()
        .filter(versions::Column::ProjectId.eq(id))
        .order_by_desc(versions::Column::N)
        .all(&ctx.db)
        .await
        .map_err(|e| into_response(e.into()))?;

    format::json(crate::dto::to_detail(project, vers))
}

/// POST /admin/projects — crée un nouveau projet.
/// Requiert un Origin same-origin (garde CSRF, contrat §4/§9.6).
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
    format::json(crate::dto::to_detail(project, vec![]))
}

/// PUT /admin/projects/{id} — met à jour le nom ou le brand_name d'un projet.
/// `UpdateProjectReq.brand_name` est `Option<Option<String>>` :
///   - `None` → ne pas toucher au champ
///   - `Some(None)` → effacer la valeur
///   - `Some(Some(x))` → remplacer par `x`
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
    // updated_at posé manuellement (le hook before_save ne s'applique pas hors tx, cf. QUIRKS).
    active.updated_at = Set(chrono::Utc::now().into());

    let saved = active
        .update(&ctx.db)
        .await
        .map_err(|e| into_response(e.into()))?;

    // Charge les versions du projet pour renvoyer un détail complet (comme GET /detail).
    let vers = versions::Entity::find()
        .filter(versions::Column::ProjectId.eq(id))
        .order_by_desc(versions::Column::N)
        .all(&ctx.db)
        .await
        .map_err(|e| into_response(e.into()))?;

    format::json(crate::dto::to_detail(saved, vers))
}

/// DELETE /admin/projects/{id} — supprime un projet et ses versions.
/// SQLite n'enforce pas les FK sans PRAGMA → suppression explicite en transaction (QUIRKS).
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
    use crate::models::_entities::projects;

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

/// POST /admin/projects/{id}/code — active le code d'accès avec le PIN fourni.
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
    Json(body): Json<SetCodeReq>,
) -> Result<Response> {
    let svc = ProjectsService::new(ctx.db.clone());
    let project = svc.set_code(id, &body.pin).await.map_err(into_response)?;
    format::json(crate::dto::to_detail(project, vec![]))
}

/// DELETE /admin/projects/{id}/code — désactive le code d'accès (PIN effacé).
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
    format::json(crate::dto::to_detail(project, vec![]))
}

/// POST /admin/projects/{id}/deploy — déploie un nouveau HTML, crée une version.
/// Si `activate=true`, repointe `active_version_id`. Répond `{id, n}`.
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
    Json(body): Json<DeployReq>,
) -> Result<Response> {
    let storage = crate::web::storage_from_ctx(&ctx);
    let svc = DeployService::new(ctx.db.clone(), storage);
    let version = svc
        .deploy(id, &body.html, body.activate)
        .await
        .map_err(into_response)?;
    format::json(crate::dto::DeployResponse {
        id: version.id,
        n: version.n,
    })
}

/// POST /admin/projects/{id}/versions/{n}/activate — bascule le pointeur actif.
/// Charge la version par (project_id, n) → 404 si absente. Met à jour le projet.
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

/// DELETE /admin/projects/{id}/versions/{n} — supprime une version NON active.
/// Charge la version par (project_id, n) → 404 si absente.
/// Refuse la suppression si elle est la version active du projet → 400.
/// Nettoyage du fichier HTML sur le storage : optionnel (cf. BACKLOG).
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

    format::json(crate::dto::OkResponse::ok())
}

/// GET /admin/projects/{id}/versions/{n}/preview — sert le HTML brut de la version.
/// Répond avec `Cache-Control: no-store` pour éviter tout cache navigateur en admin.
/// Protégé par `AdminAuth` (pas de garde Origin : GET est idempotent).
/// Confirmé via Context7 : `loco_rs::prelude::Response = axum::response::Response` ;
/// le tuple `(headers_array, body_string).into_response()` est idiomatique axum 0.8.
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
    let version = versions::Entity::find()
        .filter(versions::Column::ProjectId.eq(id))
        .filter(versions::Column::N.eq(n))
        .one(&ctx.db)
        .await
        .map_err(|e| into_response(e.into()))?
        .ok_or(loco_rs::Error::NotFound)?;

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
        ],
        html,
    )
        .into_response())
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
        // Preview : GET idempotent, pas de garde Origin, mais derrière AdminAuth.
        .add("/projects/{id}/versions/{n}/preview", get(preview_version))
}
