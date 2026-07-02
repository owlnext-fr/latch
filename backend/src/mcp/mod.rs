//! Adaptateur entrant "MCP" — fin (contrat §1, §5). Chaque tool valide le
//! `deploy_token` (services::security::secure_compare) AVANT d'appeler un service
//! du cœur. Aucune logique métier ici ; le cœur reste agnostique HTTP/MCP.

use std::sync::Arc;

use loco_rs::app::AppContext;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::{ErrorData, Implementation, ServerCapabilities, ServerInfo};
use rmcp::schemars;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::tower::StreamableHttpServerConfig;
use rmcp::transport::streamable_http_server::StreamableHttpService;
use rmcp::{tool, tool_handler, tool_router, ServerHandler};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use validator::Validate;

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
#[derive(Debug, Deserialize, schemars::JsonSchema, Validate)]
struct DeployArgs {
    /// Slug public du projet (doit déjà exister, créé via /admin).
    #[validate(length(min = 1))]
    slug: String,
    /// HTML mono-fichier complet du prototype.
    #[validate(custom(function = "crate::services::validation::validate_html"))]
    html: String,
    /// Secret de déploiement (validé contre DEPLOY_TOKEN).
    deploy_token: String, // exempté (secret) — jamais passé à `validate()`
    /// Activer immédiatement la version déployée (défaut : true).
    #[serde(default)]
    activate: Option<bool>,
    /// Notes de version en markdown léger (titres, gras, italique, listes, citation).
    /// Liens, images et code sont ignorés au rendu. Optionnel.
    #[serde(default)]
    #[validate(custom(function = "validate_release_notes_field"))]
    release_notes: Option<String>,
}

// `validator` 0.20 auto-unwrappe les champs `Option<T>` : un validateur `custom` sur un
// champ `Option<String>` ne reçoit QUE la valeur intérieure (jamais appelé sur `None`,
// qui passe automatiquement). `services::validation::validate_optional_release_notes`
// prend volontairement l'`Option` complet (source de vérité indépendante du framework
// de validation, cf. `dto::mod`). Cet adaptateur re-enveloppe la valeur dans `Some(...)`
// pour brancher les deux conventions sans dupliquer la logique de longueur.
fn validate_release_notes_field(v: &str) -> Result<(), validator::ValidationError> {
    crate::services::validation::validate_optional_release_notes(&Some(v.to_owned()))
}

/// Arguments du tool `list_projects`.
#[derive(Debug, Deserialize, schemars::JsonSchema, Validate)]
struct ListArgs {
    /// Secret de déploiement (validé contre DEPLOY_TOKEN).
    deploy_token: String, // exempté (secret) — seul champ, `validate()` est un no-op
}

/// Résumé d'un projet renvoyé par `list_projects` — sans PIN ni hash (§9.1/§9.2).
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ProjectSummary {
    /// Slug public — à passer à `deploy_prototype`.
    pub slug: String,
    pub name: String,
    /// `true` si le projet exige un code d'accès.
    pub code_protected: bool,
    /// Numéro de la version active, ou `null` si jamais déployé.
    pub active_version: Option<i32>,
}

/// Résultat enveloppe de `list_projects` — MCP exige `outputSchema` de type `object`
/// (pas `array`), donc on encapsule la liste.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ProjectListResult {
    /// Liste des projets.
    pub projects: Vec<ProjectSummary>,
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

/// Arguments du tool `pull_prototype`.
#[derive(Debug, Deserialize, schemars::JsonSchema, Validate)]
struct PullArgs {
    /// Slug public du projet (doit exister).
    #[validate(length(min = 1))]
    slug: String,
    /// Numéro de version à récupérer ; omis → version active.
    #[serde(default)]
    version: Option<i32>,
    /// Secret de déploiement (validé contre DEPLOY_TOKEN).
    deploy_token: String, // exempté (secret)
}

/// Un message d'un fil de commentaires (sans `owner_token` — §9.7).
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct PullMessage {
    /// Nom auto-déclaré de l'auteur (côté admin : "admin").
    pub author_name: String,
    /// `true` si le message vient du compte admin (dérivé, sans exposer le token).
    pub is_admin: bool,
    /// Corps du message (texte brut).
    pub body: String,
    /// Date de création (ISO 8601).
    pub created_at: String,
}

/// Un fil de commentaires ancré (pin + messages).
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct PullThread {
    /// Descripteur d'ancrage JSON brut (non interprété serveur — §3).
    pub anchor: String,
    pub messages: Vec<PullMessage>,
}

/// Résultat de `pull_prototype` : HTML de la version + tous ses fils de commentaires.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct PullResult {
    pub slug: String,
    /// Numéro de la version renvoyée.
    pub version: i32,
    /// URL publique stable du prototype.
    pub url: String,
    /// `true` si les commentaires sont activés sur le projet (informatif).
    pub comments_enabled: bool,
    /// Notes de version en markdown brut, ou `null`.
    pub release_notes: Option<String>,
    /// HTML brut du prototype (cible d'édition).
    pub html: String,
    /// Fils de commentaires non supprimés (visiteurs + admin).
    pub threads: Vec<PullThread>,
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
                       au préalable dans l'admin. Active la version par défaut. Accepte des \
                       notes de version en markdown léger (release_notes)."
    )]
    async fn deploy_prototype(
        &self,
        Parameters(args): Parameters<DeployArgs>,
    ) -> Result<Json<DeployResult>, ErrorData> {
        self.check_token(&args.deploy_token)?;
        args.validate().map_err(map_validation_err)?;

        let projects = ProjectsService::new(self.db.clone());
        let project = projects
            .get_by_slug(&args.slug)
            .await
            .map_err(map_core_err)?;

        let activate = args.activate.unwrap_or(true);
        let deploy = DeployService::new(self.db.clone(), self.storage.clone());
        let version = deploy
            .deploy(
                project.id,
                &args.html,
                activate,
                args.release_notes.as_deref(),
            )
            .await
            .map_err(map_core_err)?;

        Ok(Json(DeployResult {
            url: format!("{}/c/{}", self.public_base_url, project.slug),
            version: version.n,
            code_protected: project.code_enabled,
        }))
    }

    #[tool(
        description = "Liste les projets déployables (slug, nom, protection par code, n° de \
                       version active). Ne renvoie jamais de PIN ni de secret."
    )]
    async fn list_projects(
        &self,
        Parameters(args): Parameters<ListArgs>,
    ) -> Result<Json<ProjectListResult>, ErrorData> {
        self.check_token(&args.deploy_token)?;
        args.validate().map_err(map_validation_err)?;

        let projects = ProjectsService::new(self.db.clone());
        let rows = projects.list_with_versions().await.map_err(map_core_err)?;

        let items = rows
            .iter()
            .map(|(p, vers)| ProjectSummary {
                slug: p.slug.clone(),
                name: p.name.clone(),
                code_protected: p.code_enabled,
                active_version: p
                    .active_version_id
                    .and_then(|aid| vers.iter().find(|v| v.id == aid).map(|v| v.n)),
            })
            .collect();

        Ok(Json(ProjectListResult { projects: items }))
    }

    #[tool(
        description = "Récupère le HTML d'une version d'un prototype et TOUS ses fils de \
                       commentaires (visiteurs + admin), pour itérer dessus. `version` optionnel : \
                       défaut = version active. Gardé par `deploy_token`."
    )]
    async fn pull_prototype(
        &self,
        Parameters(args): Parameters<PullArgs>,
    ) -> Result<Json<PullResult>, ErrorData> {
        self.check_token(&args.deploy_token)?;
        args.validate().map_err(map_validation_err)?;

        let projects = ProjectsService::new(self.db.clone());
        let project = projects
            .get_by_slug(&args.slug)
            .await
            .map_err(map_core_err)?;

        let version = match args.version {
            Some(n) => projects
                .get_version(project.id, n)
                .await
                .map_err(|e| map_version_err(e, "version inconnue"))?,
            None => projects
                .get_active_version(&project)
                .await
                .map_err(|e| map_version_err(e, "aucune version active"))?,
        };

        let html = self
            .storage
            .read(&version.html_path)
            .await
            .map_err(|e| match e {
                CoreError::NotFound => ErrorData::internal_error("erreur interne", None),
                other => map_core_err(other),
            })?;

        let comments = crate::services::comments::CommentsService::new(self.db.clone());
        let rows = comments
            .list_for_version(version.id)
            .await
            .map_err(map_core_err)?;

        let threads = rows
            .into_iter()
            .map(|pwm| PullThread {
                anchor: pwm.pin.anchor,
                messages: pwm
                    .messages
                    .into_iter()
                    .map(|msg| PullMessage {
                        is_admin: crate::services::comments::is_admin_owner(&msg.owner_token),
                        author_name: msg.author_name,
                        body: msg.body,
                        created_at: msg.created_at.to_rfc3339(),
                    })
                    .collect(),
            })
            .collect();

        Ok(Json(PullResult {
            url: format!("{}/c/{}", self.public_base_url, project.slug),
            slug: project.slug,
            version: version.n,
            comments_enabled: project.comments_enabled,
            release_notes: version.release_notes,
            html,
            threads,
        }))
    }
}

#[tool_handler]
impl ServerHandler for LatchMcp {
    fn get_info(&self) -> ServerInfo {
        // `ServerInfo` (= `InitializeResult`) est `#[non_exhaustive]` (rmcp 1.8) :
        // construction interdite hors crate, on part du `Default` puis on règle
        // capabilities, server_info et instructions via les setters/champs publics.
        let mut info = ServerInfo::default();
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info = info.with_server_info(Implementation::new("latch", env!("CARGO_PKG_VERSION")));
        info.with_instructions(
            "latch — déploiement de prototypes HTML. Outils : deploy_prototype, \
             list_projects, pull_prototype. Chaque appel exige le deploy_token.",
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

/// Mappe les erreurs de forme (`validator`) vers une erreur de tool MCP (frontière §1).
/// Appelé APRÈS `check_token` (§9.3 : le token est validé EN PREMIER).
fn map_validation_err(e: validator::ValidationErrors) -> ErrorData {
    ErrorData::invalid_params(format!("arguments invalides: {e}"), None)
}

/// `NotFound` sur une résolution de version → message dédié ; autres erreurs → `map_core_err`.
fn map_version_err(e: CoreError, not_found_msg: &'static str) -> ErrorData {
    match e {
        CoreError::NotFound => ErrorData::invalid_params(not_found_msg, None),
        other => map_core_err(other),
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

    use crate::services::comments::{CommentsService, ADMIN_OWNER_TOKEN};

    const TOKEN: &str = "test-deploy-token";

    /// Déploie une version v1 (HTML donné) sur `project_id` et l'active. Renvoie le n.
    async fn deploy_v1(db: &DatabaseConnection, dir: &TempDir, project_id: i32, html: &str) {
        let storage: Arc<dyn Storage> = Arc::new(FsStorage::new(dir.path().to_path_buf()));
        DeployService::new(db.clone(), storage)
            .deploy(project_id, html, true, None)
            .await
            .unwrap();
    }

    /// Appelle `pull_prototype` avec des arguments concis (évite de répéter le littéral).
    async fn call_pull(
        m: &LatchMcp,
        slug: &str,
        version: Option<i32>,
        token: &str,
    ) -> Result<Json<PullResult>, ErrorData> {
        m.pull_prototype(Parameters(PullArgs {
            slug: slug.to_string(),
            version,
            deploy_token: token.to_string(),
        }))
        .await
    }

    /// Extrait l'erreur d'un résultat de tool (les `Json<_>` n'implémentent pas `Debug`).
    fn expect_err<T>(res: Result<T, ErrorData>) -> ErrorData {
        match res {
            Err(e) => e,
            Ok(_) => panic!("un résultat d'erreur était attendu"),
        }
    }

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
                comments_enabled: false,
            })
            .await
            .unwrap()
    }

    #[test]
    fn deploy_args_reject_empty_html() {
        let a = DeployArgs {
            slug: "s".into(),
            html: "".into(),
            deploy_token: "t".into(),
            activate: None,
            release_notes: None,
        };
        assert!(a.validate().is_err());
        let ok = DeployArgs {
            slug: "s".into(),
            html: "<h1>x</h1>".into(),
            deploy_token: "t".into(),
            activate: None,
            release_notes: None,
        };
        assert!(ok.validate().is_ok());
    }

    #[test]
    fn deploy_args_reject_release_notes_over_max_len() {
        let a = DeployArgs {
            slug: "s".into(),
            html: "<h1>x</h1>".into(),
            deploy_token: "t".into(),
            activate: None,
            release_notes: Some("x".repeat(10_001)),
        };
        assert!(a.validate().is_err());
        let ok = DeployArgs {
            slug: "s".into(),
            html: "<h1>x</h1>".into(),
            deploy_token: "t".into(),
            activate: None,
            release_notes: Some("x".repeat(10_000)),
        };
        assert!(ok.validate().is_ok());
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
                release_notes: None,
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
                release_notes: None,
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
                release_notes: None,
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
                release_notes: None,
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

    #[tokio::test]
    async fn list_rejects_bad_token() {
        let db = test_db().await;
        let dir = tempfile::tempdir().unwrap();
        let m = mcp(db, &dir);
        let res = m
            .list_projects(Parameters(ListArgs {
                deploy_token: "WRONG".to_string(),
            }))
            .await;
        // Le rejet doit venir du gate token (§9.3). `Json<ProjectListResult>`
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
    async fn list_returns_summaries_without_pin() {
        let db = test_db().await;
        let dir = tempfile::tempdir().unwrap();
        let p = make_project(&db, true).await; // code activé, pin 123456
        let m = mcp(db, &dir);
        let Json(result) = m
            .list_projects(Parameters(ListArgs {
                deploy_token: TOKEN.to_string(),
            }))
            .await
            .unwrap();

        assert_eq!(result.projects.len(), 1);
        assert_eq!(result.projects[0].slug, p.slug);
        assert!(result.projects[0].code_protected);
        assert_eq!(result.projects[0].active_version, None);
        // Invariant §9.2 : le PIN ne doit JAMAIS transiter par MCP.
        let json = serde_json::to_string(&result).unwrap();
        assert!(
            !json.contains("123456"),
            "le PIN ne doit pas apparaître via MCP (§9.2)"
        );
        assert!(
            !json.contains("\"pin\""),
            "pas de champ pin dans le résumé MCP"
        );
    }

    #[tokio::test]
    async fn pull_rejects_bad_token() {
        let db = test_db().await;
        let dir = tempfile::tempdir().unwrap();
        let p = make_project(&db, false).await;
        let m = mcp(db, &dir);
        let err = expect_err(call_pull(&m, &p.slug, None, "WRONG").await);
        assert_eq!(err.message, "deploy_token invalide");
    }

    #[tokio::test]
    async fn pull_unknown_slug_is_error() {
        let db = test_db().await;
        let dir = tempfile::tempdir().unwrap();
        let m = mcp(db, &dir);
        let res = call_pull(&m, "nope-xxxxxxxx", None, TOKEN).await;
        assert!(res.is_err(), "slug inconnu → erreur");
    }

    #[tokio::test]
    async fn pull_no_active_version_is_error() {
        let db = test_db().await;
        let dir = tempfile::tempdir().unwrap();
        let p = make_project(&db, false).await; // créé mais jamais déployé
        let m = mcp(db, &dir);
        let err = expect_err(call_pull(&m, &p.slug, None, TOKEN).await);
        assert_eq!(err.message, "aucune version active");
    }

    #[tokio::test]
    async fn pull_returns_html_and_default_active_version() {
        let db = test_db().await;
        let dir = tempfile::tempdir().unwrap();
        let p = make_project(&db, false).await;
        deploy_v1(&db, &dir, p.id, "<h1>hello</h1>").await;
        let m = mcp(db, &dir);
        let Json(out) = call_pull(&m, &p.slug, None, TOKEN).await.unwrap();
        assert_eq!(out.version, 1);
        assert_eq!(out.html, "<h1>hello</h1>");
        assert_eq!(out.slug, p.slug);
        assert_eq!(out.url, format!("https://demo.test/c/{}", p.slug));
        assert!(out.threads.is_empty());
    }

    #[tokio::test]
    async fn pull_explicit_unknown_version_is_error() {
        let db = test_db().await;
        let dir = tempfile::tempdir().unwrap();
        let p = make_project(&db, false).await;
        deploy_v1(&db, &dir, p.id, "<h1>hello</h1>").await;
        let m = mcp(db, &dir);
        let err = expect_err(call_pull(&m, &p.slug, Some(99), TOKEN).await);
        assert_eq!(err.message, "version inconnue");
    }

    #[tokio::test]
    async fn pull_returns_threads_with_is_admin_and_no_owner_token() {
        let db = test_db().await;
        let dir = tempfile::tempdir().unwrap();
        let p = make_project(&db, true).await; // code activé, PIN 123456
        deploy_v1(&db, &dir, p.id, "<h1>hello</h1>").await;

        // Résoudre la version 1 pour semer des commentaires dessus.
        let version = ProjectsService::new(db.clone())
            .get_version(p.id, 1)
            .await
            .unwrap();
        let comments = CommentsService::new(db.clone());
        // Un fil visiteur (owner_token ULID opaque) + un fil admin (sentinelle).
        let visitor_token = "01VISITORTOKENxxxxxxxxxxxx";
        comments
            .create_pin(
                version.id,
                visitor_token,
                "Alice",
                "hello from visitor",
                "{\"v\":1}",
            )
            .await
            .unwrap();
        comments
            .create_pin(
                version.id,
                ADMIN_OWNER_TOKEN,
                "admin",
                "note admin",
                "{\"v\":1}",
            )
            .await
            .unwrap();

        let m = mcp(db, &dir);
        let Json(out) = call_pull(&m, &p.slug, None, TOKEN).await.unwrap();

        assert_eq!(out.threads.len(), 2);
        let all_msgs: Vec<&PullMessage> = out.threads.iter().flat_map(|t| &t.messages).collect();
        // is_admin correct : exactement 1 message admin.
        assert_eq!(all_msgs.iter().filter(|m| m.is_admin).count(), 1);
        assert_eq!(all_msgs.iter().filter(|m| !m.is_admin).count(), 1);
        // anchor brut présent.
        assert!(out.threads.iter().all(|t| t.anchor.contains("\"v\"")));

        // Invariants : owner_token (visiteur ET sentinelle) et PIN jamais sérialisés.
        let json = serde_json::to_string(&out).unwrap();
        assert!(
            !json.contains(visitor_token),
            "owner_token visiteur ne doit pas fuiter"
        );
        assert!(
            !json.contains(ADMIN_OWNER_TOKEN),
            "sentinelle admin ne doit pas fuiter"
        );
        assert!(!json.contains("owner_token"), "pas de champ owner_token");
        assert!(
            !json.contains("123456"),
            "le PIN ne doit jamais fuiter via MCP"
        );
        assert!(
            !json.contains("\"id\""),
            "pas de champ id DB dans la réponse MCP"
        );
    }
}
