pub mod comments;
pub mod deploy;
pub mod errors;
pub mod pin;
pub mod projects;
pub mod security;
pub mod slug;
pub mod storage;
pub mod unlock_cookie;
pub mod validation;

pub use errors::CoreError;

#[cfg(test)]
pub(crate) mod test_support;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod migration_tests {
    use crate::models::_entities::{comment_pins, comments, projects, versions};
    use crate::services::test_support::test_db;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

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

    #[tokio::test]
    async fn comment_tables_accept_inserts_and_cascade() {
        let db = test_db().await;

        let p = projects::ActiveModel {
            slug: Set("demo-aaaaaaaa".to_string()),
            name: Set("Demo".to_string()),
            code_enabled: Set(false),
            comments_enabled: Set(true),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();

        let v = versions::ActiveModel {
            project_id: Set(p.id),
            n: Set(1),
            html_path: Set(format!("{}/1.html", p.id)),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();

        let pin = comment_pins::ActiveModel {
            version_id: Set(v.id),
            owner_token: Set("01OWNERTOKEN0000000000000".to_string()),
            anchor: Set(r#"{"v":1,"selector":"button"}"#.to_string()),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        assert_eq!(pin.status, "open", "status défaut = open");
        assert!(pin.deleted_at.is_none());

        comments::ActiveModel {
            pin_id: Set(pin.id),
            owner_token: Set("01OWNERTOKEN0000000000000".to_string()),
            author_name: Set("Léa".to_string()),
            body: Set("Top".to_string()),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();

        let count = comments::Entity::find().all(&db).await.unwrap().len();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn projects_have_comments_enabled_column() {
        let db = test_db().await;
        let p = projects::ActiveModel {
            slug: Set("demo-bbbbbbbb".to_string()),
            name: Set("Demo".to_string()),
            code_enabled: Set(true),
            comments_enabled: Set(true),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        assert!(p.comments_enabled);
    }
}
