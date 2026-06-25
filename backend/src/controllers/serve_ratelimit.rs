//! Extracteurs de clé `governor` pour `POST /c/{slug}/unlock` (contrat §9.5).
//! Deux couches in-memory : `IP+slug` (backoff par client) et `slug` seul
//! (plafond global, rattrape la rotation d'IP). Compteurs en RAM (reset au reboot).

use axum::http::Request;
use tower_governor::key_extractor::{KeyExtractor, SmartIpKeyExtractor};
use tower_governor::GovernorError;

/// Extrait le slug du chemin `/c/{slug}/unlock`.
pub(crate) fn slug_from_path(path: &str) -> Option<String> {
    let mut segs = path.split('/').filter(|s| !s.is_empty());
    match (segs.next(), segs.next()) {
        (Some("c"), Some(slug)) => Some(slug.to_string()),
        _ => None,
    }
}

/// Clé = `IP|slug` (backoff par client sur un projet donné).
#[derive(Clone)]
pub struct IpSlugKeyExtractor;

impl KeyExtractor for IpSlugKeyExtractor {
    type Key = String;

    fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError> {
        let ip = SmartIpKeyExtractor.extract(req)?;
        let slug = slug_from_path(req.uri().path()).ok_or(GovernorError::UnableToExtractKey)?;
        Ok(format!("{ip}|{slug}"))
    }
}

/// Clé = `slug` seul (plafond global par projet).
#[derive(Clone)]
pub struct SlugKeyExtractor;

impl KeyExtractor for SlugKeyExtractor {
    type Key = String;

    fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError> {
        slug_from_path(req.uri().path()).ok_or(GovernorError::UnableToExtractKey)
    }
}

#[cfg(test)]
mod tests {
    use super::slug_from_path;

    #[test]
    fn extracts_slug_from_unlock_path() {
        assert_eq!(
            slug_from_path("/c/demo-abc/unlock").as_deref(),
            Some("demo-abc")
        );
        assert_eq!(slug_from_path("/c/demo-abc").as_deref(), Some("demo-abc"));
    }

    #[test]
    fn rejects_non_c_paths() {
        assert_eq!(slug_from_path("/api/public/demo"), None);
        assert_eq!(slug_from_path("/"), None);
    }
}
