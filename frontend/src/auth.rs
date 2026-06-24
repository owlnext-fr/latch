//! État d'authentification dérivé (D4). Aucun token stocké : on déduit l'état des
//! codes HTTP. Sonde de boot = GET /api/projects. Tout 401 → Anonymous.

use yew::prelude::*;
use yew_router::prelude::*;

use crate::api;
use crate::routes::Route;

#[derive(Clone, PartialEq)]
pub enum AuthState {
    Checking,
    Anonymous,
    Authenticated,
}

#[derive(Clone, PartialEq)]
pub struct AuthContext {
    pub state: AuthState,
    pub set_authenticated: Callback<()>,
    pub set_anonymous: Callback<()>,
}

#[hook]
pub fn use_auth() -> AuthContext {
    use_context::<AuthContext>().expect("AuthProvider manquant au-dessus de l'arbre")
}

#[derive(Properties, PartialEq)]
pub struct AuthProviderProps {
    pub children: Html,
}

#[function_component(AuthProvider)]
pub fn auth_provider(props: &AuthProviderProps) -> Html {
    let state = use_state(|| AuthState::Checking);

    let set_authenticated = {
        let state = state.clone();
        Callback::from(move |_| state.set(AuthState::Authenticated))
    };
    let set_anonymous = {
        let state = state.clone();
        Callback::from(move |_| state.set(AuthState::Anonymous))
    };

    // Sonde de boot : une seule fois au montage.
    {
        let state = state.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                match api::client::list_projects().await {
                    Ok(_) => state.set(AuthState::Authenticated),
                    Err(_) => state.set(AuthState::Anonymous),
                }
            });
            || ()
        });
    }

    let ctx = AuthContext {
        state: (*state).clone(),
        set_authenticated,
        set_anonymous,
    };

    html! { <ContextProvider<AuthContext> context={ctx}>{ props.children.clone() }</ContextProvider<AuthContext>> }
}

/// Wrappe une page protégée : Checking → spinner ; Anonymous → redirige Login (+ spinner) ; Authenticated → enfants.
#[derive(Properties, PartialEq)]
pub struct ProtectedProps {
    pub children: Html,
}

#[function_component(Protected)]
pub fn protected(props: &ProtectedProps) -> Html {
    let auth = use_auth();
    let navigator = use_navigator().expect("router");

    {
        let state = auth.state.clone();
        use_effect_with(state, move |state| {
            if *state == AuthState::Anonymous {
                navigator.push(&Route::Login);
            }
            || ()
        });
    }

    match auth.state {
        AuthState::Authenticated => props.children.clone(),
        AuthState::Checking | AuthState::Anonymous => {
            html! { <div class="loading">{ t!("common.loading") }</div> }
        }
    }
}
