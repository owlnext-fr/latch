//! Helpers de l'adaptateur web (HTTP). Hors du cœur : c'est ici que vivent
//! session, storage concret, résolution d'environnement. Le cœur reste agnostique.

use std::path::PathBuf;
use std::sync::Arc;

use loco_rs::app::AppContext;
use loco_rs::Result;

use crate::services::storage::{FsStorage, Storage};

pub mod extract;

/// Store de session adossé au pool SQLite de Loco.
pub type SessionPool = axum_session_sqlx::SessionSqlitePool;
/// Extracteur de session injectable dans les handlers.
pub type AdminSession = axum_session::Session<SessionPool>;

/// Défaut relatif au CWD `backend/` (dev). En prod l'image pose une valeur absolue.
const STORAGE_ROOT_DEFAULT: &str = "data";
/// Défaut relatif au CWD `backend/` (dev). En prod l'image pose `/app/frontend/dist`.
const SPA_DIST_DEFAULT: &str = "../frontend/dist";

/// Racine des assets buildés de la SPA (`frontend/dist`). Surclassable par
/// `LATCH_SPA_DIST` (posée dans l'image Docker). Défaut relatif au CWD `backend/`.
pub fn spa_dist_dir() -> PathBuf {
    std::env::var("LATCH_SPA_DIST")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(SPA_DIST_DEFAULT))
}

/// Racine de stockage des HTML de versions (volume). `LATCH_STORAGE_ROOT`, défaut `data`.
pub fn storage_from_ctx(_ctx: &AppContext) -> Arc<dyn Storage> {
    let root =
        std::env::var("LATCH_STORAGE_ROOT").unwrap_or_else(|_| STORAGE_ROOT_DEFAULT.to_string());
    Arc::new(FsStorage::new(root.into()))
}

/// Chemin du `unlock.html` buildé (2ᵉ entrée Vite), sous la même racine que la SPA.
pub fn unlock_index() -> PathBuf {
    spa_dist_dir().join("unlock.html")
}

/// Chemin du `error.html` buildé (page d'erreur stylée du serving `/c`).
pub fn error_index() -> PathBuf {
    spa_dist_dir().join("error.html")
}

/// Chemin du `shell.html` buildé (entrée Vite du shell de serving `/c`).
pub fn shell_index() -> PathBuf {
    spa_dist_dir().join("shell.html")
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

/// Résout un secret/config requis SANS plancher de longueur (contrairement à
/// `resolve_cookie_secret` qui exige 64 octets pour `cookie::Key`). Fail-secure :
/// hors Development/Test, l'env var est obligatoire (pas de fallback).
fn resolve_required(
    env_value: Option<String>,
    is_prod: bool,
    dev_fallback: &str,
    label: &str,
) -> Result<String> {
    match env_value {
        Some(s) if !s.is_empty() => Ok(s),
        _ if is_prod => Err(loco_rs::Error::Message(format!(
            "{label} doit être défini en production"
        ))),
        _ => Ok(dev_fallback.to_string()),
    }
}

/// Valide qu'un chemin de configuration est ABSOLU en production (fail-secure).
/// Un chemin relatif hors Dev/Test résout depuis le WORKDIR `/app` du conteneur →
/// couche d'écriture éphémère → perte de données au redéploiement (incident
/// 2026-06-29, cf. `docs/QUIRKS.md`). Une valeur absente ou vide retombe sur le
/// défaut (relatif) → échoue donc aussi en prod, ce qui est voulu.
fn resolve_abs_path(
    env_value: Option<String>,
    is_prod: bool,
    default: &str,
    label: &str,
) -> Result<PathBuf> {
    let raw = env_value
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default.to_string());
    let path = PathBuf::from(&raw);
    if is_prod && !path.is_absolute() {
        return Err(loco_rs::Error::Message(format!(
            "{label} doit être un chemin ABSOLU en production (reçu : {raw:?}). \
             Un chemin relatif résout vers la couche éphémère /app/… du conteneur \
             et perd les données au redéploiement (cf. incident 2026-06-29)."
        )));
    }
    Ok(path)
}

/// Applique le garde-fou de chemin aux deux variables filesystem concernées.
/// Cœur pur (paramétré) pour être testable sans `AppContext` ni env.
fn validate_paths(
    storage_root: Option<String>,
    spa_dist: Option<String>,
    is_prod: bool,
) -> Result<()> {
    resolve_abs_path(
        storage_root,
        is_prod,
        STORAGE_ROOT_DEFAULT,
        "LATCH_STORAGE_ROOT",
    )?;
    resolve_abs_path(spa_dist, is_prod, SPA_DIST_DEFAULT, "LATCH_SPA_DIST")?;
    Ok(())
}

/// Fail-fast de boot : refuse de démarrer si `LATCH_STORAGE_ROOT` ou `LATCH_SPA_DIST`
/// est relatif (ou absent) en production. À appeler en tête de `after_routes`, comme
/// `unlock_secret`/`deploy_token`. Empêche la reproduction de l'incident 2026-06-29.
pub fn validate_path_config(ctx: &AppContext) -> Result<()> {
    validate_paths(
        std::env::var("LATCH_STORAGE_ROOT").ok(),
        std::env::var("LATCH_SPA_DIST").ok(),
        cookie_secure(ctx),
    )
}

/// Secret partagé validé par TOUS les tools MCP (contrat §5, §9.3). Fail-secure :
/// refuse de démarrer en prod sans `DEPLOY_TOKEN`. En dev, fallback déterministe.
pub fn deploy_token(ctx: &AppContext) -> Result<String> {
    resolve_required(
        std::env::var("DEPLOY_TOKEN").ok(),
        cookie_secure(ctx),
        "dev-only-insecure-deploy-token-please-override-in-production",
        "DEPLOY_TOKEN",
    )
}

/// URL publique de base (source de vérité de l'hôte public, contrat §5/§7).
/// Normalisée sans `/` final. Fail-secure en prod ; dev → `http://localhost:<PORT>`.
pub fn public_base_url(ctx: &AppContext) -> Result<String> {
    let port = std::env::var("PORT").unwrap_or_else(|_| "5150".to_string());
    let dev_fallback = format!("http://localhost:{port}");
    let base = resolve_required(
        std::env::var("LATCH_PUBLIC_BASE_URL").ok(),
        cookie_secure(ctx),
        &dev_fallback,
        "LATCH_PUBLIC_BASE_URL",
    )?;
    Ok(base.trim_end_matches('/').to_string())
}

/// Composant hôte (`host` ou `host:port`) d'une URL de base, pour `allowed_hosts`
/// (rmcp ≥ 1.4, validation du `Host` header). Parsing minimal (pas de crate `url`).
pub fn host_authority(base_url: &str) -> String {
    let without_scheme = base_url
        .split_once("://")
        .map_or(base_url, |(_, rest)| rest);
    without_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(without_scheme)
        .to_string()
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

/// Noms `(session, store)` des cookies de session selon l'environnement.
///
/// ⚠️ Bug `axum_session 0.16.0` : `with_prefix_with_host(true)` ÉCRIT le cookie préfixé
/// `__Host-` (via `NameType::get_name`) mais le RELIT sous le nom BRUT
/// (`get_headers_and_key` lit `session_name`/`store_name` sans repasser par `get_name`).
/// En prod, le serveur poserait `__Host-latch_admin` mais chercherait `latch_admin` → la
/// session entrante n'est jamais retrouvée → session neuve à chaque requête → AdminAuth en 401.
/// (Invisible en dev/test : `is_prod=false` → pas de préfixe → noms symétriques.)
///
/// Contournement : on pose nous-mêmes le nom `__Host-…` et on laisse `prefix_with_host` à
/// false → lecture et écriture utilisent le même nom. Le durcissement `__Host-` est préservé :
/// le navigateur impose Secure + Path=/ + pas de Domain, qu'axum_session fournit déjà. Préfixe
/// en PROD uniquement (un cookie `__Host-` sur HTTP serait rejeté). Cf. `docs/QUIRKS.md`.
pub(crate) fn session_cookie_names(is_prod: bool) -> (&'static str, &'static str) {
    if is_prod {
        ("__Host-latch_admin", "__Host-store")
    } else {
        ("latch_admin", "store")
    }
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

    let (session_name, store_name) = session_cookie_names(is_prod);

    let config = axum_session::SessionConfig::default()
        .with_table_name("sessions")
        .with_session_name(session_name)
        .with_store_name(store_name)
        .with_http_only(true)
        .with_secure(is_prod)
        .with_cookie_same_site(axum_session::SameSite::Lax)
        .with_key(key);

    let store = axum_session::SessionStore::<SessionPool>::new(Some(session_pool), config)
        .await
        .map_err(|e| loco_rs::Error::Message(format!("session store init: {e}")))?;
    Ok(store)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::{
        host_authority, resolve_abs_path, resolve_cookie_secret, resolve_required,
        session_cookie_names, validate_paths, SPA_DIST_DEFAULT, STORAGE_ROOT_DEFAULT,
    };

    // Garde anti-régression du contournement du bug axum_session 0.16.0 (cf. QUIRKS) :
    // en prod, le préfixe `__Host-` doit être posé DANS le nom (et jamais via
    // `with_prefix_with_host`, dont la lecture ne re-préfixe pas → session perdue).
    #[test]
    fn session_cookie_names_prod_carry_host_prefix() {
        assert_eq!(
            session_cookie_names(true),
            ("__Host-latch_admin", "__Host-store")
        );
    }

    // En dev/test (HTTP), noms bruts : un cookie `__Host-` serait rejeté par le navigateur.
    #[test]
    fn session_cookie_names_dev_are_bare() {
        assert_eq!(session_cookie_names(false), ("latch_admin", "store"));
    }

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

    // --- resolve_required (sans plancher de longueur) ---

    #[test]
    fn required_prod_no_env_is_err() {
        assert!(resolve_required(None, true, "dev-fallback", "DEPLOY_TOKEN").is_err());
    }

    #[test]
    fn required_prod_with_value_ok() {
        let r = resolve_required(Some("abc".to_string()), true, "dev-fallback", "X");
        assert_eq!(r.unwrap(), "abc");
    }

    #[test]
    fn required_dev_no_env_uses_fallback() {
        let r = resolve_required(None, false, "dev-fallback", "X");
        assert_eq!(r.unwrap(), "dev-fallback");
    }

    #[test]
    fn required_prod_empty_string_is_err() {
        assert!(resolve_required(Some(String::new()), true, "fb", "X").is_err());
    }

    #[test]
    fn required_dev_empty_string_uses_fallback() {
        assert_eq!(
            resolve_required(Some(String::new()), false, "fb", "X").unwrap(),
            "fb"
        );
    }

    // --- host_authority ---
    #[test]
    fn host_authority_strips_scheme_and_path() {
        assert_eq!(
            host_authority("https://latch.owlnext.fr"),
            "latch.owlnext.fr"
        );
        assert_eq!(
            host_authority("https://latch.owlnext.fr/"),
            "latch.owlnext.fr"
        );
        assert_eq!(
            host_authority("http://localhost:5150/mcp"),
            "localhost:5150"
        );
        assert_eq!(host_authority("latch.owlnext.fr"), "latch.owlnext.fr");
    }

    // --- resolve_abs_path : garde-fou chemin relatif/absolu (#9) ---

    const PATH_LABEL: &str = "LATCH_STORAGE_ROOT";

    #[test]
    fn abs_path_prod_relative_returns_err() {
        let result = resolve_abs_path(
            Some("data".to_string()),
            true,
            STORAGE_ROOT_DEFAULT,
            PATH_LABEL,
        );
        assert!(result.is_err(), "prod + chemin relatif doit échouer");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains(PATH_LABEL),
            "message doit mentionner la var : {msg}"
        );
        assert!(
            msg.contains("ABSOLU"),
            "message doit mentionner ABSOLU : {msg}"
        );
    }

    #[test]
    fn abs_path_prod_dot_relative_returns_err() {
        let result = resolve_abs_path(
            Some("./data".to_string()),
            true,
            STORAGE_ROOT_DEFAULT,
            PATH_LABEL,
        );
        assert!(result.is_err(), "prod + ./relatif doit échouer");
    }

    #[test]
    fn abs_path_prod_absolute_returns_ok() {
        let result = resolve_abs_path(
            Some("/data".to_string()),
            true,
            STORAGE_ROOT_DEFAULT,
            PATH_LABEL,
        );
        assert!(result.is_ok(), "prod + chemin absolu doit réussir");
        assert_eq!(result.unwrap(), std::path::PathBuf::from("/data"));
    }

    #[test]
    fn abs_path_prod_unset_uses_relative_default_and_errs() {
        // Défaut relatif → en prod, unset échoue (fail-secure voulu).
        let result = resolve_abs_path(None, true, SPA_DIST_DEFAULT, "LATCH_SPA_DIST");
        assert!(
            result.is_err(),
            "prod + unset (défaut relatif) doit échouer"
        );
    }

    #[test]
    fn abs_path_dev_relative_returns_ok() {
        let result = resolve_abs_path(
            Some("data".to_string()),
            false,
            STORAGE_ROOT_DEFAULT,
            PATH_LABEL,
        );
        assert!(
            result.is_ok(),
            "dev + chemin relatif doit réussir (comportement dev inchangé)"
        );
    }

    #[test]
    fn abs_path_empty_value_falls_back_to_default() {
        // Une valeur vide est traitée comme unset → défaut ; en dev le défaut relatif passe.
        let result = resolve_abs_path(Some(String::new()), false, STORAGE_ROOT_DEFAULT, PATH_LABEL);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            std::path::PathBuf::from(STORAGE_ROOT_DEFAULT)
        );
    }

    // --- validate_paths : applique le garde-fou aux 2 chemins (#9) ---

    #[test]
    fn validate_paths_prod_all_absolute_ok() {
        let result = validate_paths(
            Some("/data".to_string()),
            Some("/app/frontend/dist".to_string()),
            true,
        );
        assert!(result.is_ok(), "prod + 2 chemins absolus doit réussir");
    }

    #[test]
    fn validate_paths_prod_relative_storage_errs() {
        let result = validate_paths(
            Some("data".to_string()),
            Some("/app/frontend/dist".to_string()),
            true,
        );
        assert!(result.is_err(), "storage relatif en prod doit échouer");
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("LATCH_STORAGE_ROOT"));
    }

    #[test]
    fn validate_paths_prod_relative_spa_errs() {
        let result = validate_paths(
            Some("/data".to_string()),
            Some("../frontend/dist".to_string()),
            true,
        );
        assert!(result.is_err(), "spa_dist relatif en prod doit échouer");
        assert!(result.unwrap_err().to_string().contains("LATCH_SPA_DIST"));
    }

    #[test]
    fn validate_paths_prod_unset_errs() {
        // Les deux unset → défauts relatifs → échec (fail-secure).
        let result = validate_paths(None, None, true);
        assert!(result.is_err(), "prod + unset doit échouer");
    }

    #[test]
    fn validate_paths_dev_relative_ok() {
        let result = validate_paths(Some("data".to_string()), None, false);
        assert!(
            result.is_ok(),
            "dev + relatif doit réussir (comportement inchangé)"
        );
    }
}
