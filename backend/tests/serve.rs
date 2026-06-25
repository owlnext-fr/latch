use latch::app::App;
use latch::models::_entities::projects;
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
