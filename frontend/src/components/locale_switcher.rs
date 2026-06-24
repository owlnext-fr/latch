//! Sélecteur de langue FR/EN (deux boutons maison). Pilote la locale via le contexte.

use yew::prelude::*;

use crate::i18n::{use_locale, Locale};

#[function_component(LocaleSwitcher)]
pub fn locale_switcher() -> Html {
    let loc = use_locale();

    let mk = |target: Locale, label: &'static str| {
        let set = loc.set_locale.clone();
        let active = loc.locale == target;
        let onclick = Callback::from(move |_: MouseEvent| set.emit(target));
        let class = if active {
            "locale-btn locale-btn--active"
        } else {
            "locale-btn"
        };
        html! { <button type="button" class={class} {onclick} aria-pressed={active.to_string()}>{ label }</button> }
    };

    html! {
        <span class="locale-switcher" aria-label="Language">
            { mk(Locale::En, "EN") }
            { mk(Locale::Fr, "FR") }
        </span>
    }
}
