use latch::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;

/// Prépare un faux dist/ avec un index.html reconnaissable + un asset, pointé par
/// LATCH_SPA_DIST, et garde le tempdir vivant pour toute la durée du test.
fn fake_dist() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("index.html"),
        "<!doctype html><title>latch-spa</title>",
    )
    .expect("write index");
    std::fs::write(dir.path().join("app.js"), "// spa asset").expect("write asset");
    std::env::set_var("LATCH_SPA_DIST", dir.path());
    dir
}

#[tokio::test]
#[serial]
async fn admin_root_serves_spa_index() {
    let _dist = fake_dist();
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/admin").await;
        res.assert_status_ok();
        assert!(
            res.text().contains("latch-spa"),
            "GET /admin rend index.html"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn admin_deep_link_falls_back_to_index() {
    let _dist = fake_dist();
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/admin/projects/5").await;
        res.assert_status_ok();
        assert!(res.text().contains("latch-spa"), "deep-link → index.html");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn api_is_not_shadowed_by_spa() {
    let _dist = fake_dist();
    request::<App, _, _>(|request, _ctx| async move {
        // /api/projects sans session → 401 (pas l'index SPA en 200).
        let res = request.get("/api/projects").await;
        assert_eq!(
            res.status_code(),
            401,
            "GET /api/projects sans session doit être 401, pas l'index SPA"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn admin_serves_existing_asset_directly() {
    let _dist = fake_dist();
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/admin/app.js").await;
        res.assert_status_ok();
        assert!(
            res.text().contains("spa asset"),
            "GET /admin/app.js doit servir le fichier réel (pas le fallback index.html)"
        );
    })
    .await;
}
