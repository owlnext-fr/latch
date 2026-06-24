//! Toggle — `Switch` shadcn-rs 0.1 vendorisé et patché.
//! Bug d'origine (switch.rs) : `is_checked = if checked { checked } else { *internal }`
//! → quand le parent passe `checked=false`, le composant retombe sur son état interne
//! (déjà basculé) et ne revient jamais visuellement à off (cf. QUIRKS). Ici : état
//! 100% contrôlé (`is_checked = checked`), zéro état interne. Réutilise les classes
//! CSS `.switch` / `.size-md` / `.switch-thumb` / `.switch-checked` / `.switch-disabled`
//! déjà vendorisées (components.css).

use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct ToggleProps {
    #[prop_or(false)]
    pub checked: bool,
    #[prop_or(false)]
    pub disabled: bool,
    #[prop_or_default]
    pub id: Option<AttrValue>,
    #[prop_or_default]
    pub onchange: Option<Callback<Event>>,
    #[prop_or_default]
    pub aria_label: Option<AttrValue>,
}

#[function_component(Toggle)]
pub fn toggle(props: &ToggleProps) -> Html {
    let ToggleProps {
        checked,
        disabled,
        id,
        onchange,
        aria_label,
    } = props.clone();

    let onclick = {
        let onchange = onchange.clone();
        Callback::from(move |e: MouseEvent| {
            if !disabled {
                if let Some(cb) = onchange.as_ref() {
                    cb.emit(e.into());
                }
            }
        })
    };
    let onkeydown = {
        let onchange = onchange.clone();
        Callback::from(move |e: KeyboardEvent| {
            if !disabled && (e.key() == " " || e.key() == "Enter") {
                e.prevent_default();
                if let Some(cb) = onchange.as_ref() {
                    cb.emit(e.into());
                }
            }
        })
    };

    // size-md est LOAD-BEARING : `.switch` seul n'a ni hauteur ni largeur (cf. components.css).
    let classes = classes!(
        "switch",
        "size-md",
        checked.then_some("switch-checked"),
        disabled.then_some("switch-disabled"),
    );

    html! {
        <button
            type="button"
            role="switch"
            class={classes}
            aria-checked={checked.to_string()}
            aria-label={aria_label}
            disabled={disabled}
            onclick={onclick}
            onkeydown={onkeydown}
            id={id}
        >
            <span class="switch-thumb" aria-hidden="true"></span>
        </button>
    }
}
