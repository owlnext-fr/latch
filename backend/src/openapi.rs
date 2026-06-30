//! Document OpenAPI agrégé de l'API admin (`/api/*`). Source de vérité du contrat
//! front : le schéma exporté (`openapi.json`) sert à générer le client TypeScript.
//! Approche code-first manuelle (Loco enveloppe axum → pas d'auto-collection de routeur).

use utoipa::OpenApi;

use crate::controllers::{admin, auth, serve, settings};
use crate::dto;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "latch admin API",
        description = "API JSON de l'admin latch (session cookie same-origin).",
        version = "0.1.0"
    ),
    paths(
        auth::login,
        auth::logout,
        admin::list,
        admin::detail,
        admin::create,
        admin::update,
        admin::delete,
        admin::set_code,
        admin::clear_code,
        admin::deploy,
        admin::activate_version,
        admin::delete_version,
        admin::preview_version,
        admin::list_version_comments,
        admin::moderate_delete_comment,
        serve::public_meta,
        serve::list_comments,
        serve::create_comment,
        serve::reply_comment,
        serve::edit_comment,
        serve::delete_comment,
        serve::delete_comment_pin,
        settings::get_settings,
    ),
    components(schemas(
        dto::ProjectListItem,
        dto::ProjectDetail,
        dto::VersionItem,
        dto::CreateProjectReq,
        dto::UpdateProjectReq,
        dto::SetCodeReq,
        dto::DeployReq,
        dto::LoginReq,
        dto::OkResponse,
        dto::DeployResponse,
        dto::ActivateResponse,
        dto::PublicMeta,
        dto::SettingsResponse,
        dto::CreatePinReq,
        dto::ReplyReq,
        dto::EditMessageReq,
        dto::CommentMessage,
        dto::CommentPin,
        dto::CommentList,
        dto::AdminCommentMessage,
        dto::AdminCommentPin,
        dto::AdminCommentList,
    )),
    tags(
        (name = "auth", description = "Authentification admin"),
        (name = "projects", description = "Gestion des projets"),
        (name = "versions", description = "Déploiement et versions"),
        (name = "serving", description = "Serving client /c et meta publique"),
        (name = "settings", description = "Configuration admin (infos MCP)"),
    )
)]
pub struct ApiDoc;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn document_contains_all_paths() {
        let doc = ApiDoc::openapi();
        let paths = &doc.paths.paths;
        for expected in [
            "/api/login",
            "/api/logout",
            "/api/projects",
            "/api/projects/{id}",
            "/api/projects/{id}/code",
            "/api/projects/{id}/deploy",
            "/api/projects/{id}/versions/{n}/activate",
            "/api/projects/{id}/versions/{n}",
            "/api/projects/{id}/versions/{n}/preview",
            "/api/public/{slug}",
            "/api/settings",
        ] {
            assert!(
                paths.contains_key(expected),
                "chemin manquant dans l'OpenAPI : {expected}"
            );
        }
    }

    #[test]
    fn document_contains_core_schemas() {
        // Le JSON sérialisé doit référencer les schémas clés du contrat.
        let json = ApiDoc::openapi().to_pretty_json().unwrap();
        for schema in [
            "ProjectListItem",
            "ProjectDetail",
            "CreateProjectReq",
            "DeployResponse",
        ] {
            assert!(
                json.contains(schema),
                "schéma manquant dans l'OpenAPI : {schema}"
            );
        }
    }

    #[test]
    fn list_schema_has_no_pin_field() {
        // Invariant §9.2 reflété dans le contrat OpenAPI : ProjectListItem n'expose pas `pin`.
        // Assertion structurée (pas de fenêtre de texte fragile) : on navigue jusqu'aux
        // propriétés du schéma et on vérifie l'absence de la clé `pin`.
        let json = ApiDoc::openapi().to_pretty_json().unwrap();
        let doc: serde_json::Value = serde_json::from_str(&json).unwrap();
        let props = &doc["components"]["schemas"]["ProjectListItem"]["properties"];
        assert!(
            props.is_object(),
            "ProjectListItem doit déclarer des propriétés dans le schéma"
        );
        assert!(
            props.get("pin").is_none(),
            "ProjectListItem ne doit pas exposer pin (§9.2)"
        );
        // Sanity : ProjectDetail, lui, DOIT déclarer pin (sinon le test ci-dessus serait vacant).
        let detail_props = &doc["components"]["schemas"]["ProjectDetail"]["properties"];
        assert!(
            detail_props.get("pin").is_some(),
            "ProjectDetail doit exposer pin (garde anti-test-vacant)"
        );
    }
}
