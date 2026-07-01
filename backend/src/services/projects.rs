//! Service projets — cœur métier (contrat §1, agnostique HTTP). Suppose
//! l'appelant déjà autorisé : aucune notion de session/token/cookie ici.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};

use std::collections::HashMap;

use crate::models::_entities::{projects, versions};
use crate::services::errors::CoreError;
use crate::services::{pin, security, slug};

/// Entrée de création d'un projet.
#[derive(Debug, Clone)]
pub struct CreateProject {
    pub name: String,
    pub brand_name: Option<String>,
    pub code_enabled: bool,
    /// PIN explicite ; si `None` et `code_enabled`, un PIN est auto-généré.
    pub pin: Option<String>,
    /// Active les commentaires sur le projet (défaut sécurité-aware posé par l'appelant).
    pub comments_enabled: bool,
}

pub struct ProjectsService {
    db: DatabaseConnection,
}

impl ProjectsService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn create(&self, input: CreateProject) -> Result<projects::Model, CoreError> {
        if input.name.trim().is_empty() {
            return Err(CoreError::Validation("name is required".to_string()));
        }

        let pin_value = if input.code_enabled {
            let p = input.pin.unwrap_or_else(pin::generate_pin);
            if !pin::is_valid_pin(&p) {
                return Err(CoreError::Validation("pin must be 6 digits".to_string()));
            }
            Some(p)
        } else {
            None
        };

        let model = projects::ActiveModel {
            slug: Set(slug::generate_slug(&input.name)),
            name: Set(input.name),
            code_enabled: Set(input.code_enabled),
            pin: Set(pin_value),
            brand_name: Set(input.brand_name),
            comments_enabled: Set(input.comments_enabled),
            ..Default::default()
        }
        .insert(&self.db)
        .await?;

        Ok(model)
    }

    pub async fn list(&self) -> Result<Vec<projects::Model>, CoreError> {
        Ok(projects::Entity::find()
            .order_by_desc(projects::Column::Id)
            .all(&self.db)
            .await?)
    }

    /// Projets + leurs versions (pour enrichir la liste avec `active_version_n` + `version_count`).
    /// Deux requêtes (projets, versions) regroupées en mémoire — pas de N+1.
    pub async fn list_with_versions(
        &self,
    ) -> Result<Vec<(projects::Model, Vec<versions::Model>)>, CoreError> {
        let projects = projects::Entity::find()
            .order_by_desc(projects::Column::Id)
            .all(&self.db)
            .await?;
        let all_versions = versions::Entity::find().all(&self.db).await?;
        let mut by_project: HashMap<i32, Vec<versions::Model>> = HashMap::new();
        for v in all_versions {
            by_project.entry(v.project_id).or_default().push(v);
        }
        Ok(projects
            .into_iter()
            .map(|p| {
                let vers = by_project.remove(&p.id).unwrap_or_default();
                (p, vers)
            })
            .collect())
    }

    pub async fn get_by_slug(&self, slug: &str) -> Result<projects::Model, CoreError> {
        projects::Entity::find()
            .filter(projects::Column::Slug.eq(slug))
            .one(&self.db)
            .await?
            .ok_or(CoreError::NotFound)
    }

    /// Version d'un projet par numéro `n`. `NotFound` si absente.
    pub async fn get_version(&self, project_id: i32, n: i32) -> Result<versions::Model, CoreError> {
        versions::Entity::find()
            .filter(versions::Column::ProjectId.eq(project_id))
            .filter(versions::Column::N.eq(n))
            .one(&self.db)
            .await?
            .ok_or(CoreError::NotFound)
    }

    /// Version active d'un projet via `active_version_id`. `NotFound` si aucun pointeur
    /// ou si le pointeur référence une version disparue.
    pub async fn get_active_version(
        &self,
        project: &projects::Model,
    ) -> Result<versions::Model, CoreError> {
        let Some(active_id) = project.active_version_id else {
            return Err(CoreError::NotFound);
        };
        versions::Entity::find_by_id(active_id)
            .one(&self.db)
            .await?
            .ok_or(CoreError::NotFound)
    }

    pub async fn set_code(&self, id: i32, pin: &str) -> Result<projects::Model, CoreError> {
        if !pin::is_valid_pin(pin) {
            return Err(CoreError::Validation("pin must be 6 digits".to_string()));
        }
        let mut m: projects::ActiveModel = self.get_by_id(id).await?.into();
        m.code_enabled = Set(true);
        m.pin = Set(Some(pin.to_string()));
        m.updated_at = Set(chrono::Utc::now().into());
        Ok(m.update(&self.db).await?)
    }

    pub async fn clear_code(&self, id: i32) -> Result<projects::Model, CoreError> {
        let mut m: projects::ActiveModel = self.get_by_id(id).await?.into();
        m.code_enabled = Set(false);
        m.pin = Set(None);
        m.updated_at = Set(chrono::Utc::now().into());
        Ok(m.update(&self.db).await?)
    }

    pub async fn verify_code(&self, slug: &str, provided: &str) -> Result<bool, CoreError> {
        let project = self.get_by_slug(slug).await?;
        if !project.code_enabled {
            return Ok(true);
        }
        match project.pin {
            Some(pin) => Ok(security::secure_compare(&pin, provided)),
            None => Ok(false),
        }
    }

    async fn get_by_id(&self, id: i32) -> Result<projects::Model, CoreError> {
        projects::Entity::find_by_id(id)
            .one(&self.db)
            .await?
            .ok_or(CoreError::NotFound)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::services::test_support::test_db;

    fn svc(db: sea_orm::DatabaseConnection) -> ProjectsService {
        ProjectsService::new(db)
    }

    #[tokio::test]
    async fn create_defaults_to_code_enabled_with_generated_pin() {
        let s = svc(test_db().await);
        let p = s
            .create(CreateProject {
                name: "Mon Projet".to_string(),
                brand_name: None,
                code_enabled: true,
                pin: None,
                comments_enabled: false,
            })
            .await
            .unwrap();

        assert!(p.code_enabled);
        let pin = p.pin.expect("pin généré");
        assert_eq!(pin.len(), 6);
        assert!(p.slug.starts_with("mon-projet-"));
        assert!(p.active_version_id.is_none());
    }

    #[tokio::test]
    async fn create_without_code_has_no_pin() {
        let s = svc(test_db().await);
        let p = s
            .create(CreateProject {
                name: "Libre".to_string(),
                brand_name: Some("ACME".to_string()),
                code_enabled: false,
                pin: None,
                comments_enabled: false,
            })
            .await
            .unwrap();
        assert!(!p.code_enabled);
        assert!(p.pin.is_none());
        assert_eq!(p.brand_name.as_deref(), Some("ACME"));
    }

    #[tokio::test]
    async fn create_rejects_empty_name() {
        let s = svc(test_db().await);
        let err = s
            .create(CreateProject {
                name: "   ".to_string(),
                brand_name: None,
                code_enabled: true,
                pin: None,
                comments_enabled: false,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation(_)));
    }

    #[tokio::test]
    async fn get_by_slug_missing_is_not_found() {
        let s = svc(test_db().await);
        let err = s.get_by_slug("nope-xxxxxxxx").await.unwrap_err();
        assert!(matches!(err, CoreError::NotFound));
    }

    #[tokio::test]
    async fn set_and_clear_code() {
        let s = svc(test_db().await);
        let p = s
            .create(CreateProject {
                name: "P".to_string(),
                brand_name: None,
                code_enabled: false,
                pin: None,
                comments_enabled: false,
            })
            .await
            .unwrap();

        let p = s.set_code(p.id, "424242").await.unwrap();
        assert!(p.code_enabled);
        assert_eq!(p.pin.as_deref(), Some("424242"));

        let p = s.clear_code(p.id).await.unwrap();
        assert!(!p.code_enabled);
        assert!(p.pin.is_none());
    }

    #[tokio::test]
    async fn set_code_rejects_bad_pin() {
        let s = svc(test_db().await);
        let p = s
            .create(CreateProject {
                name: "P".to_string(),
                brand_name: None,
                code_enabled: false,
                pin: None,
                comments_enabled: false,
            })
            .await
            .unwrap();
        let err = s.set_code(p.id, "12ab").await.unwrap_err();
        assert!(matches!(err, CoreError::Validation(_)));
    }

    #[tokio::test]
    async fn verify_code_matches_and_rejects() {
        let s = svc(test_db().await);
        let p = s
            .create(CreateProject {
                name: "P".to_string(),
                brand_name: None,
                code_enabled: true,
                pin: Some("135790".to_string()),
                comments_enabled: false,
            })
            .await
            .unwrap();

        assert!(s.verify_code(&p.slug, "135790").await.unwrap());
        assert!(!s.verify_code(&p.slug, "000000").await.unwrap());
    }

    #[tokio::test]
    async fn verify_code_open_project_always_true() {
        let s = svc(test_db().await);
        let p = s
            .create(CreateProject {
                name: "Open".to_string(),
                brand_name: None,
                code_enabled: false,
                pin: None,
                comments_enabled: false,
            })
            .await
            .unwrap();
        assert!(s.verify_code(&p.slug, "whatever").await.unwrap());
    }

    #[tokio::test]
    async fn create_stores_comments_enabled() {
        let s = svc(test_db().await);
        let p = s
            .create(CreateProject {
                name: "P".to_string(),
                brand_name: None,
                code_enabled: true,
                pin: Some("424242".to_string()),
                comments_enabled: true,
            })
            .await
            .unwrap();
        assert!(p.comments_enabled);
    }

    #[tokio::test]
    async fn get_version_by_n_and_missing() {
        let db = test_db().await;
        let dir = tempfile::tempdir().unwrap();
        let storage: std::sync::Arc<dyn crate::services::storage::Storage> = std::sync::Arc::new(
            crate::services::storage::FsStorage::new(dir.path().to_path_buf()),
        );
        let svc = ProjectsService::new(db.clone());
        let p = svc
            .create(CreateProject {
                name: "P".to_string(),
                brand_name: None,
                code_enabled: false,
                pin: None,
                comments_enabled: false,
            })
            .await
            .unwrap();
        crate::services::deploy::DeployService::new(db.clone(), storage)
            .deploy(p.id, "<h1>v1</h1>", true, None)
            .await
            .unwrap();

        let v = svc.get_version(p.id, 1).await.unwrap();
        assert_eq!(v.n, 1);
        assert!(matches!(
            svc.get_version(p.id, 99).await,
            Err(CoreError::NotFound)
        ));
    }

    #[tokio::test]
    async fn get_active_version_via_pointer_and_none() {
        let db = test_db().await;
        let dir = tempfile::tempdir().unwrap();
        let storage: std::sync::Arc<dyn crate::services::storage::Storage> = std::sync::Arc::new(
            crate::services::storage::FsStorage::new(dir.path().to_path_buf()),
        );
        let svc = ProjectsService::new(db.clone());
        let p = svc
            .create(CreateProject {
                name: "P".to_string(),
                brand_name: None,
                code_enabled: false,
                pin: None,
                comments_enabled: false,
            })
            .await
            .unwrap();

        // Aucune version déployée → NotFound.
        assert!(matches!(
            svc.get_active_version(&p).await,
            Err(CoreError::NotFound)
        ));

        crate::services::deploy::DeployService::new(db.clone(), storage)
            .deploy(p.id, "<h1>v1</h1>", true, None)
            .await
            .unwrap();
        // Recharger le projet pour avoir active_version_id à jour.
        let p = svc.get_by_slug(&p.slug).await.unwrap();
        let v = svc.get_active_version(&p).await.unwrap();
        assert_eq!(v.n, 1);
    }
}
