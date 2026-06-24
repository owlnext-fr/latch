//! Erreurs du client API. Un 401 est distingué pour piloter l'état d'auth global.
// consumed in T7-T13
#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq)]
pub enum ApiError {
    /// 401 — session absente/expirée. Bascule l'app en Anonymous.
    Unauthorized,
    /// Autre code HTTP non-2xx.
    Status(u16),
    /// Échec réseau / parse JSON.
    Network(String),
}

impl From<gloo_net::Error> for ApiError {
    fn from(e: gloo_net::Error) -> Self {
        ApiError::Network(e.to_string())
    }
}

impl ApiError {
    /// Message court présentable à l'utilisateur (inline).
    pub fn user_message(&self) -> String {
        match self {
            ApiError::Unauthorized => "Session expirée, reconnecte-toi.".into(),
            ApiError::Status(c) => format!("Erreur serveur ({c})."),
            ApiError::Network(_) => "Erreur réseau, réessaie.".into(),
        }
    }
}
