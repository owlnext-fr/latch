//! Garde d'architecture (contrat §1) : le cœur `src/services/` est agnostique
//! HTTP. Aucune dépendance à axum ou loco ne doit y apparaître — y compris dans
//! d'éventuels sous-modules (scan récursif) et via des ré-exports (`pub use`).

use std::fs;
use std::path::{Path, PathBuf};

/// Collecte récursivement tous les fichiers `.rs` sous `dir`.
fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[test]
fn services_do_not_depend_on_axum_or_loco() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/services");

    let mut files = Vec::new();
    collect_rs_files(&dir, &mut files);
    assert!(
        !files.is_empty(),
        "aucun fichier service trouvé sous {} — chemin de scan invalide ?",
        dir.display()
    );

    let mut offenders = Vec::new();
    for path in &files {
        let src = fs::read_to_string(path).unwrap();
        for (i, line) in src.lines().enumerate() {
            // normalise un éventuel préfixe `pub ` (ré-export) avant le test
            let t = line.trim_start().trim_start_matches("pub ").trim_start();
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
