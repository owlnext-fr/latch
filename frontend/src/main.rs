use yew::prelude::*;
use yew_router::prelude::*;

mod api;
mod auth;
mod pages;
mod routes;
mod util;

use auth::AuthProvider;
use routes::{switch, Route};

#[function_component(App)]
fn app() -> Html {
    html! {
        <BrowserRouter basename="/admin">
            <AuthProvider>
                <Switch<Route> render={switch} />
            </AuthProvider>
        </BrowserRouter>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
