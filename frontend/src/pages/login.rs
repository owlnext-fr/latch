//! Écran de login. Sur succès, bascule l'auth en Authenticated et navigue vers la
//! liste. Erreur 401 → message inline. Rate-limit géré côté serveur.

use shadcn_rs::{Button, Card, CardContent, CardHeader, CardTitle, Input, Label, Variant};
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api;
use crate::auth::use_auth;
use crate::routes::Route;

#[function_component(LoginPage)]
pub fn login_page() -> Html {
    let auth = use_auth();
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
        Callback::from(move |_: MouseEvent| {
            let body = latch_dto::LoginReq {
                user: (*user).clone(),
                pass: (*pass).clone(),
            };
            let (error, busy, set_auth, navigator) = (
                error.clone(),
                busy.clone(),
                set_auth.clone(),
                navigator.clone(),
            );
            error.set(None);
            busy.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match api::client::login(&body).await {
                    Ok(()) => {
                        set_auth.emit(());
                        navigator.push(&Route::Home);
                    }
                    Err(_) => error.set(Some("Identifiants invalides.".into())),
                }
                busy.set(false);
            });
        })
    };

    html! {
        <div class="auth-screen">
            <Card>
                <CardHeader><CardTitle>{ "latch — admin" }</CardTitle></CardHeader>
                <CardContent>
                    <Label html_for="user">{ "Identifiant" }</Label>
                    <Input id="user" value={(*user).clone()} oninput={on_user} />
                    <Label html_for="pass">{ "Mot de passe" }</Label>
                    <Input id="pass" r#type="password" value={(*pass).clone()} oninput={on_pass} />
                    if let Some(msg) = (*error).clone() {
                        <p class="error">{ msg }</p>
                    }
                    <Button variant={Variant::Primary} full_width={true}
                            disabled={*busy} onclick={on_submit}>
                        { if *busy { "Connexion…" } else { "Se connecter" } }
                    </Button>
                </CardContent>
            </Card>
        </div>
    }
}
