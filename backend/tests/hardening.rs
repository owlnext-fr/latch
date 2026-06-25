#![allow(clippy::unwrap_used, clippy::expect_used)]

use latch::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn robots_txt_is_served() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/robots.txt").await;
        res.assert_status_ok();
        let ct = res
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(ct.starts_with("text/plain"), "content-type = {ct}");
        assert!(
            res.text().contains("Disallow: /"),
            "robots.txt doit interdire tout crawl"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn x_robots_tag_on_admin() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/admin").await;
        let tag = res
            .headers()
            .get("x-robots-tag")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(tag, "noindex, nofollow", "X-Robots-Tag manquant sur /admin");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn x_robots_tag_on_api_even_401() {
    request::<App, _, _>(|request, _ctx| async move {
        // /api/projects sans session → 401, mais l'en-tête doit quand même être posé.
        let res = request.get("/api/projects").await;
        let tag = res
            .headers()
            .get("x-robots-tag")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(
            tag, "noindex, nofollow",
            "X-Robots-Tag manquant sur /api (401)"
        );
    })
    .await;
}
