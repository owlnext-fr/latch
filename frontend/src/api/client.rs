//! Client HTTP typé vers l'API /api/*. Même origin → le cookie de session part
//! automatiquement (credentials same-origin par défaut). Les types viennent de
//! `latch-dto` (contrat de fil partagé).

use gloo_net::http::Request;
use latch_dto::{
    CreateProjectReq, DeployReq, LoginReq, ProjectDetail, ProjectListItem, SetCodeReq,
    UpdateProjectReq,
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

/// Decision T6: backend deploy handler returns `{id, n}` (not a full VersionItem).
/// Option (b) applied: do not deserialize the response body; caller reloads project detail.
pub async fn deploy(id: i32, body: &DeployReq) -> Result<(), ApiError> {
    let resp = Request::post(&format!("/api/projects/{id}/deploy"))
        .json(body)?
        .send()
        .await?;
    check_status(resp.status())
}

pub async fn activate_version(id: i32, n: i32) -> Result<(), ApiError> {
    let resp = Request::post(&format!("/api/projects/{id}/versions/{n}/activate"))
        .send()
        .await?;
    check_status(resp.status())
}

pub async fn delete_version(id: i32, n: i32) -> Result<(), ApiError> {
    let resp = Request::delete(&format!("/api/projects/{id}/versions/{n}"))
        .send()
        .await?;
    check_status(resp.status())
}

/// URL de prévisualisation (HTML brut no-store) — à ouvrir dans un nouvel onglet.
pub fn preview_url(id: i32, n: i32) -> String {
    format!("/api/projects/{id}/versions/{n}/preview")
}
