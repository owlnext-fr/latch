//! Side-panel Déployer une version : lit un fichier HTML (gloo-file) et POST /deploy.

use shadcn_rs::{
    Button, Label, Position, SheetContent, SheetFooter, SheetHeader, SheetTitle, Switch, Variant,
};
use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::api;
use latch_dto::DeployReq;

#[derive(Properties, PartialEq)]
pub struct DeployPanelProps {
    pub open: bool,
    pub project_id: i32,
    pub on_close: Callback<()>,
    pub on_deployed: Callback<()>,
}

#[function_component(DeployPanel)]
pub fn deploy_panel(props: &DeployPanelProps) -> Html {
    let html_content = use_state(|| Option::<String>::None);
    let filename = use_state(|| Option::<String>::None);
    let activate = use_state(|| true);
    let error = use_state(|| Option::<String>::None);
    let busy = use_state(|| false);

    {
        let (html_content, filename, error, activate) = (
            html_content.clone(),
            filename.clone(),
            error.clone(),
            activate.clone(),
        );
        use_effect_with(props.open, move |_| {
            html_content.set(None);
            filename.set(None);
            error.set(None);
            activate.set(true);
            || ()
        });
    }

    let on_file = {
        let (html_content, filename, error) =
            (html_content.clone(), filename.clone(), error.clone());
        Callback::from(move |e: Event| {
            let input = e.target_unchecked_into::<HtmlInputElement>();
            let Some(files) = input.files() else { return };
            let Some(file) = files.get(0) else { return };
            let name = file.name();
            let gfile = gloo_file::File::from(file);
            let (html_content, filename, error) =
                (html_content.clone(), filename.clone(), error.clone());
            filename.set(Some(name));
            wasm_bindgen_futures::spawn_local(async move {
                match gloo_file::futures::read_as_text(&gfile).await {
                    Ok(text) => html_content.set(Some(text)),
                    Err(_) => error.set(Some("Lecture du fichier impossible.".into())),
                }
            });
        })
    };

    let on_toggle = {
        let activate = activate.clone();
        Callback::from(move |_: Event| activate.set(!*activate))
    };

    let on_deploy = {
        let (html_content, activate, error, busy) = (
            html_content.clone(),
            activate.clone(),
            error.clone(),
            busy.clone(),
        );
        let (on_close, on_deployed, id) = (
            props.on_close.clone(),
            props.on_deployed.clone(),
            props.project_id,
        );
        Callback::from(move |_: MouseEvent| {
            let Some(html) = (*html_content).clone() else {
                error.set(Some("Choisis un fichier HTML.".into()));
                return;
            };
            let req = DeployReq {
                html,
                activate: *activate,
            };
            let (on_close, on_deployed, error, busy) = (
                on_close.clone(),
                on_deployed.clone(),
                error.clone(),
                busy.clone(),
            );
            busy.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match api::client::deploy(id, &req).await {
                    Ok(_) => {
                        on_deployed.emit(());
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
        Callback::from(move |_: MouseEvent| on_close.emit(()))
    };

    html! {
        <SheetContent open={props.open} on_close={props.on_close.clone()} side={Position::Right}>
            <SheetHeader><SheetTitle>{ "Déployer une version" }</SheetTitle></SheetHeader>

            <Label html_for="dp-file">{ "Fichier HTML" }</Label>
            <input id="dp-file" type="file" accept="text/html,.html" onchange={on_file} />
            if let Some(n) = (*filename).clone() { <p class="hint">{ n }</p> }

            <div class="toggle-row">
                <Switch id="dp-activate" checked={*activate} onchange={on_toggle} />
                <span class="hint">{ "Activer immédiatement : la nouvelle version devient l'active servie sur l'URL publique." }</span>
            </div>

            if let Some(msg) = (*error).clone() { <p class="error">{ msg }</p> }

            <SheetFooter>
                <Button variant={Variant::Ghost} onclick={close}>{ "Annuler" }</Button>
                <Button variant={Variant::Primary} disabled={*busy} onclick={on_deploy}>
                    { if *busy { "Déploiement…" } else { "Déployer" } }
                </Button>
            </SheetFooter>
        </SheetContent>
    }
}
