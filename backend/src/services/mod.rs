pub mod errors;
pub mod pin;
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
