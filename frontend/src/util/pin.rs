//! Génération/validation du PIN côté SPA (affichage live dans le panel). Le cœur
//! backend garde sa propre génération pour le chemin MCP (contrat §3/§7, D10).

/// Vrai si `s` fait exactement 6 caractères, tous des chiffres ASCII.
pub fn is_valid_pin(s: &str) -> bool {
    s.len() == 6 && s.bytes().all(|b| b.is_ascii_digit())
}

/// Génère un PIN de 6 chiffres (entropie via crypto.getRandomValues du navigateur).
pub fn generate_pin() -> String {
    let mut buf = [0u8; 6];
    // web-sys Crypto : getRandomValues remplit le buffer.
    if let Some(win) = web_sys::window() {
        if let Ok(crypto) = win.crypto() {
            let _ = crypto.get_random_values_with_u8_array(&mut buf);
        }
    }
    buf.iter().map(|b| char::from(b'0' + (b % 10))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;
    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn generated_pin_is_six_digits() {
        let p = generate_pin();
        assert_eq!(p.len(), 6);
        assert!(is_valid_pin(&p));
    }

    #[wasm_bindgen_test]
    fn rejects_bad_pins() {
        assert!(!is_valid_pin("12345"));
        assert!(!is_valid_pin("1234567"));
        assert!(!is_valid_pin("12a456"));
        assert!(is_valid_pin("000000"));
    }
}
