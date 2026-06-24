//! Couche de toasts maison. shadcn-rs `Toast`/`Sonner` sont déclaratifs et sans
//! auto-dismiss (cf. QUIRKS) → provider maison : Vec<Toast> + gloo-timers (4 s).

use std::cell::RefCell;
use std::rc::Rc;

use gloo_timers::callback::Timeout;
use yew::prelude::*;

#[derive(Clone, Copy, PartialEq)]
enum ToastKind {
    Success,
    Error,
}

#[derive(Clone, PartialEq)]
struct Toast {
    id: u32,
    kind: ToastKind,
    msg: String,
}

#[derive(Clone, PartialEq)]
pub struct ToastHandle {
    pub push_success: Callback<String>,
    pub push_error: Callback<String>,
}

#[hook]
pub fn use_toast() -> ToastHandle {
    use_context::<ToastHandle>().expect("ToastProvider manquant au-dessus de l'arbre")
}

#[derive(Properties, PartialEq)]
pub struct ToastProviderProps {
    pub children: Html,
}

fn make_push(
    toasts: UseStateHandle<Vec<Toast>>,
    next_id: Rc<RefCell<u32>>,
    kind: ToastKind,
) -> Callback<String> {
    Callback::from(move |msg: String| {
        let id = {
            let mut n = next_id.borrow_mut();
            *n += 1;
            *n
        };
        let mut v = (*toasts).clone();
        v.push(Toast { id, kind, msg });
        toasts.set(v);

        let toasts = toasts.clone();
        Timeout::new(4000, move || {
            let v: Vec<Toast> = (*toasts).iter().filter(|t| t.id != id).cloned().collect();
            toasts.set(v);
        })
        .forget();
    })
}

#[function_component(ToastProvider)]
pub fn toast_provider(props: &ToastProviderProps) -> Html {
    let toasts = use_state(Vec::<Toast>::new);
    let next_id = use_mut_ref(|| 0u32);

    let handle = ToastHandle {
        push_success: make_push(toasts.clone(), next_id.clone(), ToastKind::Success),
        push_error: make_push(toasts.clone(), next_id.clone(), ToastKind::Error),
    };

    let items = (*toasts)
        .iter()
        .map(|t| {
            let cls = match t.kind {
                ToastKind::Success => "toast toast--success",
                ToastKind::Error => "toast toast--error",
            };
            html! { <div key={t.id} class={cls} role="status">{ t.msg.clone() }</div> }
        })
        .collect::<Html>();

    html! {
        <ContextProvider<ToastHandle> context={handle}>
            { props.children.clone() }
            <div class="toast-stack">{ items }</div>
        </ContextProvider<ToastHandle>>
    }
}
