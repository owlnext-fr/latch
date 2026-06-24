//! Affiche un PIN masqué avec révélation à la demande + bouton copier.

use shadcn_rs::{Button, Size, Variant};
use yew::prelude::*;

use crate::components::copy_button::CopyButton;
use crate::i18n::use_locale;

#[derive(Properties, PartialEq)]
pub struct PinFieldProps {
    pub pin: String,
}

#[function_component(PinField)]
pub fn pin_field(props: &PinFieldProps) -> Html {
    let _loc = use_locale();
    let revealed = use_state(|| false);
    let toggle = {
        let revealed = revealed.clone();
        Callback::from(move |_| revealed.set(!*revealed))
    };

    html! {
        <span class="pin-field">
            <code>{ if *revealed { props.pin.clone() } else { "••••••".to_string() } }</code>
            <Button variant={Variant::Ghost} size={Size::Sm} onclick={toggle}
                    aria_label={ AttrValue::from(if *revealed { t!("detail.hide_pin").to_string() } else { t!("detail.reveal_pin").to_string() }) }>
                { if *revealed { "🙈" } else { "👁" } }
            </Button>
            <CopyButton value={props.pin.clone()} aria_label={AttrValue::from(t!("detail.copy_pin_aria").to_string())} />
        </span>
    }
}
