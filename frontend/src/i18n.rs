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

use yew::prelude::*;

const LS_KEY: &str = "latch.locale";

fn read_stored() -> Option<String> {
    let win = web_sys::window()?;
    let store = win.local_storage().ok()??;
    store.get_item(LS_KEY).ok()?
}

fn write_stored(code: &str) {
    if let Some(win) = web_sys::window() {
        if let Ok(Some(store)) = win.local_storage() {
            let _ = store.set_item(LS_KEY, code);
        }
    }
}

fn browser_lang() -> Option<String> {
    web_sys::window()?.navigator().language()
}

/// Locale au démarrage : localStorage → navigator.language → EN.
fn detect_initial() -> Locale {
    if let Some(stored) = read_stored() {
        return Locale::from_code(&stored);
    }
    if let Some(lang) = browser_lang() {
        return Locale::from_code(&lang);
    }
    Locale::En
}

#[derive(Clone, PartialEq)]
pub struct LocaleContext {
    pub locale: Locale,
    pub set_locale: Callback<Locale>,
}

#[hook]
pub fn use_locale() -> LocaleContext {
    use_context::<LocaleContext>().expect("LocaleProvider manquant au-dessus de l'arbre")
}

#[derive(Properties, PartialEq)]
pub struct LocaleProviderProps {
    pub children: Html,
}

#[function_component(LocaleProvider)]
pub fn locale_provider(props: &LocaleProviderProps) -> Html {
    // Initialise une seule fois : détecte la locale et applique la locale globale
    // rust-i18n de façon synchrone (avant le premier rendu des enfants → pas de flash).
    let locale = use_state(|| {
        let l = detect_initial();
        rust_i18n::set_locale(l.as_str());
        l
    });

    let set_locale = {
        let locale = locale.clone();
        Callback::from(move |l: Locale| {
            rust_i18n::set_locale(l.as_str());
            write_stored(l.as_str());
            locale.set(l);
        })
    };

    let ctx = LocaleContext {
        locale: *locale,
        set_locale,
    };

    html! {
        <ContextProvider<LocaleContext> context={ctx}>
            { props.children.clone() }
        </ContextProvider<LocaleContext>>
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
