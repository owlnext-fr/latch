#![allow(clippy::unwrap_used, clippy::expect_used)]

use latch::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;

/// Sans session, `/api/settings` doit répondre 401 (AdminAuth).
#[tokio::test]
#[serial]
async fn settings_is_401_without_session() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/api/settings").await;
        assert_eq!(res.status_code(), 401);
    })
    .await;
}

/// Authentifié, `/api/settings` renvoie deploy_token + mcp_url + public_base_url.
#[tokio::test]
#[serial]
async fn settings_returns_mcp_info_when_authenticated() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    std::env::set_var("DEPLOY_TOKEN", "tok-abc-123");
    std::env::set_var("LATCH_PUBLIC_BASE_URL", "https://demo.test");
    let config = RequestConfigBuilder::new().save_cookies(true).build();
    request_with_config::<App, _, _>(config, |request, _ctx| async move {
        request
            .post("/api/login")
            .json(&serde_json::json!({"user": "admin", "pass": "s3cret"}))
            .await;
        let res = request.get("/api/settings").await;
        assert_eq!(res.status_code(), 200);
        let body: serde_json::Value = res.json();
        assert_eq!(body["deploy_token"], "tok-abc-123");
        assert_eq!(body["public_base_url"], "https://demo.test");
        assert_eq!(body["mcp_url"], "https://demo.test/mcp");
    })
    .await;
}
