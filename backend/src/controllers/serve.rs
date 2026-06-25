//! Adaptateur entrant "serving client" (`/c/<slug>`) + meta publique. Surface
//! publique (pas de session admin). L'auth = code projet + cookie signé ;
//! la barrière = rate-limit (contrat §6, §9.5). Aucune réponse ne porte le PIN.

use std::sync::Arc;
use std::time::Duration;

use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum_extra::extract::cookie::{Cookie, SameSite, SignedCookieJar};
use loco_rs::prelude::*;
use tower::ServiceBuilder;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};

use crate::controllers::serve_ratelimit::{IpSlugKeyExtractor, SlugKeyExtractor};

use crate::controllers::error::into_response;
use crate::dto::UnlockReq;
use crate::services::errors::CoreError;
use crate::services::projects::ProjectsService;
use crate::services::unlock_cookie::{issue_token, verify_token};

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

/// Sert la page d'erreur stylée (`error.html` buildé) avec le status donné, `no-store`.
/// Fallback texte inline si le fichier manque — jamais de JSON brut sur `/c`.
async fn serve_error_page(status: StatusCode) -> Response {
    let path = crate::web::error_index();
    let html = tokio::fs::read_to_string(&path).await.unwrap_or_else(|_| {
        "<!doctype html><meta charset=utf-8><title>latch</title>\
         <p>Ce prototype n'est pas disponible.</p>"
            .to_string()
    });
    (
        status,
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
    // Slug inconnu → page d'erreur 404 ; erreur DB → 500 (loggée).
    let project = match svc.get_by_slug(&slug).await {
        Ok(p) => p,
        Err(CoreError::NotFound) => return Ok(serve_error_page(StatusCode::NOT_FOUND).await),
        Err(e) => {
            tracing::error!(error = %e, slug = %slug, "serve: get_by_slug failed");
            return Ok(serve_error_page(StatusCode::INTERNAL_SERVER_ERROR).await);
        }
    };

    // Pas de version active → rien à servir → page d'erreur 404.
    let Some(active_id) = project.active_version_id else {
        return Ok(serve_error_page(StatusCode::NOT_FOUND).await);
    };

    // Projet protégé sans cookie valide → page de déverrouillage (avant de lire le HTML).
    if project.code_enabled {
        let pin = project.pin.clone().unwrap_or_default();
        let key = crate::web::unlock_key(&ctx)?;
        let jar = SignedCookieJar::from_headers(&headers, key);
        let now = chrono::Utc::now().timestamp();
        let secret = crate::web::unlock_secret(&ctx)?;
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
    let version = match versions::Entity::find_by_id(active_id).one(&ctx.db).await {
        Ok(Some(v)) => v,
        Ok(None) => {
            tracing::error!(version = active_id, slug = %slug, "serve: active version row missing");
            return Ok(serve_error_page(StatusCode::INTERNAL_SERVER_ERROR).await);
        }
        Err(e) => {
            tracing::error!(error = %e, slug = %slug, "serve: version lookup failed");
            return Ok(serve_error_page(StatusCode::INTERNAL_SERVER_ERROR).await);
        }
    };
    let storage = crate::web::storage_from_ctx(&ctx);
    let html = match storage.read(&version.html_path).await {
        Ok(h) => h,
        Err(e) => {
            tracing::error!(error = %e, slug = %slug, "serve: storage read failed");
            return Ok(serve_error_page(StatusCode::INTERNAL_SERVER_ERROR).await);
        }
    };
    Ok(html_response(html))
}

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
    let ok = svc
        .verify_code(&slug, &body.pin)
        .await
        .map_err(into_response)?;
    if !ok {
        return Err(loco_rs::Error::Unauthorized("bad code".to_string()));
    }

    // PIN correct (ou projet libre) → poser le cookie signé liant le PIN courant.
    let secret = crate::web::unlock_secret(&ctx)?;
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

    let key = crate::web::unlock_key(&ctx)?;
    let jar = SignedCookieJar::from_headers(&headers, key).add(cookie);
    Ok((jar, StatusCode::NO_CONTENT).into_response())
}

fn env_u32(name: &str, default: u32) -> u32 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

pub fn routes() -> Routes {
    // Burst & période réglables par env (défauts : IP+slug 5/1s, slug global 20/3s).
    let ip_burst: u32 = env_u32("LATCH_UNLOCK_RL_IP_BURST", 5);
    let ip_per_sec: u64 = env_u64("LATCH_UNLOCK_RL_IP_PER_SECOND", 1);
    let slug_burst: u32 = env_u32("LATCH_UNLOCK_RL_SLUG_BURST", 20);
    let slug_period: u64 = env_u64("LATCH_UNLOCK_RL_SLUG_PERIOD_SECS", 3);

    let ip_layer = {
        // Init de boot : config governor invalide (burst/période hors-bornes) = bug de config.
        // Panique au démarrage acceptable — le rate-limiter est un invariant de sécurité (contrat §9.5).
        #[allow(clippy::expect_used)]
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
        // Init de boot : config governor invalide (burst/période hors-bornes) = bug de config.
        // Le burst par défaut est non-nul (20) ; panique au démarrage acceptable —
        // le rate-limiter est un invariant de sécurité (contrat §9.5).
        #[allow(clippy::expect_used)]
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

    let unlock_layers = ServiceBuilder::new().layer(ip_layer).layer(slug_layer);

    Routes::new()
        .add("/api/public/{slug}", get(public_meta))
        .add("/c/{slug}", get(serve))
        .add("/c/{slug}/unlock", post(unlock).layer(unlock_layers))
}
