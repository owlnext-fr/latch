//! Détail projet : lecture seule + actions en haut à droite (Éditer / Déployer /
//! Supprimer) + versions avec actions-icône. Tout passe par des side-panels.

use shadcn_rs::{
    Badge, Button, Card, CardContent, CardHeader, CardTitle, Size, Table, TableBody, TableCell,
    TableHead, TableHeader, TableRow, Variant,
};
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api::{self, ApiError};
use crate::auth::use_auth;
use crate::components::{copy_button::CopyButton, pin_field::PinField};
use crate::i18n::use_locale;
use crate::panels::delete_project::DeleteProjectPanel;
use crate::panels::delete_version::DeleteVersionPanel;
use crate::panels::deploy::DeployPanel;
use crate::panels::project_form::{FormMode, ProjectForm};
use crate::routes::Route;
use crate::toast::use_toast;
use crate::util::url::public_url;
use latch_dto::ProjectDetail;

#[derive(Properties, PartialEq)]
pub struct DetailProps {
    pub id: i32,
}

#[derive(Clone, PartialEq)]
enum Load {
    Loading,
    Ready(ProjectDetail),
    Failed(String),
}

#[function_component(DetailPage)]
pub fn detail_page(props: &DetailProps) -> Html {
    let id = props.id;
    let auth = use_auth();
    let _loc = use_locale();
    let toast = use_toast();
    let navigator = use_navigator().expect("router");
    let data = use_state(|| Load::Loading);
    let editing = use_state(|| false);
    let deploying = use_state(|| false);
    let deleting = use_state(|| false);
    let deleting_version = use_state(|| Option::<i32>::None);

    // reload helper
    let reload = {
        let (data, set_anon) = (data.clone(), auth.set_anonymous.clone());
        Callback::from(move |_| {
            let (data, set_anon) = (data.clone(), set_anon.clone());
            wasm_bindgen_futures::spawn_local(async move {
                match api::client::get_project(id).await {
                    Ok(d) => data.set(Load::Ready(d)),
                    Err(ApiError::Unauthorized) => set_anon.emit(()),
                    Err(e) => data.set(Load::Failed(e.user_message())),
                }
            });
        })
    };

    {
        let reload = reload.clone();
        use_effect_with((), move |_| {
            reload.emit(());
            || ()
        });
    }

    let body = match &*data {
        Load::Loading => html! { <p>{ t!("common.loading") }</p> },
        Load::Failed(msg) => html! { <p class="error">{ msg.clone() }</p> },
        Load::Ready(p) => {
            let url = public_url(&p.slug);
            let on_back = {
                let nav = navigator.clone();
                Callback::from(move |_| nav.push(&Route::Home))
            };
            let open_edit = {
                let e = editing.clone();
                Callback::from(move |_| e.set(true))
            };
            let open_deploy = {
                let d = deploying.clone();
                Callback::from(move |_| d.set(true))
            };
            let open_delete = {
                let d = deleting.clone();
                Callback::from(move |_| d.set(true))
            };

            let access = html! {
                <Card>
                    <CardHeader><CardTitle>{ t!("detail.access_title") }</CardTitle></CardHeader>
                    <CardContent>
                        <div class="kv">
                            <span class="k">{ t!("detail.url_label") }</span>
                            <span class="v">
                                <code>{ url.clone() }</code>
                                <CopyButton value={url.clone()} aria_label={AttrValue::from(t!("detail.copy_url_aria").to_string())} />
                            </span>
                        </div>
                        <div class="kv">
                            <span class="k">{ t!("detail.code_label") }</span>
                            <span class="v">
                                if p.code_enabled {
                                    if let Some(pin) = p.pin.clone() {
                                        <PinField pin={pin} />
                                    } else {
                                        <Badge variant={Variant::Outline}>{ t!("detail.pin_undefined") }</Badge>
                                    }
                                } else {
                                    <Badge variant={Variant::Outline} class={classes!("badge--warning")}>{ t!("detail.free_access") }</Badge>
                                }
                            </span>
                        </div>
                    </CardContent>
                </Card>
            };

            let config = html! {
                <Card>
                    <CardHeader><CardTitle>{ t!("detail.config_title") }</CardTitle></CardHeader>
                    <CardContent>
                        <div class="kv"><span class="k">{ t!("detail.brand_label") }</span>
                            <span class="v">{ p.brand_name.clone().unwrap_or_else(|| t!("common.dash").to_string()) }</span></div>
                        <div class="kv"><span class="k">{ t!("detail.code_label") }</span>
                            <span class="v">{ if p.code_enabled { t!("detail.code_on") } else { t!("detail.code_off") } }</span></div>
                    </CardContent>
                </Card>
            };

            let rows = p.versions.iter().map(|v| {
                let n = v.n;
                let activate = {
                    let reload = reload.clone();
                    let toast = toast.clone();
                    Callback::from(move |_| {
                        let (reload, toast) = (reload.clone(), toast.clone());
                        wasm_bindgen_futures::spawn_local(async move {
                            match api::client::activate_version(id, n).await {
                                Ok(()) => toast.push_success.emit(t!("toast.version_activated").to_string()),
                                Err(e) => toast.push_error.emit(e.user_message()),
                            }
                            reload.emit(());
                        });
                    })
                };
                let preview_href = api::client::preview_url(id, n);
                let on_del = {
                    let dv = deleting_version.clone();
                    Callback::from(move |_| dv.set(Some(n)))
                };
                html! {
                    <TableRow>
                        <TableCell>{ format!("v{}", v.n) }</TableCell>
                        <TableCell>{ v.created_at.clone() }</TableCell>
                        <TableCell>
                            if v.is_active {
                                <Badge variant={Variant::Secondary} class={classes!("badge--success")}>{ t!("common.active") }</Badge>
                            }
                        </TableCell>
                        <TableCell>
                            if !v.is_active {
                                <Button variant={Variant::Ghost} size={Size::Sm} onclick={activate}
                                        aria_label={AttrValue::from(t!("detail.activate_aria").to_string())}>{ "↑" }</Button>
                            }
                            <a href={preview_href} target="_blank" rel="noopener" class="icon-link"
                               aria-label={AttrValue::from(t!("detail.preview_aria").to_string())}>{ "↗" }</a>
                            if !v.is_active {
                                <Button variant={Variant::Ghost} size={Size::Sm} onclick={on_del}
                                        aria_label={AttrValue::from(t!("detail.delete_aria").to_string())}>{ "🗑" }</Button>
                            }
                        </TableCell>
                    </TableRow>
                }
            }).collect::<Html>();

            html! {
                <>
                    <header class="detail-head">
                        <div>
                            <button class="linkish crumb" onclick={on_back}>{ t!("detail.back") }</button>
                            <h1>{ p.name.clone() }</h1>
                        </div>
                        <div class="head-actions">
                            <Button variant={Variant::Outline} onclick={open_edit}>{ "✎ " }{ t!("common.edit") }</Button>
                            <Button variant={Variant::Outline} onclick={open_deploy}>{ "⬆ " }{ t!("common.deploy") }</Button>
                            <Button variant={Variant::Destructive} onclick={open_delete}>{ "🗑 " }{ t!("common.delete") }</Button>
                        </div>
                    </header>
                    <p class="page-intro">{ t!("detail.intro") }</p>
                    { access }
                    { config }
                    <Card>
                        <CardHeader><CardTitle>{ t!("detail.versions_title") }</CardTitle></CardHeader>
                        <CardContent>
                            <Table>
                                <TableHeader><TableRow>
                                    <TableHead>{ t!("detail.col_num") }</TableHead>
                                    <TableHead>{ t!("detail.col_date") }</TableHead>
                                    <TableHead>{ t!("detail.col_status") }</TableHead>
                                    <TableHead>{ "" }</TableHead>
                                </TableRow></TableHeader>
                                <TableBody>{ rows }</TableBody>
                            </Table>
                        </CardContent>
                    </Card>

                    // Panels
                    <ProjectForm open={*editing} mode={FormMode::Edit(p.clone())}
                        on_close={{ let e = editing.clone(); Callback::from(move |_| e.set(false)) }}
                        on_saved={reload.clone()} />
                    <DeployPanel open={*deploying} project_id={id}
                        on_close={{ let d = deploying.clone(); Callback::from(move |_| d.set(false)) }}
                        on_deployed={reload.clone()} />
                    <DeleteProjectPanel open={*deleting} project={p.clone()}
                        on_close={{ let d = deleting.clone(); Callback::from(move |_| d.set(false)) }}
                        on_deleted={{ let nav = navigator.clone(); Callback::from(move |_| nav.push(&Route::Home)) }} />
                    if let Some(n) = *deleting_version {
                        <DeleteVersionPanel open={true} project_id={id} n={n}
                            on_close={{ let dv = deleting_version.clone(); Callback::from(move |_| dv.set(None)) }}
                            on_deleted={reload.clone()} />
                    }
                </>
            }
        }
    };

    html! { <div class="admin-page">{ body }</div> }
}
