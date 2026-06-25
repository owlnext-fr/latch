//! Adaptateur entrant "settings admin". Lecture seule (GET → pas de garde Origin).
//! Sous `AdminAuth` : expose les infos de branchement MCP, dont le `deploy_token`,
//! à un admin AUTHENTIFIÉ (contrat §5/§9 : acceptable, l'admin a déjà le contrôle total).

use loco_rs::prelude::*;

use crate::controllers::auth::AdminAuth;
use crate::dto::SettingsResponse;

/// GET /api/settings — infos de branchement du connecteur MCP Claude.
#[utoipa::path(
    get, path = "/api/settings", tag = "settings",
    responses(
        (status = 200, description = "Infos MCP (deploy_token + URLs)", body = SettingsResponse),
        (status = 401, description = "Non authentifié")
    )
)]
#[debug_handler]
async fn get_settings(_auth: AdminAuth, State(ctx): State<AppContext>) -> Result<Response> {
    let public_base_url = crate::web::public_base_url(&ctx)?;
    let deploy_token = crate::web::deploy_token(&ctx)?;
    let mcp_url = format!("{public_base_url}/mcp");
    format::json(SettingsResponse {
        deploy_token,
        mcp_url,
        public_base_url,
    })
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/settings", get(get_settings))
}
