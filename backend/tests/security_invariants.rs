#![allow(clippy::unwrap_used, clippy::expect_used)]

use latch::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;

/// Le PIN d'un projet ne doit jamais apparaître dans la réponse de liste (invariant §9.2).
/// Dépend de `POST /admin/projects` (Task 7) pour créer un projet via l'API.
#[tokio::test]
#[serial]
async fn pin_never_appears_in_project_list() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    let config = RequestConfigBuilder::new().save_cookies(true).build();
    request_with_config::<App, _, _>(config, |request, _ctx| async move {
        request
            .post("/api/login")
            .json(&serde_json::json!({"user": "admin", "pass": "s3cret"}))
            .await;
        // Crée un projet protégé via l'API admin (Task 7).
        request
            .post("/api/projects")
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"name": "Mon Projet", "code_enabled": true, "pin": "424242"}))
            .await;
        let list = request.get("/api/projects").await;
        let body = list.text();
        assert!(
            !body.contains("424242"),
            "PIN fuité dans la liste (viole §9.2)"
        );
        assert!(
            !body.contains("\"pin\""),
            "champ pin présent en liste (viole §9.2)"
        );
    })
    .await;
}

/// Le PIN doit apparaître sur le détail d'un projet (invariant §9.2 — visible en admin).
/// Dépend de `POST /admin/projects` (Task 7) pour créer un projet via l'API.
#[tokio::test]
#[serial]
async fn pin_appears_on_project_detail() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    let config = RequestConfigBuilder::new().save_cookies(true).build();
    request_with_config::<App, _, _>(config, |request, _ctx| async move {
        request
            .post("/api/login")
            .json(&serde_json::json!({"user": "admin", "pass": "s3cret"}))
            .await;
        let created = request
            .post("/api/projects")
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"name": "Mon Projet", "code_enabled": true, "pin": "424242"}))
            .await;
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();
        let detail = request.get(&format!("/api/projects/{id}")).await;
        assert!(
            detail.text().contains("424242"),
            "le détail doit exposer le PIN"
        );
    })
    .await;
}

/// `owner_token` ne doit jamais apparaître dans une réponse de commentaire (public OU admin) — invariant §9.7.
#[tokio::test]
#[serial]
async fn owner_token_never_in_comment_responses() {
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
        let posted = request
            .post(&format!("/c/{slug}/comments"))
            .add_header("origin", "http://127.0.0.1")
            .add_header("x-comment-client", "1")
            .json(&serde_json::json!({"anchor":"{}","author_name":"Léa","body":"x"}))
            .await;
        assert!(
            !posted.text().contains("owner_token"),
            "POST réponse fuite owner_token"
        );
        let v = request.get(&format!("/c/{slug}/comments")).await;
        assert!(
            !v.text().contains("owner_token"),
            "GET visiteur fuite owner_token"
        );
        let a = request
            .get(&format!("/api/projects/{id}/versions/1/comments"))
            .await;
        assert!(
            !a.text().contains("owner_token"),
            "GET admin fuite owner_token"
        );
    })
    .await;
    drop(tmp);
}

/// Un projet à code non déverrouillé refuse les commentaires (403) — invariant §9.7 (gate).
#[tokio::test]
#[serial]
async fn locked_project_forbids_comments() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("LATCH_STORAGE_ROOT", tmp.path());
    let config = RequestConfigBuilder::new().save_cookies(true).build();
    request_with_config::<App, _, _>(config, |request, _ctx| async move {
        request.post("/api/login").json(&serde_json::json!({"user":"admin","pass":"s3cret"})).await;
        let created = request.post("/api/projects").add_header("origin","http://127.0.0.1")
            .json(&serde_json::json!({"name":"Demo","code_enabled":true,"pin":"424242","comments_enabled":true})).await;
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();
        let slug = created.json::<serde_json::Value>()["slug"].as_str().unwrap().to_string();
        request.post(&format!("/api/projects/{id}/deploy")).add_header("origin","http://127.0.0.1")
            .json(&serde_json::json!({"html":"<h1>v1</h1>","activate":true})).await;
        // pas de cookie unlock → 403 (note: la session admin n'est pas un cookie unlock)
        let listed = request.get(&format!("/c/{slug}/comments")).await;
        assert_eq!(listed.status_code(), 403, "projet verrouillé doit refuser (403)");
    }).await;
    drop(tmp);
}
