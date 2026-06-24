//! Side-panel Déployer une version : dropzone HTML (drag-and-drop + clic) → POST /deploy.

use shadcn_rs::{
    Button, Label, Position, SheetContent, SheetFooter, SheetHeader, SheetTitle, Variant,
};
use web_sys::{HtmlElement, HtmlInputElement};
use yew::prelude::*;

use crate::api;
use crate::components::toggle::Toggle;
use crate::i18n::use_locale;
use crate::toast::use_toast;
use latch_dto::DeployReq;

#[derive(Properties, PartialEq)]
pub struct DeployPanelProps {
    pub open: bool,
    pub project_id: i32,
    pub on_close: Callback<()>,
    pub on_deployed: Callback<()>,
}

/// Formate une taille d'octets en texte court (« 12.3 KB »).
fn human_size(bytes: f64) -> String {
    if bytes < 1024.0 {
        format!("{bytes:.0} B")
    } else if bytes < 1024.0 * 1024.0 {
        format!("{:.1} KB", bytes / 1024.0)
    } else {
        format!("{:.1} MB", bytes / (1024.0 * 1024.0))
    }
}

#[function_component(DeployPanel)]
pub fn deploy_panel(props: &DeployPanelProps) -> Html {
    let _loc = use_locale();
    let toast = use_toast();
    let html_content = use_state(|| Option::<String>::None);
    let file_label = use_state(|| Option::<String>::None);
    let activate = use_state(|| true);
    let error = use_state(|| Option::<String>::None);
    let busy = use_state(|| false);
    let over = use_state(|| false);
    let input_ref = use_node_ref();

    {
        let (html_content, file_label, error, activate, over) = (
            html_content.clone(),
            file_label.clone(),
            error.clone(),
            activate.clone(),
            over.clone(),
        );
        use_effect_with(props.open, move |_| {
            html_content.set(None);
            file_label.set(None);
            error.set(None);
            activate.set(true);
            over.set(false);
            || ()
        });
    }

    // Charge un gloo_file::File : lit le texte + pose le label.
    let load_file = {
        let (html_content, file_label, error) =
            (html_content.clone(), file_label.clone(), error.clone());
        move |file: web_sys::File| {
            let label = format!("{} ({})", file.name(), human_size(file.size()));
            file_label.set(Some(label));
            let gfile = gloo_file::File::from(file);
            let (html_content, error) = (html_content.clone(), error.clone());
            wasm_bindgen_futures::spawn_local(async move {
                match gloo_file::futures::read_as_text(&gfile).await {
                    Ok(text) => html_content.set(Some(text)),
                    Err(_) => error.set(Some(t!("deploy.err_read").to_string())),
                }
            });
        }
    };

    let on_input_change = {
        let load_file = load_file.clone();
        Callback::from(move |e: Event| {
            let input = e.target_unchecked_into::<HtmlInputElement>();
            if let Some(files) = input.files() {
                if let Some(file) = files.get(0) {
                    load_file(file);
                }
            }
        })
    };

    let on_zone_click = {
        let input_ref = input_ref.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(input) = input_ref.cast::<HtmlElement>() {
                input.click();
            }
        })
    };

    let on_dragover = {
        let over = over.clone();
        Callback::from(move |e: DragEvent| {
            e.prevent_default();
            over.set(true);
        })
    };
    let on_dragleave = {
        let over = over.clone();
        Callback::from(move |_: DragEvent| over.set(false))
    };
    let on_drop = {
        let (over, load_file) = (over.clone(), load_file.clone());
        Callback::from(move |e: DragEvent| {
            e.prevent_default();
            over.set(false);
            if let Some(dt) = e.data_transfer() {
                if let Some(files) = dt.files() {
                    if let Some(file) = files.get(0) {
                        load_file(file);
                    }
                }
            }
        })
    };

    let on_toggle = {
        let activate = activate.clone();
        Callback::from(move |_: Event| activate.set(!*activate))
    };

    let on_deploy = {
        let (html_content, activate, error, busy, toast) = (
            html_content.clone(),
            activate.clone(),
            error.clone(),
            busy.clone(),
            toast.clone(),
        );
        let (on_close, on_deployed, id) = (
            props.on_close.clone(),
            props.on_deployed.clone(),
            props.project_id,
        );
        Callback::from(move |_: MouseEvent| {
            let Some(html) = (*html_content).clone() else {
                error.set(Some(t!("deploy.err_no_file").to_string()));
                return;
            };
            let req = DeployReq {
                html,
                activate: *activate,
            };
            let (on_close, on_deployed, error, busy, toast) = (
                on_close.clone(),
                on_deployed.clone(),
                error.clone(),
                busy.clone(),
                toast.clone(),
            );
            busy.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match api::client::deploy(id, &req).await {
                    Ok(_) => {
                        toast
                            .push_success
                            .emit(t!("toast.version_deployed").to_string());
                        on_deployed.emit(());
                        on_close.emit(());
                    }
                    Err(e) => {
                        let m = e.user_message();
                        error.set(Some(m.clone()));
                        toast.push_error.emit(m);
                    }
                }
                busy.set(false);
            });
        })
    };

    let close = {
        let on_close = props.on_close.clone();
        Callback::from(move |_: MouseEvent| on_close.emit(()))
    };

    let zone_class = if *over {
        "dropzone dropzone--over"
    } else {
        "dropzone"
    };

    html! {
        <SheetContent open={props.open} on_close={props.on_close.clone()} side={Position::Right}>
            <SheetHeader><SheetTitle>{ t!("deploy.title") }</SheetTitle></SheetHeader>

            <Label html_for="dp-file">{ t!("deploy.file") }</Label>
            <div class={zone_class} onclick={on_zone_click}
                 ondragover={on_dragover} ondragleave={on_dragleave} ondrop={on_drop}>
                if let Some(label) = (*file_label).clone() {
                    <span class="dropzone__file">{ label }</span>
                } else if *over {
                    <span>{ t!("deploy.dropzone_hover") }</span>
                } else {
                    <span>{ t!("deploy.dropzone_idle") }</span>
                }
            </div>
            <input ref={input_ref} id="dp-file" type="file" accept="text/html,.html"
                   style="display:none" onchange={on_input_change} />

            <div class="toggle-row">
                <Toggle id={AttrValue::from("dp-activate")} checked={*activate} onchange={on_toggle} />
                <span class="hint">{ t!("deploy.activate_help") }</span>
            </div>

            if let Some(msg) = (*error).clone() { <p class="error">{ msg }</p> }

            <SheetFooter>
                <Button variant={Variant::Ghost} onclick={close}>{ t!("common.cancel") }</Button>
                <Button variant={Variant::Primary} disabled={*busy} onclick={on_deploy}>
                    { if *busy { t!("deploy.deploying") } else { t!("deploy.btn") } }
                </Button>
            </SheetFooter>
        </SheetContent>
    }
}
