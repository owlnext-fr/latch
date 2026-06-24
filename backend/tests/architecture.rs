//! Garde d'architecture (contrat §1) : le cœur `src/services/` est agnostique
//! HTTP. Aucune dépendance à axum ou loco ne doit y apparaître.

use std::fs;
use std::path::Path;

#[test]
fn services_do_not_depend_on_axum_or_loco() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/services");
    let mut offenders = Vec::new();

    for entry in fs::read_dir(&dir).expect("read src/services") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let src = fs::read_to_string(&path).unwrap();
        for (i, line) in src.lines().enumerate() {
            let t = line.trim_start();
            if t.starts_with("use axum") || t.starts_with("use loco_rs") {
                offenders.push(format!("{}:{}", path.display(), i + 1));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "le cœur ne doit pas dépendre d'axum/loco (contrat §1) : {offenders:?}"
    );
}
