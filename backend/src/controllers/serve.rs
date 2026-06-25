//! Adaptateur entrant "serving client" (`/c/<slug>`) + meta publique. Surface
//! publique (pas de session admin). L'auth = code projet + cookie signé ;
//! la barrière = rate-limit (contrat §6, §9.5). Aucune réponse ne porte le PIN.

use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue};
use axum::response::IntoResponse;
use axum_extra::extract::cookie::SignedCookieJar;
use loco_rs::prelude::*;

use crate::controllers::error::into_response;
use crate::services::projects::ProjectsService;
use crate::services::unlock_cookie::verify_token;

/// Nom du cookie de déverrouillage (scopé par `Path=/c/{slug}` → nom constant OK).
pub(crate) const UNLOCK_COOKIE_NAME: &str = "latch_unlock";

/// Construit la réponse HTML brute du proto actif, `no-store`.
fn html_response(html: String) -> Response {
    (
        [
            (CACHE_CONTROL, HeaderValue::from_static("no-store")),
            (
                CONTENT_TYPE,
                HeaderValue::from_static("text/html; charset=utf-8"),
            ),
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
        let secret = crate::web::unlock_secret()?;
        let ok = match jar.get(UNLOCK_COOKIE_NAME) {
            Some(c) => verify_token(secret.as_bytes(), &slug, &pin, c.value(), now),
            None => false,
        };
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
    let html = storage
        .read(&version.html_path)
        .await
        .map_err(into_response)?;
    Ok(html_response(html))
}

pub fn routes() -> Routes {
    Routes::new()
        .add("/api/public/{slug}", get(public_meta))
        .add("/c/{slug}", get(serve))
}
