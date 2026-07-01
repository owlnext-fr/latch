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

#[tokio::test]
#[serial]
async fn admin_reply_is_visible_to_the_visitor_thread_with_is_admin() {
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
        // Visiteur crée un fil (le client garde le cookie latch_comment).
        let pin = request
            .post(&format!("/c/{slug}/comments"))
            .add_header("origin", "http://127.0.0.1")
            .add_header("x-comment-client", "1")
            .json(&serde_json::json!({"anchor":"{}","author_name":"Léa","body":"coucou"}))
            .await;
        let pin_id = pin.json::<serde_json::Value>()["id"].as_i64().unwrap();

        // Admin répond dans le fil du visiteur.
        let reply = request
            .post(&format!(
                "/api/projects/{id}/comments/pins/{pin_id}/replies"
            ))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"body":"merci du retour"}))
            .await;
        assert_eq!(reply.status_code(), 200);
        assert_eq!(reply.json::<serde_json::Value>()["is_admin"], true);

        // Le visiteur (même client, cookie latch_comment) voit la réponse admin.
        let vlist = request.get(&format!("/c/{slug}/comments")).await;
        let vv = vlist.json::<serde_json::Value>();
        let msgs = &vv["pins"][0]["messages"];
        assert_eq!(msgs[1]["body"], "merci du retour");
        assert_eq!(msgs[1]["is_admin"], true);
        assert!(!vlist.text().contains("owner_token"));
    })
    .await;
    drop(tmp);
}

#[tokio::test]
#[serial]
async fn admin_create_own_pin_is_private_and_flagged() {
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
        request
            .post(&format!("/api/projects/{id}/deploy"))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"html":"<h1>v1</h1>","activate":true}))
            .await;

        let pin = request
            .post(&format!("/api/projects/{id}/versions/1/comments"))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"anchor":"{}","body":"note interne"}))
            .await;
        assert_eq!(pin.status_code(), 200);
        assert_eq!(
            pin.json::<serde_json::Value>()["messages"][0]["is_admin"],
            true
        );

        // Visible dans la liste admin.
        let alist = request
            .get(&format!("/api/projects/{id}/versions/1/comments"))
            .await;
        assert_eq!(
            alist.json::<serde_json::Value>()["pins"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    })
    .await;
    drop(tmp);
}

#[tokio::test]
#[serial]
async fn admin_edit_comment_and_delete_pin_are_scoped_to_project() {
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

        // Projet A (celui qui possède réellement le fil admin).
        let proj_a = request
            .post("/api/projects")
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"name":"Demo","code_enabled":false,"comments_enabled":true}))
            .await;
        let id_a = proj_a.json::<serde_json::Value>()["id"].as_i64().unwrap();
        request
            .post(&format!("/api/projects/{id_a}/deploy"))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"html":"<h1>A</h1>","activate":true}))
            .await;

        // Projet B (mauvais projet, utilisé pour la tentative hors-scope).
        let proj_b = request
            .post("/api/projects")
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"name":"Autre","code_enabled":false,"comments_enabled":true}))
            .await;
        let id_b = proj_b.json::<serde_json::Value>()["id"].as_i64().unwrap();

        // L'admin crée son propre fil dans le projet A.
        let pin = request
            .post(&format!("/api/projects/{id_a}/versions/1/comments"))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"anchor":"{}","body":"note interne"}))
            .await;
        assert_eq!(pin.status_code(), 200);
        let pin_json = pin.json::<serde_json::Value>();
        let pin_id = pin_json["id"].as_i64().unwrap();
        let cid = pin_json["messages"][0]["id"].as_i64().unwrap();

        // Edition via le mauvais projet → 404, le message n'est pas modifié.
        let edit_wrong = request
            .put(&format!("/api/projects/{id_b}/comments/messages/{cid}"))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"body":"intrus"}))
            .await;
        assert_eq!(edit_wrong.status_code(), 404);

        // Edition via le bon projet → 200.
        let edit_ok = request
            .put(&format!("/api/projects/{id_a}/comments/messages/{cid}"))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"body":"note corrigée"}))
            .await;
        assert_eq!(edit_ok.status_code(), 200);
        assert_eq!(edit_ok.json::<serde_json::Value>()["body"], "note corrigée");

        // Suppression via le mauvais projet → 404, le fil reste visible.
        let del_wrong = request
            .delete(&format!("/api/projects/{id_b}/comments/pins/{pin_id}"))
            .add_header("origin", "http://127.0.0.1")
            .await;
        assert_eq!(del_wrong.status_code(), 404);
        let still_there = request
            .get(&format!("/api/projects/{id_a}/versions/1/comments"))
            .await;
        assert_eq!(
            still_there.json::<serde_json::Value>()["pins"]
                .as_array()
                .unwrap()
                .len(),
            1
        );

        // Suppression via le bon projet → 200, le fil disparaît.
        let del_ok = request
            .delete(&format!("/api/projects/{id_a}/comments/pins/{pin_id}"))
            .add_header("origin", "http://127.0.0.1")
            .await;
        assert_eq!(del_ok.status_code(), 200);
        let gone = request
            .get(&format!("/api/projects/{id_a}/versions/1/comments"))
            .await;
        assert_eq!(
            gone.json::<serde_json::Value>()["pins"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
    })
    .await;
    drop(tmp);
}

#[tokio::test]
#[serial]
async fn admin_write_endpoints_require_session() {
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("LATCH_STORAGE_ROOT", tmp.path());
    request::<App, _, _>(|request, _ctx| async move {
        // Sans login : 401.
        let r = request
            .post("/api/projects/1/versions/1/comments")
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"anchor":"{}","body":"x"}))
            .await;
        assert_eq!(r.status_code(), 401);
    })
    .await;
    drop(tmp);
}
