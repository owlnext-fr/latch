# Notes de version — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Permettre la saisie de notes de version en markdown léger au déploiement (admin + MCP), et afficher ces notes en overlay au premier passage d'un visiteur sur une nouvelle version de `/c/<slug>`, via une page-coquille (shell) systématique qui encadre le prototype dans une iframe.

**Architecture:** `/c/<slug>` sert désormais **toujours** un shell (mini-SPA Vite, même moule que `unlock.html`) qui charge le prototype dans `<iframe src="/c/<slug>/raw">` et affiche un overlay de notes. Le markdown est stocké brut en base ; la barrière XSS est le **rendu restreint** (`react-markdown` sans `a`/`img`/HTML brut), partagé entre l'overlay client et l'aperçu admin. Le « déjà vu » est mémorisé en `localStorage` côté navigateur.

**Tech Stack:** Rust / Loco / SeaORM / rmcp (backend) ; React / Vite / TypeScript / TanStack Query / Tiptap / react-markdown / react-i18next (frontend).

## Global Constraints

- **Périmètre markdown unique** éditeur ET rendu : paragraphes, titres (`h1`…`h6`), `strong`/`em`, listes `ul`/`ol`/`li`, `blockquote`. **Interdits** : `a`, `img`, `code`, HTML brut. Identique des deux côtés.
- **Limite notes : 10 000 caractères** (comptage `chars()`), validée côté service → `CoreError::Validation` (HTTP 400 / `invalid_params` MCP).
- **Markdown stocké brut** (colonne `release_notes TEXT NULL`). Rendu côté client uniquement.
- **Invariants de sécurité (contrat) préservés** : aucune réponse ne renvoie de hash ; le PIN n'apparaît qu'au détail projet admin ; `deploy_token` validé sur tous les tools MCP. Les notes ne contiennent jamais de secret.
- **`Cache-Control: no-store`** sur tout `/c/*`. `/c/<slug>/raw` ajoute `Content-Security-Policy: frame-ancestors 'self'`.
- **Confidentialité** : aucun nom de client réel nulle part (placeholders fictifs : `Mon Projet`, `ACME`, `demo`).
- **Definition of done** : `cargo fmt` + `cargo clippy --all-targets -- -D warnings` verts ; `pnpm lint` + `pnpm typecheck` + `pnpm test` verts ; tests des couches touchées verts ; gate SonarCloud `new_coverage ≥ 80 %` ; docs mémoire + contrat + Fumadocs à jour.
- **Commandes backend depuis `backend/`** (Loco lit `./config` relativement au CWD). Commandes frontend depuis `frontend/`.

---

## File Structure

**Backend (créés)**
- `backend/migration/src/m20260629_000004_add_release_notes_to_versions.rs` — migration colonne.

**Backend (modifiés)**
- `backend/migration/src/lib.rs` — enregistrer la migration.
- `backend/src/models/_entities/versions.rs` — champ `release_notes`.
- `backend/src/services/deploy.rs` — param `release_notes` + validation longueur + tests.
- `backend/src/controllers/admin.rs` — handler `deploy` passe `body.notes`.
- `backend/src/dto/mod.rs` — `DeployReq.notes`, `VersionItem.release_notes`, nouveau `ReleaseNotes`, mapping `to_detail`.
- `backend/src/mcp/mod.rs` — `DeployArgs.release_notes` + passage au service.
- `backend/src/controllers/serve.rs` — shell systématique + routes `/raw` et `/notes` + helpers de gate.
- `backend/src/web/mod.rs` — helper `shell_index()`.

**Frontend (créés)**
- `frontend/shell.html` — entrée Vite du shell.
- `frontend/src/shell/main.tsx` — montage React du shell.
- `frontend/src/shell/shell-page.tsx` — iframe + overlay + localStorage.
- `frontend/src/lib/markdown.tsx` — composant de rendu markdown restreint (partagé).
- `frontend/src/components/notes-editor.tsx` — éditeur Tiptap restreint + onglet aperçu.

**Frontend (modifiés)**
- `frontend/vite.config.ts` — entrée `shell`.
- `frontend/package.json` — deps `react-markdown`, `@tiptap/*`, `tiptap-markdown`.
- `frontend/src/components/deploy-panel.tsx` — intègre l'éditeur de notes, envoie `notes`.
- `frontend/src/routes/detail.tsx` — indicateur « a des notes ».
- `frontend/src/i18n/locales/admin/{en,fr}.json` — clés notes + shell.
- `frontend/src/api/schema.d.ts` — régénéré (`pnpm gen:api`).

**Docs (modifiés)**
- `docs/contrat-deploy.md`, `public_docs/content/docs/admin/versions.mdx`, `public_docs/content/docs/publish-from-claude/tools-reference.mdx`, `public_docs/content/docs/how-it-works/*`, et docs mémoire (`INDEX`, `HANDOFF`, `QUIRKS`, `CONVENTIONS`).

---

## Task 1 : Migration — colonne `release_notes`

**Files:**
- Create: `backend/migration/src/m20260629_000004_add_release_notes_to_versions.rs`
- Modify: `backend/migration/src/lib.rs`

**Interfaces:**
- Produces: colonne `versions.release_notes` (TEXT, nullable).

- [ ] **Step 1: Écrire la migration**

Create `backend/migration/src/m20260629_000004_add_release_notes_to_versions.rs` :

```rust
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Versions::Table)
                    .add_column(ColumnDef::new(Versions::ReleaseNotes).text().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Versions::Table)
                    .drop_column(Versions::ReleaseNotes)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Versions {
    Table,
    ReleaseNotes,
}
```

- [ ] **Step 2: Enregistrer la migration**

In `backend/migration/src/lib.rs`, ajouter le module et l'entrée dans le `vec!` des migrations, après `m20260624_000003_create_sessions`. Suivre le style exact existant (`Box::new(m20260629_000004_add_release_notes_to_versions::Migration)`).

- [ ] **Step 3: Vérifier la compilation de la migration**

Run (depuis `backend/`): `cargo build -p migration`
Expected: compile sans erreur.

- [ ] **Step 4: Commit**

```bash
git add backend/migration/
git commit -m "feat(db): migration release_notes sur versions"
```

---

## Task 2 : Entité `versions` — champ `release_notes`

**Files:**
- Modify: `backend/src/models/_entities/versions.rs:8-15`

**Interfaces:**
- Produces: `versions::Model.release_notes: Option<String>` et `versions::Column::ReleaseNotes`.

- [ ] **Step 1: Ajouter le champ au modèle**

Dans `backend/src/models/_entities/versions.rs`, ajouter le champ après `html_path` :

```rust
    pub html_path: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub release_notes: Option<String>,
    pub created_at: DateTimeWithTimeZone,
```

- [ ] **Step 2: Vérifier la compilation**

Run (depuis `backend/`): `cargo build`
Expected: échec attendu dans `deploy.rs`/`dto`/`mcp` SEULEMENT si l'`ActiveModel` est construit sans `..Default::default()` — sinon compile (le champ a un défaut). Note : `deploy.rs` utilise `..Default::default()`, donc compile encore. Si compile : OK.

- [ ] **Step 3: Commit**

```bash
git add backend/src/models/_entities/versions.rs
git commit -m "feat(model): champ release_notes sur l'entité versions"
```

---

## Task 3 : Service `deploy` — paramètre notes + validation

**Files:**
- Modify: `backend/src/services/deploy.rs:30-74` (signature + corps), `:77-159` (tests)

**Interfaces:**
- Consumes: `versions::ActiveModel`, `CoreError::Validation`.
- Produces: `DeployService::deploy(&self, project_id: i32, html: &str, activate: bool, release_notes: Option<&str>) -> Result<versions::Model, CoreError>` et `pub const MAX_RELEASE_NOTES_LEN: usize = 10_000;`.

- [ ] **Step 1: Écrire le test de validation (échec attendu)**

Dans le module `tests` de `backend/src/services/deploy.rs`, ajouter :

```rust
    #[tokio::test]
    async fn deploy_persists_release_notes() {
        let db = test_db().await;
        let dir = tempfile::tempdir().unwrap();
        let p = make_project(&db).await;
        let svc = DeployService::new(db.clone(), storage(&dir));

        let v = svc
            .deploy(p.id, "<h1>hi</h1>", true, Some("# Salut\n\n- a\n- b"))
            .await
            .unwrap();
        assert_eq!(v.release_notes.as_deref(), Some("# Salut\n\n- a\n- b"));
    }

    #[tokio::test]
    async fn deploy_rejects_too_long_release_notes() {
        let db = test_db().await;
        let dir = tempfile::tempdir().unwrap();
        let p = make_project(&db).await;
        let svc = DeployService::new(db.clone(), storage(&dir));

        let long = "x".repeat(super::MAX_RELEASE_NOTES_LEN + 1);
        let err = svc.deploy(p.id, "x", true, Some(&long)).await.unwrap_err();
        assert!(matches!(err, CoreError::Validation(_)));
    }
```

- [ ] **Step 2: Mettre à jour les 3 tests existants pour la nouvelle signature**

Dans les tests existants, ajouter l'argument `None` aux appels `deploy(...)` :
- `first_deploy_is_version_one_and_writes_html` : `svc.deploy(p.id, "<h1>hi</h1>", true, None)`
- `second_deploy_increments_n` : `svc.deploy(p.id, "a", true, None)` et `svc.deploy(p.id, "b", true, None)`
- `deploy_without_activate_leaves_pointer` : `svc.deploy(p.id, "x", false, None)`

- [ ] **Step 3: Lancer les tests pour vérifier l'échec de compilation**

Run (depuis `backend/`): `cargo test -p latch --lib services::deploy`
Expected: FAIL — la signature `deploy` ne prend pas encore `release_notes`.

- [ ] **Step 4: Implémenter la signature + validation**

Dans `backend/src/services/deploy.rs`, ajouter la constante avant l'`impl` :

```rust
/// Longueur maximale des notes de version (caractères). Au-delà → Validation.
pub const MAX_RELEASE_NOTES_LEN: usize = 10_000;
```

Modifier la signature et le corps de `deploy` :

```rust
    pub async fn deploy(
        &self,
        project_id: i32,
        html: &str,
        activate: bool,
        release_notes: Option<&str>,
    ) -> Result<versions::Model, CoreError> {
        // 0. Validation des notes (barrière de fond : le rendu reste restreint côté client).
        if let Some(notes) = release_notes {
            if notes.chars().count() > MAX_RELEASE_NOTES_LEN {
                return Err(CoreError::Validation(format!(
                    "release_notes trop longues (max {MAX_RELEASE_NOTES_LEN} caractères)"
                )));
            }
        }

        // 1. n = max(n)+1 pour ce projet ...
```

(le reste du calcul de `n` et de l'écriture HTML inchangé)

Dans la construction de l'`ActiveModel`, ajouter le champ :

```rust
        let inserted = versions::ActiveModel {
            project_id: Set(project_id),
            n: Set(n),
            html_path: Set(html_path),
            release_notes: Set(release_notes.map(str::to_string)),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
```

- [ ] **Step 5: Lancer les tests pour vérifier le succès**

Run (depuis `backend/`): `cargo test -p latch --lib services::deploy`
Expected: PASS (5 tests).

- [ ] **Step 6: Commit**

```bash
git add backend/src/services/deploy.rs
git commit -m "feat(deploy): paramètre release_notes + validation longueur"
```

---

## Task 4 : DTO web + handler admin

**Files:**
- Modify: `backend/src/dto/mod.rs:26-32` (`VersionItem`), `:135-139` (`DeployReq`), `:201-222` (`to_detail`), + nouveau `ReleaseNotes`
- Modify: `backend/src/controllers/admin.rs:255-271` (handler `deploy`)

**Interfaces:**
- Consumes: `DeployService::deploy(..., release_notes)` (Task 3).
- Produces: `DeployReq.notes: Option<String>`, `VersionItem.release_notes: Option<String>`, `dto::ReleaseNotes { n: i32, notes_md: String }`.

- [ ] **Step 1: Écrire le test de mapping (échec attendu)**

Dans le module `tests` de `backend/src/dto/mod.rs`, ajouter :

```rust
    #[test]
    fn version_item_carries_release_notes() {
        let v = versions::Model {
            id: 1,
            project_id: 1,
            n: 1,
            html_path: "1/1.html".to_string(),
            release_notes: Some("# Notes".to_string()),
            created_at: chrono::Utc::now().into(),
        };
        let detail = to_detail(sample_model(), vec![v]);
        assert_eq!(
            detail.versions[0].release_notes.as_deref(),
            Some("# Notes")
        );
    }
```

- [ ] **Step 2: Lancer pour vérifier l'échec**

Run (depuis `backend/`): `cargo test -p latch --lib dto::`
Expected: FAIL — `VersionItem` n'a pas de champ `release_notes`, et la construction du `Model` casse (champ manquant).

- [ ] **Step 3: Étendre les DTO**

Dans `backend/src/dto/mod.rs`, `VersionItem` :

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct VersionItem {
    pub id: i32,
    pub n: i32,
    pub created_at: String,
    pub is_active: bool,
    pub release_notes: Option<String>,
}
```

`DeployReq` :

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct DeployReq {
    pub html: String,
    #[serde(default)]
    pub activate: bool,
    #[serde(default)]
    pub notes: Option<String>,
}
```

Ajouter le DTO de l'endpoint `/notes` (après `PublicMeta`) :

```rust
/// Réponse de `GET /c/{slug}/notes` — notes de la version active, rendues côté client.
/// `notes_md` est du markdown brut ; le rendu restreint (sans HTML/lien/image) vit côté shell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ReleaseNotes {
    pub n: i32,
    pub notes_md: String,
}
```

Dans `to_detail`, propager les notes :

```rust
        .map(|v| VersionItem {
            id: v.id,
            n: v.n,
            created_at: v.created_at.to_rfc3339(),
            is_active: Some(v.id) == active,
            release_notes: v.release_notes,
        })
```

- [ ] **Step 4: Mettre à jour le handler admin `deploy`**

Dans `backend/src/controllers/admin.rs`, handler `deploy`, passer les notes :

```rust
    let version = svc
        .deploy(id, &body.html, body.activate, body.notes.as_deref())
        .await
        .map_err(into_response)?;
```

- [ ] **Step 5: Lancer les tests**

Run (depuis `backend/`): `cargo test -p latch --lib dto::`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add backend/src/dto/mod.rs backend/src/controllers/admin.rs
git commit -m "feat(dto): release_notes dans VersionItem/DeployReq + ReleaseNotes"
```

---

## Task 5 : MCP — argument `release_notes`

**Files:**
- Modify: `backend/src/mcp/mod.rs:40-51` (`DeployArgs`), `:117-146` (tool `deploy_prototype`)

**Interfaces:**
- Consumes: `DeployService::deploy(..., release_notes)` (Task 3).
- Produces: argument MCP `release_notes` optionnel.

- [ ] **Step 1: Ajouter le champ aux arguments**

Dans `backend/src/mcp/mod.rs`, `DeployArgs`, ajouter :

```rust
    /// Activer immédiatement la version déployée (défaut : true).
    #[serde(default)]
    activate: Option<bool>,
    /// Notes de version en markdown léger (titres, gras, italique, listes, citation).
    /// Liens, images et code sont ignorés au rendu. Optionnel.
    #[serde(default)]
    release_notes: Option<String>,
```

- [ ] **Step 2: Passer les notes au service**

Dans le tool `deploy_prototype`, l'appel `deploy` devient :

```rust
        let version = deploy
            .deploy(
                project.id,
                &args.html,
                activate,
                args.release_notes.as_deref(),
            )
            .await
            .map_err(map_core_err)?;
```

Compléter la `description` du `#[tool(...)]` en ajoutant une phrase : `Accepte des notes de version en markdown léger (release_notes).`

- [ ] **Step 3: Vérifier la compilation + tests MCP existants**

Run (depuis `backend/`): `cargo test -p latch mcp`
Expected: PASS (les tests MCP existants compilent et passent avec le nouveau champ optionnel).

- [ ] **Step 4: Commit**

```bash
git add backend/src/mcp/mod.rs
git commit -m "feat(mcp): argument release_notes sur deploy_prototype"
```

---

## Task 6 : Serving — gate partagé + endpoints `/raw` et `/notes`

**Files:**
- Modify: `backend/src/web/mod.rs:31-39` (helper `shell_index`)
- Modify: `backend/src/controllers/serve.rs` (helpers de gate, `serve` → shell, handlers `raw` + `notes`, routes)

**Interfaces:**
- Consumes: `versions::Model.release_notes`, `dto::ReleaseNotes` (Task 4), `crate::web::shell_index()`.
- Produces: routes `GET /c/{slug}/raw` (HTML brut, `no-store` + `frame-ancestors 'self'`) et `GET /c/{slug}/notes` (`ReleaseNotes` JSON ou `204`). `serve` renvoie le shell.

- [ ] **Step 1: Ajouter le helper `shell_index`**

Dans `backend/src/web/mod.rs`, après `unlock_index()` :

```rust
/// Chemin du `shell.html` buildé (entrée Vite du shell de serving `/c`).
pub fn shell_index() -> PathBuf {
    spa_dist_dir().join("shell.html")
}
```

- [ ] **Step 2: Écrire le test d'intégration (échec attendu)**

Create/append dans les tests d'intégration du serving (suivre l'emplacement des tests d'intégration existants, p. ex. `backend/tests/`; si un fichier `serve` existe, y ajouter ; sinon créer `backend/tests/serve_notes.rs` sur le modèle des tests d'intégration existants). Test ciblé sur `/notes` (cas projet libre) :

```rust
// Pseudocode de structure — adapter au harnais d'intégration Loco existant
// (boot app de test, insert projet libre + version avec notes, requête HTTP).
//
// 1. Créer un projet libre (code_enabled=false), déployer une version
//    avec release_notes = "# Hello".
// 2. GET /c/<slug>/notes → 200, body JSON { "n": 1, "notes_md": "# Hello" }.
// 3. Déployer une version SANS notes et l'activer → GET /c/<slug>/notes → 204.
```

> Note d'implémentation : reprendre exactement le harnais des tests d'intégration `/c` existants (helper de boot, insertion DB, client de requête). Ne pas inventer un nouveau harnais.

- [ ] **Step 3: Lancer pour vérifier l'échec**

Run (depuis `backend/`): `cargo test -p latch --test serve_notes` (ou le nom du fichier choisi)
Expected: FAIL — route `/c/{slug}/notes` inexistante (404).

- [ ] **Step 4: Refactor — extraire le gate unlock et le chargement de version**

Dans `backend/src/controllers/serve.rs`, ajouter ces helpers (réutilisés par `serve`, `raw`, `notes`) :

```rust
use crate::models::_entities::{projects, versions};

/// `true` si l'accès au proto est autorisé : projet libre, ou cookie unlock valide.
fn unlock_ok(
    ctx: &AppContext,
    headers: &HeaderMap,
    slug: &str,
    project: &projects::Model,
) -> Result<bool> {
    if !project.code_enabled {
        return Ok(true);
    }
    let pin = project.pin.clone().unwrap_or_default();
    let key = crate::web::unlock_key(ctx)?;
    let jar = SignedCookieJar::from_headers(headers, key);
    let now = chrono::Utc::now().timestamp();
    let secret = crate::web::unlock_secret(ctx)?;
    Ok(match jar.get(UNLOCK_COOKIE_NAME) {
        Some(c) => verify_token(secret.as_bytes(), slug, &pin, c.value(), now),
        None => false,
    })
}

/// Charge la version active d'un projet, ou `None` si pas de pointeur / version absente.
async fn load_active_version(
    ctx: &AppContext,
    project: &projects::Model,
) -> Result<Option<versions::Model>> {
    let Some(active_id) = project.active_version_id else {
        return Ok(None);
    };
    Ok(versions::Entity::find_by_id(active_id)
        .one(&ctx.db)
        .await
        .map_err(|e| loco_rs::Error::Message(format!("version lookup: {e}")))?)
}
```

- [ ] **Step 5: Réécrire `serve` pour renvoyer le shell**

Remplacer le corps de `serve` (à partir de la résolution projet) par :

```rust
    let svc = ProjectsService::new(ctx.db.clone());
    let project = match svc.get_by_slug(&slug).await {
        Ok(p) => p,
        Err(CoreError::NotFound) => return Ok(serve_error_page(StatusCode::NOT_FOUND).await),
        Err(e) => {
            tracing::error!(error = %e, slug = %slug, "serve: get_by_slug failed");
            return Ok(serve_error_page(StatusCode::INTERNAL_SERVER_ERROR).await);
        }
    };

    // Pas de version active → page d'erreur 404 (comportement inchangé).
    if project.active_version_id.is_none() {
        return Ok(serve_error_page(StatusCode::NOT_FOUND).await);
    }

    // Projet protégé sans cookie valide → page de déverrouillage (top-level, hors iframe).
    if !unlock_ok(&ctx, &headers, &slug, &project)? {
        return unlock_page_response().await;
    }

    // Sinon → servir le shell (qui charge /raw en iframe et gère l'overlay de notes).
    shell_page_response().await
```

Ajouter le helper de rendu du shell, à côté de `unlock_page_response` :

```rust
/// Rend la page-coquille (`shell.html` buildé), HTTP 200, `no-store`.
async fn shell_page_response() -> Result<Response> {
    let path = crate::web::shell_index();
    let html = tokio::fs::read_to_string(&path).await.map_err(|e| {
        loco_rs::Error::Message(format!("shell.html introuvable ({}): {e}", path.display()))
    })?;
    Ok(html_response(html))
}
```

- [ ] **Step 6: Ajouter le handler `raw`**

Ajouter une réponse HTML avec CSP `frame-ancestors`, puis le handler. À côté de `html_response` :

```rust
/// Réponse HTML du proto pour l'iframe : `no-store` + `frame-ancestors 'self'`
/// (seul le shell latch peut l'encadrer).
fn raw_html_response(html: String) -> Response {
    (
        [
            (CACHE_CONTROL, HeaderValue::from_static("no-store")),
            (
                CONTENT_TYPE,
                HeaderValue::from_static("text/html; charset=utf-8"),
            ),
            (
                axum::http::header::CONTENT_SECURITY_POLICY,
                HeaderValue::from_static("frame-ancestors 'self'"),
            ),
        ],
        html,
    )
        .into_response()
}
```

Handler `raw` :

```rust
/// GET /c/{slug}/raw — HTML brut du proto (cible de l'iframe du shell). Mêmes gates.
#[debug_handler]
pub(crate) async fn raw(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Result<Response> {
    let svc = ProjectsService::new(ctx.db.clone());
    let project = match svc.get_by_slug(&slug).await {
        Ok(p) => p,
        Err(CoreError::NotFound) => return Ok(serve_error_page(StatusCode::NOT_FOUND).await),
        Err(e) => {
            tracing::error!(error = %e, slug = %slug, "raw: get_by_slug failed");
            return Ok(serve_error_page(StatusCode::INTERNAL_SERVER_ERROR).await);
        }
    };
    if !unlock_ok(&ctx, &headers, &slug, &project)? {
        // Defense-in-depth : ne jamais servir le HTML d'un proto verrouillé.
        return Ok(serve_error_page(StatusCode::FORBIDDEN).await);
    }
    let Some(version) = load_active_version(&ctx, &project).await? else {
        return Ok(serve_error_page(StatusCode::NOT_FOUND).await);
    };
    let storage = crate::web::storage_from_ctx(&ctx);
    match storage.read(&version.html_path).await {
        Ok(html) => Ok(raw_html_response(html)),
        Err(e) => {
            tracing::error!(error = %e, slug = %slug, "raw: storage read failed");
            Ok(serve_error_page(StatusCode::INTERNAL_SERVER_ERROR).await)
        }
    }
}
```

- [ ] **Step 7: Ajouter le handler `notes`**

```rust
/// GET /c/{slug}/notes — notes de la version active (ou 204). Gardé par l'unlock.
#[debug_handler]
pub(crate) async fn notes(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Result<Response> {
    let svc = ProjectsService::new(ctx.db.clone());
    let project = match svc.get_by_slug(&slug).await {
        Ok(p) => p,
        Err(CoreError::NotFound) => return Ok(StatusCode::NOT_FOUND.into_response()),
        Err(e) => {
            tracing::error!(error = %e, slug = %slug, "notes: get_by_slug failed");
            return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }
    };
    if !unlock_ok(&ctx, &headers, &slug, &project)? {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }
    let Some(version) = load_active_version(&ctx, &project).await? else {
        return Ok(StatusCode::NO_CONTENT.into_response());
    };
    match version.release_notes {
        Some(md) if !md.is_empty() => {
            let body = crate::dto::ReleaseNotes { n: version.n, notes_md: md };
            Ok((
                [(CACHE_CONTROL, HeaderValue::from_static("no-store"))],
                axum::Json(body),
            )
                .into_response())
        }
        _ => Ok(StatusCode::NO_CONTENT.into_response()),
    }
}
```

- [ ] **Step 8: Enregistrer les routes**

Dans `routes()` de `serve.rs`, ajouter les deux routes (hors rate-limit unlock) :

```rust
    Routes::new()
        .add("/api/public/{slug}", get(public_meta))
        .add("/c/{slug}", get(serve))
        .add("/c/{slug}/raw", get(raw))
        .add("/c/{slug}/notes", get(notes))
        .add("/c/{slug}/unlock", post(unlock).layer(unlock_layers))
```

- [ ] **Step 9: Nettoyer les imports**

Le `use crate::models::_entities::versions;` local dans l'ancien `serve` (ligne ~134) est désormais couvert par l'import en tête (`projects, versions`). Supprimer le `use` local redondant. Lancer `cargo build` (depuis `backend/`) et corriger tout warning d'import inutilisé.

- [ ] **Step 10: Lancer les tests d'intégration**

Run (depuis `backend/`): `cargo test -p latch --test serve_notes`
Expected: PASS. Puis `cargo test -p latch` (toute la suite) — vérifier qu'aucun test `/c` existant ne casse (ceux qui attendaient le HTML brut sur `/c/<slug>` doivent viser `/c/<slug>/raw` désormais — les ajuster si présents).

- [ ] **Step 11: fmt + clippy + commit**

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
git add backend/src/controllers/serve.rs backend/src/web/mod.rs backend/tests/
git commit -m "feat(serve): shell systématique + endpoints /raw et /notes"
```

---

## Task 7 : Régénérer le schéma OpenAPI + client typé

**Files:**
- Modify: `openapi.json` (régénéré par le backend), `frontend/src/api/schema.d.ts` (régénéré)

**Interfaces:**
- Produces: types TS `DeployReq.notes`, `VersionItem.release_notes` dans `schema.d.ts`.

- [ ] **Step 1: Régénérer `openapi.json`**

Régénérer le spec OpenAPI selon le pipeline existant du projet (vérifier `backend/src/openapi.rs` et le script/commande utilisé ; souvent un test ou une task Loco écrit `openapi.json`). Exécuter la commande de génération du repo (cf. `docs/BOOTSTRAP.md` / `ENVIRONMENT.md`). Vérifier que `DeployReq` et `VersionItem` reflètent les nouveaux champs.

- [ ] **Step 2: Régénérer le client TS**

Run (depuis `frontend/`): `pnpm gen:api`
Expected: `src/api/schema.d.ts` met à jour `DeployReq` (`notes?`) et `VersionItem` (`release_notes?`).

- [ ] **Step 3: Vérifier le typage**

Run (depuis `frontend/`): `pnpm typecheck`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add openapi.json frontend/src/api/schema.d.ts
git commit -m "chore(api): régénère openapi + client typé (release_notes/notes)"
```

---

## Task 8 : Dépendances frontend

**Files:**
- Modify: `frontend/package.json`

**Interfaces:**
- Produces: `react-markdown`, `@tiptap/react`, `@tiptap/pm`, `@tiptap/starter-kit`, `tiptap-markdown` disponibles.

- [ ] **Step 1: Installer les dépendances**

Run (depuis `frontend/`):

```bash
pnpm add react-markdown @tiptap/react @tiptap/pm @tiptap/starter-kit tiptap-markdown
```

- [ ] **Step 2: Vérifier l'install**

Run (depuis `frontend/`): `pnpm install`
Expected: lockfile cohérent, pas d'erreur.

- [ ] **Step 3: Commit**

```bash
git add frontend/package.json frontend/pnpm-lock.yaml
git commit -m "chore(deps): react-markdown + tiptap pour les notes de version"
```

---

## Task 9 : Composant de rendu markdown restreint (partagé)

**Files:**
- Create: `frontend/src/lib/markdown.tsx`
- Test: `frontend/src/lib/markdown.test.tsx`

**Interfaces:**
- Produces: `export function MarkdownView({ source }: { source: string }): JSX.Element` — rend le périmètre autorisé, neutralise le reste.

- [ ] **Step 1: Écrire les tests (échec attendu)**

Create `frontend/src/lib/markdown.test.tsx` :

```tsx
import { render, screen } from '@testing-library/react'
import { describe, it, expect } from 'vitest'
import { MarkdownView } from './markdown'

describe('MarkdownView', () => {
  it('renders allowed elements (heading, emphasis, list, blockquote)', () => {
    render(
      <MarkdownView source={'# Titre\n\n**gras** *ita*\n\n- un\n- deux\n\n> cite'} />,
    )
    expect(screen.getByRole('heading', { name: 'Titre' })).toBeInTheDocument()
    expect(screen.getByText('gras')).toBeInTheDocument()
    expect(screen.getByRole('list')).toBeInTheDocument()
  })

  it('does not render links or images', () => {
    const { container } = render(
      <MarkdownView source={'[x](https://evil.test) ![y](https://evil.test/i.png)'} />,
    )
    expect(container.querySelector('a')).toBeNull()
    expect(container.querySelector('img')).toBeNull()
  })

  it('neutralizes raw HTML / script', () => {
    const { container } = render(
      <MarkdownView source={'<script>window.__x=1</script><b>raw</b>'} />,
    )
    expect(container.querySelector('script')).toBeNull()
    // pas d'exécution : la balise est traitée comme texte, pas comme DOM actif
  })
})
```

- [ ] **Step 2: Lancer pour vérifier l'échec**

Run (depuis `frontend/`): `pnpm test markdown`
Expected: FAIL — `./markdown` n'existe pas.

- [ ] **Step 3: Implémenter le composant**

Create `frontend/src/lib/markdown.tsx` :

```tsx
import Markdown from 'react-markdown'

/**
 * Rendu markdown restreint — barrière XSS unique partagée par l'overlay client
 * et l'aperçu admin. Périmètre autorisé : paragraphes, titres, gras/italique,
 * listes, citation. Interdits : liens, images, code, HTML brut.
 */
const ALLOWED = [
  'p',
  'h1',
  'h2',
  'h3',
  'h4',
  'h5',
  'h6',
  'strong',
  'em',
  'ul',
  'ol',
  'li',
  'blockquote',
]

export function MarkdownView({ source }: Readonly<{ source: string }>) {
  return (
    <Markdown
      skipHtml
      allowedElements={ALLOWED}
      unwrapDisallowed
    >
      {source}
    </Markdown>
  )
}
```

- [ ] **Step 4: Lancer pour vérifier le succès**

Run (depuis `frontend/`): `pnpm test markdown`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/lib/markdown.tsx frontend/src/lib/markdown.test.tsx
git commit -m "feat(front): composant MarkdownView restreint partagé"
```

---

## Task 10 : Éditeur de notes Tiptap + aperçu

**Files:**
- Create: `frontend/src/components/notes-editor.tsx`
- Test: `frontend/src/components/notes-editor.test.tsx`

**Interfaces:**
- Consumes: `MarkdownView` (Task 9).
- Produces: `export function NotesEditor({ value, onChange }: { value: string; onChange: (md: string) => void }): JSX.Element` — éditeur WYSIWYG restreint qui émet du markdown, avec onglets Écrire/Aperçu.

- [ ] **Step 1: Écrire le test (échec attendu)**

Create `frontend/src/components/notes-editor.test.tsx` :

```tsx
import { render, screen } from '@testing-library/react'
import { describe, it, expect, vi } from 'vitest'
import { NotesEditor } from './notes-editor'

describe('NotesEditor', () => {
  it('renders the editor and a preview tab', () => {
    render(<NotesEditor value={'# Hi'} onChange={vi.fn()} />)
    // Onglet aperçu présent (libellé i18n ou data-testid)
    expect(screen.getByTestId('notes-editor')).toBeInTheDocument()
    expect(screen.getByTestId('notes-preview-tab')).toBeInTheDocument()
  })
})
```

> Note : Tiptap monte un ProseMirror dans jsdom ; garder le test au niveau « se monte et expose les zones », sans simuler la frappe (couvrir le markdown sérialisé via un test d'intégration léger si besoin). Le test de neutralisation XSS vit dans `markdown.test.tsx` (Task 9).

- [ ] **Step 2: Lancer pour vérifier l'échec**

Run (depuis `frontend/`): `pnpm test notes-editor`
Expected: FAIL — composant inexistant.

- [ ] **Step 3: Implémenter l'éditeur**

Create `frontend/src/components/notes-editor.tsx` :

```tsx
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useEditor, EditorContent } from '@tiptap/react'
import StarterKit from '@tiptap/starter-kit'
import { Markdown } from 'tiptap-markdown'
import { Button } from '@/components/ui/button'
import { MarkdownView } from '@/lib/markdown'

/**
 * Éditeur WYSIWYG restreint au périmètre markdown partagé (titres, gras,
 * italique, listes, citation). Sérialise en markdown via tiptap-markdown.
 * Onglet Aperçu = rendu réel (MarkdownView), identique à l'overlay client.
 */
export function NotesEditor({
  value,
  onChange,
}: Readonly<{ value: string; onChange: (md: string) => void }>) {
  const { t } = useTranslation()
  const [tab, setTab] = useState<'write' | 'preview'>('write')

  const editor = useEditor({
    extensions: [
      StarterKit.configure({
        // Hors périmètre → désactivés.
        code: false,
        codeBlock: false,
        strike: false,
        horizontalRule: false,
      }),
      Markdown,
    ],
    content: value,
    onUpdate: ({ editor }) => {
      onChange(editor.storage.markdown.getMarkdown())
    },
  })

  return (
    <div className="flex flex-col gap-2" data-testid="notes-editor">
      <div className="flex gap-1">
        <Button
          type="button"
          size="sm"
          variant={tab === 'write' ? 'secondary' : 'ghost'}
          onClick={() => setTab('write')}
        >
          {t('deploy.notes_write')}
        </Button>
        <Button
          type="button"
          size="sm"
          variant={tab === 'preview' ? 'secondary' : 'ghost'}
          onClick={() => setTab('preview')}
          data-testid="notes-preview-tab"
        >
          {t('deploy.notes_preview')}
        </Button>
      </div>

      {tab === 'write' ? (
        <EditorContent
          editor={editor}
          className="prose prose-sm max-w-none rounded-md border border-input px-3 py-2 [&_.ProseMirror]:min-h-[120px] [&_.ProseMirror]:outline-none"
        />
      ) : (
        <div className="prose prose-sm max-w-none rounded-md border border-input px-3 py-2">
          <MarkdownView source={value} />
        </div>
      )}
    </div>
  )
}
```

- [ ] **Step 4: Lancer pour vérifier le succès**

Run (depuis `frontend/`): `pnpm test notes-editor`
Expected: PASS. Si Tiptap pose souci sous jsdom, vérifier la config de test (`environment: 'jsdom'`) et que `@tiptap/pm` est installé.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/components/notes-editor.tsx frontend/src/components/notes-editor.test.tsx
git commit -m "feat(front): éditeur de notes Tiptap restreint + aperçu"
```

---

## Task 11 : Brancher l'éditeur dans le panneau de déploiement

**Files:**
- Modify: `frontend/src/components/deploy-panel.tsx`
- Modify: `frontend/src/i18n/locales/admin/en.json`, `frontend/src/i18n/locales/admin/fr.json`

**Interfaces:**
- Consumes: `NotesEditor` (Task 10), `DeployReq.notes` (Task 7).

- [ ] **Step 1: Ajouter les clés i18n**

Dans `en.json`, ajouter (à côté des clés `deploy.*`) :

```json
"deploy.notes": "Release notes (optional)",
"deploy.notes_help": "Markdown: headings, bold, italic, lists, quotes. Links and images are ignored.",
"deploy.notes_write": "Write",
"deploy.notes_preview": "Preview"
```

Dans `fr.json` :

```json
"deploy.notes": "Notes de version (optionnel)",
"deploy.notes_help": "Markdown : titres, gras, italique, listes, citations. Liens et images ignorés.",
"deploy.notes_write": "Écrire",
"deploy.notes_preview": "Aperçu"
```

- [ ] **Step 2: Intégrer l'éditeur dans `DeployPanelContent`**

Dans `frontend/src/components/deploy-panel.tsx` :
- Importer : `import { NotesEditor } from '@/components/notes-editor'`.
- Ajouter l'état : `const [notes, setNotes] = useState('')` (à côté des autres `useState`).
- Dans `handleSubmit`, inclure les notes dans le body (chaîne vide → `undefined` pour ne pas stocker du vide) :

```tsx
    deploy.mutate(
      {
        id: projectId,
        body: { html, activate, notes: notes.trim() ? notes : undefined },
      },
      { onSuccess: () => onOpenChange(false) },
    )
```

- Ajouter un bloc UI après la checkbox « activate » et avant `SheetFooter` :

```tsx
      {/* Release notes */}
      <div className="flex flex-col gap-1.5">
        <Label>{t('deploy.notes')}</Label>
        <NotesEditor value={notes} onChange={setNotes} />
        <p className="text-muted-foreground text-xs">{t('deploy.notes_help')}</p>
      </div>
```

- [ ] **Step 3: Vérifier typecheck + lint**

Run (depuis `frontend/`): `pnpm typecheck && pnpm lint`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add frontend/src/components/deploy-panel.tsx frontend/src/i18n/locales/admin/
git commit -m "feat(front): saisie des notes de version dans le panneau de déploiement"
```

---

## Task 12 : Indicateur « a des notes » dans le tableau des versions

**Files:**
- Modify: `frontend/src/routes/detail.tsx` (tableau des versions, ~lignes 172-268)
- Modify: `frontend/src/i18n/locales/admin/{en,fr}.json`

**Interfaces:**
- Consumes: `VersionItem.release_notes` (Task 7).

- [ ] **Step 1: Ajouter la clé i18n**

Dans `en.json` : `"detail.has_notes": "Has release notes"`.
Dans `fr.json` : `"detail.has_notes": "Contient des notes de version"`.

- [ ] **Step 2: Afficher l'indicateur**

Dans `frontend/src/routes/detail.tsx`, dans la cellule d'une ligne de version (là où s'affichent `n`/badges), ajouter un petit indicateur conditionnel quand `v.release_notes` est non vide. Exemple minimal (icône texte avec `title` accessible) :

```tsx
{v.release_notes ? (
  <span
    title={t('detail.has_notes')}
    aria-label={t('detail.has_notes')}
    className="text-muted-foreground ml-2 text-xs"
  >
    📝
  </span>
) : null}
```

(adapter l'emplacement exact à la structure de la ligne ; le rendre cohérent avec le badge « active » existant.)

- [ ] **Step 3: Vérifier typecheck + lint + tests existants du détail**

Run (depuis `frontend/`): `pnpm typecheck && pnpm lint && pnpm test detail`
Expected: PASS (ajuster les tests du détail s'ils figent la structure des lignes).

- [ ] **Step 4: Commit**

```bash
git add frontend/src/routes/detail.tsx frontend/src/i18n/locales/admin/
git commit -m "feat(front): indicateur de présence de notes dans la liste des versions"
```

---

## Task 13 : Shell — bundle Vite, iframe + overlay + localStorage

**Files:**
- Create: `frontend/shell.html`, `frontend/src/shell/main.tsx`, `frontend/src/shell/shell-page.tsx`
- Test: `frontend/src/shell/shell-page.test.tsx`
- Modify: `frontend/vite.config.ts`, `frontend/src/i18n/locales/admin/{en,fr}.json`

**Interfaces:**
- Consumes: `MarkdownView` (Task 9), endpoint `GET /c/<slug>/notes` (Task 6), `localStorage`.
- Produces: bundle `shell.html` chargé par `serve` (Task 6).

- [ ] **Step 1: Ajouter l'entrée Vite**

Dans `frontend/vite.config.ts`, ajouter dans `rollupOptions.input` :

```ts
        shell: fileURLToPath(new URL('./shell.html', import.meta.url)),
```

- [ ] **Step 2: Créer `shell.html`**

Create `frontend/shell.html` (calqué sur `unlock.html`) :

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <meta name="robots" content="noindex, nofollow" />
    <title>latch</title>
    <link rel="icon" type="image/svg+xml" href="/src/assets/latch-logo.svg" />
  </head>
  <body>
    <div id="shell-root"></div>
    <script type="module" src="/src/shell/main.tsx"></script>
  </body>
</html>
```

- [ ] **Step 3: Créer `main.tsx`**

Create `frontend/src/shell/main.tsx` :

```tsx
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { I18nextProvider } from 'react-i18next'
import i18n from '../unlock/../i18n'
import { ShellPage } from './shell-page'
import '@/index.css'

createRoot(document.getElementById('shell-root')!).render(
  <StrictMode>
    <I18nextProvider i18n={i18n}>
      <ShellPage />
    </I18nextProvider>
  </StrictMode>,
)
```

> Import i18n : utiliser le même chemin que `unlock/main.tsx` (`./i18n` relatif au dossier `src`). Ajuster en `import i18n from '@/i18n'` si l'alias est configuré ainsi dans le projet.

- [ ] **Step 4: Écrire le test du shell (échec attendu)**

Create `frontend/src/shell/shell-page.test.tsx` :

```tsx
import { render, screen, waitFor } from '@testing-library/react'
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { I18nextProvider } from 'react-i18next'
import i18n from '@/i18n'
import { ShellPage } from './shell-page'

function renderShell() {
  return render(
    <I18nextProvider i18n={i18n}>
      <ShellPage />
    </I18nextProvider>,
  )
}

describe('ShellPage', () => {
  beforeEach(() => {
    localStorage.clear()
    window.history.pushState({}, '', '/c/demo-abc123')
  })

  it('always renders the prototype iframe pointing at /raw', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ status: 204 }))
    const { container } = renderShell()
    const iframe = container.querySelector('iframe')
    expect(iframe?.getAttribute('src')).toBe('/c/demo-abc123/raw')
  })

  it('shows the overlay when notes are unseen, hides after dismiss', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        status: 200,
        json: async () => ({ n: 2, notes_md: '# New' }),
      }),
    )
    renderShell()
    const dismiss = await screen.findByTestId('notes-dismiss')
    expect(screen.getByRole('heading', { name: 'New' })).toBeInTheDocument()
    dismiss.click()
    await waitFor(() =>
      expect(screen.queryByTestId('notes-dismiss')).toBeNull(),
    )
    expect(localStorage.getItem('latch:seen:demo-abc123')).toBe('2')
  })

  it('does not show the overlay when the version was already seen', async () => {
    localStorage.setItem('latch:seen:demo-abc123', '2')
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        status: 200,
        json: async () => ({ n: 2, notes_md: '# New' }),
      }),
    )
    renderShell()
    await waitFor(() => {})
    expect(screen.queryByTestId('notes-dismiss')).toBeNull()
  })
})
```

- [ ] **Step 5: Lancer pour vérifier l'échec**

Run (depuis `frontend/`): `pnpm test shell-page`
Expected: FAIL — `shell-page` inexistant.

- [ ] **Step 6: Implémenter `shell-page.tsx`**

Create `frontend/src/shell/shell-page.tsx` :

```tsx
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { MarkdownView } from '@/lib/markdown'

/** Slug courant extrait de `/c/<slug>` (1er segment après `/c/`). */
function currentSlug(): string {
  return globalThis.location.pathname.split('/')[2] ?? ''
}

function seenKey(slug: string): string {
  return `latch:seen:${slug}`
}

interface Notes {
  n: number
  notes_md: string
}

export function ShellPage() {
  const { t } = useTranslation()
  const slug = currentSlug()
  const [notes, setNotes] = useState<Notes | null>(null)

  useEffect(() => {
    let cancelled = false
    fetch(`/c/${slug}/notes`)
      .then(async (res) => {
        if (res.status !== 200) return null
        return (await res.json()) as Notes
      })
      .then((data) => {
        if (cancelled || !data) return
        const seen = Number(localStorage.getItem(seenKey(slug)) ?? '0')
        if (data.n > seen) setNotes(data)
      })
      .catch(() => {
        /* notes best-effort : un échec ne doit jamais masquer le proto */
      })
    return () => {
      cancelled = true
    }
  }, [slug])

  function dismiss() {
    if (notes) localStorage.setItem(seenKey(slug), String(notes.n))
    setNotes(null)
  }

  return (
    <div className="relative h-svh w-svw">
      <iframe
        title="prototype"
        src={`/c/${slug}/raw`}
        className="h-full w-full border-0"
      />
      {notes && (
        <div className="bg-background/60 fixed inset-0 z-50 flex items-center justify-center p-4 backdrop-blur-sm">
          <div className="bg-background w-full max-w-lg rounded-xl border p-6 shadow-xl">
            <h2 className="mb-3 text-lg font-semibold">{t('shell.notes_title')}</h2>
            <div className="prose prose-sm max-h-[60vh] max-w-none overflow-y-auto">
              <MarkdownView source={notes.notes_md} />
            </div>
            <div className="mt-5 flex justify-end">
              <Button type="button" onClick={dismiss} data-testid="notes-dismiss">
                {t('shell.dismiss')}
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
```

- [ ] **Step 7: Ajouter les clés i18n du shell**

Dans `en.json` : `"shell.notes_title": "What's new"`, `"shell.dismiss": "Got it"`.
Dans `fr.json` : `"shell.notes_title": "Nouveautés"`, `"shell.dismiss": "Compris"`.

- [ ] **Step 8: Lancer pour vérifier le succès**

Run (depuis `frontend/`): `pnpm test shell-page`
Expected: PASS (3 tests).

- [ ] **Step 9: Vérifier le build multi-bundle**

Run (depuis `frontend/`): `pnpm build`
Expected: `dist/shell.html` généré aux côtés de `index.html`, `unlock.html`, `error.html`.

- [ ] **Step 10: Commit**

```bash
git add frontend/shell.html frontend/src/shell/ frontend/vite.config.ts frontend/src/i18n/locales/admin/
git commit -m "feat(front): shell de serving (iframe proto + overlay notes + localStorage)"
```

---

## Task 14 : E2E Playwright — overlay de notes

**Files:**
- Create/Modify: test e2e Playwright (suivre l'emplacement des specs e2e existantes, p. ex. `frontend/e2e/` ou `e2e/`)

**Interfaces:**
- Consumes: tout le flux (admin deploy avec notes → `/c` overlay → dismiss).

- [ ] **Step 1: Écrire le scénario e2e**

Ajouter une spec Playwright qui :
1. se connecte à l'admin, crée un projet libre (ou réutilise le helper de setup existant) ;
2. ouvre le panneau de déploiement, charge un HTML minimal, saisit des notes dans l'éditeur, déploie ;
3. visite `/c/<slug>` ;
4. attend l'overlay (`[data-testid="notes-dismiss"]`) et vérifie qu'il contient le texte des notes ;
5. clique « dismiss », recharge la page, vérifie que l'overlay **n'apparaît plus**.

> Reprendre les helpers d'auth/admin des specs e2e existantes. Garder un HTML de proto trivial (`<!doctype html><h1>demo</h1>`).

- [ ] **Step 2: Lancer le test e2e**

Run (depuis `frontend/`): `pnpm exec playwright test` (filtrer sur la nouvelle spec)
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add <chemin de la spec e2e>
git commit -m "test(e2e): overlay de notes de version sur /c"
```

---

## Task 15 : Documentation — contrat + Fumadocs + mémoire

**Files:**
- Modify: `docs/contrat-deploy.md`
- Modify: `public_docs/content/docs/admin/versions.mdx`, `public_docs/content/docs/publish-from-claude/tools-reference.mdx`, `public_docs/content/docs/how-it-works/architecture.mdx` (+ `security-model.mdx` si pertinent)
- Modify: `docs/INDEX.md`, `docs/HANDOFF.md`, `docs/QUIRKS.md`, `docs/CONVENTIONS.md`

**Interfaces:** documentation alignée sur le code livré.

- [ ] **Step 1: Mettre à jour le contrat**

Dans `docs/contrat-deploy.md` (le contrat fait loi) : documenter la nouvelle surface serving `/c/<slug>` = shell + iframe (`/raw` avec `frame-ancestors 'self'`, `no-store`), l'endpoint `/notes` gardé par l'unlock, et le champ `release_notes` dans le flux deploy (admin + MCP) avec sa limite 10 000 caractères et la barrière de rendu restreint. Préciser que `/notes` exposant `n` reste gardé par l'unlock (pas de fuite pré-auth).

- [ ] **Step 2: Mettre à jour `admin/versions.mdx`**

Section « Release notes » : saisie au déploiement (éditeur WYSIWYG léger + aperçu, périmètre titres/gras/italique/listes/citation, liens/images/code ignorés), indicateur « a des notes » dans la liste des versions.

- [ ] **Step 3: Mettre à jour `tools-reference.mdx`**

Ajouter `release_notes` à la signature et au tableau d'arguments de `deploy_prototype` (markdown léger, optionnel ; liens/images/code ignorés au rendu).

- [ ] **Step 4: Mettre à jour `how-it-works/`**

Décrire la surface `/c` = shell + iframe (`/raw`), l'endpoint `/notes` gardé par l'unlock, et l'overlay de notes côté visiteur (premier passage, mémorisé en `localStorage`, masqué au dismiss). Étendre `architecture.mdx` (et `security-model.mdx` pour la barrière XSS markdown + CSP). Mettre à jour le `meta.json` concerné si une page est ajoutée.

- [ ] **Step 5: Mettre à jour la mémoire projet**

- `docs/INDEX.md` : ligne « Notes de version » + liens spec/plan.
- `docs/HANDOFF.md` : entrée datée (Dernière chose faite / Trucs en suspens / Prochaine chose à creuser / Notes pour future Claude).
- `docs/QUIRKS.md` : « tous les protos sont désormais servis en iframe via le shell » (impacts `window.top`, fullscreen, CSP du proto) ; `release_notes` rendu côté client uniquement (jamais en HTML serveur).
- `docs/CONVENTIONS.md` : le pattern `MarkdownView` restreint partagé + le moule « mini-SPA Vite » (shell calqué sur unlock).

- [ ] **Step 6: Commit**

```bash
git add docs/ public_docs/
git commit -m "docs: notes de version (contrat, Fumadocs, mémoire)"
```

---

## Task 16 : Vérification finale

**Files:** aucun (gates).

- [ ] **Step 1: Backend — fmt, clippy, tests**

Run (depuis `backend/`):

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo nextest run
```

Expected: tout vert.

- [ ] **Step 2: Frontend — lint, typecheck, tests, build**

Run (depuis `frontend/`):

```bash
pnpm lint && pnpm typecheck && pnpm test && pnpm build
```

Expected: tout vert, `dist/shell.html` présent.

- [ ] **Step 3: E2E**

Run (depuis `frontend/`): `pnpm exec playwright test`
Expected: vert.

- [ ] **Step 4: Vérifier les invariants de sécurité**

Confirmer (lecture + tests) : aucune réponse `/c/*` ni MCP ne renvoie de hash/PIN ; `/c/<slug>/raw` et `/notes` refusent un proto protégé non déverrouillé ; le rendu markdown neutralise script/lien/image (`markdown.test.tsx` vert).

- [ ] **Step 5: Scan SonarCloud local (couverture new-code)**

Lancer le scan Sonar local selon `docs/ENVIRONMENT.md §Scan local` et vérifier `new_coverage ≥ 80 %` sur le code neuf.

---

## Self-Review (effectuée à la rédaction)

- **Couverture spec** : modèle/migration (T1-T2), validation 10 000 (T3), DTO+MCP (T4-T5), shell+`/raw`+`/notes`+gate (T6), OpenAPI (T7), deps (T8), rendu restreint partagé (T9), éditeur Tiptap+aperçu (T10-T11), indicateur notes (T12), shell overlay+localStorage (T13), e2e (T14), contrat+Fumadocs+mémoire (T15), gates (T16). ✔
- **Périmètre markdown unique** : `ALLOWED` (T9) et StarterKit restreint (T10) alignés sur titres/gras/italique/listes/citation. ✔
- **Cohérence des types** : `deploy(project_id, html, activate, release_notes: Option<&str>)` utilisé identiquement en T3/T4/T5 ; `ReleaseNotes { n, notes_md }` défini en T4, consommé en T6/T13 ; `MarkdownView({ source })` défini en T9, consommé en T10/T13 ; `NotesEditor({ value, onChange })` défini en T10, consommé en T11. ✔
- **Pas de placeholder** : chaque step de code porte le code réel. Les seules zones « adapter au harnais » concernent les tests d'intégration/e2e dont l'emplacement dépend de la structure existante — signalées explicitement, pas du code à inventer.
