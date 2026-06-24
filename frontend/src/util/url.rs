//! Construit l'URL publique absolue d'un prototype. Admin et serving partagent
//! l'origin (D9) → pas de config nécessaire.

/// `https://latch.owlnext.fr/c/<slug>` (dérivé de l'origin courant).
pub fn public_url(slug: &str) -> String {
    let origin = web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_default();
    format!("{origin}/c/{slug}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;
    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn public_url_ends_with_slug_path() {
        let u = public_url("mon-projet-k7Qp2maZ");
        assert!(u.ends_with("/c/mon-projet-k7Qp2maZ"), "got {u}");
        assert!(u.contains("://"), "doit être absolu : {u}");
    }
}
