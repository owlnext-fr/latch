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

## Adaptateur MCP type (skeleton complet) — Phase 5

Structure `LatchMcp` : `db`, `storage`, `deploy_token` (String), `public_base_url` (String),
`tool_router` (généré par `#[tool_router]`). Montage dans `after_routes` via
`nest_service("/mcp", StreamableHttpService::new(LocalSessionManager::new(latch_mcp)))`.

```rust
// backend/src/mcp/mod.rs
use rmcp::{ServerHandler, model::ServerInfo, schemars, tool};

pub struct LatchMcp {
    db: DatabaseConnection,
    storage: Arc<dyn Storage>,
    deploy_token: String,
    public_base_url: String,
    tool_router: rmcp::handler::server::tool::ToolRouter<Self>,
}

#[rmcp::tool_router]
impl LatchMcp {
    pub fn new(db: DatabaseConnection, storage: Arc<dyn Storage>,
               deploy_token: String, public_base_url: String) -> Self {
        Self { db, storage, deploy_token, public_base_url, tool_router: Self::tool_router() }
    }

    #[tool(description = "Deploy a prototype HTML file.")]
    async fn deploy_prototype(&self, #[tool(aggr)] params: DeployParams) -> Result<DeployResult, ErrorData> {
        // 1. Gate token EN PREMIER
        if !crate::services::security::secure_compare(&params.deploy_token, &self.deploy_token) {
            return Err(ErrorData::new(rmcp::model::ErrorCode::INVALID_PARAMS, "unauthorized", None));
        }
        // 2. Appeler les services
        let svc_projects = ProjectsService::new(self.db.clone());
        let project = svc_projects.get_by_slug(&params.slug).await
            .map_err(map_core_err)?;
        let svc_deploy = DeployService::new(self.db.clone(), self.storage.clone());
        let version = svc_deploy.deploy(project.id, &params.html, params.activate.unwrap_or(true)).await
            .map_err(map_core_err)?;
        Ok(DeployResult {
            url: format!("{}/c/{}", self.public_base_url, params.slug),
            version: version.n,
            code_protected: project.code_enabled,
        })
    }

    #[tool(description = "List all projects.")]
    async fn list_projects(&self, #[tool(aggr)] params: ListParams) -> Result<ProjectListResult, ErrorData> {
        if !crate::services::security::secure_compare(&params.deploy_token, &self.deploy_token) {
            return Err(ErrorData::new(rmcp::model::ErrorCode::INVALID_PARAMS, "unauthorized", None));
        }
        let projects = ProjectsService::new(self.db.clone()).list().await.map_err(map_core_err)?;
        Ok(ProjectListResult {
            projects: projects.into_iter().map(|p| ProjectSummary { /* ... */ }).collect(),
        })
    }
}

#[rmcp::tool_handler]
impl ServerHandler for LatchMcp {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.server_info.name = "latch".into();
        info.server_info.version = env!("CARGO_PKG_VERSION").into();
        info.with_instructions("Deploy and list prototypes via latch.")
    }
}

fn map_core_err(e: CoreError) -> ErrorData {
    ErrorData::new(rmcp::model::ErrorCode::INTERNAL_ERROR, e.to_string(), None)
}
```

**Règles :**
- Gate `deploy_token` **toujours en premier**, avant toute lecture en base.
- Envelopper les listes dans un struct objet (jamais `Vec<T>` directement — rmcp 1.8 panique si le schéma racine est `array`, cf. QUIRKS).
- `ServerInfo` est `#[non_exhaustive]` en 1.8 → `ServerInfo::default()` + champs + `.with_instructions()` (cf. QUIRKS).
- `map_core_err` : helper privé, une seule définition par module `mcp/`.
- `allowed_hosts` dans `StreamableHttpServerConfig` dérivé de `LATCH_PUBLIC_BASE_URL` via `web::host_authority(base)`.

## Test de handler MCP (niveau handler, sans transport HTTP) — Phase 5

rmcp 1.8 : la macro `#[tool]` produit des méthodes directement `await`-ables (cf. QUIRKS).
Pattern de test : instancier `LatchMcp` avec une DB in-memory et un storage tempdir, appeler
la méthode du handler directement.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::test_support::test_db;

    async fn make_mcp(token: &str) -> LatchMcp {
        let db = test_db().await;
        let storage = Arc::new(crate::services::storage::MemStorage::new()); // ou tempdir FsStorage
        LatchMcp::new(db, storage, token.into(), "http://localhost:5150".into())
    }

    #[tokio::test]
    async fn deploy_rejects_bad_token() {
        let m = make_mcp("good-token").await;
        let result = m.deploy_prototype(DeployParams {
            slug: "demo-abc12345".into(),
            html: "<html></html>".into(),
            deploy_token: "bad-token".into(),
            activate: None,
        }).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, rmcp::model::ErrorCode::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn deploy_unknown_slug_is_error() {
        let m = make_mcp("tok").await;
        let result = m.deploy_prototype(DeployParams {
            slug: "inexistant-abc12345".into(), html: "<html/>".into(),
            deploy_token: "tok".into(), activate: None,
        }).await;
        assert!(result.is_err()); // slug inconnu → CoreError::NotFound → ErrorData
    }
}
```

**Règles :**
- Pas de serveur HTTP → pas de `#[serial]` nécessaire (DB in-memory par test).
- Tester : gate token bad → erreur, gate token ok → succès, slug inconnu → erreur.

## Test e2e MCP via transport Streamable HTTP (harness loco) — Phase 6

Tests d'intégration du transport HTTP réel : le harness loco `request::<App>` démarre l'app entière
(dont `/mcp`) et `axum-test` envoie de vraies requêtes HTTP. `backend/tests/mcp_http.rs`.

```rust
// Helper : environnement + serveur
fn setup_env() {
    std::env::set_var("DEPLOY_TOKEN", "test-token");
    std::env::set_var("LATCH_PUBLIC_BASE_URL", "http://localhost:5150");
    std::env::set_var("LATCH_STORAGE_ROOT", "/tmp/latch-mcp-test");
}

// Helper : POST /mcp avec host explicite (OBLIGATOIRE — cf. QUIRKS)
async fn mcp_post(server: &TestServer, body: serde_json::Value, session: Option<&str>)
    -> axum_test::TestResponse
{
    let mut req = server.post("/mcp")
        .add_header("content-type", "application/json")
        .add_header("accept", "application/json, text/event-stream")
        .add_header("host", "localhost:5150")  // LOAD-BEARING — cf. QUIRKS
        .json(&body);
    if let Some(s) = session {
        req = req.add_header("mcp-session-id", s);
    }
    req.await
}

// Helper : extraire la première ligne JSON d'une réponse SSE (ignore les `data:` vides)
fn parse_mcp_body(body: &str) -> serde_json::Value {
    for line in body.lines() {
        if let Some(payload) = line.strip_prefix("data:") {
            let trimmed = payload.trim();
            if !trimmed.is_empty() {
                return serde_json::from_str(trimmed).expect("JSON valide");
            }
        }
    }
    panic!("Aucune ligne data non-vide dans la réponse SSE");
}

#[tokio::test]
#[serial]
async fn mcp_initialize_handshake() {
    setup_env();
    request::<App, _, _>(|server, _ctx| async move {
        let res = mcp_post(&server, serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": { "protocolVersion": "2025-06-18",
                        "clientInfo": { "name": "test", "version": "0.0.1" },
                        "capabilities": {} }
        }), None).await;
        res.assert_status_ok();
        let session = res.header("mcp-session-id").to_str().unwrap().to_string();
        assert!(!session.is_empty());
        let json = parse_mcp_body(&res.text());
        assert_eq!(json["result"]["protocolVersion"], "2025-06-18");
    }).await;
}
```

**Règles :**
- `#[serial]` obligatoire (l'app entière est démarrée par `request::<App>`, accès DB).
- `host: localhost:5150` **dans chaque requête MCP** : rmcp valide `allowed_hosts` dérivé de `LATCH_PUBLIC_BASE_URL` (cf. QUIRKS).
- `parse_mcp_body` **ignore les lignes `data:` vides** : rmcp 1.8 débute le SSE par un keepalive (cf. QUIRKS).
- `LATCH_STORAGE_ROOT`, `LATCH_PUBLIC_BASE_URL`, `DEPLOY_TOKEN` posés via `set_var` AVANT `request::<App>`.
- Session MCP capturée du header `mcp-session-id` de la réponse `initialize`, transmise dans les requêtes suivantes.
- Vérifier `structuredContent` (pas `content[0].text`) pour les résultats de tool (cf. QUIRKS T2).

## Durcissement en-têtes HTTP global dans `after_routes` — Phase 6

En-têtes de sécurité applicables à toutes les surfaces : posés via `map_response` en **dernier**
dans `after_routes` pour couvrir `/admin`, `/api`, `/c`, `/mcp`, `/robots.txt`.

```rust
// Dans backend/src/app.rs — after_routes (à la fin, après tous les mount)
use axum::middleware::{self, map_response};
use axum::http::HeaderValue;

let router = router.layer(map_response(|mut res: Response| async move {
    res.headers_mut().insert(
        "X-Robots-Tag",
        HeaderValue::from_static("noindex, nofollow"),
    );
    res
}));
```

**Règles :**
- Posé **après** tous les `nest_service` (layer axum s'applique de l'extérieur vers l'intérieur —
  le dernier ajouté enveloppe le tout).
- `map_response` (pas `from_fn`) : modifie la réponse sans court-circuiter la chaîne.
- Test : vérifier la présence du header sur au moins une route de chaque surface (`/admin`,
  `/api`, `/robots.txt`) pour éviter les régressions si l'ordre des layers change.

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

## Adaptateur entrant "serving public" (`controllers/serve.rs`) — Phase 4

Surface **publique** (pas de session admin) : l'auth est le code projet + cookie signé, la
barrière est le rate-limit. Le handler décide **côté serveur** quoi renvoyer, et sert le HTML
stocké en **octets bruts** (jamais du React). Toute réponse `/c` est en `Cache-Control: no-store`.

```rust
// GET /c/{slug} — arbre de décision (contrat §6)
let project = svc.get_by_slug(&slug).await.map_err(into_response)?;   // 404 si inconnu
let Some(active_id) = project.active_version_id else {               // 404 si rien d'actif
    return Err(loco_rs::Error::NotFound);
};
if project.code_enabled {
    let secret = crate::web::unlock_secret(&ctx)?;                   // PAS de ? dans une closure
    let ok = match jar.get(UNLOCK_COOKIE_NAME) {
        Some(c) => verify_token(secret.as_bytes(), &slug, &pin, c.value(), now),
        None => false,
    };
    if !ok { return unlock_page_response().await; }                 // sert unlock.html, HTTP 200
}
// libre OU cookie valide → HTML stocké, no-store
let html = crate::web::storage_from_ctx(&ctx).read(&version.html_path).await.map_err(into_response)?;
Ok(html_response(html))                                             // (headers no-store, body).into_response()
```

**Règles :**
- Vérifier le cookie **avant** de lire le HTML (ne jamais lire/exposer le proto si l'accès échoue).
- Page de déverrouillage = **HTTP 200** (pas 401, contrat §6 — sinon popup natif du navigateur).
- Cookie unlock : `SignedCookieJar::from_headers(&headers, key)` (pas d'extracteur `FromRef`/state) ;
  la valeur signée porte une **empreinte HMAC du PIN courant** (`issue_token`/`verify_token`, cœur pur)
  → rotation du PIN = révocation. Attributs : `HttpOnly`, `Secure`(prod via `cookie_secure(&ctx)`),
  `SameSite=Lax`, `Path=/c/{slug}`, `Max-Age`. Succès = **204** (pas 303 — incompatible avec un client `fetch`).
- Secrets cookie via `web::resolve_cookie_secret(...)` : **fail-secure** (hors Dev/Test, env var obligatoire,
  ≥ 64 octets ; pas de fallback en prod). Vaut pour `UNLOCK_COOKIE_SECRET` ET `SESSION_SECRET`.
- Rate-limit `/unlock` : deux `GovernorLayer` empilés via `tower::ServiceBuilder` (pas `.layer().layer()`
  direct sur le `MethodRouter` → casse l'inférence axum 0.8) ; extracteurs custom `IpSlugKeyExtractor`
  (clé `"{ip}|{slug}"`) + `SlugKeyExtractor` (clé `slug`). In-memory (reset au reboot, assumé).

## `useSettings` — hook Settings + PinField pour secret générique — Phase 5

La page Settings suit le même pattern que les hooks Query : un hook dédié + `PinField` pour
afficher un secret masqué/révéler/copier.

```ts
// frontend/src/hooks/use-settings.ts
import { useQuery } from '@tanstack/react-query'
import { client } from '@/api/client'

export function useSettings() {
  return useQuery({
    queryKey: ['settings'],
    queryFn: async () => {
      const { data, error } = await client.GET('/api/settings')
      if (error) throw error
      return data
    },
  })
}
```

```tsx
// frontend/src/routes/settings.tsx (extrait)
import { useSettings } from '@/hooks/use-settings'
import { PinField } from '@/components/pin-field'

export function SettingsRoute() {
  const { data, isLoading, isError } = useSettings()
  if (isLoading) return <Spinner />
  if (isError || !data) return <ErrorState />
  return (
    <div>
      {/* deploy_token masqué, révéler, copier — même PinField qu'en Detail */}
      <PinField pin={data.deploy_token} />
      {/* mcp_url : texte + CopyButton */}
      <span>{data.mcp_url}</span><CopyButton text={data.mcp_url} />
      {/* public_base_url : texte seul */}
      <span>{data.public_base_url}</span>
    </div>
  )
}
```

**Règles :**
- `PinField` est réutilisable pour tout secret (pas seulement le PIN projet) : il masque, révèle et copie n'importe quelle chaîne. Pas besoin de créer un composant ad hoc pour `deploy_token`.
- La page Settings n'a **pas de mutation** (lecture seule) — invalider uniquement si une opération externe peut changer les valeurs.
- `queryKey: ['settings']` — distinct de `['projects']`.

## Bouton avec état de chargement (`Button loading`) — Phase 4

Le `Button` shadcn porte un prop `loading?: boolean` : spinner `Loader2 animate-spin` injecté +
`disabled` effectif. On câble l'`isPending` de la mutation TanStack Query (jamais d'état local).

```tsx
// Bouton simple
<Button type="submit" loading={deploy.isPending}>{t('deploy.btn')}</Button>

// Agrégat (plusieurs mutations derrière une action)
const saving = createProject.isPending || updateProject.isPending || setCode.isPending || clearCode.isPending
<Button type="submit" loading={saving}>{t('common.save')}</Button>

// Spinner par ligne (mutation partagée) via mutation.variables (TanStack Query v5)
<Button loading={activateVersion.isPending && activateVersion.variables?.n === v.n} ...>
```

**Règles :**
- Label **stable** (pas de swap "…ing" : le spinner suffit).
- `loading` ne s'applique pas si `asChild` (nav links).
- Pour un spinner ciblé sur une ligne avec une mutation partagée : `isPending && variables?.<clé> === <valeur>`.

## Composant Select (radix) + helper-text généralisé (Phase 7 Lot 2)

`components/ui/select.tsx` vendorise le Select radix via le package unifié
(`import { Select as SelectPrimitive } from "radix-ui"`, même style que `ui/sheet.tsx`).
Pattern de réglage dans un panneau : `flex flex-col gap-1.5` → label (`text-sm font-medium`)
+ contrôle + helper text (`text-muted-foreground text-xs`). Pour un sélecteur dépendant des
locales découvertes, mapper sur l'export `locales` de `@/i18n` (jamais de liste en dur).
La CSS d'un asset spécifique-admin (ex. `flag-icons`) s'importe DANS le composant qui
l'utilise (`language-select.tsx`), pas dans `index.css` partagé, pour ne pas alourdir le
bundle public unlock.

## Logo, titres de page, liens externes (Phase 7 Lot 3)

- Logo : composant `components/logo.tsx` (`<img src={logoUrl} alt="latch">`, importe
  `src/assets/latch-logo.svg`), mutualisé admin + unlock, taille par CSS (`size-6` topbar,
  `size-12` login/unlock).
- Favicon : SVG-only, `<link rel="icon" type="image/svg+xml" href="/src/assets/latch-logo.svg">`
  dans index.html ET unlock.html. JAMAIS de fichier favicon à la racine (le backend ne sert que
  `/assets` → 404 ; cf. QUIRKS). Vite réécrit `/src/assets/...` vers `/assets/<hash>`.
- Titres : hook `hooks/use-document-title.ts` appelé par route. Schéma « Page — latch admin »
  (clés i18n `title.*`).
- Liens externes : centralisés dans `lib/links.ts` (`GITHUB_URL`, `DOCS_URL`), rendus via
  `Button asChild` enveloppant un `<a target="_blank" rel="noopener noreferrer">`.

## i18n — locales auto-découvertes (Phase 7 Lot 1)

Ajouter une langue = déposer `src/i18n/locales/{admin,unlock}/<code>.json` avec une
clé `_meta` en tête : `{ "_meta": { "name": "<Nom natif>", "flag": "<ISO pays>" }, ... }`.
Aucun code à toucher : `parseLocales` (`src/i18n/available-locales.ts`) découvre les
fichiers via `import.meta.glob(..., { eager: true })`, strip `_meta`, dérive
`supportedLngs`, et expose `locales: LocaleInfo[]` (lu par le sélecteur de langue).
Le drapeau est un code pays ISO (rendu décidé au Lot 2). Deux dossiers/globs distincts
= séparation garantie admin (106 clés) / unlock (8 clés, bundle public minimal).

## 2ᵉ entrée Vite (page server-rendered découplée) — Phase 4

Une page publique server-rendered (ex. `unlock.html`) qui réutilise le thème/les composants
shadcn **sans embarquer le SPA admin** : 2ᵉ entrée Vite, bundle isolé.

```ts
// vite.config.ts
base: '/',  // PAS '/admin/' — assets en /assets, servis hors /admin (découplage public)
build: { rollupOptions: { input: { main: '…/index.html', unlock: '…/unlock.html' } } }
```

**Règles :**
- `base: '/'` + `nest_service("/assets", ServeDir(dist/assets))` côté backend (assets publics,
  pas sous `/admin`). Le routeur admin garde `basepath: '/admin'` (orthogonal à la base Vite).
- L'entrée dédiée n'importe **aucun** code admin (pas de router/Query/openapi-fetch) — bundle isolé.
- i18n minimal propre à la page (`createInstance()`), pas le catalogue admin complet.
- Tests Vitest avec `<InputOTP>` : mocker `document.elementFromPoint` dans `vitest.setup.ts` (cf. QUIRKS).

## Page d'erreur serving /c (Phase 7 Lot 4)

3ᵉ entrée Vite `error.html` (calquée sur unlock : `src/error/{main,error-page,i18n}.tsx` +
`locales/error/*.json` auto-découverts). Servie par `serve.rs::serve_error_page(status)` qui lit
`web::error_index()` (= `dist/error.html`) et renvoie HTML + `no-store` + status, avec un fallback
texte inline si le fichier manque. Les branches `Err` terminales de `serve` deviennent des
`Ok(serve_error_page(...))` (décision locale à l'adaptateur public ; le renderer Loco global reste
JSON pour admin/MCP). Message **générique unique** (zéro injection, pas de leak d'existence de slug).

## Composant `MarkdownView` restreint — Phase 9

Composant React partagé entre l'aperçu admin (panneau de déploiement) et l'overlay visiteur
(shell). Repose sur `react-markdown` avec deux gardes obligatoires :

```tsx
// frontend/src/components/markdown-view.tsx
import ReactMarkdown from 'react-markdown'

const ALLOWED_ELEMENTS = [
  'p', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6',
  'strong', 'em',
  'ul', 'ol', 'li',
  'blockquote',
]

// frontend/src/lib/markdown.tsx — prop publique : `source`
export function MarkdownView({ source }: { source: string }) {
  return (
    <ReactMarkdown
      skipHtml          // drop les balises HTML brutes dans le markdown
      allowedElements={ALLOWED_ELEMENTS}  // allow-list stricte — liens/images/code bloqués
      unwrapDisallowed  // garde le texte des éléments retirés (ex. libellé d'un lien)
      // components={...} dans l'impl réelle pour styler titres/listes/citation (Tailwind)
    >
      {source}
    </ReactMarkdown>
  )
}
```

**Règles :**
- `skipHtml` + `allowedElements` sont **tous les deux obligatoires**. L'un sans l'autre ne suffit pas.
- La liste `ALLOWED_ELEMENTS` est la **source de vérité** du périmètre markdown documenté dans le
  contrat (§3) et dans Fumadocs. Si elle change, mettre à jour les deux docs.
- Le composant est partagé : toute modification affecte à la fois l'aperçu admin et l'overlay
  visiteur — ce que l'admin voit = ce que le visiteur voit.
- Ne jamais rendre les notes côté serveur. Le champ `notes_md` reçu de `/c/<slug>/notes` est passé
  directement à `MarkdownView`, sans traitement intermédiaire.

## Helper `previewUrl` et panel read-only par version — feat/release-notes-ux

### `previewUrl` (`@/lib/utils`)

Fonction utilitaire qui construit l'URL de preview admin pour une version donnée :

```ts
// frontend/src/lib/utils.ts
export function previewUrl(projectId: number, n: number): string {
  return `/api/projects/${projectId}/versions/${n}/preview`
}
```

Réutiliser ce helper partout où l'on a besoin d'un lien de preview admin (liste projets, liste
versions, panel Détail). La route est derrière `AdminAuth` + `no-store` — l'ouvrir via un vrai
`<a href={previewUrl(...)} target="_blank" rel="noopener noreferrer">` (cohérent avec la règle « garder
les vrais liens » plus haut), pas `window.open`.

### Panel read-only via `<Sheet>` + `MarkdownView`

Pattern pour afficher les détails d'une version (read-only, pas de formulaire) :

```tsx
import { Sheet, SheetContent, SheetHeader, SheetTitle } from '@/components/ui/sheet'
import { Badge } from '@/components/ui/badge'
import { MarkdownView } from '@/lib/markdown'

<Sheet open={open} onOpenChange={(v) => !v && onClose()}>
  <SheetContent>
    <SheetHeader>
      <SheetTitle className="flex items-center gap-2">
        {t('version_detail.title', { n: version.n })}
        {version.is_active && <Badge>{t('common.active')}</Badge>}
      </SheetTitle>
    </SheetHeader>
    <div className="flex flex-col gap-4 p-4">
      <div>
        <p className="text-muted-foreground text-xs">{t('version_detail.date_label')}</p>
        <p className="text-sm">{formatDate(version.created_at)}</p>
      </div>
      <div>
        <p className="text-muted-foreground text-xs">{t('version_detail.notes_label')}</p>
        {version.release_notes
          ? <MarkdownView source={version.release_notes} />
          : <p className="text-muted-foreground text-sm">{t('version_detail.no_notes')}</p>}
      </div>
    </div>
  </SheetContent>
</Sheet>
```

**Règles :**
- Le `<Sheet>` intègre nativement un bouton de fermeture X — ne pas en ajouter un manuellement.
- Le panel est **read-only** : pas de `<form>`, pas de mutation.
- `MarkdownView` rend les notes exactement comme le visiteur les voit (même composant que l'overlay shell).
- Les actions de ligne (Activer/Preview/Supprimer) restent sur la ligne ; le panel Détail est additif.

## Mini-SPA Vite isolée (moule shell) — Phase 9

Le **shell visiteur** (`src/shell/`) suit le même moule que `unlock` et `error` : entrée Vite
dédiée, bundle isolé, instance i18n propre. Moule à reproduire pour toute future page publique.

```
frontend/src/shell/
  main.tsx          # point d'entrée Vite (monte ShellApp dans <div id="root">)
  shell-app.tsx     # composant racine : charge le proto en iframe, gère l'overlay notes
  i18n.ts           # createInstance() propre — glob locales/shell/**
  locales/shell/
    en.json
    fr.json
```

```ts
// src/shell/i18n.ts (même pattern que src/unlock/i18n.ts)
import i18next from 'i18next'
import { initReactI18next } from 'react-i18next'
import type { LocaleInfo } from '@/i18n/available-locales'

const resources = Object.fromEntries(
  Object.entries(import.meta.glob('../i18n/locales/shell/*.json', { eager: true }))
    .map(([path, mod]) => {
      const lang = path.replace('../i18n/locales/shell/', '').replace('.json', '')
      return [lang, { translation: (mod as Record<string, unknown>) }]
    })
)

const i18n = i18next.createInstance()
i18n.use(initReactI18next).init({
  lng: localStorage.getItem('latch.locale') ?? navigator.language.split('-')[0] ?? 'en',
  fallbackLng: 'en',
  resources,
})
export default i18n
```

**Règles :**
- `createInstance()` — ne pas réutiliser l'instance admin globale (isolation bundle).
- N'importer **aucun** code admin (pas de router/Query/openapi-fetch).
- La clé `localStorage['latch:seen:<slug>']` vaut le dernier numéro de version `n` vu (entier ou
  chaîne). Comparer avec `String(n)` pour être robuste aux deux types.

## Service cœur `CommentsService` — soft-delete + owner-check (Plan 1 commentaires, 2026-06-30)
Même moule que les autres services (struct `db`, `new`, méthodes `async -> Result<_, CoreError>`, sans
axum/loco). Spécificités du domaine :
- **Owner-check** : un edit/delete/reply charge la ligne et compare `secure_compare(&row.owner_token, token)` ;
  non-correspondance → `CoreError::NotFound` (pas d'oracle d'existence, pas de fuite).
- **Soft-delete** : `deleted_at = Some(now)` ; toutes les lectures filtrent `deleted_at IS NULL` ; supprimer
  le dernier message vivant d'un pin tombstone le pin (`soft_delete_pin_if_empty`).
- **Pas de N+1** : `list_pins`/`count_comments_by_version` = 2 requêtes (pins, puis `is_in(pin_ids)`) +
  group-by mémoire. Ne JAMAIS reboucler une requête par pin (Sonar/perf).
- **Modération admin** : `moderate_delete_message(project_id, comment_id)` walk message→pin→version→projet
  AVANT toute mutation.

## Adaptateur public commentaires — gate + identité (serve.rs, 2026-06-30)
- `comments_gate(ctx, headers, slug) -> Result<projects::Model, Response>` : 404 (slug inconnu /
  `comments_enabled=false`), 403 (verrouillé). Pattern `Result<_, Response>` (statuts exacts, cf. QUIRKS).
- `comment_write_owner(ctx, headers, slug) -> Result<String, Response>` : gate + `require_owner` (cookie →
  token, sinon 401), partagé par tous les handlers d'écriture.
- Identité : `mint_owner_token()` (ULID), `read_owner_token`/`comment_identity_cookie` réutilisent
  `crate::web::unlock_key` (clé `UNLOCK_COOKIE_SECRET`) — **pas de nouveau secret**. Cookie `latch_comment`
  `HttpOnly`/`Secure`/`SameSite=Lax`/`Path=/c/{slug}`, posé seulement si frais.
- Anti-CSRF des écritures : `require_same_origin` + middleware `require_comment_client` (403 si header
  `X-Comment-Client` absent) + couches Governor `LATCH_COMMENT_RL_*`, bundlées via `ServiceBuilder`.

## DTOs commentaires — `owner_token` jamais sérialisé, `editable` calculé (dto/mod.rs, 2026-06-30)
Les DTOs de réponse n'ont **structurellement pas** de champ `owner_token` (ni public ni admin). Les
conversions `to_comment_pin(pin, msgs, caller)` (public, `editable = m.owner_token == caller`) et
`to_admin_comment_pin` (admin, sans `editable`) ne le copient jamais ; helper privé `message_base_fields`
partagé. Invariant testé sur 3 surfaces (POST + GET visiteur + GET admin) dans `security_invariants.rs`.

## Helpers de lookup admin réutilisables (admin.rs, 2026-06-30)
`find_version(ctx, id, n)` et `find_project(ctx, id)` factorisent les lookups `find_by_id().ok_or(NotFound)`
répétés (detail/update/activate/delete/preview/comments). `project_detail_json(ctx, project, id)` factorise
versions + `count_comments_by_version` + `to_detail`. Extraits notamment pour passer la gate Sonar duplication.

## Module frontend partagé derrière un seam + adaptateur + capabilities (Plan 2 commentaires, 2026-06-30)

Pattern pour une feature front chargée en contexte multiple (visiteur / admin) :

- **Seam `Picker`** : interface `src/comments/picker/picker.ts` avec une seule impl concrète `SameOriginPicker` (`picker/same-origin-picker.ts`). Interne au module ; le consommateur ne voit que `CommentsApp`. Facile à mocker en test (un `FrameRef` factice suffit). Une future impl cross-origin (`PostMessagePicker`) se brancherait sans toucher au reste.
- **Adaptateur de données** (`data/adapter.ts` : interface `CommentsAdapter` + `Capabilities`) : objet passé au composant racine, portant lecture/écritures (`list/createPin/addReply/editMessage/deleteMessage/deletePin`). Pas de couplage au transport. Impl visiteur = `data/visitor-adapter.ts` (`createVisitorAdapter(slug)`).
- **Objet `capabilities`** (`canAuthor / canEditOwn / canModerate`) : détermine ce que l'UI affiche. L'autorisation réelle est au backend — `capabilities` sert uniquement à masquer des affordances non pertinentes (visiteur = `{canAuthor:true, canEditOwn:true, canModerate:false}`).

```ts
// src/comments/index.ts — point d'entrée du import() dynamique (lazy)
export { CommentsApp as default } from './comments-app'
// CommentsApp({ slug, frame }) : crée son propre QueryClient (useMemo) puis monte
// picker + useFollow + overlay + popups + ActionBar, câblés à createVisitorAdapter(slug).
```

Arborescence réelle : `anchor/` (descriptor, describe, similarity, resolve), `picker/`, `follow/` (controller, use-follow), `data/` (adapter, visitor-adapter, use-comments), `state/pick-machine`, `ui/` (overlay-layer, pin-badge, compose-popup, thread-popup, action-bar, use-floating-rect, name-prompt), `comments-app.tsx`, `index.ts`.

## Hooks React Query paramétrés par un `adapter` — clé `commentsKey(slug)` (Plan 2, 2026-06-30)

Les hooks de commentaires (`useCommentList`, `useCreatePin`, `useAddReply`, `useEditMessage`, `useDeleteMessage`, `useDeletePin`) reçoivent `(slug, adapter: CommentsAdapter)` au lieu d'importer directement `openapi-fetch`. Avantage : testable sans MSW (adapter factice), réutilisable en contexte admin. Chaque mutation invalide `commentsKey(slug)` au succès.

```ts
// src/comments/data/use-comments.ts
export function commentsKey(slug: string): unknown[] {
  return ['comments', slug]
}

export function useCommentList(slug: string, adapter: CommentsAdapter) {
  return useQuery({ queryKey: commentsKey(slug), queryFn: () => adapter.list() })
}
```

## Module lazy avec son propre `QueryClient` (React Query confiné au chunk) (Plan 2, 2026-06-30)

Quand un module est chargé en `React.lazy`, il porte son propre `QueryClientProvider` pour confiner le cache. Le bundle shell (`src/shell/main.tsx`) **n'importe pas** React Query — il n'entre que par le `import('@/comments')` dynamique. Vérifié au niveau bundle (revue finale Plan 2).

```tsx
// src/comments/comments-app.tsx
export function CommentsApp({ slug, frame }: Readonly<CommentsAppProps>) {
  const client = useMemo(
    () => new QueryClient({ defaultOptions: { queries: { retry: false } } }),
    [],
  )
  return (
    <QueryClientProvider client={client}>
      <CommentsInner slug={slug} frame={frame} />
    </QueryClientProvider>
  )
}
```

**Règles :**
- Le `QueryClient` du chunk est créé via `useMemo(…, [])` (une fois par montage), pas inline à chaque render.
- Le shell ne voit jamais le cache commentaires.
- Le chargement est déclenché par `src/shell/comments-mount.tsx` (`lazy(() => import('@/comments'))` sous `Suspense`), monté seulement si `PublicMeta.comments_enabled` et l'iframe présente ; un `key`-bump sur l'event `load` de l'iframe reconstruit l'app (nouvelle scène proto).

## En-tête `X-Comment-Client` sur les writes commentaire (Plan 2, 2026-06-30)

Tous les appels d'écriture de commentaire (POST pin, POST reply, PUT edit, DELETE) doivent porter l'en-tête `X-Comment-Client: '1'`. Sans lui, le backend renvoie 403.

`openapi-fetch` ne supporte que les en-têtes définis dans le schéma OpenAPI. Pour un en-tête hors-spec, caster le type :

```ts
client.POST('/c/{slug}/comments', {
  params: { path: { slug } },
  headers: { 'X-Comment-Client': '1' } as Record<string, string>,
  body: { ... },
})
```

## `CommentsApp` — adaptateur injecté (Plan 3, 2026-06-30)

`CommentsApp` reçoit trois props : `cacheKey` (clé React Query opaque), `frame` (ref iframe), `adapter` (objet implémentant `CommentsAdapter`).

- **Visiteur** : `createVisitorAdapter(slug)` + `cacheKey=slug`.
- **Admin** : `createAdminAdapter(projectId, n)` + `cacheKey=\`admin:${id}:${n}\``.

```tsx
// shell visiteur (src/shell/comments-mount.tsx)
<CommentsApp cacheKey={slug} frame={frameRef} adapter={createVisitorAdapter(slug)} />

// Review admin (src/routes/review.tsx)
<CommentsApp cacheKey={`admin:${id}:${n}`} frame={frameRef} adapter={createAdminAdapter(id, n)} />
```

`createAdminAdapter` mappe `AdminCommentMessage`→`CommentMessage` avec `editable:false` (l'admin ne possède pas les commentaires des visiteurs) ; `canModerate:true` seul. Les méthodes `createPin`/`addReply`/`editMessage`/`deletePin` lèvent une erreur (l'admin ne peut qu'observer et modérer).

## i18n d'un module partagé entre bundles (Plan 3, 2026-06-30)

Quand un module React est partagé entre plusieurs bundles Vite (ex. `src/comments/` utilisé dans le shell et dans le bundle admin), ses clés i18n vivent dans une source unique `src/i18n/locales/comments/{en,fr}.json`.

**Fusion côté admin (singleton)** — dans `src/i18n/index.ts` :
```ts
import { mergeFragmentGlob } from './merge-fragment-glob'
const commentResources = import.meta.glob('./locales/comments/*.json', { eager: true })
// après init i18next :
mergeFragmentGlob(i18n, commentResources)
```

**Fusion côté shell (multi-instance)** — via `createBundleI18n` :
```ts
const shellResources = import.meta.glob('./locales/shell/*.json', { eager: true })
const commentResources = import.meta.glob('../i18n/locales/comments/*.json', { eager: true })
createBundleI18n(shellResources, commentResources)
```

**Règle** : tout nouveau consommateur du module partagé DOIT fusionner le glob `locales/comments/`. Oublier cette étape → clés affichées en texte brut (cf. QUIRKS "RÉSOLU par L2").
