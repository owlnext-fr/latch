//! Service de déploiement — cœur (contrat §1, §8). Même point d'entrée pour
//! l'admin (upload manuel) et le tool MCP. Ordre **imposé** : écrire le HTML
//! d'abord (atomique), puis la transaction DB (insert version + flip pointeur).
//! Un échec DB après écriture ne laisse qu'un fichier orphelin inoffensif ;
//! l'ordre inverse laisserait un pointeur actif vers un fichier absent.

use std::sync::Arc;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};

use crate::models::_entities::{projects, versions};
use crate::services::errors::CoreError;
use crate::services::storage::Storage;

pub struct DeployService {
    db: DatabaseConnection,
    storage: Arc<dyn Storage>,
}

impl DeployService {
    pub fn new(db: DatabaseConnection, storage: Arc<dyn Storage>) -> Self {
        Self { db, storage }
    }

    /// Déploie `html` comme nouvelle version de `project_id`. Si `activate`,
    /// repointe `projects.active_version_id`. Renvoie la version créée.
    pub async fn deploy(
        &self,
        project_id: i32,
        html: &str,
        activate: bool,
        release_notes: Option<&str>,
    ) -> Result<versions::Model, CoreError> {
        // `html`/`release_notes` : forme déjà validée à la frontière (contrat §1,
        // `ValidatedJson<DeployReq>` côté admin, `args.validate()` côté MCP).

        // 1. n = max(n)+1 pour ce projet (hors transaction ; UNIQUE(project_id,n)
        //    est le backstop si deux deploys concurrents calculaient le même n).
        let last = versions::Entity::find()
            .filter(versions::Column::ProjectId.eq(project_id))
            .order_by_desc(versions::Column::N)
            .one(&self.db)
            .await?;
        let n = last.map_or(1, |v| v.n + 1);
        let html_path = format!("{project_id}/{n}.html");

        // 2. Écrire le HTML d'abord (atomique).
        self.storage.write(&html_path, html.as_bytes()).await?;

        // 3. Transaction : insérer la version, puis flipper le pointeur si demandé.
        let txn = self.db.begin().await?;

        let inserted = versions::ActiveModel {
            project_id: Set(project_id),
            n: Set(n),
            html_path: Set(html_path),
            release_notes: Set(release_notes.map(str::to_string)),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        if activate {
            let mut project: projects::ActiveModel = projects::Entity::find_by_id(project_id)
                .one(&txn)
                .await?
                .ok_or(CoreError::NotFound)?
                .into();
            project.active_version_id = Set(Some(inserted.id));
            project.updated_at = Set(chrono::Utc::now().into());
            project.update(&txn).await?;
        }

        txn.commit().await?;
        Ok(inserted)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use std::sync::Arc;

    use sea_orm::EntityTrait;
    use tempfile::TempDir;

    use super::*;
    use crate::models::_entities::projects;
    use crate::services::projects::{CreateProject, ProjectsService};
    use crate::services::storage::FsStorage;
    use crate::services::test_support::test_db;

    async fn make_project(db: &sea_orm::DatabaseConnection) -> projects::Model {
        ProjectsService::new(db.clone())
            .create(CreateProject {
                name: "P".to_string(),
                brand_name: None,
                code_enabled: false,
                pin: None,
                comments_enabled: false,
            })
            .await
            .unwrap()
    }

    fn storage(dir: &TempDir) -> Arc<dyn Storage> {
        Arc::new(FsStorage::new(dir.path().to_path_buf()))
    }

    #[tokio::test]
    async fn first_deploy_is_version_one_and_writes_html() {
        let db = test_db().await;
        let dir = tempfile::tempdir().unwrap();
        let p = make_project(&db).await;

        let svc = DeployService::new(db.clone(), storage(&dir));
        let v = svc.deploy(p.id, "<h1>hi</h1>", true, None).await.unwrap();

        assert_eq!(v.n, 1);
        assert_eq!(v.project_id, p.id);
        // HTML écrit dans le storage
        let written = std::fs::read_to_string(dir.path().join(&v.html_path)).unwrap();
        assert_eq!(written, "<h1>hi</h1>");
        // pointeur flippé
        let p = projects::Entity::find_by_id(p.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(p.active_version_id, Some(v.id));
    }

    #[tokio::test]
    async fn second_deploy_increments_n() {
        let db = test_db().await;
        let dir = tempfile::tempdir().unwrap();
        let p = make_project(&db).await;
        let svc = DeployService::new(db.clone(), storage(&dir));

        let v1 = svc.deploy(p.id, "a", true, None).await.unwrap();
        let v2 = svc.deploy(p.id, "b", true, None).await.unwrap();
        assert_eq!(v1.n, 1);
        assert_eq!(v2.n, 2);
    }

    #[tokio::test]
    async fn deploy_without_activate_leaves_pointer() {
        let db = test_db().await;
        let dir = tempfile::tempdir().unwrap();
        let p = make_project(&db).await;
        let svc = DeployService::new(db.clone(), storage(&dir));

        let v = svc.deploy(p.id, "x", false, None).await.unwrap();
        let p = projects::Entity::find_by_id(p.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert!(p.active_version_id.is_none());
        assert_eq!(v.n, 1);
    }

    #[tokio::test]
    async fn deploy_persists_release_notes() {
        let db = test_db().await;
        let dir = tempfile::tempdir().unwrap();
        let p = make_project(&db).await;
        let svc = DeployService::new(db.clone(), storage(&dir));

        let v = svc
            .deploy(p.id, "<h1>hi</h1>", true, Some("# Salut\n\n- a\n- b"))
            .await
            .unwrap();
        assert_eq!(v.release_notes.as_deref(), Some("# Salut\n\n- a\n- b"));
    }
}
