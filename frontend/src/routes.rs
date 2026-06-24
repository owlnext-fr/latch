use yew::prelude::*;
use yew_router::prelude::*;

/// Routes client. `basename="/admin"` est posé sur le `<BrowserRouter>` ; les
/// chemins ci-dessous sont ABSOLUS (incluent /admin) — combo robuste en yew-router 0.18.
#[derive(Clone, Routable, PartialEq)]
pub enum Route {
    #[at("/admin")]
    Home,
    #[at("/admin/login")]
    Login,
    #[at("/admin/projects/:id")]
    Project { id: i32 },
    #[not_found]
    #[at("/admin/404")]
    NotFound,
}

pub fn switch(route: Route) -> Html {
    match route {
        Route::Home => html! { <h1>{ "Liste (placeholder)" }</h1> },
        Route::Login => html! { <h1>{ "Login (placeholder)" }</h1> },
        Route::Project { id } => html! { <h1>{ format!("Détail {} (placeholder)", id) }</h1> },
        Route::NotFound => html! { <h1>{ "404" }</h1> },
    }
}
