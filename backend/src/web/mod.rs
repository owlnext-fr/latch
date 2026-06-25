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

/// Résout un secret de cookie. Fail-secure : hors Development/Test, l'env var
/// est OBLIGATOIRE (pas de fallback). Le fallback de dev n'est toléré qu'en
/// Development/Test. Erreur si trop court (< 64 octets, exigence de `cookie::Key`).
fn resolve_cookie_secret(
    env_value: Option<String>,
    is_prod: bool,
    dev_fallback: &str,
    label: &str,
) -> Result<String> {
    let secret = match env_value {
        Some(s) => s,
        None if is_prod => {
            return Err(loco_rs::Error::Message(format!(
                "{label} doit être défini en production (≥ 64 octets aléatoires)"
            )))
        }
        None => dev_fallback.to_string(),
    };
    if secret.len() < 64 {
        return Err(loco_rs::Error::Message(format!(
            "{label} trop court : {} octets (minimum 64)",
            secret.len()
        )));
    }
    Ok(secret)
}

/// `true` si l'on doit poser `Secure` sur le cookie (fail-secure : tout env hors
/// Development/Test). Aligné sur `build_session_store`.
pub fn cookie_secure(ctx: &AppContext) -> bool {
    !matches!(
        ctx.environment,
        loco_rs::environment::Environment::Development | loco_rs::environment::Environment::Test
    )
}

/// Secret HMAC du cookie de déverrouillage client. Doit faire ≥ 64 bytes
/// (exigence de `cookie::Key`). En dev/test, clé de secours déterministe (insécurisée).
/// En prod, `UNLOCK_COOKIE_SECRET` doit être défini avec ≥ 64 bytes d'entropie.
/// Fail-secure : refuse de démarrer en prod sans secret explicite.
pub fn unlock_secret(ctx: &AppContext) -> Result<String> {
    resolve_cookie_secret(
        std::env::var("UNLOCK_COOKIE_SECRET").ok(),
        cookie_secure(ctx),
        "dev-only-insecure-unlock-cookie-secret-please-override-in-production!!",
        "UNLOCK_COOKIE_SECRET",
    )
}

/// `Key` du `SignedCookieJar` (signature anti-falsification du cookie unlock).
pub fn unlock_key(ctx: &AppContext) -> Result<axum_extra::extract::cookie::Key> {
    Ok(axum_extra::extract::cookie::Key::from(
        unlock_secret(ctx)?.as_bytes(),
    ))
}

/// Construit le `SessionStore` : pool SQLite dérivé de la connexion Loco, table
/// `sessions` (déjà migrée), cookie signé + flags adaptés à l'environnement.
///
/// En production, la variable `SESSION_SECRET` doit être définie (≥ 64 bytes).
/// En dev/test, une clé de secours déterministe est utilisée (insécurisée, suffisante pour
/// le développement local uniquement). Un `SESSION_SECRET` trop court retourne une
/// erreur claire au lieu de paniquer.
/// Fail-secure : refuse de démarrer en prod sans secret explicite.
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

    let secret = resolve_cookie_secret(
        std::env::var("SESSION_SECRET").ok(),
        is_prod,
        "dev-only-insecure-session-secret-please-override-in-production!!",
        "SESSION_SECRET",
    )?;

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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::resolve_cookie_secret;

    const DEV_FALLBACK: &str =
        "dev-only-insecure-unlock-cookie-secret-please-override-in-production!!";
    const LABEL: &str = "UNLOCK_COOKIE_SECRET";

    /// Une valeur exactement 64 octets ASCII.
    fn secret_64() -> String {
        "a".repeat(64)
    }

    /// Une valeur trop courte (< 64 octets).
    fn secret_short() -> String {
        "toocourt".to_string()
    }

    // --- prod (is_prod = true) ---

    #[test]
    fn prod_no_env_var_returns_err() {
        let result = resolve_cookie_secret(None, true, DEV_FALLBACK, LABEL);
        assert!(
            result.is_err(),
            "prod sans env var doit retourner une erreur"
        );
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains(LABEL),
            "le message d'erreur doit mentionner la variable : {msg}"
        );
    }

    #[test]
    fn prod_valid_env_var_returns_ok() {
        let val = secret_64();
        let result = resolve_cookie_secret(Some(val.clone()), true, DEV_FALLBACK, LABEL);
        assert!(result.is_ok(), "prod avec secret valide doit réussir");
        assert_eq!(result.unwrap(), val);
    }

    #[test]
    fn prod_short_env_var_returns_err() {
        let result = resolve_cookie_secret(Some(secret_short()), true, DEV_FALLBACK, LABEL);
        assert!(
            result.is_err(),
            "prod avec secret trop court doit retourner une erreur"
        );
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("trop court"),
            "message d'erreur doit mentionner 'trop court' : {msg}"
        );
    }

    // --- dev (is_prod = false) ---

    #[test]
    fn dev_no_env_var_uses_fallback() {
        let result = resolve_cookie_secret(None, false, DEV_FALLBACK, LABEL);
        assert!(result.is_ok(), "dev sans env var doit utiliser le fallback");
        assert_eq!(result.unwrap(), DEV_FALLBACK);
    }

    #[test]
    fn dev_short_env_var_returns_err() {
        let result = resolve_cookie_secret(Some(secret_short()), false, DEV_FALLBACK, LABEL);
        assert!(
            result.is_err(),
            "dev avec secret explicite trop court doit retourner une erreur"
        );
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("trop court"),
            "message d'erreur doit mentionner 'trop court' : {msg}"
        );
    }
}
