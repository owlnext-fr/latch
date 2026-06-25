# Phase 6 — Finalisation (e2e, durcissement, packaging) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Clore la v1 publiable de latch : couverture e2e de bout en bout (navigateur + transport MCP réel), durcissement « hide » porté par l'app, et packaging FOSS (README + captures + badges, CHANGELOG, sonar.tests).

**Architecture:** Travail sur la branche `feat/phase-6-finalisation` (déjà créée depuis `main`). Le contrat `docs/contrat-deploy.md` fait loi. Spec de référence : `docs/superpowers/specs/2026-06-25-phase-6-finalisation-design.md`. On ajoute des tests (backend `nextest` + Playwright), un durcissement dans `after_routes`, et des fichiers de packaging — sans toucher aux décisions d'archi/sécu existantes.

**Tech Stack:** Rust/Loco/axum 0.8 + SeaORM + rmcp 1.8 (backend) ; React/Vite + Playwright (frontend) ; SonarCloud ; git-cliff (CHANGELOG).

## Global Constraints

- **Confidentialité (NON-NÉGOCIABLE)** : aucun nom de client réel nulle part (code, tests, fixtures, captures, docs, commits). Placeholders génériques uniquement : `Mon Projet` / `mon-projet`, `ACME`, `demo`.
- **Langue** : tout en français (commentaires, docs, messages de commit, README), diacritiques correctes.
- **Commits** : conventionnels + gitmoji, format `<gitmoji> <type>: <description>` (ex. `✨ feat:`, `🐛 fix:`, `🧱 chore:`, `📝 docs:`, `✅ test:`).
- **Pas d'`unwrap`/`expect`** hors tests (les fichiers de test portent déjà `#![allow(clippy::unwrap_used, clippy::expect_used)]` en tête).
- **Invariants sécu testés** (contrat §9) : aucune réponse ne renvoie de hash ; PIN jamais en liste/MCP ; `deploy_token` validé sur tous les tools MCP.
- **Définition de « terminé »** : `cargo fmt --all` + `cargo clippy --all-targets --all-features -- -D warnings` + `cargo nextest run` verts ; `cd frontend && pnpm lint && pnpm typecheck && pnpm test` verts ; e2e Playwright vert ; gate SonarCloud `new_coverage ≥ 80%`.
- **Serveur Loco** : se lance depuis `backend/` (`cd backend && cargo loco start`). `fmt`/`clippy`/`nextest` depuis la racine.
- **Régénération OpenAPI** (si DTO/handler changé, pas attendu ici) : `UPDATE_OPENAPI=1 cargo test --test openapi_drift` + `cd frontend && pnpm gen:api`.

---

## File Structure

**Créés :**
- `backend/tests/hardening.rs` — tests d'intégration robots.txt + X-Robots-Tag.
- `backend/tests/mcp_http.rs` — test e2e du transport MCP Streamable HTTP réel.
- `frontend/e2e/serve-unlock.spec.ts` — e2e navigateur du serving `/c` + unlock + bascule.
- `frontend/e2e/fixtures/proto-v2.html` — 2ᵉ fixture HTML (marqueur v2 distinct).
- `frontend/scripts/screenshots.spec.ts` — script Playwright **hors CI** générant les captures.
- `docs/assets/admin-list.png`, `docs/assets/unlock.png` — captures committées.
- `cliff.toml` — config git-cliff (racine).
- `CHANGELOG.md` — généré par git-cliff (racine).

**Modifiés :**
- `backend/src/app.rs` — route `/robots.txt` + layer `X-Robots-Tag` dans `after_routes`.
- `sonar-project.properties` — `sonar.tests=frontend/src,backend/tests`.
- `README.md` — refonte complète.
- `docs/` (mémoire) — INDEX, HANDOFF, ENVIRONMENT, QUIRKS, CONVENTIONS, ROADMAP (stub Phase 8) en tâche finale.

---

## Task 1 : Durcissement « hide » — robots.txt + X-Robots-Tag

**Files:**
- Modify: `backend/src/app.rs` (fn `after_routes`, ~ligne 65-110)
- Test: `backend/tests/hardening.rs` (créer)

**Interfaces:**
- Consumes: `after_routes(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter>` (existant).
- Produces: route `GET /robots.txt` (200, `text/plain`, corps `User-agent: *\nDisallow: /\n`) ; en-tête `X-Robots-Tag: noindex, nofollow` sur **toutes** les réponses.

- [ ] **Step 1 : Écrire le test d'intégration (échoue)**

Créer `backend/tests/hardening.rs` :

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]

use latch::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn robots_txt_is_served() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/robots.txt").await;
        res.assert_status_ok();
        let ct = res
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(ct.starts_with("text/plain"), "content-type = {ct}");
        assert!(
            res.text().contains("Disallow: /"),
            "robots.txt doit interdire tout crawl"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn x_robots_tag_on_admin() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/admin").await;
        let tag = res
            .headers()
            .get("x-robots-tag")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(tag, "noindex, nofollow", "X-Robots-Tag manquant sur /admin");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn x_robots_tag_on_api_even_401() {
    request::<App, _, _>(|request, _ctx| async move {
        // /api/projects sans session → 401, mais l'en-tête doit quand même être posé.
        let res = request.get("/api/projects").await;
        let tag = res
            .headers()
            .get("x-robots-tag")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(tag, "noindex, nofollow", "X-Robots-Tag manquant sur /api (401)");
    })
    .await;
}
```

- [ ] **Step 2 : Lancer le test, vérifier l'échec**

Run: `cargo nextest run --test hardening`
Expected: FAIL (route `/robots.txt` → 404 ; en-tête `x-robots-tag` absent).

- [ ] **Step 3 : Implémenter dans `after_routes`**

Dans `backend/src/app.rs`, ajouter les imports en tête (à côté des `use axum`) :

```rust
use axum::http::{HeaderName, HeaderValue};
use axum::routing::get;
```

Ajouter un handler libre (au-dessus de `impl Hooks for App`) :

```rust
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
```

Dans `after_routes`, **juste avant** `Ok(router)` final, ajouter la route puis le layer global :

```rust
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
```

> Le layer est posé **en dernier** : il englobe toutes les routes et tous les `nest_service` déjà montés (`/admin`, `/assets`, `/mcp`, `/c`, `/api`, `/robots.txt`).

- [ ] **Step 4 : Lancer le test, vérifier le succès**

Run: `cargo nextest run --test hardening`
Expected: PASS (3 tests).

- [ ] **Step 5 : fmt + clippy**

Run: `cargo fmt --all && cargo clippy --all-targets --all-features -- -D warnings`
Expected: 0 warning.

- [ ] **Step 6 : Commit**

```bash
git add backend/src/app.rs backend/tests/hardening.rs
git commit -m "🔒 feat(hardening): robots.txt + X-Robots-Tag servis par l'app + tests"
```

---

## Task 2 : E2E MCP — transport Streamable HTTP réel

**Files:**
- Test: `backend/tests/mcp_http.rs` (créer)

**Interfaces:**
- Consumes: harness `request::<App, _, _>` (confirmé : route bien les mounts `after_routes`, dont `/mcp`). Tools MCP : `deploy_prototype` (args `{slug, html, deploy_token, activate?}` → `DeployResult {url, version, code_protected}`), `list_projects` (args `{deploy_token}` → `{projects: [{slug, name, code_protected, active_version}]}`).
- Produces: aucune interface consommée plus loin (test terminal).

> **Note transport (résolu)** : le harness loco boot l'app une fois par closure ; le `StreamableHttpService` + `LocalSessionManager` (état `Arc`) persiste entre les `request.post()` d'une même closure → la session MCP ouverte à `initialize` reste valide pour les appels suivants. On capture le header de session de la réponse `initialize` et on le rejoue.
>
> **À ajuster à l'exécution si besoin** : la valeur exacte de `protocolVersion` et le nom du header de session (`mcp-session-id`) sont ceux de rmcp 1.8 ; si `initialize` échoue, lire la réponse réelle (status + body) pour récupérer la version annoncée par le serveur et le nom exact du header, puis aligner. Consigner toute surprise dans `docs/QUIRKS.md`.

- [ ] **Step 1 : Écrire un helper + le test `initialize` (échoue d'abord faute de fichier)**

Créer `backend/tests/mcp_http.rs` :

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]

use latch::app::App;
use latch::models::_entities::projects;
use loco_rs::testing::prelude::*;
use sea_orm::{ActiveModelTrait, Set};
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
fn parse_mcp_body(body: &str) -> serde_json::Value {
    let trimmed = body.trim_start();
    if trimmed.starts_with('{') {
        return serde_json::from_str(trimmed).expect("json direct");
    }
    // SSE : chercher la 1re ligne `data: ...`
    for line in body.lines() {
        if let Some(rest) = line.strip_prefix("data:") {
            return serde_json::from_str(rest.trim()).expect("json sse");
        }
    }
    panic!("corps MCP non parsable : {body}");
}

/// POST JSON-RPC vers /mcp avec les en-têtes requis par rmcp 1.8.
/// `session` = header de session à rejouer (None pour initialize).
async fn mcp_post(
    request: &loco_rs::testing::prelude::TestServer,
    body: serde_json::Value,
    session: Option<&str>,
) -> (axum::http::HeaderMap, serde_json::Value) {
    let mut req = request
        .post("/mcp")
        .add_header("accept", "application/json, text/event-stream")
        .add_header("content-type", "application/json")
        .json(&body);
    if let Some(sid) = session {
        req = req.add_header("mcp-session-id", sid);
    }
    let res = req.await;
    let headers = res.headers().clone();
    let value = parse_mcp_body(&res.text());
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
        let name = value["result"]["serverInfo"]["name"].as_str().unwrap_or("");
        assert_eq!(name, "latch", "serverInfo.name attendu = latch");
    })
    .await;
}
```

- [ ] **Step 2 : Lancer, vérifier l'échec puis le succès du handshake**

Run: `cargo nextest run --test mcp_http mcp_initialize_handshake`
Expected: d'abord ajuster si nécessaire (`protocolVersion`/header — cf. note transport), puis PASS.

- [ ] **Step 3 : Ajouter le test `tools/list`**

Ajouter dans `backend/tests/mcp_http.rs` :

```rust
#[tokio::test]
#[serial]
async fn mcp_tools_list_exposes_two_tools() {
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
        assert!(names.contains(&"deploy_prototype"), "deploy_prototype absent : {names:?}");
        assert!(names.contains(&"list_projects"), "list_projects absent : {names:?}");
    })
    .await;
}
```

- [ ] **Step 4 : Ajouter le test `deploy_prototype` (token valide, slug préexistant)**

```rust
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
        let sid = headers.get("mcp-session-id").unwrap().to_str().unwrap().to_string();

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
        assert_eq!(structured["url"], "http://localhost:5150/c/mon-projet-aaaaaaaa");
        assert_eq!(structured["version"], 1);
        assert_eq!(structured["code_protected"], true);

        // Invariant §9 : aucun PIN ni hash dans la réponse.
        let raw = value.to_string();
        assert!(!raw.contains("123456"), "le PIN ne doit jamais fuiter via MCP");
    })
    .await;
}
```

> **À ajuster à l'exécution** : l'emplacement exact du résultat (`structuredContent` vs `content[0].text` JSON) dépend de la sérialisation rmcp 1.8 du type `Json<DeployResult>`. Si `structuredContent` est absent, parser `result.content[0].text` (JSON encodé en texte) — adapter et noter dans QUIRKS.

- [ ] **Step 5 : Ajouter les tests `list_projects` + gate token rejeté**

```rust
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
        let sid = headers.get("mcp-session-id").unwrap().to_str().unwrap().to_string();

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
        let sid = headers.get("mcp-session-id").unwrap().to_str().unwrap().to_string();

        let body = serde_json::json!({
            "jsonrpc": "2.0", "id": 5, "method": "tools/call",
            "params": { "name": "list_projects", "arguments": { "deploy_token": "MAUVAIS" } }
        });
        let (_, value) = mcp_post(&request, body, Some(&sid)).await;
        // Le tool renvoie une erreur (isError ou error JSON-RPC) — pas de liste.
        let is_error = value["result"]["isError"].as_bool().unwrap_or(false)
            || value.get("error").is_some();
        assert!(is_error, "un deploy_token invalide doit être rejeté : {value}");
    })
    .await;
}
```

- [ ] **Step 6 : Lancer toute la suite MCP**

Run: `cargo nextest run --test mcp_http`
Expected: PASS (6 tests). Ajuster les chemins de résultat si la note s'applique.

- [ ] **Step 7 : fmt + clippy + commit**

```bash
cargo fmt --all && cargo clippy --all-targets --all-features -- -D warnings
git add backend/tests/mcp_http.rs
git commit -m "✅ test(mcp): e2e du transport Streamable HTTP réel (initialize, tools, gate token)"
```

---

## Task 3 : E2E navigateur — serving `/c` + unlock + bascule

**Files:**
- Create: `frontend/e2e/serve-unlock.spec.ts`
- Create: `frontend/e2e/fixtures/proto-v2.html`
- Reference: `frontend/e2e/admin-smoke.spec.ts`, `frontend/playwright.config.ts`, `openapi.json`

**Interfaces:**
- Consumes: `webServer` Playwright (`baseURL = http://127.0.0.1:5150`, `ADMIN_USER=admin`/`ADMIN_PASS=secret`). Setup via **API admin réelle** (pas l'UI) :
  - `POST /api/login` body `{ user, pass }` → 200 (login **public**, pas de garde Origin) ;
  - `POST /api/projects` body `{ name, code_enabled?, pin?, brand_name? }` → `ProjectDetail { id, slug, pin, code_enabled, versions, ... }` ;
  - `POST /api/projects/{id}/deploy` body `{ html, activate? }` → `DeployResponse { id, n }` ;
  - `POST /api/projects/{id}/versions/{n}/activate` → 200.
  - **Toutes les mutations** (`/api/projects*`) exigent l'en-tête `Origin` same-origin (garde `require_same_origin`, contrat §9.6) → ajouter `Origin: <baseURL>`.
- Produces: aucune (test terminal).

> **Pourquoi setup via API** : le smoke admin couvre déjà l'UI admin. Ici la cible est la surface **publique `/c`** ; piloter le setup par l'API est déterministe (slug + PIN lus dans la réponse `ProjectDetail`) et évite de dépendre des sélecteurs de formulaire. Marqueurs proto : v1 = chaîne « Demo proto » (fixture `proto.html` existante, inchangée) ; v2 = chaîne « PROTO-V2 » (nouvelle fixture).

- [ ] **Step 1 : Créer la 2ᵉ fixture HTML**

Créer `frontend/e2e/fixtures/proto-v2.html` :

```html
<!doctype html>
<html lang="en">
  <head><meta charset="utf-8" /><title>Demo proto v2</title></head>
  <body><h1>PROTO-V2 prototype</h1><p>Second version for e2e.</p></body>
</html>
```

(La fixture `proto.html` existante n'est **pas** modifiée : son marqueur est « Demo proto ».)

- [ ] **Step 2 : Écrire le spec e2e**

Créer `frontend/e2e/serve-unlock.spec.ts` :

```ts
import { test, expect, type APIRequestContext } from '@playwright/test'
import path from 'node:path'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const protoV1 = readFileSync(path.resolve(__dirname, 'fixtures/proto.html'), 'utf8')
const protoV2 = readFileSync(path.resolve(__dirname, 'fixtures/proto-v2.html'), 'utf8')

// Connexion admin via l'API (le cookie de session reste dans le contexte `request`).
async function apiLogin(request: APIRequestContext) {
  const res = await request.post('/api/login', { data: { user: 'admin', pass: 'secret' } })
  expect(res.ok()).toBeTruthy()
}

// Crée un projet via l'API. `Origin` requis (garde same-origin sur les mutations).
async function createProject(
  request: APIRequestContext,
  baseURL: string,
  opts: { name: string; code_enabled: boolean; pin?: string },
) {
  const res = await request.post('/api/projects', {
    headers: { Origin: baseURL },
    data: opts,
  })
  expect(res.ok()).toBeTruthy()
  return res.json() as Promise<{ id: number; slug: string; pin: string | null }>
}

async function deploy(
  request: APIRequestContext,
  baseURL: string,
  id: number,
  html: string,
  activate = true,
) {
  const res = await request.post(`/api/projects/${id}/deploy`, {
    headers: { Origin: baseURL },
    data: { html, activate },
  })
  expect(res.ok()).toBeTruthy()
  return res.json() as Promise<{ id: number; n: number }>
}

test('projet libre : /c sert le proto en no-store', async ({ request, baseURL }) => {
  await apiLogin(request)
  const project = await createProject(request, baseURL!, { name: 'ACME', code_enabled: false })
  await deploy(request, baseURL!, project.id, protoV1)

  const res = await request.get(`/c/${project.slug}`)
  expect(res.status()).toBe(200)
  expect(res.headers()['cache-control']).toContain('no-store')
  expect(await res.text()).toContain('Demo proto')
})

test('projet protégé : unlock par PIN puis proto servi', async ({ page, request, baseURL }) => {
  await apiLogin(request)
  const project = await createProject(request, baseURL!, {
    name: 'Mon Projet',
    code_enabled: true,
    pin: '135790', // PIN explicite → déterministe pour la saisie
  })
  await deploy(request, baseURL!, project.id, protoV1)

  // 1) Sans cookie → page d'unlock (l'input OTP #pin n'existe QUE sur l'unlock).
  await page.goto(`/c/${project.slug}`)
  await expect(page.locator('#pin')).toBeVisible()
  await expect(page.getByText('Demo proto')).toHaveCount(0)

  // 2) Mauvais PIN → reste sur l'unlock, proto non servi.
  await page.locator('#pin').click()
  await page.locator('#pin').pressSequentially('000000')
  await expect(page.locator('#pin')).toBeVisible()
  await expect(page.getByText('Demo proto')).toHaveCount(0)

  // 3) Bon PIN → auto-submit (onComplete) → cookie posé → reload → proto servi.
  await page.reload()
  await page.locator('#pin').click()
  await page.locator('#pin').pressSequentially('135790')
  await expect(page.getByText('Demo proto')).toBeVisible()
})

test('bascule de version : /c reflète la v2 activée', async ({ request, baseURL }) => {
  await apiLogin(request)
  const project = await createProject(request, baseURL!, { name: 'demo', code_enabled: false })
  await deploy(request, baseURL!, project.id, protoV1) // v1 active

  let res = await request.get(`/c/${project.slug}`)
  expect(await res.text()).toContain('Demo proto')

  const v2 = await deploy(request, baseURL!, project.id, protoV2) // v2 active
  expect(v2.n).toBe(2)

  res = await request.get(`/c/${project.slug}`)
  const body = await res.text()
  expect(body).toContain('PROTO-V2')
  expect(body).not.toContain('Demo proto')
})
```

> **À l'exécution** : si l'auto-submit OTP ne se déclenche pas via `pressSequentially`, replier sur `page.locator('#pin').fill('135790')` puis cliquer le bouton submit (`page.getByRole('button', { name: /unlock|submit|déverrou/i })`). Le PIN explicite à la création rend la saisie déterministe.

- [ ] **Step 3 : Lancer l'e2e (la stack monte via webServer)**

Run: `cd frontend && pnpm exec playwright test serve-unlock`
Expected: PASS (3 tests). En cas d'échec de bind, `LATCH_BINDING=127.0.0.1` est déjà câblé dans `playwright.config.ts`.

- [ ] **Step 4 : Lancer toute la suite e2e (non-régression du smoke)**

Run: `cd frontend && pnpm exec playwright test`
Expected: smoke admin + serve-unlock verts.

- [ ] **Step 5 : Commit**

```bash
git add frontend/e2e/serve-unlock.spec.ts frontend/e2e/fixtures/proto-v2.html
git commit -m "✅ test(e2e): serving /c libre + unlock par PIN + bascule de version"
```

---

## Task 4 : sonar.tests + vérification supply-chain

**Files:**
- Modify: `sonar-project.properties`

**Interfaces:** aucune (config + vérification).

- [ ] **Step 1 : Lire la valeur actuelle**

Run: `grep -n "sonar.tests\|sonar.sources" sonar-project.properties`
Expected: `sonar.tests=frontend/src` (et `sonar.sources` listant `backend/src`, etc.).

- [ ] **Step 2 : Modifier `sonar.tests`**

Éditer `sonar-project.properties` :

```properties
sonar.tests=frontend/src,backend/tests
```

> Aucun impact couverture (canal lcov Rust séparé). Classe enfin les tests d'intégration `backend/tests/*.rs` comme tests (et non code de prod ignoré). Ne corrige pas les tests inline `#[cfg(test)]` (granularité fichier — connue).

- [ ] **Step 3 : Vérifier la supply-chain**

Run: `cargo deny check 2>&1 | tail -20`
Expected: `licenses ok`, `advisories ok`. Si une licence (ex. `Zlib` via `utoipa-swagger-ui 9`) est rejetée, l'ajouter à `allow` dans `deny.toml` (modèle liste blanche, cf. QUIRKS) et re-vérifier.

Run: `cargo audit 2>&1 | tail -10`
Expected: pas de vulnérabilité bloquante (advisories informatives tolérées).

- [ ] **Step 4 : Commit**

```bash
git add sonar-project.properties deny.toml
git commit -m "🧱 chore(sonar): classer backend/tests comme tests + supply-chain vérifiée"
```

---

## Task 5 : Captures Playwright (hors CI)

**Files:**
- Create: `frontend/e2e/screenshots.capture.ts`
- Create: `docs/assets/admin-list.png`, `docs/assets/unlock.png`

**Interfaces:** Consumes le `webServer` Playwright + les helpers de setup API de Task 3. Produces les 2 PNG référencés par le README (Task 7).

> **Pourquoi dans `e2e/` + skip conditionnel** : `playwright.config.ts` a `testDir: './e2e'`. Un fichier sous `scripts/` ne serait **pas** découvert ; un fichier sous `e2e/` tournerait en CI (indésirable — on ne régénère pas les captures à chaque run). Solution : le placer dans `e2e/` et le **skipper sauf si `CAPTURE=1`** via `test.skip(!process.env.CAPTURE, ...)`. Nom en `.capture.ts` (pas `.spec.ts`) pour le distinguer visuellement.

- [ ] **Step 1 : Écrire le script de capture**

Créer `frontend/e2e/screenshots.capture.ts` (réutilise le setup API de Task 3) :

```ts
import { test, type APIRequestContext } from '@playwright/test'
import path from 'node:path'
import { readFileSync, mkdirSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

// Skippé par défaut : ne tourne que lancé explicitement avec CAPTURE=1.
test.skip(!process.env.CAPTURE, 'capture manuelle uniquement (CAPTURE=1)')

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const assetsDir = path.resolve(__dirname, '../../docs/assets')
const protoV1 = readFileSync(path.resolve(__dirname, 'fixtures/proto.html'), 'utf8')
mkdirSync(assetsDir, { recursive: true })

async function apiLogin(request: APIRequestContext) {
  await request.post('/api/login', { data: { user: 'admin', pass: 'secret' } })
}
async function createDeployed(
  request: APIRequestContext, baseURL: string,
  name: string, code_enabled: boolean, pin?: string,
) {
  const res = await request.post('/api/projects', {
    headers: { Origin: baseURL }, data: { name, code_enabled, pin },
  })
  const project = await res.json()
  await request.post(`/api/projects/${project.id}/deploy`, {
    headers: { Origin: baseURL }, data: { html: protoV1, activate: true },
  })
  return project as { id: number; slug: string }
}

test('capture liste admin', async ({ page, request, baseURL }) => {
  await apiLogin(request)
  // 2 projets de démo FACTICES : "Mon Projet" (protégé) + "ACME" (libre).
  await createDeployed(request, baseURL!, 'Mon Projet', true, '135790')
  await createDeployed(request, baseURL!, 'ACME', false)
  await page.goto('/admin/login')
  await page.getByLabel('Username').fill('admin')
  await page.getByLabel('Password').fill('secret')
  await page.getByRole('button', { name: 'Sign in' }).click()
  await page.waitForURL(/\/admin\/?$/)
  await page.screenshot({ path: `${assetsDir}/admin-list.png`, fullPage: true })
})

test('capture page unlock', async ({ page, request, baseURL }) => {
  await apiLogin(request)
  const project = await createDeployed(request, baseURL!, 'Mon Projet', true, '135790')
  await page.goto(`/c/${project.slug}`)
  await page.locator('#pin').waitFor()
  await page.screenshot({ path: `${assetsDir}/unlock.png`, fullPage: true })
})
```

> Données **manifestement fictives** (`Mon Projet`, `ACME`). Aucun nom client.

- [ ] **Step 2 : Générer les captures (manuel, avec un serveur frais)**

Run: `cd frontend && CAPTURE=1 CI=1 pnpm exec playwright test screenshots.capture`
(`CI=1` force `reuseExistingServer:false` → backend neuf incluant les routes récentes.)
Expected: `docs/assets/admin-list.png` + `docs/assets/unlock.png` créés. Inspecter visuellement (pas de nom client, rendu correct).

- [ ] **Step 3 : Vérifier que la capture ne tourne PAS en run normal**

Run: `cd frontend && pnpm exec playwright test screenshots.capture`
Expected: 2 tests **skipped** (sans `CAPTURE=1`).

- [ ] **Step 4 : Commit**

```bash
git add frontend/e2e/screenshots.capture.ts docs/assets/admin-list.png docs/assets/unlock.png
git commit -m "📝 docs(assets): script de capture Playwright (skip sauf CAPTURE=1) + captures admin/unlock"
```

---

## Task 6 : CHANGELOG via git-cliff

**Files:**
- Create: `cliff.toml`, `CHANGELOG.md`

**Interfaces:** aucune (outillage + artefact doc).

- [ ] **Step 1 : Vérifier git-cliff disponible**

Run: `git cliff --version || cargo install git-cliff`
Expected: une version affichée (installer si absent).

- [ ] **Step 2 : Écrire `cliff.toml` avec preprocessor gitmoji**

Créer `cliff.toml` à la racine :

```toml
[changelog]
header = "# Changelog\n\nToutes les évolutions notables de latch. Format inspiré de Keep a Changelog ; versionnage SemVer.\n"
body = """
{% for group, commits in commits | group_by(attribute="group") %}
### {{ group | upper_first }}
{% for commit in commits %}
- {{ commit.message | upper_first }}\
{% endfor %}
{% endfor %}
"""
trim = true

[git]
conventional_commits = true
filter_unconventional = true
# Strippe le gitmoji (et l'espace) en tête de message AVANT le parsing conventionnel.
commit_preprocessors = [
    { pattern = '^[\p{Emoji_Presentation}\p{Extended_Pictographic}\u{FE0F}\u{200D}]+\s*', replace = "" },
]
commit_parsers = [
    { message = "^feat", group = "Ajouts" },
    { message = "^fix", group = "Corrections" },
    { message = "^(chore|refactor|perf|style)", group = "Interne" },
    { message = "^docs", group = "Documentation" },
    { message = "^test", group = "Tests" },
    { message = "^.*\\bsécu|hardening|robots\\b", group = "Sécurité" },
    { message = ".*", group = "Divers" },
]
protect_breaking_commits = true
tag_pattern = "v[0-9]*"
```

- [ ] **Step 3 : Générer le CHANGELOG et vérifier le parsing gitmoji**

Run: `git cliff --tag v0.1.0 --output CHANGELOG.md`
Expected: `CHANGELOG.md` créé, entrée `[v0.1.0]` peuplée, **descriptions sans gitmoji résiduel** (vérifier que `✨`/`🐛`/… ont bien été strippés et que les commits sont regroupés en sections).

- [ ] **Step 4 : Relecture manuelle**

Run: `head -60 CHANGELOG.md`
Vérifier : aucun nom client, sections cohérentes (Ajouts/Corrections/Sécurité/Documentation…), descriptions lisibles. Corriger le `cliff.toml` (regex preprocessor ou parsers) si un emoji subsiste ou si un commit est mal classé, puis régénérer.

- [ ] **Step 5 : Commit**

```bash
git add cliff.toml CHANGELOG.md
git commit -m "📝 docs(changelog): CHANGELOG via git-cliff (preprocessor gitmoji) — v0.1.0"
```

---

## Task 7 : README — refonte complète

**Files:**
- Modify: `README.md` (réécriture)

**Interfaces:** Consumes `docs/assets/*.png` (Task 5). Aucune sortie consommée plus loin.

- [ ] **Step 1 : Réécrire `README.md`**

Structure (cf. spec §4.1), tout en français, docs succinctes → liens TBD Phase 8. Remplacer le contenu par :

1. **En-tête** : titre + badges (CI + Quality Gate + Coverage + License) :

```md
[![CI](https://github.com/owlnext-fr/latch/actions/workflows/ci.yml/badge.svg)](https://github.com/owlnext-fr/latch/actions/workflows/ci.yml)
[![Quality Gate](https://sonarcloud.io/api/project_badges/measure?project=owlnext-fr_latch&metric=alert_status)](https://sonarcloud.io/summary/new_code?id=owlnext-fr_latch)
[![Coverage](https://sonarcloud.io/api/project_badges/measure?project=owlnext-fr_latch&metric=coverage)](https://sonarcloud.io/summary/new_code?id=owlnext-fr_latch)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#licence)
```

2. **Captures** : `![Admin](docs/assets/admin-list.png)` + `![Déverrouillage](docs/assets/unlock.png)`.
3. **Les trois surfaces** (`/c`, `/admin`, `/mcp`) — condensé.
4. **Quickstart** — Docker (`cp .env.example .env` → secrets → `docker compose up -d`) avec tableau des variables **obligatoires en prod** (`ADMIN_USER`, `ADMIN_PASS`, `DEPLOY_TOKEN`, `LATCH_PUBLIC_BASE_URL`, `SESSION_SECRET`, `UNLOCK_COOKIE_SECRET`) + `openssl rand -hex 32` ; puis Dev local (`cd backend && cargo loco start` ; `cd frontend && pnpm dev`).
5. **Connecter Claude (MCP)** — 3 étapes (Settings → connecteur → `list_projects`) + « → doc détaillée (à venir, Phase 8) ».
6. **Architecture** — couches (cœur agnostique HTTP + adaptateurs) + 3 invariants sécu en puces, renvoi `docs/contrat-deploy.md`.
7. **Stack** — backend / frontend (condensé).
8. **Développement & Qualité** — commandes clés + gate Sonar `new_coverage ≥ 80%`, renvoi `docs/BOOTSTRAP.md`.
9. **Déploiement** — GHCR public + `deploy.sh`, renvoi `docs/BOOTSTRAP.md §7-8`.
10. **Sécurité & confidentialité** — `robots.txt` + `X-Robots-Tag` servis par l'app ; le vrai gating reste l'auth.
11. **Licence** — dual MIT/Apache + lien `CHANGELOG.md`.

Inclure en tête de la zone docs un lien proéminent :
`> 📚 Documentation détaillée (quickstart approfondi, guides) : https://latch.owlnext.fr/docs *(à venir — Phase 8, Fumadocs)*`

- [ ] **Step 2 : Vérifier le rendu Markdown + absence de nom client**

Run: `grep -niE "client|owlnext-fr/latch" README.md | head` puis relire visuellement (les images se résolvent en chemins relatifs `docs/assets/...`).
Expected: aucun nom de client réel ; liens cohérents.

- [ ] **Step 3 : Commit**

```bash
git add README.md
git commit -m "📝 docs(readme): refonte complète (badges Sonar, captures, quickstart, archi, sécurité)"
```

---

## Task 8 : Vérification finale + mise à jour mémoire

**Files:**
- Review: `.env.example`, `deploy.sh`
- Modify: `docs/INDEX.md`, `docs/HANDOFF.md`, `docs/ENVIRONMENT.md`, `docs/QUIRKS.md`, `docs/CONVENTIONS.md`, `docs/ROADMAP.md` (stub Phase 8), `docs/BACKLOG.md` (si items reportés)

**Interfaces:** aucune (clôture).

- [ ] **Step 0 : Relecture `.env.example` + `deploy.sh` (spec §4.4)**

Run: `grep -nE "ADMIN_USER|ADMIN_PASS|DEPLOY_TOKEN|LATCH_PUBLIC_BASE_URL|SESSION_SECRET|UNLOCK_COOKIE_SECRET|LATCH_IMAGE_TAG" .env.example`
Vérifier que **toutes** les variables obligatoires en prod sont présentes et commentées (déjà à jour Phase 5 — confirmation, pas de réécriture attendue). Si une manque, l'ajouter.
Run: `cat deploy.sh`
Vérifier : `set -euo pipefail`, `docker compose pull/up -d/image prune -f`, garde `chown 65532:65532 data` idempotente. Pas de test sur la box (humain). Aucun secret en dur.

- [ ] **Step 1 : Suite de qualité complète (backend)**

Run: `cargo fmt --all --check && cargo clippy --all-targets --all-features -- -D warnings && cargo nextest run`
Expected: tout vert (dont `hardening`, `mcp_http`).

- [ ] **Step 2 : Suite frontend + e2e**

Run: `cd frontend && pnpm lint && pnpm typecheck && pnpm test && pnpm exec playwright test`
Expected: tout vert.

- [ ] **Step 3 : Scan Sonar local (optionnel mais recommandé avant push)**

Suivre `docs/ENVIRONMENT.md §Scan local` (générer `backend-lcov.info` + `coverage/lcov.info`, **remap des chemins `/usr/src`**, scan Docker scoped sur la branche).
Expected: gate `new_coverage ≥ 80%` verte.

- [ ] **Step 4 : Mettre à jour la mémoire**

- `docs/INDEX.md` : lignes Phase 6 (hardening robots/X-Robots-Tag, e2e MCP HTTP, e2e serve/unlock, sonar.tests, README/CHANGELOG/captures).
- `docs/HANDOFF.md` : entrée datée en tête (Dernière chose faite / Trucs en suspens / Prochaine chose à creuser / Notes pour future Claude).
- `docs/ENVIRONMENT.md` : git-cliff (toolchain), note captures, badges Sonar (visibilité publique requise).
- `docs/QUIRKS.md` : pièges découverts (transport MCP en test : emplacement du résultat `structuredContent` vs `content[0].text`, header de session ; preprocessor gitmoji git-cliff ; layer `X-Robots-Tag` global).
- `docs/CONVENTIONS.md` : pattern « test e2e transport MCP via harness loco » + « durcissement en-tête global dans after_routes ».
- `docs/ROADMAP.md` : marquer Phase 6 LIVRÉE + ajouter un **stub Phase 8 (Fumadocs)** (landing + doc détaillée GH Pages / serving, lien doc TBD du README).
- `docs/BACKLOG.md` : reporter ce qui n'a pas été fait (ex. Caddyfile d'exemple si jamais ré-évoqué, git-cliff en CI).

- [ ] **Step 5 : Commit mémoire**

```bash
git add docs/
git commit -m "📝 docs(phase-6): Phase 6 LIVRÉE — mémoire à jour + stub Phase 8 (Fumadocs)"
```

- [ ] **Step 6 : Revue de branche avant merge**

Demander une revue finale (opus) du diff de branche `main..feat/phase-6-finalisation` avant de proposer le merge/PR (via la skill `finishing-a-development-branch`). Ne pas merger sans revue verte.

---

## Notes d'exécution

- **Ordre conseillé** : Task 1 → 4 d'abord (backend/config, rapides et indépendants), puis 2 (MCP, le point de risque), puis 3 (e2e nav), puis 5 → 7 (packaging), puis 8 (clôture). Tasks 5/7 sont couplées (captures avant README).
- **Point de risque unique** : Task 2 (emplacement du résultat dans la réponse MCP + handshake). Les notes « À ajuster à l'exécution » disent quoi inspecter ; consigner dans QUIRKS.
- **Confidentialité** : revérifier qu'aucune capture ni fixture ne porte de nom client avant chaque commit.
