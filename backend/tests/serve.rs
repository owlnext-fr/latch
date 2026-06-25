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

/// Prépare un faux `dist/` avec un unlock.html reconnaissable + pointe LATCH_SPA_DIST.
fn fake_dist() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("unlock.html"),
        "<!doctype html><title>latch-unlock</title>",
    )
    .expect("write unlock.html");
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

#[tokio::test]
#[serial]
async fn open_project_serves_active_html_no_store() {
    let _dist = fake_dist();
    request::<App, _, _>(|request, ctx| async move {
        let p = make_project(&ctx.db, "libre-aaaaaaaa", false, None, None).await;
        let _storage = deploy_active(&ctx.db, &p, "<h1>PROTO-LIBRE</h1>").await;
        let res = request.get("/c/libre-aaaaaaaa").await;
        res.assert_status_ok();
        assert!(res.text().contains("PROTO-LIBRE"));
        assert_eq!(
            res.headers().get("cache-control").unwrap(),
            "no-store",
            "tout /c doit être no-store (§6)"
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
async fn unknown_slug_is_404() {
    let _dist = fake_dist();
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/c/nope-xxxxxxxx").await;
        assert_eq!(res.status_code(), 404);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn project_without_active_version_is_404() {
    let _dist = fake_dist();
    request::<App, _, _>(|request, ctx| async move {
        make_project(&ctx.db, "vide-aaaaaaaa", false, None, None).await; // aucune version
        let res = request.get("/c/vide-aaaaaaaa").await;
        assert_eq!(res.status_code(), 404);
    })
    .await;
}
