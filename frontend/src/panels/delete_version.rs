//! Side-panel danger : supprimer une version (inactive).

use shadcn_rs::{Button, Position, SheetContent, SheetFooter, SheetHeader, SheetTitle, Variant};
use yew::prelude::*;

use crate::api;

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
    let error = use_state(|| Option::<String>::None);
    let busy = use_state(|| false);
    let on_confirm = {
        let (on_close, on_deleted, error, busy, id, n) = (
            props.on_close.clone(),
            props.on_deleted.clone(),
            error.clone(),
            busy.clone(),
            props.project_id,
            props.n,
        );
        Callback::from(move |_| {
            let (on_close, on_deleted, error, busy) = (
                on_close.clone(),
                on_deleted.clone(),
                error.clone(),
                busy.clone(),
            );
            busy.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match api::client::delete_version(id, n).await {
                    Ok(()) => {
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
            <SheetHeader><SheetTitle>{ format!("Supprimer la version v{}", props.n) }</SheetTitle></SheetHeader>
            <p>{ "Cette version et son fichier HTML seront supprimés. Action irréversible." }</p>
            if let Some(msg) = (*error).clone() { <p class="error">{ msg }</p> }
            <SheetFooter>
                <Button variant={Variant::Ghost} onclick={close}>{ "Annuler" }</Button>
                <Button variant={Variant::Destructive} disabled={*busy} onclick={on_confirm}>
                    { if *busy { "Suppression…" } else { "Oui, supprimer" } }
                </Button>
            </SheetFooter>
        </SheetContent>
    }
}
