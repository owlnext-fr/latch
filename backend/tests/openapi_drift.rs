//! Verrou anti-drift : `openapi.json` (racine) doit toujours refléter `ApiDoc`.
//! Régénérer après tout changement de DTO/route : `UPDATE_OPENAPI=1 cargo test --test openapi_drift`.

use std::path::PathBuf;

use latch::openapi::ApiDoc;
use utoipa::OpenApi;

fn openapi_json_path() -> PathBuf {
    // CARGO_MANIFEST_DIR = .../backend ; le schéma vit à la racine du repo.
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../openapi.json")
}

#[test]
fn openapi_json_is_in_sync() {
    let generated = ApiDoc::openapi()
        .to_pretty_json()
        .expect("sérialisation OpenAPI");
    let path = openapi_json_path();

    if std::env::var("UPDATE_OPENAPI").is_ok() {
        std::fs::write(&path, format!("{generated}\n")).expect("écriture openapi.json");
        return;
    }

    let on_disk = std::fs::read_to_string(&path).expect(
        "openapi.json manquant — générer avec: UPDATE_OPENAPI=1 cargo test --test openapi_drift",
    );
    assert_eq!(
        on_disk.trim_end(),
        generated.trim_end(),
        "openapi.json périmé — régénérer: UPDATE_OPENAPI=1 cargo test --test openapi_drift"
    );
}
