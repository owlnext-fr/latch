//! Extracteur JSON validant : désérialise puis appelle `.validate()` (contrat §1,
//! validation de forme à la frontière). Échec de validation → 400.

use axum::extract::{FromRequest, Request};
use axum::response::{IntoResponse, Response};
use axum::Json;
use validator::Validate;

use crate::controllers::error::into_response;
use crate::services::errors::CoreError;

pub struct ValidatedJson<T>(pub T);

impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: serde::de::DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(IntoResponse::into_response)?;
        value
            .validate()
            .map_err(|e| into_response(CoreError::Validation(e.to_string())).into_response())?;
        Ok(ValidatedJson(value))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::{routing::post, Router};
    use tower::ServiceExt;
    use validator::Validate;

    #[derive(serde::Deserialize, Validate)]
    struct Toy {
        #[validate(length(min = 1, max = 3))]
        s: String,
    }

    async fn handler(ValidatedJson(_t): ValidatedJson<Toy>) -> &'static str {
        "ok"
    }

    #[tokio::test]
    async fn rejects_invalid_with_400() {
        let app = Router::new().route("/t", post(handler));
        let res = app
            .clone()
            .oneshot(
                Request::post("/t")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"s":"toolong"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let ok = app
            .oneshot(
                Request::post("/t")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"s":"ok"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(ok.status(), StatusCode::OK);
    }
}
