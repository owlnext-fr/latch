//! Side-panel danger : supprimer une version (inactive).

use shadcn_rs::{Button, Position, SheetContent, SheetFooter, SheetHeader, SheetTitle, Variant};
use yew::prelude::*;

use crate::api;
use crate::i18n::use_locale;
use crate::toast::use_toast;

#[derive(Properties, PartialEq)]
pub struct DeleteVersionPanelProps {
    pub open: bool,
    pub project_id: i32,
    pub n: i32,
    pub on_close: Callback<()>,
    pub on_deleted: Callback<()>,
}

#[function_component(DeleteVersionPanel)]
pub fn delete_version_panel(props: &DeleteVersionPanelProps) -> Html {
    let _loc = use_locale();
    let toast = use_toast();
    let error = use_state(|| Option::<String>::None);
    let busy = use_state(|| false);
    let on_confirm = {
        let (on_close, on_deleted, error, busy, id, n, toast) = (
            props.on_close.clone(),
            props.on_deleted.clone(),
            error.clone(),
            busy.clone(),
            props.project_id,
            props.n,
            toast.clone(),
        );
        Callback::from(move |_| {
            let (on_close, on_deleted, error, busy, toast) = (
                on_close.clone(),
                on_deleted.clone(),
                error.clone(),
                busy.clone(),
                toast.clone(),
            );
            busy.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match api::client::delete_version(id, n).await {
                    Ok(()) => {
                        toast
                            .push_success
                            .emit(t!("toast.version_deleted").to_string());
                        on_deleted.emit(());
                        on_close.emit(());
                    }
                    Err(e) => error.set(Some(e.user_message())),
                }
                busy.set(false);
            });
        })
    };
    let close = {
        let on_close = props.on_close.clone();
        Callback::from(move |_| on_close.emit(()))
    };
    html! {
        <SheetContent open={props.open} on_close={props.on_close.clone()} side={Position::Right}
                      class={classes!("sheet-danger")}>
            <SheetHeader><SheetTitle>{ t!("danger.del_version_title", n = props.n) }</SheetTitle></SheetHeader>
            <p>{ t!("danger.del_version_intro") }</p>
            if let Some(msg) = (*error).clone() { <p class="error">{ msg }</p> }
            <SheetFooter>
                <Button variant={Variant::Ghost} onclick={close}>{ t!("common.cancel") }</Button>
                <Button variant={Variant::Destructive} disabled={*busy} onclick={on_confirm}>
                    { if *busy { t!("danger.deleting") } else { t!("danger.del_version_confirm") } }
                </Button>
            </SheetFooter>
        </SheetContent>
    }
}
