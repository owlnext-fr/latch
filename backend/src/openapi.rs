//! Document OpenAPI agrégé de l'API admin (`/api/*`). Source de vérité du contrat
//! front : le schéma exporté (`openapi.json`) sert à générer le client TypeScript.
//! Approche code-first manuelle (Loco enveloppe axum → pas d'auto-collection de routeur).

use utoipa::OpenApi;

use crate::controllers::{admin, auth};
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
    )),
    tags(
        (name = "auth", description = "Authentification admin"),
        (name = "projects", description = "Gestion des projets"),
        (name = "versions", description = "Déploiement et versions"),
    )
)]
pub struct ApiDoc;

#[cfg(test)]
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
        // Invariant §9.2 reflété dans le contrat : ProjectListItem n'expose pas `pin`.
        let json = ApiDoc::openapi().to_pretty_json().unwrap();
        // Le bloc de schéma ProjectListItem ne doit pas déclarer de propriété "pin".
        // (ProjectDetail, lui, le déclare — d'où une recherche ciblée sur le nom de schéma.)
        let marker = "\"ProjectListItem\"";
        let start = json.find(marker).expect("schéma ProjectListItem présent");
        // Fenêtre raisonnable couvrant la définition du schéma.
        let window = &json[start..(start + 600).min(json.len())];
        assert!(
            !window.contains("\"pin\""),
            "ProjectListItem ne doit pas exposer pin (§9.2)"
        );
    }
}
