#![allow(clippy::unwrap_used, clippy::expect_used)]

use latch::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn admin_lists_all_comments_without_owner_token() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("LATCH_STORAGE_ROOT", tmp.path());
    let config = RequestConfigBuilder::new().save_cookies(true).build();
    request_with_config::<App, _, _>(config, |request, _ctx| async move {
        request
            .post("/api/login")
            .json(&serde_json::json!({"user":"admin","pass":"s3cret"}))
            .await;
        let created = request
            .post("/api/projects")
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"name":"Demo","code_enabled":false,"comments_enabled":true}))
            .await;
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();
        let slug = created.json::<serde_json::Value>()["slug"]
            .as_str()
            .unwrap()
            .to_string();
        request
            .post(&format!("/api/projects/{id}/deploy"))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"html":"<h1>v1</h1>","activate":true}))
            .await;
        request
            .post(&format!("/c/{slug}/comments"))
            .add_header("origin", "http://127.0.0.1")
            .add_header("x-comment-client", "1")
            .json(&serde_json::json!({"anchor":"{}","author_name":"Léa","body":"coucou"}))
            .await;

        let admin = request
            .get(&format!("/api/projects/{id}/versions/1/comments"))
            .await;
        assert_eq!(admin.status_code(), 200);
        let v = admin.json::<serde_json::Value>();
        assert_eq!(v["pins"][0]["messages"][0]["author_name"], "Léa");
        assert_eq!(v["pins"][0]["messages"][0]["body"], "coucou");
        assert!(!admin.text().contains("owner_token"));
        assert!(!admin.text().contains("editable"));

        // comment_count visible dans le détail projet
        let detail = request.get(&format!("/api/projects/{id}")).await;
        let dv = detail.json::<serde_json::Value>();
        assert_eq!(dv["versions"][0]["comment_count"], 1);
    })
    .await;
    drop(tmp);
}

#[tokio::test]
#[serial]
async fn admin_moderates_a_message() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("LATCH_STORAGE_ROOT", tmp.path());
    let config = RequestConfigBuilder::new().save_cookies(true).build();
    request_with_config::<App, _, _>(config, |request, _ctx| async move {
        request
            .post("/api/login")
            .json(&serde_json::json!({"user":"admin","pass":"s3cret"}))
            .await;
        let created = request
            .post("/api/projects")
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"name":"Demo","code_enabled":false,"comments_enabled":true}))
            .await;
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();
        let slug = created.json::<serde_json::Value>()["slug"]
            .as_str()
            .unwrap()
            .to_string();
        request
            .post(&format!("/api/projects/{id}/deploy"))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"html":"<h1>v1</h1>","activate":true}))
            .await;
        let pin = request
            .post(&format!("/c/{slug}/comments"))
            .add_header("origin", "http://127.0.0.1")
            .add_header("x-comment-client", "1")
            .json(&serde_json::json!({"anchor":"{}","author_name":"Léa","body":"spam"}))
            .await;
        let mid = pin.json::<serde_json::Value>()["messages"][0]["id"]
            .as_i64()
            .unwrap();

        let del = request
            .delete(&format!("/api/projects/{id}/comments/messages/{mid}"))
            .add_header("origin", "http://127.0.0.1")
            .await;
        assert_eq!(del.status_code(), 200);
        let admin = request
            .get(&format!("/api/projects/{id}/versions/1/comments"))
            .await;
        assert_eq!(
            admin.json::<serde_json::Value>()["pins"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
    })
    .await;
    drop(tmp);
}
