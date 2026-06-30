#![allow(clippy::unwrap_used, clippy::expect_used)]

use latch::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn create_then_list_returns_only_own_comment() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("LATCH_STORAGE_ROOT", tmp.path());
    let config = RequestConfigBuilder::new().save_cookies(true).build();
    request_with_config::<App, _, _>(config, |request, _ctx| async move {
        request
            .post("/api/login")
            .json(&serde_json::json!({"user": "admin", "pass": "s3cret"}))
            .await;
        let created = request
            .post("/api/projects")
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"name": "Demo", "code_enabled": false, "comments_enabled": true}))
            .await;
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();
        let slug = created.json::<serde_json::Value>()["slug"].as_str().unwrap().to_string();
        request
            .post(&format!("/api/projects/{id}/deploy"))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"html": "<h1>v1</h1>", "activate": true}))
            .await;

        // POST comment (visitor): needs Origin + X-Comment-Client
        let posted = request
            .post(&format!("/c/{slug}/comments"))
            .add_header("origin", "http://127.0.0.1")
            .add_header("x-comment-client", "1")
            .json(&serde_json::json!({"anchor": "{\"v\":1}", "author_name": "Léa", "body": "trop petit"}))
            .await;
        assert_eq!(posted.status_code(), 200);
        assert_eq!(posted.header("cache-control"), "no-store");
        let body = posted.text();
        assert!(!body.contains("owner_token"));

        // GET list (same cookie jar = same owner)
        let listed = request.get(&format!("/c/{slug}/comments")).await;
        assert_eq!(listed.status_code(), 200);
        let v = listed.json::<serde_json::Value>();
        assert_eq!(v["pins"].as_array().unwrap().len(), 1);
        assert_eq!(v["pins"][0]["messages"][0]["author_name"], "Léa");
        assert_eq!(v["pins"][0]["messages"][0]["editable"], true);
        assert!(!listed.text().contains("owner_token"));
    })
    .await;
    drop(tmp);
}

#[tokio::test]
#[serial]
async fn create_requires_comment_client_header() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("LATCH_STORAGE_ROOT", tmp.path());
    let config = RequestConfigBuilder::new().save_cookies(true).build();
    request_with_config::<App, _, _>(config, |request, _ctx| async move {
        request
            .post("/api/login")
            .json(&serde_json::json!({"user": "admin", "pass": "s3cret"}))
            .await;
        let created = request
            .post("/api/projects")
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"name": "Demo", "code_enabled": false, "comments_enabled": true}))
            .await;
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();
        let slug = created.json::<serde_json::Value>()["slug"].as_str().unwrap().to_string();
        request
            .post(&format!("/api/projects/{id}/deploy"))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"html": "<h1>v1</h1>", "activate": true}))
            .await;
        // No X-Comment-Client → 403
        let posted = request
            .post(&format!("/c/{slug}/comments"))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"anchor": "{}", "author_name": "Léa", "body": "x"}))
            .await;
        assert_eq!(posted.status_code(), 403);
    })
    .await;
    drop(tmp);
}

#[tokio::test]
#[serial]
async fn comments_disabled_project_returns_404() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("LATCH_STORAGE_ROOT", tmp.path());
    let config = RequestConfigBuilder::new().save_cookies(true).build();
    request_with_config::<App, _, _>(config, |request, _ctx| async move {
        request
            .post("/api/login")
            .json(&serde_json::json!({"user": "admin", "pass": "s3cret"}))
            .await;
        let created = request
            .post("/api/projects")
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"name": "Demo", "code_enabled": false, "comments_enabled": false}))
            .await;
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();
        let slug = created.json::<serde_json::Value>()["slug"].as_str().unwrap().to_string();
        request
            .post(&format!("/api/projects/{id}/deploy"))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"html": "<h1>v1</h1>", "activate": true}))
            .await;
        let listed = request.get(&format!("/c/{slug}/comments")).await;
        assert_eq!(listed.status_code(), 404);
    })
    .await;
    drop(tmp);
}
