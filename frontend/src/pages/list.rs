//! Liste des projets : table (nom, URL+copie, code, version active),
//! clic ligne → détail, bouton Nouveau projet, logout, i18n, badges colorés.

use shadcn_rs::{
    Badge, Button, Table, TableBody, TableCell, TableHead, TableHeader, TableRow, Variant,
};
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api::{self, ApiError};
use crate::auth::use_auth;
use crate::components::copy_button::CopyButton;
use crate::components::locale_switcher::LocaleSwitcher;
use crate::i18n::use_locale;
use crate::panels::project_form::{FormMode, ProjectForm};
use crate::routes::Route;
use crate::toast::use_toast;
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
    let _loc = use_locale();
    let toast = use_toast();

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
        Load::Loading => html! { <p>{ t!("common.loading") }</p> },
        Load::Failed(msg) => html! { <p class="error">{ msg.clone() }</p> },
        Load::Ready(items) if items.is_empty() => html! {
            <div class="empty-state">
                <p>{ t!("list.empty") }</p>
                <Button variant={Variant::Primary} onclick={on_new.clone()}>
                    { t!("list.create_first") }
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
                    html! { <Badge variant={Variant::Secondary} class={classes!("badge--success")}>{ t!("list.badge_code_on") }</Badge> }
                } else {
                    html! { <Badge variant={Variant::Outline} class={classes!("badge--warning")}>{ t!("list.badge_free") }</Badge> }
                };
                let version = match p.active_version_id {
                    Some(_) => html! { <span>{ t!("list.active") }</span> },
                    None => html! { <span>{ t!("common.dash") }</span> },
                };
                html! {
                    <TableRow>
                        <TableCell>
                            <button class="linkish" onclick={onclick.clone()}>{ p.name.clone() }</button>
                        </TableCell>
                        <TableCell>
                            <code>{ url.clone() }</code>
                            <CopyButton value={url} aria_label={AttrValue::from(t!("list.copy_url_aria").to_string())} />
                        </TableCell>
                        <TableCell>
                            <button class="linkish" onclick={onclick.clone()}>{ badge }</button>
                        </TableCell>
                        <TableCell>
                            <button class="linkish" onclick={onclick}>{ version }</button>
                        </TableCell>
                    </TableRow>
                }
            }).collect::<Html>();
            html! {
                <Table>
                    <TableHeader>
                        <TableRow>
                            <TableHead>{ t!("list.col_name") }</TableHead>
                            <TableHead>{ t!("list.col_url") }</TableHead>
                            <TableHead>{ t!("list.col_code") }</TableHead>
                            <TableHead>{ t!("list.col_version") }</TableHead>
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
                    <LocaleSwitcher />
                    <Button variant={Variant::Primary} onclick={on_new}>{ t!("common.new_project") }</Button>
                    <Button variant={Variant::Ghost} onclick={on_logout}>{ t!("common.logout") }</Button>
                </span>
            </header>
            <p class="page-intro">{ t!("list.intro") }</p>
            { body }
            <ProjectForm
                open={*creating}
                mode={FormMode::Create}
                on_close={{ let c = creating.clone(); Callback::from(move |_| c.set(false)) }}
                on_saved={{
                    let data = data.clone();
                    let toast = toast.clone();
                    Callback::from(move |_| {
                        toast.push_success.emit(t!("toast.project_created").to_string());
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
