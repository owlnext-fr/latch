#![allow(clippy::unwrap_used, clippy::expect_used)]

use latch::app::App;
use latch::models::_entities::{projects, versions};
use loco_rs::testing::prelude::*;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set};
use serial_test::serial;

const TOKEN: &str = "test-deploy-token";

/// Pose les env vars MCP (lues au boot dans after_routes) + un storage tempdir.
/// Retourne le tempdir (à garder vivant jusqu'à la fin du test).
fn setup_env() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::env::set_var("DEPLOY_TOKEN", TOKEN);
    std::env::set_var("LATCH_PUBLIC_BASE_URL", "http://localhost:5150");
    std::env::set_var("LATCH_STORAGE_ROOT", dir.path());
    dir
}

/// Extrait le payload JSON d'une réponse MCP (corps JSON brut OU flux SSE `data: {...}`).
/// SSE rmcp 1.8 : des lignes `data:` vides séparent les events ; on prend la 1re non-vide.
fn parse_mcp_body(body: &str) -> serde_json::Value {
    let trimmed = body.trim_start();
    if trimmed.starts_with('{') {
        return serde_json::from_str(trimmed).expect("json direct");
    }
    // SSE : chercher la 1re ligne `data: <non-vide>`
    for line in body.lines() {
        if let Some(rest) = line.strip_prefix("data:") {
            let payload = rest.trim();
            if !payload.is_empty() {
                return serde_json::from_str(payload).expect("json sse");
            }
        }
    }
    panic!("corps MCP non parsable : {body}");
}

/// POST JSON-RPC vers /mcp avec les en-têtes requis par rmcp 1.8.
/// `session` = header de session à rejouer (None pour initialize).
async fn mcp_post(
    request: &axum_test::TestServer,
    body: serde_json::Value,
    session: Option<&str>,
) -> (axum::http::HeaderMap, serde_json::Value) {
    let mut req = request
        .post("/mcp")
        .add_header("accept", "application/json, text/event-stream")
        .add_header("content-type", "application/json")
        .add_header("host", "localhost:5150")
        .json(&body);
    if let Some(sid) = session {
        req = req.add_header("mcp-session-id", sid);
    }
    let res = req.await;
    // Les erreurs de tool restent en HTTP 200 (isError dans le corps) : ce check
    // n'attrape que les échecs de transport (403 Host, 4xx/5xx réels).
    assert!(
        res.status_code().is_success(),
        "POST /mcp a échoué : {}",
        res.status_code()
    );
    let headers = res.headers().clone();
    let body_text = res.text();
    let value = parse_mcp_body(&body_text);
    (headers, value)
}

fn init_body() -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": { "name": "latch-test", "version": "0" }
        }
    })
}

#[tokio::test]
#[serial]
async fn mcp_initialize_handshake() {
    let _dir = setup_env();
    request::<App, _, _>(|request, _ctx| async move {
        let (headers, value) = mcp_post(&request, init_body(), None).await;
        assert!(
            headers.get("mcp-session-id").is_some(),
            "initialize doit renvoyer un header de session"
        );
        // Preuve structurelle d'un vrai résultat initialize (pas une erreur déguisée).
        assert!(
            value["result"]["protocolVersion"].as_str().is_some(),
            "protocolVersion absent de la réponse initialize"
        );
        // Le serveur doit s'annoncer sous le nom "latch" (pas le défaut "rmcp").
        assert_eq!(
            value["result"]["serverInfo"]["name"], "latch",
            "serverInfo.name doit être latch"
        );
        // On vérifie aussi les instructions comme preuve de bon câblage.
        let instructions = value["result"]["instructions"].as_str().unwrap_or("");
        assert!(
            instructions.contains("latch"),
            "les instructions doivent mentionner latch : {instructions}"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn mcp_tools_list_exposes_three_tools() {
    let _dir = setup_env();
    request::<App, _, _>(|request, _ctx| async move {
        let (headers, _) = mcp_post(&request, init_body(), None).await;
        let sid = headers
            .get("mcp-session-id")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string)
            .expect("session id");

        let body = serde_json::json!({
            "jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}
        });
        let (_, value) = mcp_post(&request, body, Some(&sid)).await;
        let tools = value["result"]["tools"].as_array().expect("tools array");
        let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
        assert_eq!(names.len(), 3, "nombre de tools inattendu : {names:?}");
        assert!(
            names.contains(&"deploy_prototype"),
            "deploy_prototype absent : {names:?}"
        );
        assert!(
            names.contains(&"list_projects"),
            "list_projects absent : {names:?}"
        );
        assert!(
            names.contains(&"pull_prototype"),
            "pull_prototype absent : {names:?}"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn mcp_deploy_prototype_creates_version() {
    let _dir = setup_env();
    request::<App, _, _>(|request, ctx| async move {
        // Le slug doit préexister (pas d'auto-création — contrat §5.1).
        projects::ActiveModel {
            slug: Set("mon-projet-aaaaaaaa".to_string()),
            name: Set("Mon Projet".to_string()),
            code_enabled: Set(true),
            pin: Set(Some("123456".to_string())),
            ..Default::default()
        }
        .insert(&ctx.db)
        .await
        .expect("insert project");

        let (headers, _) = mcp_post(&request, init_body(), None).await;
        let sid = headers
            .get("mcp-session-id")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        let body = serde_json::json!({
            "jsonrpc": "2.0", "id": 3, "method": "tools/call",
            "params": {
                "name": "deploy_prototype",
                "arguments": {
                    "slug": "mon-projet-aaaaaaaa",
                    "html": "<!doctype html><title>proto</title>",
                    "deploy_token": TOKEN
                }
            }
        });
        let (_, value) = mcp_post(&request, body, Some(&sid)).await;

        // Le résultat structuré du tool est dans structuredContent (rmcp 1.8, Json<_>).
        let structured = &value["result"]["structuredContent"];
        assert_eq!(
            structured["url"],
            "http://localhost:5150/c/mon-projet-aaaaaaaa"
        );
        assert_eq!(structured["version"], 1);
        assert_eq!(structured["code_protected"], true);

        // Invariant §9 : aucun PIN ni hash dans la réponse.
        let raw = value.to_string();
        assert!(
            !raw.contains("123456"),
            "le PIN ne doit jamais fuiter via MCP"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn mcp_list_projects_is_object_envelope() {
    let _dir = setup_env();
    request::<App, _, _>(|request, ctx| async move {
        projects::ActiveModel {
            slug: Set("demo-bbbbbbbb".to_string()),
            name: Set("ACME".to_string()),
            code_enabled: Set(false),
            ..Default::default()
        }
        .insert(&ctx.db)
        .await
        .expect("insert");

        let (headers, _) = mcp_post(&request, init_body(), None).await;
        let sid = headers
            .get("mcp-session-id")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        let body = serde_json::json!({
            "jsonrpc": "2.0", "id": 4, "method": "tools/call",
            "params": { "name": "list_projects", "arguments": { "deploy_token": TOKEN } }
        });
        let (_, value) = mcp_post(&request, body, Some(&sid)).await;
        let projects_arr = value["result"]["structuredContent"]["projects"]
            .as_array()
            .expect("enveloppe objet { projects: [...] }");
        assert!(projects_arr.iter().any(|p| p["slug"] == "demo-bbbbbbbb"));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn mcp_bad_token_is_rejected() {
    let _dir = setup_env();
    request::<App, _, _>(|request, _ctx| async move {
        let (headers, _) = mcp_post(&request, init_body(), None).await;
        let sid = headers
            .get("mcp-session-id")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        let body = serde_json::json!({
            "jsonrpc": "2.0", "id": 5, "method": "tools/call",
            "params": { "name": "list_projects", "arguments": { "deploy_token": "MAUVAIS" } }
        });
        let (_, value) = mcp_post(&request, body, Some(&sid)).await;
        // Le tool renvoie une erreur (isError ou error JSON-RPC) — pas de liste.
        let is_error =
            value["result"]["isError"].as_bool().unwrap_or(false) || value.get("error").is_some();
        assert!(
            is_error,
            "un deploy_token invalide doit être rejeté : {value}"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn mcp_deploy_bad_token_no_side_effect() {
    let _dir = setup_env();
    request::<App, _, _>(|request, ctx| async move {
        // Projet préexistant : le gate token doit échouer AVANT toute écriture (§9.3).
        let project = projects::ActiveModel {
            slug: Set("mon-projet-cccccccc".to_string()),
            name: Set("Mon Projet".to_string()),
            code_enabled: Set(true),
            pin: Set(Some("123456".to_string())),
            ..Default::default()
        }
        .insert(&ctx.db)
        .await
        .expect("insert project");

        let (headers, _) = mcp_post(&request, init_body(), None).await;
        let sid = headers
            .get("mcp-session-id")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        let body = serde_json::json!({
            "jsonrpc": "2.0", "id": 6, "method": "tools/call",
            "params": {
                "name": "deploy_prototype",
                "arguments": {
                    "slug": "mon-projet-cccccccc",
                    "html": "<!doctype html><title>nope</title>",
                    "deploy_token": "MAUVAIS"
                }
            }
        });
        let (_, value) = mcp_post(&request, body, Some(&sid)).await;
        let is_error =
            value["result"]["isError"].as_bool().unwrap_or(false) || value.get("error").is_some();
        assert!(
            is_error,
            "un deploy_token invalide doit être rejeté : {value}"
        );

        // Aucune version ne doit avoir été créée pour ce projet (gate AVANT write path).
        let count = versions::Entity::find()
            .filter(versions::Column::ProjectId.eq(project.id))
            .count(&ctx.db)
            .await
            .expect("count versions");
        assert_eq!(
            count, 0,
            "un token invalide ne doit créer aucune version (effet de bord sur le write path)"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn mcp_pull_prototype_roundtrip() {
    let _dir = setup_env();
    request::<App, _, _>(|request, ctx| async move {
        // Projet préexistant + version déployée via le service (chemin réel).
        let project = projects::ActiveModel {
            slug: Set("pull-me-cccccccc".to_string()),
            name: Set("Pull Me".to_string()),
            code_enabled: Set(false),
            pin: Set(None),
            comments_enabled: Set(true),
            ..Default::default()
        }
        .insert(&ctx.db)
        .await
        .expect("insert project");

        let storage = latch::web::storage_from_ctx(&ctx);
        latch::services::deploy::DeployService::new(ctx.db.clone(), storage)
            .deploy(project.id, "<h1>pulled</h1>", true, None)
            .await
            .expect("deploy v1");

        let (headers, _) = mcp_post(&request, init_body(), None).await;
        let sid = headers
            .get("mcp-session-id")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        let body = serde_json::json!({
            "jsonrpc": "2.0", "id": 7, "method": "tools/call",
            "params": {
                "name": "pull_prototype",
                "arguments": {
                    "slug": "pull-me-cccccccc",
                    "deploy_token": TOKEN
                }
            }
        });
        let (_, value) = mcp_post(&request, body, Some(&sid)).await;

        let structured = &value["result"]["structuredContent"];
        assert_eq!(structured["version"], 1);
        assert_eq!(structured["html"], "<h1>pulled</h1>");
        assert_eq!(
            structured["url"],
            "http://localhost:5150/c/pull-me-cccccccc"
        );
        assert_eq!(structured["comments_enabled"], true);
        assert!(structured["threads"].as_array().unwrap().is_empty());
    })
    .await;
}
