//! Écran de login. Sur succès, bascule l'auth en Authenticated et navigue vers la
//! liste. Erreur 401 → message inline + toast. Rate-limit géré côté serveur.

use shadcn_rs::{Button, Card, CardContent, CardHeader, CardTitle, Input, Label, Variant};
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api;
use crate::auth::use_auth;
use crate::components::locale_switcher::LocaleSwitcher;
use crate::i18n::use_locale;
use crate::routes::Route;
use crate::toast::use_toast;

#[function_component(LoginPage)]
pub fn login_page() -> Html {
    let auth = use_auth();
    let _loc = use_locale();
    let toast = use_toast();
    let navigator = use_navigator().expect("router");
    let user = use_state(String::new);
    let pass = use_state(String::new);
    let error = use_state(|| Option::<String>::None);
    let busy = use_state(|| false);

    let on_user = {
        let user = user.clone();
        Callback::from(move |e: InputEvent| {
            let v = e.target_unchecked_into::<HtmlInputElement>().value();
            user.set(v);
        })
    };
    let on_pass = {
        let pass = pass.clone();
        Callback::from(move |e: InputEvent| {
            let v = e.target_unchecked_into::<HtmlInputElement>().value();
            pass.set(v);
        })
    };

    let on_submit = {
        let (user, pass, error, busy) = (user.clone(), pass.clone(), error.clone(), busy.clone());
        let set_auth = auth.set_authenticated.clone();
        let navigator = navigator.clone();
        let toast = toast.clone();
        Callback::from(move |_: MouseEvent| {
            let body = latch_dto::LoginReq {
                user: (*user).clone(),
                pass: (*pass).clone(),
            };
            let (error, busy, set_auth, navigator, toast) = (
                error.clone(),
                busy.clone(),
                set_auth.clone(),
                navigator.clone(),
                toast.clone(),
            );
            error.set(None);
            busy.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match api::client::login(&body).await {
                    Ok(()) => {
                        set_auth.emit(());
                        navigator.push(&Route::Home);
                    }
                    Err(_) => {
                        error.set(Some(t!("login.error_invalid").to_string()));
                        toast.push_error.emit(t!("login.error_invalid").to_string());
                    }
                }
                busy.set(false);
            });
        })
    };

    html! {
        <div class="auth-screen">
            <Card>
                <CardHeader>
                    <CardTitle>{ t!("login.title") }</CardTitle>
                </CardHeader>
                <CardContent>
                    <Label html_for="user">{ t!("login.user") }</Label>
                    <Input id="user" value={(*user).clone()} oninput={on_user} />
                    <Label html_for="pass">{ t!("login.pass") }</Label>
                    <Input id="pass" r#type="password" value={(*pass).clone()} oninput={on_pass} />
                    if let Some(msg) = (*error).clone() {
                        <p class="error">{ msg }</p>
                    }
                    <Button variant={Variant::Primary} full_width={true}
                            class={classes!("login-submit")}
                            disabled={*busy} onclick={on_submit}>
                        { if *busy { t!("login.submitting") } else { t!("login.submit") } }
                    </Button>
                    <div class="auth-footer"><LocaleSwitcher /></div>
                </CardContent>
            </Card>
        </div>
    }
}
