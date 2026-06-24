//! Helpers de test du cœur : une base SQLite **in-memory** isolée par test,
//! migrée via `Migrator`. Jamais le disque de prod (ROADMAP Phase 1).

use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};

/// Connexion SQLite in-memory migrée. `max_connections(1)` est **load-bearing** :
/// chaque connexion `sqlite::memory:` a sa propre base ; avec un pool > 1, les
/// requêtes taperaient des bases vides différentes (QUIRK).
pub(crate) async fn test_db() -> DatabaseConnection {
    let mut opt = ConnectOptions::new("sqlite::memory:");
    opt.max_connections(1);
    let db = Database::connect(opt)
        .await
        .expect("connect in-memory sqlite");
    Migrator::up(&db, None).await.expect("run migrations");
    db
}
