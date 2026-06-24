//! Couche i18n : enum `Locale`, détection au boot, `LocaleProvider`, `use_locale`.
//! (Le provider est complété en Task 2 ; ici on pose l'enum + un test sur `t!`.)

/// Locales supportées par l'admin.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Locale {
    En,
    Fr,
}

impl Locale {
    pub fn as_str(self) -> &'static str {
        match self {
            Locale::En => "en",
            Locale::Fr => "fr",
        }
    }

    /// Dérive la locale d'un code (`navigator.language` ou valeur stockée).
    pub fn from_code(code: &str) -> Locale {
        if code.to_ascii_lowercase().starts_with("fr") {
            Locale::Fr
        } else {
            Locale::En
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;
    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn from_code_maps_fr_and_defaults_en() {
        assert_eq!(Locale::from_code("fr-FR"), Locale::Fr);
        assert_eq!(Locale::from_code("FR"), Locale::Fr);
        assert_eq!(Locale::from_code("en-US"), Locale::En);
        assert_eq!(Locale::from_code("de"), Locale::En);
    }

    #[wasm_bindgen_test]
    fn t_macro_resolves_per_locale() {
        rust_i18n::set_locale("en");
        assert_eq!(&*t!("login.submit"), "Sign in");
        rust_i18n::set_locale("fr");
        assert_eq!(&*t!("login.submit"), "Se connecter");
        rust_i18n::set_locale("en"); // restaure pour les autres tests
    }
}
