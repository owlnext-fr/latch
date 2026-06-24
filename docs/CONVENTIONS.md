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
_(à remplir : un handler JSON qui extrait, appelle un service, mappe `CoreError` →
status + JSON, avec la vérif `Origin` sur mutation.)_

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

## Tool MCP type
_(à remplir : un tool qui valide `deploy_token` en premier, puis appelle le service,
puis mappe l'erreur en tool error.)_

## Composant Yew (shadcn-rs) type
_(à remplir : un écran admin type, side-panel + appel API JSON.)_

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
