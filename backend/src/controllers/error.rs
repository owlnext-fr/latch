//! Traduction du `CoreError` (cœur, agnostique HTTP) vers l'erreur Loco/axum.
//! C'est ICI que vit la frontière HTTP — jamais dans `services/` (contrat §1).

use crate::services::errors::CoreError;

/// Mappe une erreur métier vers le type d'erreur Loco (→ status HTTP).
pub fn into_response(err: CoreError) -> loco_rs::Error {
    match err {
        CoreError::NotFound => loco_rs::Error::NotFound,
        CoreError::Validation(msg) => loco_rs::Error::BadRequest(msg),
        CoreError::Db(e) => loco_rs::Error::Message(format!("db error: {e}")),
        CoreError::Io(e) => loco_rs::Error::Message(format!("io error: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::errors::CoreError;

    #[test]
    fn not_found_maps_to_404() {
        let e = into_response(CoreError::NotFound);
        assert!(matches!(e, loco_rs::Error::NotFound));
    }

    #[test]
    fn validation_maps_to_400() {
        let e = into_response(CoreError::Validation("bad".into()));
        assert!(matches!(e, loco_rs::Error::BadRequest(_)));
    }
}
