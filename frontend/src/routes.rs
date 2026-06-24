use yew::prelude::*;
use yew_router::prelude::*;

use crate::auth::Protected;

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
        Route::Home => {
            html! { <Protected>{ html!{ <crate::pages::list::ListPage /> } }</Protected> }
        }
        Route::Login => html! { <crate::pages::login::LoginPage /> },
        Route::Project { id } => {
            html! { <Protected>{ html!{ <crate::pages::detail::DetailPage {id} /> } }</Protected> }
        }
        Route::NotFound => html! { <h1>{ "404" }</h1> },
    }
}
