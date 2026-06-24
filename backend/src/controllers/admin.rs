//! Adaptateur entrant "web admin". Chaque handler : auth via `AdminAuth`,
//! appelle un service du cœur, mappe `CoreError` → HTTP (error::into_response),
//! sérialise un DTO. Aucune logique métier ici.

use axum::middleware::from_fn;
use loco_rs::prelude::*;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};

use crate::controllers::auth::AdminAuth;
use crate::controllers::dto::{
    CreateProjectReq, ProjectDetail, ProjectListItem, SetCodeReq, UpdateProjectReq,
};
use crate::controllers::error::into_response;
use crate::controllers::middleware::origin::require_same_origin;
use crate::models::_entities::versions;
use crate::services::projects::{CreateProject, ProjectsService};

/// GET /admin/projects — liste tous les projets (sans PIN, invariant §9.2).
#[debug_handler]
async fn list(_auth: AdminAuth, State(ctx): State<AppContext>) -> Result<Response> {
    let svc = ProjectsService::new(ctx.db.clone());
    let projects = svc.list().await.map_err(into_response)?;
    let items: Vec<ProjectListItem> = projects.iter().map(ProjectListItem::from).collect();
    format::json(items)
}

/// GET /admin/projects/{id} — détail d'un projet avec ses versions et son PIN.
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

    format::json(ProjectDetail::from_model(project, vers))
}

/// POST /admin/projects — crée un nouveau projet.
/// Requiert un Origin same-origin (garde CSRF, contrat §4/§9.6).
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

/// PUT /admin/projects/{id} — met à jour le nom ou le brand_name d'un projet.
/// `UpdateProjectReq.brand_name` est `Option<Option<String>>` :
///   - `None` → ne pas toucher au champ
///   - `Some(None)` → effacer la valeur
///   - `Some(Some(x))` → remplacer par `x`
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

    format::json(ProjectDetail::from_model(saved, vec![]))
}

/// DELETE /admin/projects/{id} — supprime un projet et ses versions.
/// SQLite n'enforce pas les FK sans PRAGMA → suppression explicite en transaction (QUIRKS).
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

    txn.commit().await.map_err(|e| into_response(e.into()))?;

    if res.rows_affected == 0 {
        return Err(loco_rs::Error::NotFound);
    }
    format::json(serde_json::json!({"ok": true}))
}

/// POST /admin/projects/{id}/code — active le code d'accès avec le PIN fourni.
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

/// DELETE /admin/projects/{id}/code — désactive le code d'accès (PIN effacé).
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

/// Routes de l'adaptateur admin. Les endpoints de lecture (GET) sont publics après
/// auth ; les mutations (POST/PUT/DELETE) sont également protégées par la garde
/// `require_same_origin` (contrat §4/§9.6 — CSRF complémentaire au SameSite).
///
/// Loco 0.16/axum 0.8 : plusieurs `.add()` sur le même chemin avec des verbes
/// différents sont fusionnés par axum (`Router::route` merge les `MethodRouter`).
/// Le `.layer(...)` par handler s'applique uniquement au verbe concerné.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/admin")
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
}
