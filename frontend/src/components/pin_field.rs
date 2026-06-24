//! Affiche un PIN masqué avec révélation à la demande + bouton copier.

use shadcn_rs::{Button, Size, Variant};
use yew::prelude::*;

use crate::components::copy_button::CopyButton;

// consumed in T13
#[allow(dead_code)]
#[derive(Properties, PartialEq)]
pub struct PinFieldProps {
    pub pin: String,
}

#[function_component(PinField)]
pub fn pin_field(props: &PinFieldProps) -> Html {
    let revealed = use_state(|| false);
    let toggle = {
        let revealed = revealed.clone();
        Callback::from(move |_| revealed.set(!*revealed))
    };

    html! {
        <span class="pin-field">
            <code>{ if *revealed { props.pin.clone() } else { "••••••".to_string() } }</code>
            <Button variant={Variant::Ghost} size={Size::Sm} onclick={toggle}
                    aria_label={ if *revealed { "Masquer le PIN" } else { "Révéler le PIN" } }>
                { if *revealed { "🙈" } else { "👁" } }
            </Button>
            <CopyButton value={props.pin.clone()} aria_label={AttrValue::from("Copier le PIN")} />
        </span>
    }
}
