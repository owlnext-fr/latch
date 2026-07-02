//! Adaptateur entrant "auth admin". L'auth est décidée ICI, avant tout service
//! (contrat §1, §9.4). Compte unique env, comparaison à temps constant.

use std::sync::Arc;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::middleware::from_fn;
use loco_rs::prelude::*;
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor, GovernorLayer,
};

use crate::controllers::serve::{env_u32, env_u64};
use crate::dto::{LoginReq, OkResponse};
use crate::services::security::secure_compare;
use crate::web::extract::ValidatedJson;
use crate::web::AdminSession;

/// Clé de session portant le flag d'authentification admin.
pub const ADMIN_FLAG: &str = "admin";

/// Extracteur axum : présent ⇒ session authentifiée (flag `admin == true`). Sinon 401.
/// Consommé par tous les handlers de `admin.rs`.
pub struct AdminAuth;

impl<S> FromRequestParts<S> for AdminAuth
where
    S: Send + Sync,
{
    type Rejection = loco_rs::Error;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        let session = AdminSession::from_request_parts(parts, state)
            .await
            .map_err(|_| loco_rs::Error::Unauthorized("no session".to_string()))?;
        if session.get::<bool>(ADMIN_FLAG).unwrap_or(false) {
            Ok(AdminAuth)
        } else {
            Err(loco_rs::Error::Unauthorized(
                "not authenticated".to_string(),
            ))
        }
    }
}

/// POST /api/login — vérifie ADMIN_USER/ADMIN_PASS à temps constant, pose la session.
// Les deux comparaisons sont TOUJOURS effectuées (pas de court-circuit) pour ne pas
// révéler quel champ a échoué (contrat §9).
#[utoipa::path(
    post, path = "/api/login", tag = "auth",
    request_body = LoginReq,
    responses((status = 200, description = "Authentifié (cookie de session posé)", body = OkResponse),
              (status = 401, description = "Identifiants invalides"))
)]
#[debug_handler]
async fn login(
    session: AdminSession,
    ValidatedJson(body): ValidatedJson<LoginReq>,
) -> Result<Response> {
    let expected_user = std::env::var("ADMIN_USER").unwrap_or_default();
    let expected_pass = std::env::var("ADMIN_PASS").unwrap_or_default();

    // Comparer toujours les deux champs en temps constant.
    // Si l'env n'est pas configuré (vide), on refuse.
    let user_ok = secure_compare(&expected_user, &body.user);
    let pass_ok = secure_compare(&expected_pass, &body.pass);

    if !user_ok || !pass_ok || expected_user.is_empty() || expected_pass.is_empty() {
        return Err(loco_rs::Error::Unauthorized("bad credentials".to_string()));
    }

    session.set(ADMIN_FLAG, true);
    format::json(crate::dto::OkResponse::ok())
}

/// POST /api/logout — invalide la session côté serveur (supprime la ligne en DB).
// `destroy()` marque la session pour suppression en DB à la phase de réponse,
// ce qui assure la révocation immédiate côté serveur (contrat §4).
#[utoipa::path(
    post, path = "/api/logout", tag = "auth",
    responses((status = 200, description = "Session détruite", body = OkResponse),
              (status = 403, description = "Origin invalide (CSRF)"))
)]
#[debug_handler]
async fn logout(session: AdminSession) -> Result<Response> {
    session.destroy();
    format::json(crate::dto::OkResponse::ok())
}

pub fn routes() -> Routes {
    // Rate-limit sur le login uniquement (contrat §9.5 : charge-bearing).
    // Défauts : 1 jeton réapprovisionné / 2s (per_second = 2s de période, pas 2 req/s),
    // burst de 5. Réglables via LATCH_LOGIN_RL_PER_SECOND / LATCH_LOGIN_RL_BURST.
    // SmartIpKeyExtractor lit X-Forwarded-For (posé par Caddy en façade) avant de tomber
    // sur l'IP peer. Le webServer e2e Playwright pose LATCH_LOGIN_RL_BURST=100000
    // pour désarmer le throttle en tests — le défaut reste load-bearing pour
    // le test `login_is_rate_limited` (cargo nextest).
    let login_burst: u32 = env_u32("LATCH_LOGIN_RL_BURST", 5);
    let login_per_sec: u64 = env_u64("LATCH_LOGIN_RL_PER_SECOND", 2);
    let login_governor = {
        // Init de boot : une config governor invalide = bug de programmation (burst ou période
        // hors-bornes). Panique au démarrage est acceptable — l'app ne peut pas fonctionner
        // sans rate-limiter (invariant de sécurité §9.5).
        #[allow(clippy::expect_used)]
        let config = Arc::new(
            GovernorConfigBuilder::default()
                .per_second(login_per_sec)
                .burst_size(login_burst)
                .key_extractor(SmartIpKeyExtractor)
                .finish()
                .expect("governor config valide"),
        );
        GovernorLayer { config }
    };

    Routes::new()
        .prefix("/api")
        .add("/login", post(login).layer(login_governor))
        // Garde same-origin sur logout (CSRF, contrat §4/§9.6). Pas d'AdminAuth :
        // logout reste accessible sans session valide (idempotent/sans effet), mais
        // doit venir du même Origin pour ne pas permettre un logout CSRF.
        .add(
            "/logout",
            post(logout).layer(from_fn(
                crate::controllers::middleware::origin::require_same_origin,
            )),
        )
}
