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

use crate::dto::LoginReq;
use crate::services::security::secure_compare;
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

/// POST /admin/login — vérifie ADMIN_USER/ADMIN_PASS à temps constant, pose la session.
/// Les deux comparaisons sont TOUJOURS effectuées (pas de court-circuit) pour ne pas
/// révéler quel champ a échoué (contrat §9).
#[debug_handler]
async fn login(session: AdminSession, Json(body): Json<LoginReq>) -> Result<Response> {
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
    format::json(serde_json::json!({"ok": true}))
}

/// POST /admin/logout — invalide la session côté serveur (supprime la ligne en DB).
/// `destroy()` marque la session pour suppression en DB à la phase de réponse,
/// ce qui assure la révocation immédiate côté serveur (contrat §4).
#[debug_handler]
async fn logout(session: AdminSession) -> Result<Response> {
    session.destroy();
    format::json(serde_json::json!({"ok": true}))
}

pub fn routes() -> Routes {
    // Rate-limit sur le login uniquement (contrat §9.5 : charge-bearing).
    // Limites : 2 req/s, burst de 5. SmartIpKeyExtractor lit X-Forwarded-For
    // (posé par Caddy en façade) avant de tomber sur l'IP peer.
    let login_governor = {
        let config = Arc::new(
            GovernorConfigBuilder::default()
                .per_second(2)
                .burst_size(5)
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
