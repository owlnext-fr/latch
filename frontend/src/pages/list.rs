//! Liste des projets : table (nom, URL+copie, code, version active),
//! clic ligne → détail, bouton Nouveau projet (no-op T10 — branché T11), logout.

use shadcn_rs::{
    Badge, Button, Table, TableBody, TableCell, TableHead, TableHeader, TableRow, Variant,
};
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api::{self, ApiError};
use crate::auth::use_auth;
use crate::components::copy_button::CopyButton;
use crate::panels::project_form::{FormMode, ProjectForm};
use crate::routes::Route;
use crate::util::url::public_url;
use latch_dto::ProjectListItem;

#[derive(Clone, PartialEq)]
enum Load {
    Loading,
    Ready(Vec<ProjectListItem>),
    Failed(String),
}

#[function_component(ListPage)]
pub fn list_page() -> Html {
    let auth = use_auth();
    let navigator = use_navigator().expect("router");
    let data = use_state(|| Load::Loading);
    let creating = use_state(|| false);

    // Chargement au montage.
    {
        let data = data.clone();
        let set_anon = auth.set_anonymous.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                match api::client::list_projects().await {
                    Ok(items) => data.set(Load::Ready(items)),
                    Err(ApiError::Unauthorized) => set_anon.emit(()),
                    Err(e) => data.set(Load::Failed(e.user_message())),
                }
            });
            || ()
        });
    }

    let on_logout = {
        let set_anon = auth.set_anonymous.clone();
        Callback::from(move |_: MouseEvent| {
            let set_anon = set_anon.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let _ = api::client::logout().await;
                set_anon.emit(());
            });
        })
    };

    let on_new = {
        let creating = creating.clone();
        Callback::from(move |_: MouseEvent| creating.set(true))
    };

    let body = match &*data {
        Load::Loading => html! { <p>{ "Chargement…" }</p> },
        Load::Failed(msg) => html! { <p class="error">{ msg.clone() }</p> },
        Load::Ready(items) if items.is_empty() => html! {
            <div class="empty-state">
                <p>{ "Aucun projet pour l'instant." }</p>
                <Button variant={Variant::Primary} onclick={on_new.clone()}>
                    { "+ Créer le premier projet" }
                </Button>
            </div>
        },
        Load::Ready(items) => {
            let rows = items.iter().map(|p| {
                let id = p.id;
                let nav = navigator.clone();
                let onclick = Callback::from(move |_: MouseEvent| nav.push(&Route::Project { id }));
                let url = public_url(&p.slug);
                let badge = if p.code_enabled {
                    html! { <Badge variant={Variant::Secondary}>{ "code activé" }</Badge> }
                } else {
                    html! { <Badge variant={Variant::Outline}>{ "libre" }</Badge> }
                };
                let version = match p.active_version_id {
                    Some(_) => html! { <span>{ "active" }</span> },
                    None => html! { <span>{ "\u{2014}" }</span> },
                };
                html! {
                    <TableRow>
                        <TableCell>
                            <a onclick={onclick.clone()} style="cursor:pointer">{ p.name.clone() }</a>
                        </TableCell>
                        <TableCell>
                            <code>{ url.clone() }</code>
                            <CopyButton value={url} aria_label={AttrValue::from("Copier l'URL")} />
                        </TableCell>
                        <TableCell>
                            <a onclick={onclick.clone()} style="cursor:pointer">{ badge }</a>
                        </TableCell>
                        <TableCell>
                            <a onclick={onclick} style="cursor:pointer">{ version }</a>
                        </TableCell>
                    </TableRow>
                }
            }).collect::<Html>();
            html! {
                <Table>
                    <TableHeader>
                        <TableRow>
                            <TableHead>{ "Nom" }</TableHead>
                            <TableHead>{ "URL publique" }</TableHead>
                            <TableHead>{ "Code" }</TableHead>
                            <TableHead>{ "Version active" }</TableHead>
                        </TableRow>
                    </TableHeader>
                    <TableBody>{ rows }</TableBody>
                </Table>
            }
        }
    };

    html! {
        <div class="admin-page">
            <header class="topbar">
                <span class="brand">{ "latch" }</span>
                <span class="actions">
                    <Button variant={Variant::Primary} onclick={on_new}>{ "+ Nouveau projet" }</Button>
                    <Button variant={Variant::Ghost} onclick={on_logout}>{ "Logout" }</Button>
                </span>
            </header>
            { body }
            <ProjectForm
                open={*creating}
                mode={FormMode::Create}
                on_close={{ let c = creating.clone(); Callback::from(move |_| c.set(false)) }}
                on_saved={{
                    let data = data.clone();
                    Callback::from(move |_| {
                        let data = data.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            if let Ok(items) = api::client::list_projects().await {
                                data.set(Load::Ready(items));
                            }
                        });
                    })
                }}
            />
        </div>
    }
}
