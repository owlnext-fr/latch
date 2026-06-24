use yew::prelude::*;

/// Placeholder Phase 0 : prouve que la chaîne Yew + Trunk → wasm fonctionne.
/// Les vrais écrans admin (liste, détail, side-panel) arrivent en Phase 3.
#[function_component(App)]
fn app() -> Html {
    html! {
        <main>
            <h1>{ "latch — admin" }</h1>
            <p>{ "Squelette Phase 0. SPA Yew servie en statique par Loco." }</p>
        </main>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
