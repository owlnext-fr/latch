use latch::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;

/// Smoke test : l'application démarre avec le layer session monté et répond à
/// `/_ping` (route de monitoring par défaut de Loco). Si `after_routes` ou
/// `build_session_store` paniquent, ce test échoue au boot.
#[tokio::test]
#[serial]
async fn boots_with_session_layer_and_serves_health() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/_ping").await;
        assert_eq!(res.status_code(), 200);
    })
    .await;
}

/// `POST /admin/login` avec des credentials incorrects doit retourner 401.
#[tokio::test]
#[serial]
async fn login_rejects_bad_credentials() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    request::<App, _, _>(|request, _ctx| async move {
        let res = request
            .post("/admin/login")
            .json(&serde_json::json!({"user": "admin", "pass": "wrong"}))
            .await;
        assert_eq!(res.status_code(), 401);
    })
    .await;
}

/// Une route protégée par `AdminAuth` doit renvoyer 401 sans session active.
#[tokio::test]
#[serial]
async fn protected_route_is_401_without_session() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/admin/projects").await;
        assert_eq!(res.status_code(), 401);
    })
    .await;
}

/// Login correct puis accès à une route protégée doit retourner 200.
/// `save_cookies(true)` est requis pour que axum-test propage le cookie de session.
#[tokio::test]
#[serial]
async fn login_then_access_protected_route() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    let config = RequestConfigBuilder::new().save_cookies(true).build();
    request_with_config::<App, _, _>(config, |request, _ctx| async move {
        let login = request
            .post("/admin/login")
            .json(&serde_json::json!({"user": "admin", "pass": "s3cret"}))
            .await;
        assert_eq!(login.status_code(), 200);
        // axum-test propage le cookie de session grâce à save_cookies(true).
        let listed = request.get("/admin/projects").await;
        assert_eq!(listed.status_code(), 200);
    })
    .await;
}

/// Cross-origin sur une mutation admin doit retourner 403 (contrat §4/§9.6).
/// `POST /admin/projects` n'est pas encore implémentée (Task 7) ; le test sera
/// activé quand la route existera.
#[tokio::test]
#[serial]
async fn mutation_rejected_on_cross_origin() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    let config = RequestConfigBuilder::new().save_cookies(true).build();
    request_with_config::<App, _, _>(config, |request, _ctx| async move {
        // login d'abord (sinon 401 masquerait le 403)
        request
            .post("/admin/login")
            .json(&serde_json::json!({"user": "admin", "pass": "s3cret"}))
            .await;
        let res = request
            .post("/admin/projects")
            .add_header(
                axum::http::HeaderName::from_static("origin"),
                axum::http::HeaderValue::from_static("https://evil.example"),
            )
            .json(&serde_json::json!({"name": "X"}))
            .await;
        assert_eq!(res.status_code(), 403, "Origin étranger sur mutation ⇒ 403");
    })
    .await;
}

/// Le login doit être rate-limité : après plusieurs tentatives échouées,
/// on doit recevoir un 429. Header X-Forwarded-For injecté pour garantir
/// que SmartIpKeyExtractor peut extraire une clé stable.
#[tokio::test]
#[serial]
async fn login_is_rate_limited() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    request::<App, _, _>(|request, _ctx| async move {
        let mut saw_429 = false;
        for _ in 0..20 {
            let res = request
                .post("/admin/login")
                .add_header(
                    axum::http::HeaderName::from_static("x-forwarded-for"),
                    axum::http::HeaderValue::from_static("1.2.3.4"),
                )
                .json(&serde_json::json!({"user": "admin", "pass": "wrong"}))
                .await;
            if res.status_code() == 429 {
                saw_429 = true;
                break;
            }
        }
        assert!(
            saw_429,
            "le login doit finir par renvoyer 429 (rate-limit load-bearing)"
        );
    })
    .await;
}

/// GET /admin/projects avec session active et base vide doit renvoyer 200 + tableau vide.
/// `save_cookies(true)` est requis pour que axum-test propage le cookie de session.
#[tokio::test]
#[serial]
async fn list_projects_returns_empty_array_when_none() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    let config = RequestConfigBuilder::new().save_cookies(true).build();
    request_with_config::<App, _, _>(config, |request, _ctx| async move {
        request
            .post("/admin/login")
            .json(&serde_json::json!({"user": "admin", "pass": "s3cret"}))
            .await;
        let res = request.get("/admin/projects").await;
        assert_eq!(res.status_code(), 200);
        assert_eq!(res.json::<serde_json::Value>(), serde_json::json!([]));
    })
    .await;
}

/// Création, lecture, suppression d'un projet via l'API — flux complet.
/// Origin same-origin requis sur les mutations (contrat §4/§9.6).
#[tokio::test]
#[serial]
async fn create_then_get_and_delete_project() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    let config = RequestConfigBuilder::new().save_cookies(true).build();
    request_with_config::<App, _, _>(config, |request, _ctx| async move {
        request
            .post("/admin/login")
            .json(&serde_json::json!({"user": "admin", "pass": "s3cret"}))
            .await;

        let created = request
            .post("/admin/projects")
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"name": "Mon Projet", "code_enabled": false}))
            .await;
        assert_eq!(created.status_code(), 200);
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();

        let got = request.get(&format!("/admin/projects/{id}")).await;
        assert_eq!(got.status_code(), 200);

        let deleted = request
            .delete(&format!("/admin/projects/{id}"))
            .add_header("origin", "http://127.0.0.1")
            .await;
        assert_eq!(deleted.status_code(), 200);

        let gone = request.get(&format!("/admin/projects/{id}")).await;
        assert_eq!(gone.status_code(), 404);
    })
    .await;
}

/// Activation et désactivation du code d'accès via les endpoints dédiés.
#[tokio::test]
#[serial]
async fn set_and_clear_code_via_api() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    let config = RequestConfigBuilder::new().save_cookies(true).build();
    request_with_config::<App, _, _>(config, |request, _ctx| async move {
        request
            .post("/admin/login")
            .json(&serde_json::json!({"user": "admin", "pass": "s3cret"}))
            .await;
        let created = request
            .post("/admin/projects")
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"name": "Mon Projet", "code_enabled": false}))
            .await;
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();

        let set = request
            .post(&format!("/admin/projects/{id}/code"))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"pin": "135790"}))
            .await;
        assert_eq!(set.status_code(), 200);
        assert!(set.text().contains("135790"));

        let clear = request
            .delete(&format!("/admin/projects/{id}/code"))
            .add_header("origin", "http://127.0.0.1")
            .await;
        assert_eq!(clear.status_code(), 200);
    })
    .await;
}

/// PUT /admin/projects/{id} — renomme un projet et rejette un nom vide.
#[tokio::test]
#[serial]
async fn update_project_changes_name_and_rejects_empty() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    let config = RequestConfigBuilder::new().save_cookies(true).build();
    request_with_config::<App, _, _>(config, |request, _ctx| async move {
        request
            .post("/admin/login")
            .json(&serde_json::json!({"user": "admin", "pass": "s3cret"}))
            .await;
        let created = request
            .post("/admin/projects")
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"name": "Mon Projet", "code_enabled": false}))
            .await;
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();

        // Renommage valide.
        let updated = request
            .put(&format!("/admin/projects/{id}"))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"name": "Renommé"}))
            .await;
        assert_eq!(updated.status_code(), 200);
        assert_eq!(
            updated.json::<serde_json::Value>()["name"]
                .as_str()
                .unwrap(),
            "Renommé"
        );

        // Nom vide (whitespace) → 400.
        let bad = request
            .put(&format!("/admin/projects/{id}"))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"name": "   "}))
            .await;
        assert_eq!(bad.status_code(), 400);
    })
    .await;
}

/// GET /admin/projects/{id} sur un id inexistant doit renvoyer 404.
/// `save_cookies(true)` est requis pour que axum-test propage le cookie de session.
#[tokio::test]
#[serial]
async fn detail_returns_404_for_unknown_id() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    let config = RequestConfigBuilder::new().save_cookies(true).build();
    request_with_config::<App, _, _>(config, |request, _ctx| async move {
        request
            .post("/admin/login")
            .json(&serde_json::json!({"user": "admin", "pass": "s3cret"}))
            .await;
        let res = request.get("/admin/projects/999999").await;
        assert_eq!(res.status_code(), 404);
    })
    .await;
}
