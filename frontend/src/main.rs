use yew::prelude::*;
use yew_router::prelude::*;

mod api;
mod routes;
mod util;

use routes::{switch, Route};

#[function_component(App)]
fn app() -> Html {
    html! {
        <BrowserRouter basename="/admin">
            <Switch<Route> render={switch} />
        </BrowserRouter>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
