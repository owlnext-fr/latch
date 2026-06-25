//! Adaptateur entrant "MCP" — fin (contrat §1, §5). Chaque tool valide le
//! `deploy_token` (services::security::secure_compare) AVANT d'appeler un service
//! du cœur. Aucune logique métier ici ; le cœur reste agnostique HTTP/MCP.

use std::sync::Arc;

use loco_rs::app::AppContext;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::model::{ErrorData, ServerCapabilities, ServerInfo};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::tower::StreamableHttpServerConfig;
use rmcp::transport::streamable_http_server::StreamableHttpService;
use rmcp::{tool_handler, tool_router, ServerHandler};
use sea_orm::DatabaseConnection;

use crate::services::errors::CoreError;
use crate::services::storage::Storage;

/// Serveur d'outils MCP. Porte des dépendances concrètes (pas l'`AppContext`) :
/// l'auth/config est résolue au montage (`service`), le serveur reste testable
/// sans booter Loco.
#[derive(Clone)]
pub struct LatchMcp {
    #[allow(dead_code)]
    db: DatabaseConnection,
    #[allow(dead_code)]
    storage: Arc<dyn Storage>,
    #[allow(dead_code)]
    deploy_token: String,
    #[allow(dead_code)]
    public_base_url: String,
    // Lu par le code généré de `#[tool_handler]` (câblage `call_tool`/`list_tools`).
    // Tant que le routeur est vide (Tasks 3-4), l'analyse dead-code ne « voit » pas
    // de lecture directe : on tolère explicitement le warning jusqu'à l'ajout des tools.
    #[allow(dead_code)]
    tool_router: ToolRouter<LatchMcp>,
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
#[allow(dead_code)]
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
