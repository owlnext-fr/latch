use yew::prelude::*;
use yew_router::prelude::*;

use crate::auth::Protected;

/// Routes client. PAS de `basename` sur le `<BrowserRouter>` : yew-router 0.18 a un
/// bug dans `strip_basename` qui transforme l'URL racine exacte `/admin` en `//admin`
/// (jamais matchée). On utilise donc des `#[at]` ABSOLUS incluant `/admin`, sans
/// basename. Trunk n'injecte pas de `<base>` (public_url réécrit les assets en absolu).
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
