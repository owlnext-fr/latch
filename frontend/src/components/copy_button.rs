//! Bouton-icône « copier » avec confirmation éphémère (pas de toast global :
//! shadcn Toast/Sonner n'auto-dismiss pas — D : feedback inline + gloo-timers).

use gloo_timers::callback::Timeout;
use shadcn_rs::{Button, Size, Variant};
use yew::prelude::*;

use crate::util::clipboard;

#[derive(Properties, PartialEq)]
pub struct CopyButtonProps {
    pub value: String,
    #[prop_or_default]
    pub aria_label: Option<AttrValue>,
}

#[function_component(CopyButton)]
pub fn copy_button(props: &CopyButtonProps) -> Html {
    let copied = use_state(|| false);

    let onclick = {
        let (value, copied) = (props.value.clone(), copied.clone());
        Callback::from(move |_| {
            clipboard::copy(value.clone());
            copied.set(true);
            let copied = copied.clone();
            // reset après 2 s ; Timeout::forget garde le timer vivant.
            Timeout::new(2000, move || copied.set(false)).forget();
        })
    };

    html! {
        <Button variant={Variant::Ghost} size={Size::Sm} onclick={onclick}
                aria_label={props.aria_label.clone()}>
            { if *copied { "Copié !" } else { "⧉" } }
        </Button>
    }
}
