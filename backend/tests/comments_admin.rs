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
async fn admin_cannot_moderate_other_projects_message() {
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

        // Projet A
        let proj_a = request
            .post("/api/projects")
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"name":"ACME","code_enabled":false,"comments_enabled":true}))
            .await;
        let id_a = proj_a.json::<serde_json::Value>()["id"].as_i64().unwrap();

        // Projet B
        let proj_b = request
            .post("/api/projects")
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"name":"Demo","code_enabled":false,"comments_enabled":true}))
            .await;
        let id_b = proj_b.json::<serde_json::Value>()["id"].as_i64().unwrap();
        let slug_b = proj_b.json::<serde_json::Value>()["slug"]
            .as_str()
            .unwrap()
            .to_string();

        // Déploiement d'une version sur chaque projet
        request
            .post(&format!("/api/projects/{id_a}/deploy"))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"html":"<h1>A</h1>","activate":true}))
            .await;
        request
            .post(&format!("/api/projects/{id_b}/deploy"))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"html":"<h1>B</h1>","activate":true}))
            .await;

        // Commentaire sur le projet B (flux visiteur)
        let pin = request
            .post(&format!("/c/{slug_b}/comments"))
            .add_header("origin", "http://127.0.0.1")
            .add_header("x-comment-client", "1")
            .json(&serde_json::json!({"anchor":"{}","author_name":"Léa","body":"bonjour B"}))
            .await;
        let mid_b = pin.json::<serde_json::Value>()["messages"][0]["id"]
            .as_i64()
            .unwrap();

        // Tentative de modération via le projet A → doit renvoyer 404
        let del = request
            .delete(&format!("/api/projects/{id_a}/comments/messages/{mid_b}"))
            .add_header("origin", "http://127.0.0.1")
            .await;
        assert_eq!(del.status_code(), 404);

        // Le commentaire du projet B est toujours présent
        let list_b = request
            .get(&format!("/api/projects/{id_b}/versions/1/comments"))
            .await;
        assert_eq!(list_b.status_code(), 200);
        let pins_len = list_b.json::<serde_json::Value>()["pins"]
            .as_array()
            .unwrap()
            .len();
        assert_eq!(
            pins_len, 1,
            "le commentaire du projet B ne doit pas avoir été supprimé"
        );
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
