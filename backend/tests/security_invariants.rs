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
