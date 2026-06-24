//! Side-panel Créer/Éditer un projet (même composant, 2 modes). Pilote SheetContent
//! manuellement (open + on_close). Code = toggle + explication ; PIN généré côté SPA.

use shadcn_rs::{
    Button, Input, Label, Position, SheetContent, SheetFooter, SheetHeader, SheetTitle, Switch,
    Variant,
};
use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::api;
use crate::util::pin;
use latch_dto::{CreateProjectReq, ProjectDetail, SetCodeReq, UpdateProjectReq};

#[derive(Clone, PartialEq)]
pub enum FormMode {
    Create,
    Edit(ProjectDetail),
}

#[derive(Properties, PartialEq)]
pub struct ProjectFormProps {
    pub open: bool,
    pub mode: FormMode,
    pub on_close: Callback<()>,
    pub on_saved: Callback<()>,
}

#[function_component(ProjectForm)]
pub fn project_form(props: &ProjectFormProps) -> Html {
    let is_edit = matches!(props.mode, FormMode::Edit(_));
    let initial = match &props.mode {
        FormMode::Edit(d) => d.clone(),
        FormMode::Create => ProjectDetail {
            id: 0,
            slug: String::new(),
            name: String::new(),
            code_enabled: true,
            pin: Some(pin::generate_pin()),
            brand_name: None,
            active_version_id: None,
            versions: vec![],
        },
    };

    let name = use_state(|| initial.name.clone());
    let brand = use_state(|| initial.brand_name.clone().unwrap_or_default());
    let code_on = use_state(|| initial.code_enabled);
    let pin_val = use_state(|| initial.pin.clone().unwrap_or_else(pin::generate_pin));
    let error = use_state(|| Option::<String>::None);
    let busy = use_state(|| false);

    // Fix A: reset form fields whenever the panel (re)opens.
    {
        let (name, brand, code_on, pin_val, error) = (
            name.clone(),
            brand.clone(),
            code_on.clone(),
            pin_val.clone(),
            error.clone(),
        );
        let mode = props.mode.clone();
        use_effect_with(props.open, move |&open| {
            if open {
                let (n, b, c, p) = match &mode {
                    FormMode::Create => (String::new(), String::new(), true, pin::generate_pin()),
                    FormMode::Edit(d) => (
                        d.name.clone(),
                        d.brand_name.clone().unwrap_or_default(),
                        d.code_enabled,
                        d.pin.clone().unwrap_or_else(pin::generate_pin),
                    ),
                };
                name.set(n);
                brand.set(b);
                code_on.set(c);
                pin_val.set(p);
                error.set(None);
            }
            || ()
        });
    }

    let on_name = {
        let name = name.clone();
        Callback::from(move |e: InputEvent| {
            name.set(e.target_unchecked_into::<HtmlInputElement>().value())
        })
    };
    let on_brand = {
        let brand = brand.clone();
        Callback::from(move |e: InputEvent| {
            brand.set(e.target_unchecked_into::<HtmlInputElement>().value())
        })
    };
    let on_pin = {
        let pin_val = pin_val.clone();
        Callback::from(move |e: InputEvent| {
            pin_val.set(e.target_unchecked_into::<HtmlInputElement>().value())
        })
    };
    let on_code_toggle = {
        let code_on = code_on.clone();
        Callback::from(move |_: Event| code_on.set(!*code_on))
    };
    let on_regen = {
        let pin_val = pin_val.clone();
        Callback::from(move |_: MouseEvent| pin_val.set(pin::generate_pin()))
    };

    let on_save = {
        let (name, brand, code_on, pin_val, error) = (
            name.clone(),
            brand.clone(),
            code_on.clone(),
            pin_val.clone(),
            error.clone(),
        );
        let busy = busy.clone();
        let (on_saved, on_close, mode) = (
            props.on_saved.clone(),
            props.on_close.clone(),
            props.mode.clone(),
        );
        Callback::from(move |_: MouseEvent| {
            // Validation locale.
            if name.trim().is_empty() {
                error.set(Some("Le nom est requis.".into()));
                return;
            }
            if *code_on && !pin::is_valid_pin(&pin_val) {
                error.set(Some("Le PIN doit faire 6 chiffres.".into()));
                return;
            }
            let brand_opt = if brand.trim().is_empty() {
                None
            } else {
                Some((*brand).clone())
            };
            let (name_v, code_v, pin_v) = ((*name).clone(), *code_on, (*pin_val).clone());
            let (on_saved, on_close, error, mode, busy) = (
                on_saved.clone(),
                on_close.clone(),
                error.clone(),
                mode.clone(),
                busy.clone(),
            );

            // Fix B: mark busy before the async call to prevent duplicate submits.
            busy.set(true);

            wasm_bindgen_futures::spawn_local(async move {
                let res: Result<(), api::ApiError> = async {
                    match &mode {
                        FormMode::Create => {
                            let req = CreateProjectReq {
                                name: name_v,
                                brand_name: brand_opt,
                                code_enabled: code_v,
                                pin: if code_v { Some(pin_v) } else { None },
                            };
                            api::client::create_project(&req).await.map(|_| ())
                        }
                        FormMode::Edit(d) => {
                            // 1) nom + brand
                            let upd = UpdateProjectReq {
                                name: Some(name_v),
                                brand_name: Some(brand_opt),
                            };
                            api::client::update_project(d.id, &upd).await?;
                            // 2) code : activer/changer le PIN, ou désactiver.
                            if code_v {
                                api::client::set_code(d.id, &SetCodeReq { pin: pin_v }).await?;
                            } else if d.code_enabled {
                                api::client::clear_code(d.id).await?;
                            }
                            Ok(())
                        }
                    }
                }
                .await;

                match res {
                    Ok(()) => {
                        busy.set(false);
                        on_saved.emit(());
                        on_close.emit(());
                    }
                    Err(e) => {
                        busy.set(false);
                        error.set(Some(e.user_message()));
                    }
                }
            });
        })
    };

    let close = {
        let on_close = props.on_close.clone();
        Callback::from(move |_: MouseEvent| on_close.emit(()))
    };

    html! {
        <SheetContent open={props.open} on_close={props.on_close.clone()} side={Position::Right}>
            <SheetHeader>
                <SheetTitle>{ if is_edit { "Éditer le projet" } else { "Nouveau projet" } }</SheetTitle>
            </SheetHeader>

            <Label html_for="pf-name" required={true}>{ "Nom" }</Label>
            <Input id="pf-name" value={(*name).clone()} oninput={on_name} />

            if is_edit {
                <Label html_for="pf-slug">{ "Slug (auto)" }</Label>
                <Input id="pf-slug" value={initial.slug.clone()} readonly={true} />
            }

            <Label html_for="pf-brand">{ "Nom de marque (optionnel)" }</Label>
            <Input id="pf-brand" value={(*brand).clone()} oninput={on_brand} />

            <Label html_for="pf-code">{ "Code d'accès" }</Label>
            <div class="toggle-row">
                <Switch id="pf-code" checked={*code_on} onchange={on_code_toggle} />
                <span class="hint">
                    { "Quand activé, les visiteurs saisissent un PIN à 6 chiffres avant d'accéder au prototype. Désactivé = accès libre par l'URL." }
                </span>
            </div>

            if *code_on {
                <Label html_for="pf-pin">{ "PIN (6 chiffres)" }</Label>
                <div class="pin-row">
                    <Input id="pf-pin" value={(*pin_val).clone()} oninput={on_pin} />
                    <Button variant={Variant::Outline} onclick={on_regen}>{ "⟳ régénérer" }</Button>
                </div>
            }

            if let Some(msg) = (*error).clone() {
                <p class="error">{ msg }</p>
            }

            <SheetFooter>
                <Button variant={Variant::Ghost} onclick={close}>{ "Annuler" }</Button>
                <Button variant={Variant::Primary} onclick={on_save} disabled={*busy}>
                    { if *busy { "Enregistrement…" } else { "Enregistrer" } }
                </Button>
            </SheetFooter>
        </SheetContent>
    }
}
