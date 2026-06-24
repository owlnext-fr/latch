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
