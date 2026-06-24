//! Helpers de l'adaptateur web (HTTP). Hors du cœur : c'est ici que vivent
//! session, storage concret, résolution d'environnement. Le cœur reste agnostique.

use std::sync::Arc;

use loco_rs::app::AppContext;
use loco_rs::Result;

use crate::services::storage::{FsStorage, Storage};

/// Store de session adossé au pool SQLite de Loco.
pub type SessionPool = axum_session_sqlx::SessionSqlitePool;
/// Extracteur de session injectable dans les handlers.
pub type AdminSession = axum_session::Session<SessionPool>;

/// Racine de stockage des HTML de versions (volume). `LATCH_STORAGE_ROOT`, défaut `data`.
pub fn storage_from_ctx(_ctx: &AppContext) -> Arc<dyn Storage> {
    let root = std::env::var("LATCH_STORAGE_ROOT").unwrap_or_else(|_| "data".to_string());
    Arc::new(FsStorage::new(root.into()))
}

/// Construit le `SessionStore` : pool SQLite dérivé de la connexion Loco, table
/// `sessions` (déjà migrée), cookie signé + flags adaptés à l'environnement.
///
/// En production, la variable `SESSION_SECRET` doit être définie (≥ 64 bytes).
/// En dev, une clé de secours déterministe est utilisée (insécurisée, suffisante pour
/// le développement local uniquement). Un `SESSION_SECRET` trop court retourne une
/// erreur claire au lieu de paniquer.
pub async fn build_session_store(
    ctx: &AppContext,
) -> Result<axum_session::SessionStore<SessionPool>> {
    // `get_sqlite_connection_pool()` retourne `&sqlx::SqlitePool` directement (pas de Result).
    let pool = ctx.db.get_sqlite_connection_pool().clone();
    let session_pool = SessionPool::from(pool);

    // `Secure` et `__Host-` prefix exigent HTTPS → activer uniquement en Production.
    // Development et Test utilisent HTTP (tests mock ou serveur local sans TLS).
    let is_prod = matches!(
        ctx.environment,
        loco_rs::environment::Environment::Production
    );

    let secret = std::env::var("SESSION_SECRET").unwrap_or_else(|_| {
        // En dev uniquement : clé déterministe de secours (64 bytes, non-aléatoire).
        // En prod, SESSION_SECRET doit être défini avec ≥ 64 bytes d'entropie.
        "dev-only-insecure-session-secret-please-override-in-production!!".to_string()
    });
    // Garde explicite : `Key::from` exige ≥ 64 bytes et panique sinon.
    // On renvoie une erreur claire plutôt qu'un panic au démarrage.
    if secret.len() < 64 {
        return Err(loco_rs::Error::Message(format!(
            "SESSION_SECRET trop court : {} octets (minimum 64)",
            secret.len()
        )));
    }
    let key = axum_session::Key::from(secret.as_bytes());

    let config = axum_session::SessionConfig::default()
        .with_table_name("sessions")
        // Nom du cookie/header de session (confirmé : with_session_name, pas with_cookie_name).
        .with_session_name("latch_admin")
        .with_http_only(true)
        .with_secure(is_prod)
        .with_cookie_same_site(axum_session::SameSite::Lax)
        // `__Host-` exige Secure → prod uniquement.
        .with_prefix_with_host(is_prod)
        .with_key(key);

    let store = axum_session::SessionStore::<SessionPool>::new(Some(session_pool), config)
        .await
        .map_err(|e| loco_rs::Error::Message(format!("session store init: {e}")))?;
    Ok(store)
}
