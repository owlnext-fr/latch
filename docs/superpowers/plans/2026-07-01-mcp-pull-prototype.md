# MCP `pull_prototype` — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ajouter un 3ᵉ tool MCP `pull_prototype(slug, version?, deploy_token)` qui renvoie le HTML brut d'une version d'un prototype + tous ses fils de commentaires (visiteurs + admin), pour un `/latch-pull` côté Claude.

**Architecture:** Adaptateur entrant MCP fin (`backend/src/mcp/mod.rs`) : gate `deploy_token` d'abord, puis orchestration de méthodes de cœur (`ProjectsService`, `CommentsService`, `Storage`). Deux petites méthodes de résolution de version ajoutées au cœur. Nouveaux DTO de sortie propres à MCP (sans `owner_token`/PIN/hash/id). Lecture seule.

**Tech Stack:** Rust, rmcp 1.8 (`#[tool_router]`/`#[tool]`, `Parameters<_>` → `Json<_>`), SeaORM, Loco. Tests : handler-level (`.await` direct) + e2e Streamable HTTP (`axum-test` via `request::<App>`).

## Global Constraints

- **Token gate en PREMIER** sur le tool (`secure_compare`), avant tout accès DB (contrat §9.3). Rejet → `ErrorData::invalid_params("deploy_token invalide", None)`.
- **`owner_token` JAMAIS sérialisé** (contrat §9.7) : les DTO de sortie n'ont pas le champ ; on expose `is_admin: bool` dérivé (`owner_token == ADMIN_OWNER_TOKEN`).
- **Jamais de PIN, hash, ni `id` DB** dans une réponse MCP (contrat §5.1/§9.1/§9.2).
- **Ancre non interprétée** (contrat §3) : le descripteur `anchor` est passé brut, zéro parsing serveur.
- **Schéma de sortie racine = objet** (jamais `array` à la racine) sinon rmcp panique au boot (cf. `docs/QUIRKS.md`). `PullResult` est un objet → OK.
- **Le cœur (`src/services/`) ne contient ni `use axum::` ni `use loco_rs::`** (contrat §1, garde `backend/tests/architecture.rs`).
- Pas de `unwrap`/`expect` hors tests. `cargo fmt` + `cargo clippy --all-targets -- -D warnings` verts.
- Commits conventionnels + gitmoji.
- **Pas** de régénération `openapi.json` (les schémas MCP sont générés par rmcp, hors OpenAPI REST).

---

## Fichiers touchés

- **Modifier** `backend/src/services/projects.rs` — ajouter `get_version` + `get_active_version` (+ tests).
- **Modifier** `backend/src/mcp/mod.rs` — `PullArgs`, `PullResult`, `PullThread`, `PullMessage`, tool `pull_prototype`, helper `map_version_err`, MAJ de la string `instructions` de `get_info`, tests handler.
- **Modifier** `backend/tests/mcp_http.rs` — passer le test tools/list à 3 tools, ajouter un test e2e `pull_prototype`.
- **Modifier** `docs/contrat-deploy.md` — §5 (3ᵉ tool) + §5.1 (forme `PullResult`).
- **Modifier** `public_docs/content/docs/publish-from-claude/tools-reference.mdx` — documenter le tool.
- **Modifier** `docs/INDEX.md`, `docs/HANDOFF.md` — fin d'implémentation.

---

## Task 1 : Cœur — résolution de version

**Files:**
- Modify: `backend/src/services/projects.rs` (impl `ProjectsService` + `#[cfg(test)] mod tests`)

**Interfaces:**
- Consomme : `ProjectsService::new(db)`, `ProjectsService::create(CreateProject)`, `ProjectsService::get_by_slug(&str)`, `DeployService::deploy(project_id, html, activate, release_notes)` — existants.
- Produit : 
  - `pub async fn get_version(&self, project_id: i32, n: i32) -> Result<versions::Model, CoreError>` (NotFound si absente)
  - `pub async fn get_active_version(&self, project: &projects::Model) -> Result<versions::Model, CoreError>` (NotFound si pas de pointeur ou version disparue)

- [ ] **Step 1 : Écrire les tests qui échouent**

Ajouter dans le `#[cfg(test)] mod tests` de `backend/src/services/projects.rs` :

```rust
#[tokio::test]
async fn get_version_by_n_and_missing() {
    let db = test_db().await;
    let dir = tempfile::tempdir().unwrap();
    let storage: std::sync::Arc<dyn crate::services::storage::Storage> =
        std::sync::Arc::new(crate::services::storage::FsStorage::new(dir.path().to_path_buf()));
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
    let storage: std::sync::Arc<dyn crate::services::storage::Storage> =
        std::sync::Arc::new(crate::services::storage::FsStorage::new(dir.path().to_path_buf()));
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
```

- [ ] **Step 2 : Lancer les tests → échec de compilation**

Run: `cargo test -p latch --lib services::projects::tests::get_version_by_n_and_missing`
Expected: FAIL — `no method named get_version found` (et `get_active_version`).

- [ ] **Step 3 : Implémenter les deux méthodes**

Ajouter dans `impl ProjectsService` (après `get_by_slug`) :

```rust
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
```

(Les traits `EntityTrait`/`ColumnTrait`/`QueryFilter` et les entités `projects`/`versions` sont déjà importés en tête du fichier.)

- [ ] **Step 4 : Lancer les tests → succès**

Run: `cargo test -p latch --lib services::projects::tests::get_version_by_n_and_missing services::projects::tests::get_active_version_via_pointer_and_none`
Expected: PASS (2 tests).

- [ ] **Step 5 : Gate + commit**

Run: `cargo fmt --all && cargo clippy --all-targets -- -D warnings`
Expected: 0 warning.

```bash
git add backend/src/services/projects.rs
git commit -m "✨ feat(core): résolution de version (get_version, get_active_version)"
```

---

## Task 2 : Tool MCP `pull_prototype`

**Files:**
- Modify: `backend/src/mcp/mod.rs` (DTO + tool + helper + `get_info` + tests)

**Interfaces:**
- Consomme : `ProjectsService::{get_by_slug, get_version, get_active_version}` (Task 1), `CommentsService::new(db)`, `CommentsService::list_for_version(version_id) -> Vec<PinWithMessages>`, `PinWithMessages { pin: comment_pins::Model, messages: Vec<comments::Model> }`, `ADMIN_OWNER_TOKEN`, `Storage::read(&str) -> Result<String, CoreError>`, `self.public_base_url`.
- Produit : tool `pull_prototype` + types `PullResult { slug, version, url, comments_enabled, release_notes, html, threads }`, `PullThread { anchor, messages }`, `PullMessage { author_name, is_admin, body, created_at }`.

- [ ] **Step 1 : Écrire les tests handler qui échouent**

Ajouter dans le `#[cfg(test)] mod tests` de `backend/src/mcp/mod.rs`. Helper de seed (à ajouter en haut du module de tests) :

```rust
use crate::services::comments::{CommentsService, ADMIN_OWNER_TOKEN};

/// Déploie une version v1 (HTML donné) sur `project_id` et l'active. Renvoie le n.
async fn deploy_v1(db: &DatabaseConnection, dir: &TempDir, project_id: i32, html: &str) {
    let storage: Arc<dyn Storage> = Arc::new(FsStorage::new(dir.path().to_path_buf()));
    DeployService::new(db.clone(), storage)
        .deploy(project_id, html, true, None)
        .await
        .unwrap();
}
```

Tests :

```rust
#[tokio::test]
async fn pull_rejects_bad_token() {
    let db = test_db().await;
    let dir = tempfile::tempdir().unwrap();
    let p = make_project(&db, false).await;
    let m = mcp(db, &dir);
    let res = m
        .pull_prototype(Parameters(PullArgs {
            slug: p.slug.clone(),
            version: None,
            deploy_token: "WRONG".to_string(),
        }))
        .await;
    let err = match res {
        Err(e) => e,
        Ok(_) => panic!("token invalide doit être rejeté"),
    };
    assert_eq!(err.message, "deploy_token invalide");
}

#[tokio::test]
async fn pull_unknown_slug_is_error() {
    let db = test_db().await;
    let dir = tempfile::tempdir().unwrap();
    let m = mcp(db, &dir);
    let res = m
        .pull_prototype(Parameters(PullArgs {
            slug: "nope-xxxxxxxx".to_string(),
            version: None,
            deploy_token: TOKEN.to_string(),
        }))
        .await;
    assert!(res.is_err(), "slug inconnu → erreur");
}

#[tokio::test]
async fn pull_no_active_version_is_error() {
    let db = test_db().await;
    let dir = tempfile::tempdir().unwrap();
    let p = make_project(&db, false).await; // créé mais jamais déployé
    let m = mcp(db, &dir);
    let res = m
        .pull_prototype(Parameters(PullArgs {
            slug: p.slug.clone(),
            version: None,
            deploy_token: TOKEN.to_string(),
        }))
        .await;
    let err = match res {
        Err(e) => e,
        Ok(_) => panic!("aucune version active → erreur"),
    };
    assert_eq!(err.message, "aucune version active");
}

#[tokio::test]
async fn pull_returns_html_and_default_active_version() {
    let db = test_db().await;
    let dir = tempfile::tempdir().unwrap();
    let p = make_project(&db, false).await;
    deploy_v1(&db, &dir, p.id, "<h1>hello</h1>").await;
    let m = mcp(db, &dir);
    let Json(out) = m
        .pull_prototype(Parameters(PullArgs {
            slug: p.slug.clone(),
            version: None,
            deploy_token: TOKEN.to_string(),
        }))
        .await
        .unwrap();
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
    let res = m
        .pull_prototype(Parameters(PullArgs {
            slug: p.slug.clone(),
            version: Some(99),
            deploy_token: TOKEN.to_string(),
        }))
        .await;
    let err = match res {
        Err(e) => e,
        Ok(_) => panic!("version inconnue → erreur"),
    };
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
        .create_pin(version.id, visitor_token, "Alice", "hello from visitor", "{\"v\":1}")
        .await
        .unwrap();
    comments
        .create_pin(version.id, ADMIN_OWNER_TOKEN, "admin", "note admin", "{\"v\":1}")
        .await
        .unwrap();

    let m = mcp(db, &dir);
    let Json(out) = m
        .pull_prototype(Parameters(PullArgs {
            slug: p.slug.clone(),
            version: None,
            deploy_token: TOKEN.to_string(),
        }))
        .await
        .unwrap();

    assert_eq!(out.threads.len(), 2);
    let all_msgs: Vec<&PullMessage> = out.threads.iter().flat_map(|t| &t.messages).collect();
    // is_admin correct : exactement 1 message admin.
    assert_eq!(all_msgs.iter().filter(|m| m.is_admin).count(), 1);
    assert_eq!(all_msgs.iter().filter(|m| !m.is_admin).count(), 1);
    // anchor brut présent.
    assert!(out.threads.iter().all(|t| t.anchor.contains("\"v\"")));

    // Invariants : owner_token (visiteur ET sentinelle) et PIN jamais sérialisés.
    let json = serde_json::to_string(&out).unwrap();
    assert!(!json.contains(visitor_token), "owner_token visiteur ne doit pas fuiter");
    assert!(!json.contains(ADMIN_OWNER_TOKEN), "sentinelle admin ne doit pas fuiter");
    assert!(!json.contains("owner_token"), "pas de champ owner_token");
    assert!(!json.contains("123456"), "le PIN ne doit jamais fuiter via MCP");
}
```

- [ ] **Step 2 : Lancer les tests → échec de compilation**

Run: `cargo test -p latch --lib mcp::tests::pull_returns_html_and_default_active_version`
Expected: FAIL — `PullArgs`/`pull_prototype`/`PullMessage` inexistants.

- [ ] **Step 3 : Ajouter les DTO**

Dans `backend/src/mcp/mod.rs`, après `struct ListArgs` / les `*Result` existants :

```rust
/// Arguments du tool `pull_prototype`.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct PullArgs {
    /// Slug public du projet (doit exister).
    slug: String,
    /// Numéro de version à récupérer ; omis → version active.
    #[serde(default)]
    version: Option<i32>,
    /// Secret de déploiement (validé contre DEPLOY_TOKEN).
    deploy_token: String,
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
```

- [ ] **Step 4 : Ajouter le helper d'erreur de version**

À côté de `map_core_err` (bas du fichier) :

```rust
/// `NotFound` sur une résolution de version → message dédié ; autres erreurs → `map_core_err`.
fn map_version_err(e: CoreError, not_found_msg: &'static str) -> ErrorData {
    match e {
        CoreError::NotFound => ErrorData::invalid_params(not_found_msg, None),
        other => map_core_err(other),
    }
}
```

- [ ] **Step 5 : Implémenter le tool**

Dans `#[tool_router] impl LatchMcp`, après `list_projects` :

```rust
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

    let projects = ProjectsService::new(self.db.clone());
    let project = projects.get_by_slug(&args.slug).await.map_err(map_core_err)?;

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
        .map_err(map_core_err)?;

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
                    is_admin: msg.owner_token
                        == crate::services::comments::ADMIN_OWNER_TOKEN,
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
```

- [ ] **Step 6 : Mettre à jour la string `instructions` de `get_info`**

Dans `get_info`, remplacer la liste des outils :

```rust
info.with_instructions(
    "latch — déploiement de prototypes HTML. Outils : deploy_prototype, \
     list_projects, pull_prototype. Chaque appel exige le deploy_token.",
)
```

- [ ] **Step 7 : Lancer les tests handler → succès**

Run: `cargo test -p latch --lib mcp::tests`
Expected: PASS (tous les tests mcp, dont les 6 nouveaux `pull_*`).

- [ ] **Step 8 : Gate + commit**

Run: `cargo fmt --all && cargo clippy --all-targets -- -D warnings`
Expected: 0 warning.

```bash
git add backend/src/mcp/mod.rs
git commit -m "✨ feat(mcp): tool pull_prototype (HTML + fils de commentaires)"
```

---

## Task 3 : e2e transport HTTP

**Files:**
- Modify: `backend/tests/mcp_http.rs`

**Interfaces:**
- Consomme : helpers existants `setup_env()`, `init_body()`, `mcp_post(&request, body, session)`, constante `TOKEN`, entité `projects::ActiveModel`.

- [ ] **Step 1 : Passer le test tools/list à 3 tools (échec attendu)**

Dans `mcp_http.rs`, renommer `mcp_tools_list_exposes_two_tools` → `mcp_tools_list_exposes_three_tools` et remplacer l'assertion de compte + ajouter `pull_prototype` :

```rust
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
```

- [ ] **Step 2 : Ajouter le test e2e roundtrip**

Ajouter en fin de `mcp_http.rs` :

```rust
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
        assert_eq!(structured["url"], "http://localhost:5150/c/pull-me-cccccccc");
        assert_eq!(structured["comments_enabled"], true);
        assert!(structured["threads"].as_array().unwrap().is_empty());
    })
    .await;
}
```

> `projects`/`versions` et `Set` sont déjà importés en tête de `mcp_http.rs`. `latch::web::storage_from_ctx` (`pub fn` → `Arc<dyn Storage>`) et `latch::services::deploy::DeployService` sont bien publics (vérifié) — utilisables directement.

- [ ] **Step 3 : Lancer les tests e2e → succès**

Run: `cargo test -p latch --test mcp_http`
Expected: PASS (dont `mcp_tools_list_exposes_three_tools` et `mcp_pull_prototype_roundtrip`). Le boot ne panique pas → confirme que le schéma de sortie `PullResult` est bien un objet racine.

- [ ] **Step 4 : Gate complète backend + commit**

Run: `cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo nextest run`
Expected: 0 warning, tous les tests verts.

```bash
git add backend/tests/mcp_http.rs
git commit -m "✅ test(mcp): e2e HTTP pull_prototype + tools/list à 3"
```

---

## Task 4 : Documentation & mémoire

**Files:**
- Modify: `docs/contrat-deploy.md`, `public_docs/content/docs/publish-from-claude/tools-reference.mdx`, `docs/INDEX.md`, `docs/HANDOFF.md`

- [ ] **Step 1 : Contrat §5 — mentionner le 3ᵉ tool**

Dans `docs/contrat-deploy.md` §5, à « Surface minimale », ajouter `pull_prototype` : « `pull_prototype(slug, version?, deploy_token)` — lecture seule : renvoie le HTML d'une version + tous ses fils de commentaires (visiteurs + admin), pour itérer côté Claude. Gardé par le token comme les autres. »

- [ ] **Step 2 : Contrat §5.1 — forme de la réponse**

Ajouter un bloc « `pull_prototype(...)` » sous les deux tools existants décrivant `PullResult { slug, version, url, comments_enabled, release_notes?, html, threads[] }`, `PullThread { anchor (brut), messages[] }`, `PullMessage { author_name, is_admin, body, created_at }`. Réaffirmer : **jamais** `owner_token`/PIN/hash/`id` DB ; ancre passée brute.

- [ ] **Step 3 : Doc publique**

Dans `public_docs/content/docs/publish-from-claude/tools-reference.mdx`, ajouter une section EN « `pull_prototype` » : usage (`/latch-pull`), arguments (`slug`, optional `version`, `deploy_token`), et la forme de la réponse (HTML + comment threads). Vérifier le build : `cd public_docs && pnpm exec fumadocs-mdx && pnpm types:check`.

- [ ] **Step 4 : Mémoire projet**

- `docs/INDEX.md` : ligne sous « Phase 5 — Endpoint MCP » → `pull_prototype` livré (Issue #2).
- `docs/HANDOFF.md` : entrée datée (dernière chose faite / trucs en suspens / notes future Claude).

- [ ] **Step 5 : Commit**

```bash
git add docs/contrat-deploy.md public_docs/content/docs/publish-from-claude/tools-reference.mdx docs/INDEX.md docs/HANDOFF.md
git commit -m "📝 docs(mcp): documente pull_prototype (contrat §5/§5.1 + public_docs + mémoire)"
```

---

## Self-review (rempli à la rédaction)

- **Couverture spec :** §3 signature/séquence → Task 2 Step 5 ; §4 réponse → Task 2 Step 3 ; §5 cœur réutilisé + méthode version → Task 1 + Task 2 ; §6 erreurs → Task 2 (map_core_err + map_version_err) ; §7 invariants → Task 2 test `pull_returns_threads_with_is_admin_and_no_owner_token` + `pull_rejects_bad_token` ; §8 tests → Task 2 (handler) + Task 3 (e2e) ; §9 doc → Task 4. ✅ Aucun trou.
- **Placeholders :** aucun — tout le code est fourni. ✅
- **Cohérence de types :** `PullArgs`/`PullResult`/`PullThread`/`PullMessage`, `get_version`/`get_active_version`, `map_version_err`, `list_for_version`/`PinWithMessages`, `ADMIN_OWNER_TOKEN`, `to_rfc3339()` — noms utilisés à l'identique entre tâches et conformes aux signatures réelles vérifiées dans le code. ✅
- **Fallback e2e storage** documenté (Task 3 Step 2) si `storage_from_ctx` non `pub`.
