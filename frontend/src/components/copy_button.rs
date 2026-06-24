//! Bouton-icône « copier » : confirmation éphémère inline + toast global.

use gloo_timers::callback::Timeout;
use shadcn_rs::{Button, Size, Variant};
use yew::prelude::*;

use crate::toast::use_toast;
use crate::util::clipboard;

#[derive(Properties, PartialEq)]
pub struct CopyButtonProps {
    pub value: String,
    #[prop_or_default]
    pub aria_label: Option<AttrValue>,
}

#[function_component(CopyButton)]
pub fn copy_button(props: &CopyButtonProps) -> Html {
    let _loc = crate::i18n::use_locale(); // abonnement i18n (re-render au switch de langue)
    let toast = use_toast();
    let copied = use_state(|| false);

    let onclick = {
        let (value, copied, toast) = (props.value.clone(), copied.clone(), toast.clone());
        Callback::from(move |_| {
            clipboard::copy(value.clone());
            copied.set(true);
            toast.push_success.emit(t!("toast.copied").to_string());
            let copied = copied.clone();
            Timeout::new(2000, move || copied.set(false)).forget();
        })
    };

    html! {
        <Button variant={Variant::Ghost} size={Size::Sm} onclick={onclick}
                aria_label={props.aria_label.clone()}>
            { if *copied { t!("common.copied") } else { std::borrow::Cow::Borrowed("⧉") } }
        </Button>
    }
}
