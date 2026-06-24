#[macro_use]
extern crate rust_i18n;

use yew::prelude::*;
use yew_router::prelude::*;

mod api;
mod auth;
mod components;
mod i18n;
mod pages;
mod panels;
mod routes;
mod toast;
mod util;

use auth::AuthProvider;
use routes::{switch, Route};

rust_i18n::i18n!("locales");

#[function_component(App)]
fn app() -> Html {
    html! {
        <BrowserRouter>
            <AuthProvider>
                <Switch<Route> render={switch} />
            </AuthProvider>
        </BrowserRouter>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
