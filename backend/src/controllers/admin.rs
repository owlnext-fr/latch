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

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/admin")
        .add("/projects", get(list))
        .add("/projects/{id}", get(detail))
}
