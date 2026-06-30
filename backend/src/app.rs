use async_trait::async_trait;
use axum::http::{HeaderName, HeaderValue};
use axum::routing::get;
use axum::Router as AxumRouter;
use loco_rs::{
    app::{AppContext, Hooks, Initializer},
    bgworker::Queue,
    boot::{create_app, BootResult, StartMode},
    config::Config,
    controller::AppRoutes,
    environment::Environment,
    task::Tasks,
    Result,
};
use migration::Migrator;
use std::path::Path;
use tower_http::services::{ServeDir, ServeFile};

#[allow(unused_imports)]
use crate::{controllers, tasks};

/// La racine `/` n'a pas de surface propre : on renvoie vers l'admin, seul point
/// d'entrée humain. Redirection **temporaire** (307) — non mise en cache dur, donc
/// réversible si `/` doit un jour servir autre chose (landing, etc.).
async fn root_redirect() -> axum::response::Redirect {
    axum::response::Redirect::temporary("/admin")
}

/// robots.txt servi par l'app (le « hide » ne dépend pas d'un proxy externe).
async fn robots_txt() -> impl axum::response::IntoResponse {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; charset=utf-8",
        )],
        "User-agent: *\nDisallow: /\n",
    )
}

pub struct App;
#[async_trait]
impl Hooks for App {
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    fn app_version() -> String {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("dev")
        )
    }

    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: Config,
    ) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment, config).await
    }

    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![])
    }

    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes() // controller routes below
            .add_route(controllers::home::routes())
            .add_route(controllers::auth::routes())
            .add_route(controllers::admin::routes())
            .add_route(controllers::serve::routes())
            .add_route(controllers::settings::routes())
    }
    async fn connect_workers(_ctx: &AppContext, _queue: &Queue) -> Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    fn register_tasks(tasks: &mut Tasks) {
        // tasks-inject (do not remove)
    }
    async fn after_routes(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
        let store = crate::web::build_session_store(ctx).await?;
        // Fail-fast : un UNLOCK_COOKIE_SECRET trop court en prod doit casser le boot,
        // pas produire un 500 à la première requête /c protégée.
        crate::web::unlock_secret(ctx)?;
        let router = router.layer(axum_session::SessionLayer::new(store));

        // SPA servie sous /admin : assets si le fichier existe, sinon index.html
        // (deep-links client). nest_service strip le préfixe /admin ; les routes
        // /api/* et /_health restent prioritaires (non masquées).
        let dist = crate::web::spa_dist_dir();
        // Assets statiques (JS/CSS) exposés sous /assets — partagés entre le bundle
        // admin et le bundle unlock. La base Vite est '/' donc les deux bundles
        // référencent /assets/... sans préfixe /admin.
        let assets = ServeDir::new(dist.join("assets"));
        let router = router.nest_service("/assets", assets);
        let index = dist.join("index.html");
        let spa = ServeDir::new(&dist).fallback(ServeFile::new(index));
        let router = router.nest_service("/admin", spa);

        // Swagger UI : confort dev uniquement. Jamais en production (surface + poids).
        // Fail-secure : exclure Production via le même critère que le cookie Secure.
        let is_prod = !matches!(
            ctx.environment,
            loco_rs::environment::Environment::Development
                | loco_rs::environment::Environment::Test
        );
        let router = if is_prod {
            router
        } else {
            use utoipa::OpenApi;
            router.merge(
                utoipa_swagger_ui::SwaggerUi::new("/api-docs")
                    .url("/api-docs/openapi.json", crate::openapi::ApiDoc::openapi()),
            )
        };

        // MCP (contrat §5) : fail-fast des secrets/config au boot (comme unlock_secret),
        // puis montage du service Streamable HTTP sous /mcp. allowed_hosts dérivé de
        // LATCH_PUBLIC_BASE_URL (défense Host-header, CVE rmcp < 1.4.0).
        crate::web::deploy_token(ctx)?;
        let mcp = crate::mcp::service(ctx)?;
        let router = router.nest_service("/mcp", mcp);

        // Racine → admin (point d'entrée humain). Évite la page welcome Loco en dev
        // et le 404 en prod sur `/`.
        let router = router.route("/", get(root_redirect));

        // robots.txt + X-Robots-Tag : « hide » porté par l'app (contrat §9 « Hide this thing »).
        let router = router.route("/robots.txt", get(robots_txt));
        let router = router.layer(axum::middleware::map_response(
            |mut res: axum::response::Response| async move {
                res.headers_mut().insert(
                    HeaderName::from_static("x-robots-tag"),
                    HeaderValue::from_static("noindex, nofollow"),
                );
                res
            },
        ));

        Ok(router)
    }

    async fn truncate(_ctx: &AppContext) -> Result<()> {
        Ok(())
    }
    async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
        Ok(())
    }
}
