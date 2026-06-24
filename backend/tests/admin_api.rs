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
