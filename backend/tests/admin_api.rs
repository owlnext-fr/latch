#![allow(clippy::unwrap_used, clippy::expect_used)]

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
            .post("/api/login")
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
        let res = request.get("/api/projects").await;
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
            .post("/api/login")
            .json(&serde_json::json!({"user": "admin", "pass": "s3cret"}))
            .await;
        assert_eq!(login.status_code(), 200);
        // axum-test propage le cookie de session grâce à save_cookies(true).
        let listed = request.get("/api/projects").await;
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
            .post("/api/login")
            .json(&serde_json::json!({"user": "admin", "pass": "s3cret"}))
            .await;
        let res = request
            .post("/api/projects")
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
                .post("/api/login")
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

/// Cross-origin sur `POST /admin/logout` doit retourner 403 (garde same-origin,
/// contrat §4/§9.6). Même-origin doit laisser passer (200).
#[tokio::test]
#[serial]
async fn logout_rejected_on_cross_origin() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    let config = RequestConfigBuilder::new().save_cookies(true).build();
    request_with_config::<App, _, _>(config, |request, _ctx| async move {
        // Login préalable pour avoir une session valide.
        request
            .post("/api/login")
            .json(&serde_json::json!({"user": "admin", "pass": "s3cret"}))
            .await;

        // Cross-origin → 403.
        let cross = request
            .post("/api/logout")
            .add_header(
                axum::http::HeaderName::from_static("origin"),
                axum::http::HeaderValue::from_static("https://evil.example"),
            )
            .await;
        assert_eq!(cross.status_code(), 403, "Origin étranger sur logout ⇒ 403");

        // Same-origin → 200 (la garde laisse passer).
        let same = request
            .post("/api/logout")
            .add_header(
                axum::http::HeaderName::from_static("origin"),
                axum::http::HeaderValue::from_static("http://127.0.0.1"),
            )
            .await;
        assert_eq!(same.status_code(), 200, "Same-origin sur logout ⇒ 200");
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
            .post("/api/login")
            .json(&serde_json::json!({"user": "admin", "pass": "s3cret"}))
            .await;
        let res = request.get("/api/projects").await;
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
            .post("/api/login")
            .json(&serde_json::json!({"user": "admin", "pass": "s3cret"}))
            .await;

        let created = request
            .post("/api/projects")
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"name": "Mon Projet", "code_enabled": false}))
            .await;
        assert_eq!(created.status_code(), 200);
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();

        let got = request.get(&format!("/api/projects/{id}")).await;
        assert_eq!(got.status_code(), 200);

        let deleted = request
            .delete(&format!("/api/projects/{id}"))
            .add_header("origin", "http://127.0.0.1")
            .await;
        assert_eq!(deleted.status_code(), 200);

        let gone = request.get(&format!("/api/projects/{id}")).await;
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
            .post("/api/login")
            .json(&serde_json::json!({"user": "admin", "pass": "s3cret"}))
            .await;
        let created = request
            .post("/api/projects")
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"name": "Mon Projet", "code_enabled": false}))
            .await;
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();

        let set = request
            .post(&format!("/api/projects/{id}/code"))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"pin": "135790"}))
            .await;
        assert_eq!(set.status_code(), 200);
        assert!(set.text().contains("135790"));

        let clear = request
            .delete(&format!("/api/projects/{id}/code"))
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
            .post("/api/login")
            .json(&serde_json::json!({"user": "admin", "pass": "s3cret"}))
            .await;
        let created = request
            .post("/api/projects")
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"name": "Mon Projet", "code_enabled": false}))
            .await;
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();

        // Renommage valide.
        let updated = request
            .put(&format!("/api/projects/{id}"))
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
            .put(&format!("/api/projects/{id}"))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"name": "   "}))
            .await;
        assert_eq!(bad.status_code(), 400);
    })
    .await;
}

/// POST /admin/projects/{id}/deploy puis GET /admin/projects/{id}/versions/1/preview.
/// Vérifie : la version est créée, preview sert le HTML brut avec Cache-Control: no-store.
/// Le storage est redirigé vers un tempdir (jamais le volume de prod).
#[tokio::test]
#[serial]
async fn deploy_creates_version_and_preview_serves_html() {
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
            .json(&serde_json::json!({"name": "Mon Projet", "code_enabled": false}))
            .await;
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();

        let deployed = request
            .post(&format!("/api/projects/{id}/deploy"))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"html": "<h1>v1</h1>", "activate": true}))
            .await;
        assert_eq!(deployed.status_code(), 200);
        let v = deployed.json::<serde_json::Value>();
        assert_eq!(v["n"], 1);

        let preview = request
            .get(&format!("/api/projects/{id}/versions/1/preview"))
            .await;
        assert_eq!(preview.status_code(), 200);
        assert!(preview.text().contains("<h1>v1</h1>"));
        assert_eq!(preview.header("cache-control"), "no-store");
        assert_eq!(
            preview
                .headers()
                .get("content-security-policy")
                .map(|v| v.to_str().unwrap()),
            Some("frame-ancestors 'self'"),
        );
    })
    .await;
    // Garder `tmp` vivant jusqu'ici (drop après le test, pas avant).
    drop(tmp);
}

/// Régression : un HTML > 2 Mo (l'ancien défaut Loco `limit_payload`) doit déployer.
/// Le `body_limit` est désormais configurable (env LATCH_BODY_LIMIT, défaut 5 Mo) ;
/// sans la config, ce deploy renverrait 413 (length limit exceeded). Cf. config/*.yaml.
#[tokio::test]
#[serial]
async fn deploy_accepts_html_larger_than_2mb() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("LATCH_STORAGE_ROOT", tmp.path());
    // ~2,5 Mo : au-dessus de l'ancien défaut 2 Mo, sous le défaut 5 Mo de test.yaml.
    let big_html = format!("<h1>big</h1>{}", "a".repeat(2_500_000));
    let config = RequestConfigBuilder::new().save_cookies(true).build();
    request_with_config::<App, _, _>(config, |request, _ctx| async move {
        request
            .post("/api/login")
            .json(&serde_json::json!({"user": "admin", "pass": "s3cret"}))
            .await;
        let created = request
            .post("/api/projects")
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"name": "Mon Projet", "code_enabled": false}))
            .await;
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();

        let deployed = request
            .post(&format!("/api/projects/{id}/deploy"))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"html": big_html, "activate": true}))
            .await;
        assert_eq!(
            deployed.status_code(),
            200,
            "un HTML > 2 Mo doit déployer (body_limit configuré à 5 Mo en test)"
        );
        assert_eq!(deployed.json::<serde_json::Value>()["n"], 1);
    })
    .await;
    drop(tmp);
}

/// Bascule de version active via POST /versions/{n}/activate.
/// Vérifie que le pointeur active_version_id est bien mis à jour.
#[tokio::test]
#[serial]
async fn activate_switches_active_version() {
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
            .json(&serde_json::json!({"name": "Mon Projet", "code_enabled": false}))
            .await;
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();

        // Déployer v1 active, puis v2 inactive.
        request
            .post(&format!("/api/projects/{id}/deploy"))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"html": "a", "activate": true}))
            .await;
        request
            .post(&format!("/api/projects/{id}/deploy"))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"html": "b", "activate": false}))
            .await;

        // Basculer vers v2.
        let act = request
            .post(&format!("/api/projects/{id}/versions/2/activate"))
            .add_header("origin", "http://127.0.0.1")
            .await;
        assert_eq!(act.status_code(), 200);

        // Vérifier que le détail reflète le changement de pointeur.
        let detail = request.get(&format!("/api/projects/{id}")).await;
        let v = detail.json::<serde_json::Value>();
        let active_id = v["active_version_id"].as_i64().unwrap();
        let v2 = v["versions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["n"] == 2)
            .unwrap();
        assert_eq!(v2["id"].as_i64().unwrap(), active_id);
        assert_eq!(v2["is_active"], true);
    })
    .await;
    drop(tmp);
}

/// DELETE /versions/{n} doit supprimer une version inactive, et refuser la version active (400).
#[tokio::test]
#[serial]
async fn delete_version_refuses_active_and_removes_inactive() {
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
            .json(&serde_json::json!({"name": "Mon Projet", "code_enabled": false}))
            .await;
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();

        // v1 active, v2 inactive.
        request
            .post(&format!("/api/projects/{id}/deploy"))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"html": "a", "activate": true}))
            .await;
        request
            .post(&format!("/api/projects/{id}/deploy"))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"html": "b", "activate": false}))
            .await;

        // Refus de supprimer la version active (v1).
        let refused = request
            .delete(&format!("/api/projects/{id}/versions/1"))
            .add_header("origin", "http://127.0.0.1")
            .await;
        assert_eq!(refused.status_code(), 400);

        // Suppression de la version inactive (v2) : 200.
        let deleted = request
            .delete(&format!("/api/projects/{id}/versions/2"))
            .add_header("origin", "http://127.0.0.1")
            .await;
        assert_eq!(deleted.status_code(), 200);
    })
    .await;
    drop(tmp);
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
            .post("/api/login")
            .json(&serde_json::json!({"user": "admin", "pass": "s3cret"}))
            .await;
        let res = request.get("/api/projects/999999").await;
        assert_eq!(res.status_code(), 404);
    })
    .await;
}

/// GET /admin/projects/{id}/versions/{n}/preview sans session doit renvoyer 401.
/// `AdminAuth` rejette avant toute logique handler → pas besoin de créer un projet.
/// Invariant de sécurité : le HTML ne doit jamais être servi à un appelant non authentifié.
#[tokio::test]
#[serial]
async fn preview_requires_auth() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/api/projects/1/versions/1/preview").await;
        assert_eq!(res.status_code(), 401, "preview sans session doit être 401");
    })
    .await;
}
