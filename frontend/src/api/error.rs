//! Erreurs du client API. Un 401 est distingué pour piloter l'état d'auth global.

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
            ApiError::Unauthorized => t!("error.unauthorized").to_string(),
            ApiError::Status(c) => t!("error.server", code = c).to_string(),
            ApiError::Network(_) => t!("error.network").to_string(),
        }
    }
}
