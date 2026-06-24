pub mod deploy;
pub mod errors;
pub mod pin;
pub mod projects;
pub mod security;
pub mod slug;
pub mod storage;

pub use errors::CoreError;

#[cfg(test)]
pub(crate) mod test_support;

#[cfg(test)]
mod migration_tests {
    use crate::models::_entities::{projects, versions};
    use crate::services::test_support::test_db;
    use sea_orm::{ActiveModelTrait, Set};

    #[tokio::test]
    async fn sessions_table_has_axum_session_schema() {
        use sea_orm::{ConnectionTrait, Statement};
        let db = test_db().await;
        // INSERT au schéma axum-session : id TEXT PK, expires INTEGER NULL, session TEXT NOT NULL.
        let stmt = Statement::from_string(
            db.get_database_backend(),
            "INSERT INTO sessions (id, expires, session) VALUES ('abc', NULL, '{}')".to_string(),
        );
        db.execute(stmt)
            .await
            .expect("insert dans sessions doit réussir");
    }

    #[tokio::test]
    async fn unique_project_n_is_enforced() {
        let db = test_db().await;

        let p = projects::ActiveModel {
            slug: Set("a-aaaaaaaa".to_string()),
            name: Set("A".to_string()),
            code_enabled: Set(false),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();

        versions::ActiveModel {
            project_id: Set(p.id),
            n: Set(1),
            html_path: Set(format!("{}/1.html", p.id)),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();

        let dup = versions::ActiveModel {
            project_id: Set(p.id),
            n: Set(1),
            html_path: Set(format!("{}/1-bis.html", p.id)),
            ..Default::default()
        }
        .insert(&db)
        .await;

        assert!(
            dup.is_err(),
            "UNIQUE(project_id, n) doit rejeter le doublon"
        );
    }
}
