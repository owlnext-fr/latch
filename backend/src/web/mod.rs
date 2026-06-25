//! Helpers de l'adaptateur web (HTTP). Hors du cœur : c'est ici que vivent
//! session, storage concret, résolution d'environnement. Le cœur reste agnostique.

use std::path::PathBuf;
use std::sync::Arc;

use loco_rs::app::AppContext;
use loco_rs::Result;

use crate::services::storage::{FsStorage, Storage};

/// Store de session adossé au pool SQLite de Loco.
pub type SessionPool = axum_session_sqlx::SessionSqlitePool;
/// Extracteur de session injectable dans les handlers.
pub type AdminSession = axum_session::Session<SessionPool>;

/// Racine des assets buildés de la SPA (`frontend/dist`). Surclassable par
/// `LATCH_SPA_DIST` (posée dans l'image Docker). Défaut relatif au CWD `backend/`.
pub fn spa_dist_dir() -> PathBuf {
    std::env::var("LATCH_SPA_DIST")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("../frontend/dist"))
}

/// Racine de stockage des HTML de versions (volume). `LATCH_STORAGE_ROOT`, défaut `data`.
pub fn storage_from_ctx(_ctx: &AppContext) -> Arc<dyn Storage> {
    let root = std::env::var("LATCH_STORAGE_ROOT").unwrap_or_else(|_| "data".to_string());
    Arc::new(FsStorage::new(root.into()))
}

/// Chemin du `unlock.html` buildé (2ᵉ entrée Vite), sous la même racine que la SPA.
pub fn unlock_index() -> PathBuf {
    spa_dist_dir().join("unlock.html")
}

/// Secret HMAC du cookie de déverrouillage client. Doit faire ≥ 64 bytes
/// (exigence de `cookie::Key`). En dev, clé de secours déterministe (insécurisée).
/// En prod, `UNLOCK_COOKIE_SECRET` doit être défini avec ≥ 64 bytes d'entropie.
pub fn unlock_secret() -> Result<String> {
    let secret = std::env::var("UNLOCK_COOKIE_SECRET").unwrap_or_else(|_| {
        "dev-only-insecure-unlock-cookie-secret-please-override-in-production!!".to_string()
    });
    if secret.len() < 64 {
        return Err(loco_rs::Error::Message(format!(
            "UNLOCK_COOKIE_SECRET trop court : {} octets (minimum 64)",
            secret.len()
        )));
    }
    Ok(secret)
}

/// `Key` du `SignedCookieJar` (signature anti-falsification du cookie unlock).
pub fn unlock_key() -> Result<axum_extra::extract::cookie::Key> {
    Ok(axum_extra::extract::cookie::Key::from(
        unlock_secret()?.as_bytes(),
    ))
}

/// `true` si l'on doit poser `Secure` sur le cookie (fail-secure : tout env hors
/// Development/Test). Aligné sur `build_session_store`.
pub fn cookie_secure(ctx: &AppContext) -> bool {
    !matches!(
        ctx.environment,
        loco_rs::environment::Environment::Development | loco_rs::environment::Environment::Test
    )
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

    // `Secure` et `__Host-` prefix exigent HTTPS. Fail-secure : activer pour TOUT
    // environnement sauf Development et Test (HTTP). Tout env inconnu futur → Secure.
    // Ne jamais écrire `matches!(..., Production)` (fail-open si un nouvel env est ajouté).
    let is_prod = !matches!(
        ctx.environment,
        loco_rs::environment::Environment::Development | loco_rs::environment::Environment::Test
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
