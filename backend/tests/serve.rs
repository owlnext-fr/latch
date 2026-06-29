#![allow(clippy::unwrap_used, clippy::expect_used)]

use latch::app::App;
use latch::models::_entities::{projects, versions};
use loco_rs::testing::prelude::*;
use sea_orm::{ActiveModelTrait, Set};
use serial_test::serial;

/// Insère un projet et renvoie son modèle.
async fn make_project(
    db: &sea_orm::DatabaseConnection,
    slug: &str,
    code_enabled: bool,
    pin: Option<&str>,
    brand: Option<&str>,
) -> projects::Model {
    projects::ActiveModel {
        slug: Set(slug.to_string()),
        name: Set("Mon Projet".to_string()),
        code_enabled: Set(code_enabled),
        pin: Set(pin.map(str::to_string)),
        brand_name: Set(brand.map(str::to_string)),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("insert project")
}

/// Prépare un faux `dist/` avec un unlock.html + error.html + shell.html reconnaissables,
/// et pointe LATCH_SPA_DIST.
fn fake_dist() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("unlock.html"),
        "<!doctype html><title>latch-unlock</title>",
    )
    .expect("write unlock.html");
    std::fs::write(
        dir.path().join("error.html"),
        "<!doctype html><title>latch</title><div id=\"error-root\">latch-error</div>",
    )
    .expect("write error.html");
    std::fs::write(
        dir.path().join("shell.html"),
        "<!doctype html><title>latch</title><div id=\"shell-root\">latch-shell</div>",
    )
    .expect("write shell.html");
    std::env::set_var("LATCH_SPA_DIST", dir.path());
    dir
}

/// Crée une version + écrit son HTML dans un storage temporaire (LATCH_STORAGE_ROOT),
/// active la version sur le projet. Renvoie le tempdir storage (à garder vivant).
async fn deploy_active(
    db: &sea_orm::DatabaseConnection,
    project: &projects::Model,
    html: &str,
) -> tempfile::TempDir {
    let storage = tempfile::tempdir().expect("storage tempdir");
    std::env::set_var("LATCH_STORAGE_ROOT", storage.path());
    let html_path = format!("{}/1.html", project.id);
    std::fs::create_dir_all(storage.path().join(project.id.to_string())).unwrap();
    std::fs::write(storage.path().join(&html_path), html).unwrap();
    let v = versions::ActiveModel {
        project_id: Set(project.id),
        n: Set(1),
        html_path: Set(html_path),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("insert version");
    let mut p: projects::ActiveModel = project.clone().into();
    p.active_version_id = Set(Some(v.id));
    p.update(db).await.expect("activate");
    storage
}

/// Comme `deploy_active` mais avec release_notes renseignées.
async fn deploy_active_with_notes(
    db: &sea_orm::DatabaseConnection,
    project: &projects::Model,
    html: &str,
    notes: &str,
) -> tempfile::TempDir {
    let storage = tempfile::tempdir().expect("storage tempdir");
    std::env::set_var("LATCH_STORAGE_ROOT", storage.path());
    let html_path = format!("{}/1.html", project.id);
    std::fs::create_dir_all(storage.path().join(project.id.to_string())).unwrap();
    std::fs::write(storage.path().join(&html_path), html).unwrap();
    let v = versions::ActiveModel {
        project_id: Set(project.id),
        n: Set(1),
        html_path: Set(html_path),
        release_notes: Set(Some(notes.to_string())),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("insert version");
    let mut p: projects::ActiveModel = project.clone().into();
    p.active_version_id = Set(Some(v.id));
    p.update(db).await.expect("activate");
    storage
}

#[tokio::test]
#[serial]
async fn public_meta_returns_brand_and_code_without_pin() {
    request::<App, _, _>(|request, ctx| async move {
        make_project(&ctx.db, "demo-aaaaaaaa", true, Some("424242"), Some("ACME")).await;
        let res = request.get("/api/public/demo-aaaaaaaa").await;
        res.assert_status_ok();
        let body = res.text();
        assert!(body.contains("ACME"), "brand_name attendu");
        assert!(body.contains("code_enabled"));
        assert!(
            !body.contains("424242"),
            "le PIN ne doit JAMAIS fuiter (§9.2)"
        );
        assert!(!body.contains("\"pin\""), "pas de champ pin (§9.2)");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn public_meta_unknown_slug_is_404() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/api/public/nope-xxxxxxxx").await;
        assert_eq!(res.status_code(), 404);
    })
    .await;
}

// --- Tests adaptés : /c/<slug> sert maintenant le shell, le HTML brut est sur /raw ---

#[tokio::test]
#[serial]
async fn open_project_serves_active_html_no_store() {
    let _dist = fake_dist();
    request::<App, _, _>(|request, ctx| async move {
        let p = make_project(&ctx.db, "libre-aaaaaaaa", false, None, None).await;
        let _storage = deploy_active(&ctx.db, &p, "<h1>PROTO-LIBRE</h1>").await;

        // /c/<slug>/raw → HTML brut du proto + no-store + frame-ancestors 'self'.
        let res = request.get("/c/libre-aaaaaaaa/raw").await;
        res.assert_status_ok();
        assert!(res.text().contains("PROTO-LIBRE"));
        assert_eq!(
            res.headers().get("cache-control").unwrap(),
            "no-store",
            "tout /c doit être no-store (§6)"
        );
        assert_eq!(
            res.headers().get("content-security-policy").unwrap(),
            "frame-ancestors 'self'",
            "/raw doit porter frame-ancestors 'self' (CSP iframe)"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn protected_project_without_cookie_serves_unlock_page() {
    let _dist = fake_dist();
    request::<App, _, _>(|request, ctx| async move {
        let p = make_project(&ctx.db, "prot-aaaaaaaa", true, Some("123456"), None).await;
        let _storage = deploy_active(&ctx.db, &p, "<h1>SECRET</h1>").await;
        let res = request.get("/c/prot-aaaaaaaa").await;
        res.assert_status_ok(); // 200, PAS 401 (contrat §6 / QUIRKS)
        assert!(res.text().contains("latch-unlock"), "rend unlock.html");
        assert!(
            !res.text().contains("SECRET"),
            "le proto ne fuit pas sans déverrouillage"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn unknown_slug_serves_styled_error_404() {
    let _dist = fake_dist();
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/c/nope-xxxxxxxx").await;
        assert_eq!(res.status_code(), 404);
        assert_eq!(
            res.headers().get("content-type").unwrap(),
            "text/html; charset=utf-8",
            "page d'erreur HTML, pas du JSON"
        );
        assert_eq!(res.headers().get("cache-control").unwrap(), "no-store");
        assert!(res.text().contains("error-root"), "rend error.html");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn project_without_active_version_serves_styled_error_404() {
    let _dist = fake_dist();
    request::<App, _, _>(|request, ctx| async move {
        make_project(&ctx.db, "vide-aaaaaaaa", false, None, None).await;
        let res = request.get("/c/vide-aaaaaaaa").await;
        assert_eq!(res.status_code(), 404);
        assert_eq!(
            res.headers().get("content-type").unwrap(),
            "text/html; charset=utf-8"
        );
        assert!(res.text().contains("error-root"));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn unlock_wrong_pin_is_401_no_cookie() {
    let _dist = fake_dist();
    request::<App, _, _>(|request, ctx| async move {
        let p = make_project(&ctx.db, "prot-bbbbbbbb", true, Some("123456"), None).await;
        let _storage = deploy_active(&ctx.db, &p, "<h1>SECRET</h1>").await;
        let res = request
            .post("/c/prot-bbbbbbbb/unlock")
            .json(&serde_json::json!({ "pin": "000000" }))
            .await;
        assert_eq!(res.status_code(), 401);
        // Vérifie qu'aucun cookie `latch_unlock` n'est posé sur échec.
        // (Le session middleware peut poser son propre cookie `latch_admin` — ignoré ici.)
        let has_unlock_cookie = res
            .headers()
            .get_all("set-cookie")
            .iter()
            .any(|v| v.to_str().unwrap_or("").contains("latch_unlock"));
        assert!(!has_unlock_cookie, "pas de cookie unlock sur échec");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn unlock_good_pin_sets_cookie_then_serves_proto() {
    let _dist = fake_dist();
    let config = RequestConfigBuilder::new().save_cookies(true).build();
    request_with_config::<App, _, _>(config, |request, ctx| async move {
        let p = make_project(&ctx.db, "prot-cccccccc", true, Some("123456"), None).await;
        let _storage = deploy_active(&ctx.db, &p, "<h1>SECRET-OK</h1>").await;

        let unlocked = request
            .post("/c/prot-cccccccc/unlock")
            .json(&serde_json::json!({ "pin": "123456" }))
            .await;
        assert_eq!(unlocked.status_code(), 204);
        assert!(
            unlocked.headers().get("set-cookie").is_some(),
            "cookie posé"
        );

        // save_cookies(true) renvoie le cookie → le GET /raw sert maintenant le proto brut.
        let served = request.get("/c/prot-cccccccc/raw").await;
        served.assert_status_ok();
        assert!(served.text().contains("SECRET-OK"));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn unlock_rate_limited_after_burst() {
    let _dist = fake_dist();
    request::<App, _, _>(|request, ctx| async move {
        let p = make_project(&ctx.db, "prot-eeeeeeee", true, Some("123456"), None).await;
        let _storage = deploy_active(&ctx.db, &p, "<h1>x</h1>").await;
        // Burst IP+slug = 5 ; au-delà → 429. Clé IP fixée via X-Forwarded-For.
        let mut got_429 = false;
        for _ in 0..12 {
            let res = request
                .post("/c/prot-eeeeeeee/unlock")
                .add_header(
                    axum::http::HeaderName::from_static("x-forwarded-for"),
                    axum::http::HeaderValue::from_static("9.9.9.9"),
                )
                .json(&serde_json::json!({ "pin": "000000" }))
                .await;
            if res.status_code() == 429 {
                got_429 = true;
                break;
            }
        }
        assert!(got_429, "le burst dépassé doit déclencher un 429 (§9.5)");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn missing_error_html_falls_back_to_inline_text() {
    // dist sans error.html → fallback inline (pas de JSON brut), toujours no-store.
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("unlock.html"), "<title>u</title>").unwrap();
    std::env::set_var("LATCH_SPA_DIST", dir.path());
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/c/nope-yyyyyyyy").await;
        assert_eq!(res.status_code(), 404);
        assert_eq!(res.headers().get("cache-control").unwrap(), "no-store");
        assert!(
            res.text().contains("pas disponible"),
            "fallback inline HTML, pas du JSON"
        );
        assert!(!res.text().contains("{"), "pas de JSON brut");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn rotating_pin_invalidates_cookie() {
    let _dist = fake_dist();
    let config = RequestConfigBuilder::new().save_cookies(true).build();
    request_with_config::<App, _, _>(config, |request, ctx| async move {
        let p = make_project(&ctx.db, "prot-dddddddd", true, Some("123456"), None).await;
        let _storage = deploy_active(&ctx.db, &p, "<h1>SECRET-ROT</h1>").await;

        // Déverrouille → cookie valide → proto servi sur /raw.
        request
            .post("/c/prot-dddddddd/unlock")
            .json(&serde_json::json!({ "pin": "123456" }))
            .await;
        assert!(request
            .get("/c/prot-dddddddd/raw")
            .await
            .text()
            .contains("SECRET-ROT"));

        // Rotation du PIN (set_code) → le cookie émis sous l'ancien PIN doit être rejeté.
        latch::services::projects::ProjectsService::new(ctx.db.clone())
            .set_code(p.id, "654321")
            .await
            .unwrap();
        // Le shell /c/<slug> doit maintenant afficher la page de déverrouillage.
        let after = request.get("/c/prot-dddddddd").await;
        after.assert_status_ok();
        assert!(
            after.text().contains("latch-unlock"),
            "rotation → re-déverrouillage exigé (§6)"
        );
        assert!(!after.text().contains("SECRET-ROT"));
    })
    .await;
}

// --- Nouveaux tests ---

#[tokio::test]
#[serial]
async fn serve_serves_shell_for_active_project() {
    let _dist = fake_dist();
    request::<App, _, _>(|request, ctx| async move {
        let p = make_project(&ctx.db, "libre-bbbbbbbb", false, None, None).await;
        let _storage = deploy_active(&ctx.db, &p, "<h1>PROTO-SHELL</h1>").await;

        // /c/<slug> → shell page, body contient latch-shell, no-store.
        let res = request.get("/c/libre-bbbbbbbb").await;
        res.assert_status_ok();
        assert!(
            res.text().contains("latch-shell"),
            "le shell doit être servi sur /c/<slug>"
        );
        assert_eq!(
            res.headers().get("cache-control").unwrap(),
            "no-store",
            "shell doit être no-store (§6)"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn notes_returns_release_notes_for_active_version() {
    let _dist = fake_dist();
    request::<App, _, _>(|request, ctx| async move {
        let p = make_project(&ctx.db, "libre-cccccccc", false, None, None).await;
        let _storage = deploy_active_with_notes(&ctx.db, &p, "<h1>PROTO</h1>", "# Hello").await;

        let res = request.get("/c/libre-cccccccc/notes").await;
        res.assert_status_ok();
        let body = res.text();
        // JSON contient n=1 et les notes
        assert!(
            body.contains("\"n\":1") || body.contains("\"n\": 1"),
            "n=1 attendu"
        );
        assert!(body.contains("Hello"), "notes_md attendu");
        assert_eq!(
            res.headers().get("cache-control").unwrap(),
            "no-store",
            "notes doit être no-store"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn notes_returns_204_when_no_notes() {
    let _dist = fake_dist();
    request::<App, _, _>(|request, ctx| async move {
        let p = make_project(&ctx.db, "libre-dddddddd", false, None, None).await;
        let _storage = deploy_active(&ctx.db, &p, "<h1>PROTO</h1>").await;

        let res = request.get("/c/libre-dddddddd/notes").await;
        assert_eq!(res.status_code(), 204, "sans notes → 204 No Content");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn notes_forbidden_when_locked() {
    let _dist = fake_dist();
    request::<App, _, _>(|request, ctx| async move {
        let p = make_project(&ctx.db, "prot-ffffffff", true, Some("123456"), None).await;
        let _storage = deploy_active_with_notes(&ctx.db, &p, "<h1>SECRET</h1>", "# Private").await;

        // Sans cookie → /notes → 403
        let res = request.get("/c/prot-ffffffff/notes").await;
        assert_eq!(res.status_code(), 403, "notes doit être 403 sans cookie");

        // Sans cookie → /raw → 403 (défense en profondeur)
        let res_raw = request.get("/c/prot-ffffffff/raw").await;
        assert_eq!(res_raw.status_code(), 403, "/raw doit être 403 sans cookie");
    })
    .await;
}
