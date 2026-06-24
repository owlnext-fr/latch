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
/// La route `/admin/projects` n'est pas encore implémentée (Task 6),
/// mais l'extracteur AdminAuth est testé via un 404 (route absente) ≠ 401.
/// Ce test sera pleinement significatif après Task 6.
#[tokio::test]
#[serial]
#[ignore = "needs /admin/projects (Task 6)"]
async fn protected_route_is_401_without_session() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/admin/projects").await;
        assert_eq!(res.status_code(), 401);
    })
    .await;
}

/// Login correct puis accès à une route protégée doit retourner 200.
/// Dépend de `/admin/projects` (Task 6).
#[tokio::test]
#[serial]
#[ignore = "needs /admin/projects (Task 6)"]
async fn login_then_access_protected_route() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    request::<App, _, _>(|request, _ctx| async move {
        let login = request
            .post("/admin/login")
            .json(&serde_json::json!({"user": "admin", "pass": "s3cret"}))
            .await;
        assert_eq!(login.status_code(), 200);
        // axum-test propage le cookie de session entre requêtes du même `request`.
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
#[ignore = "needs POST /admin/projects (Task 7)"]
async fn mutation_rejected_on_cross_origin() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    request::<App, _, _>(|request, _ctx| async move {
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
