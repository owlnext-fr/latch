//! Erreur du cœur métier — agnostique HTTP (contrat §1).
//! Chaque adaptateur (web, MCP) mappe `CoreError` vers son propre type de réponse.

use sea_orm::DbErr;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    /// Ressource absente (projet/slug/version inconnu).
    #[error("resource not found")]
    NotFound,

    /// Entrée invalide (nom vide, PIN mal formé…).
    #[error("validation error: {0}")]
    Validation(String),

    /// Erreur de la couche ORM/DB.
    #[error(transparent)]
    Db(#[from] DbErr),

    /// Erreur d'I/O (couche `Storage`).
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_from_io_error() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let err: CoreError = io.into();
        assert!(matches!(err, CoreError::Io(_)));
    }

    #[test]
    fn not_found_displays_message() {
        assert_eq!(CoreError::NotFound.to_string(), "resource not found");
    }
}
