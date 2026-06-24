//! Copie best-effort dans le presse-papier (Clipboard API). Échec silencieux si
//! l'API n'est pas dispo (le composant appelant affiche quand même « Copié ! »).

use wasm_bindgen_futures::JsFuture;

pub fn copy(text: String) {
    wasm_bindgen_futures::spawn_local(async move {
        if let Some(win) = web_sys::window() {
            let clipboard = win.navigator().clipboard();
            let _ = JsFuture::from(clipboard.write_text(&text)).await;
        }
    });
}
