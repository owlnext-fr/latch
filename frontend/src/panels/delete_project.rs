//! Side-panel danger : supprimer un projet (confirmation in-panel).

use shadcn_rs::{Button, Position, SheetContent, SheetFooter, SheetHeader, SheetTitle, Variant};
use yew::prelude::*;

use crate::api;
use latch_dto::ProjectDetail;

#[derive(Properties, PartialEq)]
pub struct DeleteProjectPanelProps {
    pub open: bool,
    pub project: ProjectDetail,
    pub on_close: Callback<()>,
    pub on_deleted: Callback<()>,
}

#[function_component(DeleteProjectPanel)]
pub fn delete_project_panel(props: &DeleteProjectPanelProps) -> Html {
    let error = use_state(|| Option::<String>::None);
    let busy = use_state(|| false);
    let n_versions = props.project.versions.len();

    // Reset error+busy whenever the panel opens/closes.
    {
        let (error, busy) = (error.clone(), busy.clone());
        use_effect_with(props.open, move |_| {
            error.set(None);
            busy.set(false);
            || ()
        });
    }

    let on_confirm = {
        let (on_close, on_deleted, error, busy, id) = (
            props.on_close.clone(),
            props.on_deleted.clone(),
            error.clone(),
            busy.clone(),
            props.project.id,
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
                match api::client::delete_project(id).await {
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
            <SheetHeader><SheetTitle>{ format!("Supprimer « {} »", props.project.name) }</SheetTitle></SheetHeader>
            <p>{ "Cette action est irréversible. Seront supprimés définitivement :" }</p>
            <ul>
                <li>{ "le projet et sa configuration ;" }</li>
                <li>{ format!("ses {n_versions} version(s) et leurs fichiers HTML ;") }</li>
                <li>{ "l'URL publique (404 ensuite)." }</li>
            </ul>
            if let Some(msg) = (*error).clone() { <p class="error">{ msg }</p> }
            <SheetFooter>
                <Button variant={Variant::Ghost} onclick={close}>{ "Annuler" }</Button>
                <Button variant={Variant::Destructive} disabled={*busy} onclick={on_confirm}>
                    { "Oui, supprimer définitivement" }
                </Button>
            </SheetFooter>
        </SheetContent>
    }
}
