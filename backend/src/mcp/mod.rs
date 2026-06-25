//! Adaptateur entrant "MCP" — fin (contrat §1, §5). Chaque tool valide le
//! `deploy_token` (services::security::secure_compare) AVANT d'appeler un service
//! du cœur. Aucune logique métier ici ; le cœur reste agnostique HTTP/MCP.

use std::sync::Arc;

use loco_rs::app::AppContext;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::{ErrorData, ServerCapabilities, ServerInfo};
use rmcp::schemars;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::tower::StreamableHttpServerConfig;
use rmcp::transport::streamable_http_server::StreamableHttpService;
use rmcp::{tool, tool_handler, tool_router, ServerHandler};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};

use crate::services::deploy::DeployService;
use crate::services::errors::CoreError;
use crate::services::projects::ProjectsService;
use crate::services::security::secure_compare;
use crate::services::storage::Storage;

/// Serveur d'outils MCP. Porte des dépendances concrètes (pas l'`AppContext`) :
/// l'auth/config est résolue au montage (`service`), le serveur reste testable
/// sans booter Loco.
#[derive(Clone)]
pub struct LatchMcp {
    db: DatabaseConnection,
    storage: Arc<dyn Storage>,
    deploy_token: String,
    public_base_url: String,
    // Lu par le code généré de `#[tool_handler]` (câblage `call_tool`/`list_tools`).
    #[allow(dead_code)]
    tool_router: ToolRouter<LatchMcp>,
}

/// Arguments du tool `deploy_prototype`.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct DeployArgs {
    /// Slug public du projet (doit déjà exister, créé via /admin).
    slug: String,
    /// HTML mono-fichier complet du prototype.
    html: String,
    /// Secret de déploiement (validé contre DEPLOY_TOKEN).
    deploy_token: String,
    /// Activer immédiatement la version déployée (défaut : true).
    #[serde(default)]
    activate: Option<bool>,
}

/// Résultat renvoyé par `deploy_prototype`.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct DeployResult {
    /// URL publique stable du prototype (sert toujours la version active).
    pub url: String,
    /// Numéro de la version créée.
    pub version: i32,
    /// `true` si le projet exige un code d'accès (un PIN sera demandé au visiteur).
    pub code_protected: bool,
}

#[tool_router]
impl LatchMcp {
    pub fn new(
        db: DatabaseConnection,
        storage: Arc<dyn Storage>,
        deploy_token: String,
        public_base_url: String,
    ) -> Self {
        Self {
            db,
            storage,
            deploy_token,
            public_base_url,
            tool_router: Self::tool_router(),
        }
    }

    /// Valide le `deploy_token` à temps constant (contrat §9.3). Avant tout appel au cœur.
    fn check_token(&self, provided: &str) -> Result<(), ErrorData> {
        if secure_compare(provided, &self.deploy_token) {
            Ok(())
        } else {
            Err(ErrorData::invalid_params("deploy_token invalide", None))
        }
    }

    #[tool(
        description = "Déploie un prototype HTML mono-fichier comme nouvelle version d'un \
                       projet EXISTANT (identifié par son slug). Le projet doit avoir été créé \
                       au préalable dans l'admin. Active la version par défaut."
    )]
    async fn deploy_prototype(
        &self,
        Parameters(args): Parameters<DeployArgs>,
    ) -> Result<Json<DeployResult>, ErrorData> {
        self.check_token(&args.deploy_token)?;

        let projects = ProjectsService::new(self.db.clone());
        let project = projects
            .get_by_slug(&args.slug)
            .await
            .map_err(map_core_err)?;

        let activate = args.activate.unwrap_or(true);
        let deploy = DeployService::new(self.db.clone(), self.storage.clone());
        let version = deploy
            .deploy(project.id, &args.html, activate)
            .await
            .map_err(map_core_err)?;

        Ok(Json(DeployResult {
            url: format!("{}/c/{}", self.public_base_url, project.slug),
            version: version.n,
            code_protected: project.code_enabled,
        }))
    }
}

#[tool_handler]
impl ServerHandler for LatchMcp {
    fn get_info(&self) -> ServerInfo {
        // `ServerInfo` (= `InitializeResult`) est `#[non_exhaustive]` (rmcp 1.8) :
        // construction interdite hors crate, on part du `Default` puis on règle
        // capabilities + instructions via les setters/champs publics.
        let mut info = ServerInfo::default();
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info.with_instructions(
            "latch — déploiement de prototypes HTML. Outils : deploy_prototype, \
             list_projects. Chaque appel exige le deploy_token.",
        )
    }
}

/// Mappe une erreur du cœur vers une erreur de tool MCP (jamais de fuite de détail DB/IO).
fn map_core_err(e: CoreError) -> ErrorData {
    match e {
        CoreError::NotFound => ErrorData::invalid_params("projet inconnu", None),
        CoreError::Validation(msg) => ErrorData::invalid_params(msg, None),
        CoreError::Db(_) | CoreError::Io(_) => ErrorData::internal_error("erreur interne", None),
    }
}

/// Construit le service HTTP MCP montable dans axum (`nest_service("/mcp", …)`).
/// `allowed_hosts` dérivé de `LATCH_PUBLIC_BASE_URL` (source unique, défense Host-header).
pub fn service(
    ctx: &AppContext,
) -> loco_rs::Result<StreamableHttpService<LatchMcp, LocalSessionManager>> {
    let db = ctx.db.clone();
    let storage = crate::web::storage_from_ctx(ctx);
    let token = crate::web::deploy_token(ctx)?;
    let base = crate::web::public_base_url(ctx)?;
    let host = crate::web::host_authority(&base);

    let config = StreamableHttpServerConfig::default().with_allowed_hosts([host]);

    let factory = move || {
        Ok(LatchMcp::new(
            db.clone(),
            storage.clone(),
            token.clone(),
            base.clone(),
        ))
    };

    Ok(StreamableHttpService::new(
        factory,
        Arc::new(LocalSessionManager::default()),
        config,
    ))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use std::sync::Arc;

    use sea_orm::EntityTrait;
    use tempfile::TempDir;

    use super::*;
    use crate::services::projects::{CreateProject, ProjectsService};
    use crate::services::storage::FsStorage;
    use crate::services::test_support::test_db;

    const TOKEN: &str = "test-deploy-token";

    fn mcp(db: DatabaseConnection, dir: &TempDir) -> LatchMcp {
        let storage: Arc<dyn Storage> = Arc::new(FsStorage::new(dir.path().to_path_buf()));
        LatchMcp::new(
            db,
            storage,
            TOKEN.to_string(),
            "https://demo.test".to_string(),
        )
    }

    async fn make_project(
        db: &DatabaseConnection,
        code: bool,
    ) -> crate::models::_entities::projects::Model {
        ProjectsService::new(db.clone())
            .create(CreateProject {
                name: "Mon Projet".to_string(),
                brand_name: None,
                code_enabled: code,
                pin: if code {
                    Some("123456".to_string())
                } else {
                    None
                },
            })
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn deploy_rejects_bad_token() {
        let db = test_db().await;
        let dir = tempfile::tempdir().unwrap();
        let p = make_project(&db, false).await;
        let m = mcp(db, &dir);
        let res = m
            .deploy_prototype(Parameters(DeployArgs {
                slug: p.slug.clone(),
                html: "<h1>x</h1>".to_string(),
                deploy_token: "WRONG".to_string(),
                activate: None,
            }))
            .await;
        // Le rejet doit venir du gate token (vérifié AVANT tout appel DB), pas
        // d'un chemin DB : on assert le message exact pour exclure toute fuite
        // d'existence via timing/erreur (contrat §9.3). `Json<DeployResult>`
        // n'implémente pas `Debug`, donc on `match` plutôt que `unwrap_err`.
        let err = match res {
            Err(e) => e,
            Ok(_) => panic!("token invalide doit être rejeté"),
        };
        assert_eq!(
            err.message, "deploy_token invalide",
            "le rejet doit venir du gate token, pas d'un chemin DB"
        );
    }

    #[tokio::test]
    async fn deploy_unknown_slug_is_error() {
        let db = test_db().await;
        let dir = tempfile::tempdir().unwrap();
        let m = mcp(db, &dir);
        let res = m
            .deploy_prototype(Parameters(DeployArgs {
                slug: "nope-xxxxxxxx".to_string(),
                html: "<h1>x</h1>".to_string(),
                deploy_token: TOKEN.to_string(),
                activate: None,
            }))
            .await;
        assert!(
            res.is_err(),
            "slug inconnu doit être une erreur (pas d'auto-création)"
        );
    }

    #[tokio::test]
    async fn deploy_creates_version_and_activates_by_default() {
        let db = test_db().await;
        let dir = tempfile::tempdir().unwrap();
        let p = make_project(&db, true).await;
        let m = mcp(db.clone(), &dir);
        let Json(out) = m
            .deploy_prototype(Parameters(DeployArgs {
                slug: p.slug.clone(),
                html: "<h1>hi</h1>".to_string(),
                deploy_token: TOKEN.to_string(),
                activate: None, // défaut = true
            }))
            .await
            .unwrap();

        assert_eq!(out.version, 1);
        assert!(out.code_protected);
        assert_eq!(out.url, format!("https://demo.test/c/{}", p.slug));
        // pointeur flippé
        let reloaded = crate::models::_entities::projects::Entity::find_by_id(p.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert!(
            reloaded.active_version_id.is_some(),
            "activate défaut → pointeur posé"
        );
    }

    #[tokio::test]
    async fn deploy_without_activate_leaves_pointer_null() {
        let db = test_db().await;
        let dir = tempfile::tempdir().unwrap();
        let p = make_project(&db, false).await;
        let m = mcp(db.clone(), &dir);
        let Json(out) = m
            .deploy_prototype(Parameters(DeployArgs {
                slug: p.slug.clone(),
                html: "<h1>draft</h1>".to_string(),
                deploy_token: TOKEN.to_string(),
                activate: Some(false),
            }))
            .await
            .unwrap();
        assert_eq!(out.version, 1);
        assert!(!out.code_protected);
        let reloaded = crate::models::_entities::projects::Entity::find_by_id(p.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert!(
            reloaded.active_version_id.is_none(),
            "activate=false → pas de bascule"
        );
    }
}
