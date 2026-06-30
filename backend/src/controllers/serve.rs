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
use crate::models::_entities::{projects, versions};
use crate::services::errors::CoreError;
use crate::services::projects::ProjectsService;
use crate::services::unlock_cookie::{issue_token, verify_token};

/// Nom du cookie de déverrouillage (scopé par `Path=/c/{slug}` → nom constant OK).
pub(crate) const UNLOCK_COOKIE_NAME: &str = "latch_unlock";

/// Cookie d'identité visiteur pour les commentaires (ULID opaque, signé). Scopé par slug.
#[allow(dead_code)]
pub(crate) const COMMENT_COOKIE_NAME: &str = "latch_comment";
/// Durée de vie du cookie d'identité (jours).
#[allow(dead_code)]
const COMMENT_IDENTITY_TTL_DAYS: i64 = 365;

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

/// Réponse HTML du proto pour l'iframe : `no-store` + `frame-ancestors 'self'`
/// (seul le shell latch peut l'encadrer).
fn raw_html_response(html: String) -> Response {
    (
        [
            (CACHE_CONTROL, HeaderValue::from_static("no-store")),
            (
                CONTENT_TYPE,
                HeaderValue::from_static("text/html; charset=utf-8"),
            ),
            (
                axum::http::header::CONTENT_SECURITY_POLICY,
                HeaderValue::from_static("frame-ancestors 'self'"),
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

/// Rend la page-coquille (`shell.html` buildé), HTTP 200, `no-store`.
async fn shell_page_response() -> Result<Response> {
    let path = crate::web::shell_index();
    let html = tokio::fs::read_to_string(&path).await.map_err(|e| {
        loco_rs::Error::Message(format!("shell.html introuvable ({}): {e}", path.display()))
    })?;
    Ok(html_response(html))
}

/// `true` si l'accès au proto est autorisé : projet libre, ou cookie unlock valide.
fn unlock_ok(
    ctx: &AppContext,
    headers: &HeaderMap,
    slug: &str,
    project: &projects::Model,
) -> Result<bool> {
    if !project.code_enabled {
        return Ok(true);
    }
    let pin = project.pin.clone().unwrap_or_default();
    let key = crate::web::unlock_key(ctx)?;
    let jar = SignedCookieJar::from_headers(headers, key);
    let now = chrono::Utc::now().timestamp();
    let secret = crate::web::unlock_secret(ctx)?;
    Ok(match jar.get(UNLOCK_COOKIE_NAME) {
        Some(c) => verify_token(secret.as_bytes(), slug, &pin, c.value(), now),
        None => false,
    })
}

/// Charge la version active d'un projet, ou `None` si pas de pointeur / version absente.
async fn load_active_version(
    ctx: &AppContext,
    project: &projects::Model,
) -> Result<Option<versions::Model>> {
    let Some(active_id) = project.active_version_id else {
        return Ok(None);
    };
    versions::Entity::find_by_id(active_id)
        .one(&ctx.db)
        .await
        .map_err(|e| loco_rs::Error::Message(format!("version lookup: {e}")))
}

/// Résout un projet par slug pour les handlers HTML (serve, raw).
/// En cas d'erreur, renvoie directement une page d'erreur stylée.
/// Retourne `Ok(Some(project))` si trouvé, `Ok(None)` si le handler doit rendre lui-même
/// la réponse d'erreur (via le `Response` dans l'`Err`).
async fn resolve_project_html(
    svc: &ProjectsService,
    slug: &str,
    caller: &str,
) -> Result<projects::Model, Response> {
    match svc.get_by_slug(slug).await {
        Ok(p) => Ok(p),
        Err(CoreError::NotFound) => Err(serve_error_page(StatusCode::NOT_FOUND).await),
        Err(e) => {
            tracing::error!(error = %e, slug = %slug, "{caller}: get_by_slug failed");
            Err(serve_error_page(StatusCode::INTERNAL_SERVER_ERROR).await)
        }
    }
}

/// Résout un projet par slug pour les handlers JSON/status (notes).
/// En cas d'erreur, renvoie directement un StatusCode brut.
async fn resolve_project_status(
    svc: &ProjectsService,
    slug: &str,
    caller: &str,
) -> Result<projects::Model, Response> {
    match svc.get_by_slug(slug).await {
        Ok(p) => Ok(p),
        Err(CoreError::NotFound) => Err(StatusCode::NOT_FOUND.into_response()),
        Err(e) => {
            tracing::error!(error = %e, slug = %slug, "{caller}: get_by_slug failed");
            Err(StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
    }
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

/// GET /c/{slug} — sert la page-coquille (shell) si le projet est accessible.
/// Le gate unlock vit ici : projets protégés sans cookie valide → page de déverrouillage.
#[debug_handler]
pub(crate) async fn serve(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Result<Response> {
    let svc = ProjectsService::new(ctx.db.clone());
    let project = match resolve_project_html(&svc, &slug, "serve").await {
        Ok(p) => p,
        Err(resp) => return Ok(resp),
    };

    // Pas de version active → page d'erreur 404 (comportement inchangé).
    if project.active_version_id.is_none() {
        return Ok(serve_error_page(StatusCode::NOT_FOUND).await);
    }

    // Projet protégé sans cookie valide → page de déverrouillage (top-level, hors iframe).
    if !unlock_ok(&ctx, &headers, &slug, &project)? {
        return unlock_page_response().await;
    }

    // Sinon → servir le shell (qui charge /raw en iframe et gère l'overlay de notes).
    shell_page_response().await
}

/// GET /c/{slug}/raw — HTML brut du proto (cible de l'iframe du shell). Mêmes gates.
#[debug_handler]
pub(crate) async fn raw(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Result<Response> {
    let svc = ProjectsService::new(ctx.db.clone());
    let project = match resolve_project_html(&svc, &slug, "raw").await {
        Ok(p) => p,
        Err(resp) => return Ok(resp),
    };
    if !unlock_ok(&ctx, &headers, &slug, &project)? {
        // Defense-in-depth : ne jamais servir le HTML d'un proto verrouillé.
        return Ok(serve_error_page(StatusCode::FORBIDDEN).await);
    }
    let Some(version) = load_active_version(&ctx, &project).await? else {
        return Ok(serve_error_page(StatusCode::NOT_FOUND).await);
    };
    let storage = crate::web::storage_from_ctx(&ctx);
    match storage.read(&version.html_path).await {
        Ok(html) => Ok(raw_html_response(html)),
        Err(e) => {
            tracing::error!(error = %e, slug = %slug, "raw: storage read failed");
            Ok(serve_error_page(StatusCode::INTERNAL_SERVER_ERROR).await)
        }
    }
}

/// GET /c/{slug}/notes — notes de la version active (ou 204). Gardé par l'unlock.
#[debug_handler]
pub(crate) async fn notes(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Result<Response> {
    let svc = ProjectsService::new(ctx.db.clone());
    let project = match resolve_project_status(&svc, &slug, "notes").await {
        Ok(p) => p,
        Err(resp) => return Ok(resp),
    };
    if !unlock_ok(&ctx, &headers, &slug, &project)? {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }
    let Some(version) = load_active_version(&ctx, &project).await? else {
        return Ok(StatusCode::NO_CONTENT.into_response());
    };
    match version.release_notes {
        Some(md) if !md.is_empty() => {
            let body = crate::dto::ReleaseNotes {
                n: version.n,
                notes_md: md,
            };
            Ok((
                [(CACHE_CONTROL, HeaderValue::from_static("no-store"))],
                axum::Json(body),
            )
                .into_response())
        }
        _ => Ok(StatusCode::NO_CONTENT.into_response()),
    }
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

/// Génère un `owner_token` opaque (ULID Crockford base32, 26 chars).
#[allow(dead_code)]
pub(crate) fn mint_owner_token() -> String {
    ulid::Ulid::new().to_string()
}

/// Lit l'`owner_token` du cookie signé `latch_comment`, s'il est présent et valide.
#[allow(dead_code)]
pub(crate) fn read_owner_token(ctx: &AppContext, headers: &HeaderMap) -> Result<Option<String>> {
    let key = crate::web::unlock_key(ctx)?;
    let jar = SignedCookieJar::from_headers(headers, key);
    Ok(jar.get(COMMENT_COOKIE_NAME).map(|c| c.value().to_string()))
}

/// Construit le cookie d'identité signé pour `slug` (réutilise la clé `UNLOCK_COOKIE_SECRET`).
#[allow(dead_code)]
pub(crate) fn comment_identity_cookie(
    ctx: &AppContext,
    slug: &str,
    token: &str,
) -> Cookie<'static> {
    Cookie::build((COMMENT_COOKIE_NAME, token.to_string()))
        .path(format!("/c/{slug}"))
        .http_only(true)
        .secure(crate::web::cookie_secure(ctx))
        .same_site(SameSite::Lax)
        .max_age(time::Duration::days(COMMENT_IDENTITY_TTL_DAYS))
        .build()
}

/// Middleware : exige le header `X-Comment-Client` sur les écritures de commentaires
/// (anti-CSRF complémentaire au SameSite + garde Origin). 403 si absent.
#[allow(dead_code)]
pub(crate) async fn require_comment_client(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> std::result::Result<Response, StatusCode> {
    if req.headers().contains_key("x-comment-client") {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::FORBIDDEN)
    }
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
        .add("/c/{slug}/raw", get(raw))
        .add("/c/{slug}/notes", get(notes))
        .add("/c/{slug}/unlock", post(unlock).layer(unlock_layers))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod comment_identity_tests {
    use super::*;

    #[test]
    fn minted_token_is_26_char_ulid() {
        let t = mint_owner_token();
        assert_eq!(t.len(), 26, "ULID Crockford base32 = 26 chars");
        assert!(t.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn two_tokens_differ() {
        assert_ne!(mint_owner_token(), mint_owner_token());
    }
}
