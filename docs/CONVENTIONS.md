# Conventions — squelettes de code du projet

> Les patterns *découverts en cours de route* (un service type, un endpoint type, un
> composant Yew type, un test type). À remplir au fil de l'implémentation : dès qu'un
> motif se répète, on le fige ici pour que les sessions suivantes le copient au lieu
> de le réinventer. Les règles *normatives fixées d'avance* (pas d'`unwrap`, commits
> conventionnels…) restent dans `BOOTSTRAP §4`, pas ici.

## Service (cœur) type

Un service cœur est une struct tenant ses dépendances injectées, construite via `new(...)`, avec des méthodes `async` renvoyant `Result<_, CoreError>`. Les helpers sans état (génération de slug, PIN, comparaison sécurisée) sont des fonctions libres dans leur propre module.

```rust
// Exemple réel : backend/src/services/deploy.rs
pub struct DeployService {
    db: DatabaseConnection,
    storage: Arc<dyn Storage>,
}

impl DeployService {
    pub fn new(db: DatabaseConnection, storage: Arc<dyn Storage>) -> Self {
        Self { db, storage }
    }

    pub async fn deploy(
        &self,
        project_id: i32,
        html: &str,
        activate: bool,
    ) -> Result<versions::Model, CoreError> {
        // ... logique purement métier, sans axum/loco
    }
}

// Exemple réel : backend/src/services/projects.rs
pub struct ProjectsService {
    db: DatabaseConnection,
}

impl ProjectsService {
    pub fn new(db: DatabaseConnection) -> Self { Self { db } }

    pub async fn create(&self, input: CreateProject) -> Result<projects::Model, CoreError> {
        // ...
    }
}
```

**Règles :**
- Aucun `use axum::` ni `use loco_rs::` (contrat §1 — vérifié par `backend/tests/architecture.rs`).
- Le service suppose l'appelant déjà autorisé : pas de session/token/cookie ici.
- Les erreurs DB (`sea_orm::DbErr`) se mappe via `impl From<DbErr> for CoreError`.

## Endpoint admin (adaptateur web) type

Un handler admin lit l'état via `AdminAuth` + `State(ctx)`, appelle le service du cœur,
mappe `CoreError` → `loco_rs::Error` via `error::into_response`, et sérialise avec `format::json`.

```rust
// Exemple réel : backend/src/controllers/admin.rs
#[debug_handler]
async fn list(_auth: AdminAuth, State(ctx): State<AppContext>) -> Result<Response> {
    let svc = ProjectsService::new(ctx.db.clone());
    let items: Vec<ProjectListItem> = svc.list().await.map_err(into_response)?
        .iter().map(ProjectListItem::from).collect();
    format::json(items)
}

#[debug_handler]
async fn detail(_auth: AdminAuth, State(ctx): State<AppContext>, Path(id): Path<i32>) -> Result<Response> {
    let project = projects::Entity::find_by_id(id)
        .one(&ctx.db).await.map_err(|e| into_response(e.into()))?
        .ok_or(loco_rs::Error::NotFound)?;
    // ...
    format::json(ProjectDetail::from_model(project, vers))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/admin")
        .add("/projects", get(list))
        .add("/projects/{id}", get(detail))  // axum 0.8 : {id}, pas :id
}
```

**Règles :**
- `AdminAuth` en premier argument (FromRequestParts) ; `State`/`Path` ensuite.
- `#[debug_handler]` sur chaque handler pour un diagnostic de type complet.
- Path params : syntaxe `{id}` (axum 0.8) dans les routes, `Path(id): Path<i32>` dans le handler.
- Erreurs DB : `.map_err(|e| into_response(e.into()))` (CoreError::Db via From<DbErr>).
- Not found : `.ok_or(loco_rs::Error::NotFound)` directement (pas via into_response).

## Câblage multi-verbe avec garde Origin par handler (Phase 2, Task 7)

Loco 0.16 / axum 0.8 : plusieurs `.add(path, method_router)` sur le même chemin
avec des verbes distincts sont **fusionnés** par axum (`Router::route` merge les `MethodRouter`).
Le `.layer(mw)` par `MethodRouter` s'applique uniquement aux verbes définis dans ce `MethodRouter`.

```rust
// Dans routes() — un .add() par verbe, layer uniquement sur les mutations.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/admin")
        // Lecture : pas de layer (GET idempotent)
        .add("/projects", get(list))
        .add("/projects/{id}", get(detail))
        // Mutations : garde Origin obligatoire (contrat §4/§9.6)
        .add("/projects", post(create).layer(from_fn(require_same_origin)))
        .add("/projects/{id}", put(update).layer(from_fn(require_same_origin)))
        .add(
            "/projects/{id}",
            // Nommer explicitement axum::routing::delete si le handler se nomme aussi `delete`.
            axum::routing::delete(delete).layer(from_fn(require_same_origin)),
        )
}
```

**Règles :**
- `axum::routing::delete(handler)` (namespaced) si le handler se nomme `delete` (évite ambiguïté).
- `.layer()` sur `MethodRouter` s'applique à tous ses verbes définis — donc `post(h).layer(mw)` = seul POST+mw, pas GET.
- Fusionner GET (sans layer) + POST+layer : résultat = `{ GET: handler_sans_layer, POST: handler_avec_layer }`.
- En tests, utiliser `Origin: http://127.0.0.1` (pas `localhost`) — le harness Loco envoie `Host: 127.0.0.1:PORT`.

## Extracteur d'auth axum (FromRequestParts, axum 0.8)

Pattern `AdminAuth` — pas de `#[async_trait]`, fn async native :

```rust
pub struct AdminAuth;

impl<S> FromRequestParts<S> for AdminAuth
where
    S: Send + Sync,
{
    type Rejection = loco_rs::Error;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        let session = AdminSession::from_request_parts(parts, state)
            .await
            .map_err(|_| loco_rs::Error::Unauthorized("no session".to_string()))?;
        if session.get::<bool>(ADMIN_FLAG).unwrap_or(false) {
            Ok(AdminAuth)
        } else {
            Err(loco_rs::Error::Unauthorized("not authenticated".to_string()))
        }
    }
}
```

**Règles :**
- Pas de `#[async_trait]` en axum 0.8 — async fn in trait est native.
- `type Rejection = loco_rs::Error` — `loco_rs::Error` implémente `IntoResponse`.
- `Session<T>` a `Rejection = (StatusCode, &'static str)` → mapper avec `.map_err(|_| loco_rs::Error::Unauthorized(...))`.

## Rate-limit tower_governor (layer par route)

```rust
// Dans routes() — inline pour éviter l'annotation de type verbeuse
let login_governor = {
    let config = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(2)
            .burst_size(5)
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .expect("governor config valide"),
    );
    GovernorLayer { config }  // struct literal, pas ::new()
};

Routes::new()
    .prefix("/admin")
    .add("/login", post(login).layer(login_governor))
```

**Règles :**
- `GovernorLayer { config: Arc::new(config) }` (struct literal, le champ `config` est public).
- `.finish()` retourne `Option<_>` → `.expect(...)` acceptable en init de boot.
- `SmartIpKeyExtractor` lit `X-Forwarded-For` / `X-Real-IP` avant l'IP peer.
- Dans les tests, injecter `X-Forwarded-For: 1.2.3.4` pour garantir l'extraction de clé.

## Câblage d'un layer axum dans after_routes (Phase 2)

`after_routes` est le hook Loco pour enrichir le routeur axum avant le démarrage.
Les helpers de session vivent dans `src/web/mod.rs` (séparation adaptateur / cœur).

```rust
// Dans backend/src/app.rs — ajout d'un layer session
async fn after_routes(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
    let store = crate::web::build_session_store(ctx).await?;
    let router = router.layer(axum_session::SessionLayer::new(store));
    Ok(router)
}
```

**Règles :**
- La signature exacte est `(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter>` où `AxumRouter = axum::Router` (importer en tête avec `use axum::Router as AxumRouter`).
- Les helpers de résolution (session store, storage) vivent dans `src/web/`, jamais dans `src/services/`.
- `build_session_store` retourne `loco_rs::Result<_>` — propager via `?`.

## Test d'intégration Loco type (harness HTTP réel)

Tests d'intégration sur les routes Loco utilisant le harness `request_with_config`.
Réf. `backend/tests/admin_api.rs`, `backend/tests/security_invariants.rs`.

```rust
// Pattern : login puis accès protégé — save_cookies OBLIGATOIRE
#[tokio::test]
#[serial]  // tests Loco partagent la même base SQLite de test → sérialiser
async fn login_then_access_protected_route() {
    request::<App, _, _>(|request, _ctx| async move {
        // save_cookies(true) propagate les Set-Cookie entre requêtes
        let request = request.with_config(
            RequestConfigBuilder::new().save_cookies(true).build()
        );
        let _ = request
            .post("/admin/login")
            .json(&serde_json::json!({"username": "admin", "password": "secret"}))
            .await;

        // Origin = http://127.0.0.1 (pas localhost) — harness Loco utilise Host: 127.0.0.1:PORT
        let res = request
            .get("/admin/projects")
            .add_header(header::ORIGIN, "http://127.0.0.1")
            .await;
        res.assert_status_ok();
    })
    .await;
}
```

**Règles :**
- `#[serial]` sur tout test qui touche la DB partagée (évite les races entre tests Loco).
- `save_cookies(true)` dans `RequestConfigBuilder` pour tout test login + accès protégé.
- `Origin: http://127.0.0.1` (sans port) dans les tests de mutation (garde `require_same_origin`).
- `LATCH_STORAGE_ROOT` : utiliser `tempfile::tempdir()` + variable vivante jusqu'à la fin du test.
- `X-Forwarded-For: 1.2.3.4` pour déclencher le rate-limit `tower_governor` (garantit l'extraction de clé IP).

## Tool MCP type
_(à remplir : un tool qui valide `deploy_token` en premier, puis appelle le service,
puis mappe l'erreur en tool error.)_

## Composants React (shadcn/ui) — patterns courants

### Hook TanStack Query par endpoint (`use-projects.ts`)

Un fichier de hooks par domaine (`hooks/use-projects.ts`). Chaque mutation invalide le cache
et affiche un toast sonner. Pattern :

```ts
// frontend/src/hooks/use-projects.ts
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { client } from '@/api/client'

export function useProjects() {
  return useQuery({
    queryKey: ['projects'],
    queryFn: async () => {
      const { data, error } = await client.GET('/api/projects')
      if (error) throw error
      return data
    },
  })
}

export function useCreateProject() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: async (body: CreateProjectReq) => {
      const { data, error } = await client.POST('/api/projects', { body })
      if (error) throw error
      return data
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['projects'] })
      toast.success(t('toast.project_created'))
    },
    onError: () => toast.error(t('toast.error_generic')),
  })
}
```

**Règles :**
- Un hook par endpoint, pas de client générique magic.
- `invalidateQueries` après toute mutation (liste + détail si besoin).
- Toast sonner dans `onSuccess` / `onError` (pas dans le composant).
- Le client `openapi-fetch` est typé depuis `schema.d.ts` → pas de cast `as`.

### Side-panel via Radix `<Sheet>`

```tsx
import { Sheet, SheetContent, SheetHeader, SheetTitle } from '@/components/ui/sheet'

<Sheet open={open} onOpenChange={(v) => !v && onClose()}>
  <SheetContent>
    <SheetHeader><SheetTitle>{t('form.title_create')}</SheetTitle></SheetHeader>
    {/* formulaire */}
  </SheetContent>
</Sheet>
```

**Règles :**
- `onOpenChange={(v) => !v && onClose()}` : fermer sur Escape ou clic scrim.
- Confirmations destructives = `<Sheet>` avec variante danger (classe `destructive`).

### Formulaire react-hook-form + zod

```tsx
import { useForm } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { z } from 'zod'

const schema = z.object({ name: z.string().min(1), pin: z.string().length(6) })
type FormValues = z.infer<typeof schema>

function ProjectForm() {
  const { register, handleSubmit, formState: { errors } } = useForm<FormValues>({
    resolver: zodResolver(schema),
  })
  // ...
}
```

**Règles :**
- Un schéma zod par formulaire ; les erreurs sont typées.
- `register` + `errors` pour chaque champ ; pas de state local pour les valeurs.

### Client `openapi-fetch` typé

```ts
// frontend/src/api/client.ts
import createClient from 'openapi-fetch'
import type { paths } from './schema.d.ts'

export const client = createClient<paths>({
  fetch: (input) => globalThis.fetch(input),  // LOAD-BEARING pour MSW — cf. QUIRKS
  credentials: 'include',
})
```

**Règles :**
- `fetch: (input) => globalThis.fetch(input)` obligatoire pour que MSW intercepte en test.
- `credentials: 'include'` pour le cookie de session admin.
- Typage complet : `client.GET('/api/projects')` → `{ data, error }` typés depuis `schema.d.ts`.

### Tests MSW avec `renderWithProviders` / `renderWithRouter`

```tsx
// frontend/src/test/utils.tsx
import { render } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { I18nextProvider } from 'react-i18next'
import i18n from '../i18n'

export function renderWithProviders(ui: React.ReactElement) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(
    <I18nextProvider i18n={i18n}>
      <QueryClientProvider client={qc}>{ui}</QueryClientProvider>
    </I18nextProvider>
  )
}
```

**Règles :**
- `retry: false` dans le QueryClient de test (évite les retries qui font timeout).
- MSW server : `beforeAll(server.listen)`, `afterEach(server.resetHandlers)`, `afterAll(server.close)`.
- `server.use(jsonOnce('GET', '/api/projects', 200, body))` pour surcharger par test.
- Tester l'invariant §9.2 dans chaque test de liste : `expect(screen.queryByText('1234')).not.toBeInTheDocument()`.

---

## Historique Yew — obsolète depuis migration React (2026-06-25)

> Ces conventions concernaient la crate Yew (`latch-ui`, `shadcn-rs`, Trunk, wasm32) retirée du
> workspace lors de la migration React (Plans 1-3, feat/admin-react). Conservées pour référence
> en cas de consultation de l'historique git.

## Composant Yew (shadcn-rs) type

Une **page** charge ses données via `use_state` + `use_effect_with((), ...)` (spawn_local + client API), gère trois états (`Loading` / `Ready(data)` / `Failed(msg)`), et bascule l'auth sur `ApiError::Unauthorized`.
Un **side-panel** utilise `<SheetContent open on_close>` piloté manuellement + `use_effect_with(props.open, ...)` pour réinitialiser les champs à la (ré)ouverture.

```rust
// Extrait représentatif — pages/list.rs (page liste)
#[function_component(ListPage)]
pub fn list_page() -> Html {
    let auth = use_auth();
    let projects: UseStateHandle<Load<Vec<ProjectListItem>>> = use_state(|| Load::Loading);

    {
        let projects = projects.clone();
        let set_anon = auth.set_anonymous.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                match api::client::list_projects().await {
                    Ok(items) => projects.set(Load::Ready(items)),
                    Err(ApiError::Unauthorized) => set_anon.emit(()),
                    Err(e) => projects.set(Load::Failed(e.user_message())),
                }
            });
            || ()
        });
    }

    match (*projects).clone() {
        Load::Loading => html! { <p>{ "Chargement…" }</p> },
        Load::Failed(msg) => html! { <p>{ msg }</p> },
        Load::Ready(items) => html! { /* tableau shadcn-rs */ },
    }
}

// Extrait représentatif — panels/project_form.rs (side-panel création/édition)
#[derive(Properties, PartialEq)]
pub struct ProjectFormProps {
    pub open: bool,
    pub on_close: Callback<()>,
    // ...
}

#[function_component(ProjectForm)]
pub fn project_form(props: &ProjectFormProps) -> Html {
    let name = use_state(String::new);

    // Réinitialiser les champs à chaque ouverture du panel
    {
        let name = name.clone();
        use_effect_with(props.open, move |open| {
            if *open {
                name.set(String::new());
            }
        });
    }

    html! {
        // PAS <Sheet> (coquille) — piloter <SheetContent> directement
        <SheetContent open={props.open} on_close={props.on_close.clone()}>
            // champs du formulaire…
        </SheetContent>
    }
}
```

**Règles :**
- `use_effect_with((), ...)` pour le chargement initial (dépendance vide = une seule fois au mount).
- `use_effect_with(props.open, ...)` pour réinitialiser les champs à la (ré)ouverture.
- Basculer l'auth via `auth.set_anonymous.emit(())` sur `ApiError::Unauthorized` (pas de redirect manuel).
- `<SheetContent open on_close>` directement — `<Sheet>` ignore ses props (cf. QUIRKS).
- Ne pas oublier `use_context::<AuthContext>()` dans tout composant qui appelle l'API.

## Client API SPA type

Une fonction `async` par endpoint. Le pattern `gloo-net 0.6` : construire la requête, appeler `.json(&body)?` **avant** `.send().await?` (le builder est consommé), puis inspecter `resp.status()` (un 401/404 est `Ok(Response)`, pas une `Err`).

```rust
// Extrait représentatif — api/client.rs

#[derive(Debug, Clone, PartialEq)]
pub enum ApiError {
    /// 401 — session absente/expirée. Bascule l'app en Anonymous.
    Unauthorized,
    /// Autre code HTTP non-2xx.
    Status(u16),
    /// Échec réseau / parse JSON.
    Network(String),
}

/// Vérifie le status HTTP et produit un ApiError approprié.
fn check_status(status: u16) -> Result<(), ApiError> {
    match status {
        200..=299 => Ok(()),
        401 => Err(ApiError::Unauthorized),
        _ => Err(ApiError::Status(status)),
    }
}

pub async fn list_projects() -> Result<Vec<ProjectListItem>, ApiError> {
    let resp = Request::get("/api/projects")
        .send()
        .await
        .map_err(|e| ApiError::Network(e.to_string()))?;
    check_status(resp.status())?;
    resp.json::<Vec<ProjectListItem>>()
        .await
        .map_err(|e| ApiError::Network(e.to_string()))
}

pub async fn create_project(req: &CreateProjectReq) -> Result<ProjectDetail, ApiError> {
    // .json(&body)? consomme le builder (retourne Result<Request>) AVANT .send()
    let resp = Request::post("/api/projects")
        .json(req)
        .map_err(|e| ApiError::Network(e.to_string()))?
        .send()
        .await
        .map_err(|e| ApiError::Network(e.to_string()))?;
    check_status(resp.status())?;
    resp.json::<ProjectDetail>()
        .await
        .map_err(|e| ApiError::Network(e.to_string()))
}
```

**Règles :**
- Une fonction `async` par endpoint — pas de client générique / wrapper magique.
- `.json(&body)?` **avant** `.send().await?` (builder consommé à l'appel de `.json()`).
- Toujours `check_status(resp.status())?` — un 401/404 est `Ok(Response)`, pas une `Err`.
- `ApiError::Unauthorized` doit être propagé jusqu'au composant qui appelle `auth.logout()`.

## Test d'intégration type

Pattern SQLite in-memory avec migrations, utilisé dans tous les tests de service (`projects.rs`, `deploy.rs`). Réf. `backend/src/services/test_support.rs`.

```rust
// Helper dans test_support.rs (interne au crate)
pub(crate) async fn test_db() -> DatabaseConnection {
    let mut opt = ConnectOptions::new("sqlite::memory:");
    opt.max_connections(1); // LOAD-BEARING — cf. QUIRKS
    let db = Database::connect(opt).await.expect("connect in-memory sqlite");
    Migrator::up(&db, None).await.expect("run migrations");
    db
}

// Usage dans un test de service
#[cfg(test)]
mod tests {
    use crate::services::test_support::test_db;

    #[tokio::test]
    async fn my_test() {
        let db = test_db().await;
        // chaque test obtient sa propre base vierge et migrée
        // ...
    }
}
```

**Règles :**
- `max_connections(1)` est **non-négociable** pour SQLite `:memory:` (chaque connexion = base distincte).
- Utiliser `tempfile::tempdir()` pour un `FsStorage` isolé dans les tests `DeployService`.
- Les tests `#[cfg(test)]` inline (dans `src/`) s'appliquent aux services. `backend/tests/` accueille les tests d'intégration cross-couche (ex. garde d'architecture).

## i18n (rust-i18n) — provider, hook, et usage `t!`

Couche i18n FR+EN via `rust-i18n 3`. Source unique des chaînes : `frontend/locales/{en,fr}.yml`
(format `_version: 1`, clés plates pointées groupées par écran : `login.*`, `list.*`, `detail.*`,
`form.*`, `deploy.*`, `danger.*`, `common.*`, `toast.*`). Embarquées à la compilation par
`rust_i18n::i18n!("locales")` au crate root (`main.rs`, avec `#[macro_use] extern crate rust_i18n;`
→ `t!` disponible crate-wide).

```rust
// frontend/src/i18n.rs — provider réactif (calqué sur AuthProvider)
#[function_component(LocaleProvider)]
pub fn locale_provider(props: &LocaleProviderProps) -> Html {
    let locale = use_state(|| {
        let l = detect_initial();           // localStorage "latch.locale" → navigator.language → En
        rust_i18n::set_locale(l.as_str());  // synchrone, AVANT le 1er rendu (pas de flash)
        l
    });
    let set_locale = { /* set_locale + write localStorage + locale.set(l) */ };
    html! { <ContextProvider<LocaleContext> context={LocaleContext { locale: *locale, set_locale }}>
        { props.children.clone() }</ContextProvider<LocaleContext>> }
}

// Dans CHAQUE composant qui affiche du texte traduit :
let _loc = use_locale();          // ABONNEMENT obligatoire → re-render au switch de langue
// ... { t!("login.submit") }     // t! lit la locale globale (déjà à jour)
// Variables : t!("danger.del_project_title", name = project.name.clone())
// Attribut/aria : aria_label={AttrValue::from(t!("detail.copy_pin_aria").to_string())}
```

**Règles :**
- `use_locale()` en tête de TOUT composant à texte traduit (même inutilisé) — sinon pas de re-render (cf. QUIRKS).
- `t!` renvoie `Cow<'static, str>` → rendu direct par yew ; pour un attribut, `.to_string()` puis `AttrValue::from`.
- Ajouter une clé = l'ajouter dans **les deux** YAML. Ajouter une locale = un nouveau YAML `xx.yml` + `available-locales` dans `Cargo.toml`.

## Toasts (couche maison) — provider + hook

`shadcn-rs` `Toast`/`Sonner` sont déclaratifs sans auto-dismiss → couche maison.

```rust
// frontend/src/toast.rs
#[derive(Clone, PartialEq)]
pub struct ToastHandle { pub push_success: Callback<String>, pub push_error: Callback<String> }
#[hook] pub fn use_toast() -> ToastHandle { use_context::<ToastHandle>().expect("ToastProvider manquant") }
// ToastProvider : Vec<Toast> en use_state, id via use_mut_ref, auto-dismiss gloo_timers::Timeout(4s),
// overlay .toast-stack rendu après les children. make_push(...) -> Callback<String> (pas de Rc<Fn>).
```

Usage : `let toast = use_toast();` puis `toast.push_success.emit(t!("toast.project_created").to_string());`
sur le bras `Ok`, `toast.push_error.emit(e.user_message());` sur `Err`. Provider monté entre
`LocaleProvider` et `AuthProvider` dans `main.rs`.

## Vendoriser un composant shadcn-rs cassé (règle de projet)

`shadcn-rs` est en 0.1 (lib jeune, composants à moitié implémentés). **Quand un composant est cassé
ou bloquant, le vendoriser dans `frontend/src/components/` et le patcher**, plutôt que de le
contourner par des hacks fragiles (remount par `key`, etc.). Réutiliser les classes CSS déjà
vendorisées pour un rendu identique. Précédents : la CSS shadcn-rs (5 fichiers patchés), puis le
`Switch` → `components/toggle.rs` (état contrôlé pur, classe `size-md` load-bearing — cf. QUIRKS).

```rust
// Toggle vendorisé : controlled-only, réutilise .switch/.size-md/.switch-checked/.switch-disabled
let classes = classes!("switch", "size-md",
    checked.then_some("switch-checked"), disabled.then_some("switch-disabled"));
// is_checked = checked  (PAS d'état interne — c'est le bug d'origine du Switch)
```

## Endpoint OpenAPI (utoipa) type

Chaque handler `/api/*` porte un `#[utoipa::path(...)]` (avant `#[debug_handler]`) décrivant
méthode, `path` (préfixe `/api` inclus), `params` (path params typés), `request_body`, et
`responses` (avec `body = <DTO ToSchema>`). Les réponses non-DTO (`{ok:true}`, `{id,n}`) sont
des structs `ToSchema` dédiées dans `crate::dto`, pas des `serde_json::json!`. Le handler est
ajouté à `openapi::ApiDoc` (`paths(...)`), son DTO de réponse à `components(schemas(...))`.
Après tout changement : `UPDATE_OPENAPI=1 cargo test --test openapi_drift` pour régénérer
`openapi.json`. Les handlers annotés sont `pub(crate)` si la macro exige la visibilité.

```rust
// Exemple — backend/src/controllers/admin.rs
#[utoipa::path(
    get,
    path = "/api/projects",
    responses(
        (status = 200, description = "Liste des projets", body = Vec<ProjectListItem>),
        (status = 401, description = "Non authentifié"),
    ),
    security(("admin_session" = []))
)]
#[debug_handler]
pub(crate) async fn list(_auth: AdminAuth, State(ctx): State<AppContext>) -> Result<Response> {
    let svc = ProjectsService::new(ctx.db.clone());
    let items: Vec<ProjectListItem> = svc.list().await.map_err(into_response)?
        .iter().map(ProjectListItem::from_model).collect();
    format::json(items)
}

// backend/src/openapi.rs — enregistrement du handler et du DTO
#[derive(OpenApi)]
#[openapi(
    paths(controllers::admin::list, /* ... */),
    components(schemas(ProjectListItem, ProjectDetail, /* ... */)),
)]
pub struct ApiDoc;
```

**Règles :**
- `#[utoipa::path(...)]` TOUJOURS avant `#[debug_handler]` — l'ordre inverse casse la dérivation macro.
- Les réponses typed utilisent `body = <Type>` avec `<Type>` qui dérive `utoipa::ToSchema` (dans `crate::dto`).
- Les structs de réponse ad-hoc (`OkResponse`, `DeployResponse`, `ActivateResponse`) vivent dans `crate::dto`, pas inline.
- Enregistrer dans `ApiDoc` : `paths(module::handler)` + `components(schemas(Type))` — les deux sont nécessaires.
- Régénérer `openapi.json` : `UPDATE_OPENAPI=1 cargo test --test openapi_drift` (écrit à la racine du workspace).
- Garder les doc-comments des handlers **concis et orientés API** : utoipa les déverse en `description` dans le JSON → fuite dans le client TS généré (Plan 2).

## Accessibilité : `<a onclick>` sans href → `<button class="linkish">`

Une cellule/élément cliquable qui navigue via le router (pas une vraie URL) doit être un
`<button class="linkish" onclick=...>`, pas un `<a onclick style="cursor:pointer">` (lien sans
`href` = non focusable, non actionnable au clavier). La classe `.linkish` (app.css) neutralise le
style bouton et imite un lien. **Garder les vrais `<a href target="_blank">`** (ex. preview de
version) tels quels — seul leur `aria-label` passe par `t!`.
