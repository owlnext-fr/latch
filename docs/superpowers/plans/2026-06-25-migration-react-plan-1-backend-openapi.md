# Plan 1 — Backend OpenAPI (migration React) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Faire du backend Rust la source de vérité OpenAPI : inliner les DTO dans `backend/src/dto/`, annoter chaque route `/api/*` avec `utoipa`, générer un `openapi.json` commité, et le verrouiller par un test de drift — pour que le front React (Plan 2) génère son client TS sans dépendre d'un backend qui tourne.

**Architecture :** Approche **code-first manuelle** d'utoipa (pas d'auto-collection de routeur, car Loco enveloppe axum dans son propre `Routes`/`AppRoutes`). Chaque handler porte `#[utoipa::path(...)]` (déclaratif) ; un `#[derive(OpenApi)] ApiDoc` agrège paths + schemas ; un test régénère le JSON et le compare au fichier commité. La crate workspace `latch-dto` est dissoute (plus de consommateur Rust externe).

**Tech Stack :** Rust, Loco 0.16 (axum 0.8), SeaORM 1.1, **utoipa 5** (`#[derive(OpenApi)]`, `#[utoipa::path]`, `#[derive(ToSchema)]`, `OpenApi::to_pretty_json`), `cargo nextest`.

## Global Constraints

- **Le cœur (`backend/src/services/`) ne contient jamais `use axum::` ni `use loco_rs::`** — garde `backend/tests/architecture.rs`. (utoipa ne touche que `dto/`, `openapi.rs`, `controllers/`.)
- **Invariant §9.1** : aucune réponse ne contient de hash. **§9.2** : `ProjectListItem` n'a **structurellement pas** de champ `pin` (champ absent du type, pas un `skip`).
- **Pas d'`unwrap`/`expect`** hors tests et hors init de boot. Erreurs propagées.
- **Commits gitmoji + conventionnels** : `<gitmoji> <type>: <description>` (ex. `✨ feat:`, `♻️ refactor:`, `🧱 chore:`). Terminer chaque message par les deux trailers :
  ```
  Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01XhGRMj78xsAunbwfpyFUbR
  ```
- **Confidentialité** : aucun nom de client réel. Placeholders fictifs uniquement (`Mon Projet`, `mon-projet`, `ACME`).
- **Versions épinglées** : résoudre la version exacte d'utoipa via Context7 au moment de l'ajout (cible : lignée **5.x**, ex. `5.5.0`). Référence = lockfile.
- **Vérification** : `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run` doivent être verts à la fin de chaque tâche qui touche du code Rust.
- **Run des commandes** : `fmt`/`clippy`/`test` depuis la **racine** du repo ; le serveur Loco depuis `backend/` (non requis dans ce plan).

---

## File Structure

| Fichier | Responsabilité | Action |
|---|---|---|
| `backend/src/dto/mod.rs` | Contrat de fil : DTO requête/réponse (`ToSchema`) + conversions `Model → DTO` | **Créer** (migre `latch-dto` + `controllers/dto.rs`) |
| `backend/src/lib.rs` | Déclaration des modules du crate | **Modifier** (`pub mod dto;` + `pub mod openapi;`) |
| `backend/src/controllers/dto.rs` | (ancien) re-export + conversions | **Supprimer** (déplacé dans `dto/`) |
| `backend/src/controllers/mod.rs` | Liste des sous-modules controllers | **Modifier** (retirer `pub mod dto;`) |
| `backend/src/controllers/admin.rs` | Handlers admin `/api/*` | **Modifier** (imports `crate::dto`, response DTO typés, `#[utoipa::path]`) |
| `backend/src/controllers/auth.rs` | Handlers login/logout | **Modifier** (imports `crate::dto`, `OkResponse`, `#[utoipa::path]`) |
| `backend/src/openapi.rs` | `#[derive(OpenApi)] ApiDoc` : agrège paths + schemas | **Créer** |
| `backend/tests/openapi_drift.rs` | Test de drift : régénère et compare `openapi.json` | **Créer** |
| `openapi.json` (racine repo) | Schéma OpenAPI commité (input du build front) | **Créer** (généré) |
| `backend/Cargo.toml` | Deps backend | **Modifier** (ajouter `utoipa`, retirer `latch-dto`) |
| `Cargo.toml` (racine) | Membres du workspace | **Modifier** (retirer `latch-dto`) |
| `latch-dto/` | (ancienne) crate partagée | **Supprimer** (`git rm -r`) |
| `deny.toml` | Allowlist licences | **Modifier si besoin** (licences utoipa) |
| `docs/INDEX.md`, `HANDOFF.md`, `CONVENTIONS.md`, `QUIRKS.md` | Mémoire projet | **Modifier** (clôture) |

---

## Task 1 : Module `dto` — migrer les types depuis `latch-dto` + `ToSchema`

Crée le module `crate::dto` qui rassemble les types du contrat de fil (ex-`latch-dto`) **et** les conversions `Model → DTO` (ex-`controllers/dto.rs`), avec la dérivation `utoipa::ToSchema`. Rebranche tous les imports vers `crate::dto` et supprime `controllers/dto.rs`. La crate `latch-dto` reste présente mais devient orpheline (supprimée en Task 2).

**Files:**
- Create: `backend/src/dto/mod.rs`
- Modify: `backend/src/lib.rs` (ajouter `pub mod dto;`)
- Modify: `backend/Cargo.toml` (ajouter la dépendance `utoipa`)
- Modify: `backend/src/controllers/mod.rs` (retirer `pub mod dto;`)
- Delete: `backend/src/controllers/dto.rs`
- Modify: `backend/src/controllers/admin.rs:13-15` et appels `crate::controllers::dto::*`
- Modify: `backend/src/controllers/auth.rs:16` (`use latch_dto::LoginReq;`)

**Interfaces:**
- Produces (consommé par toutes les tâches suivantes) :
  - Types : `dto::ProjectListItem`, `dto::ProjectDetail`, `dto::VersionItem`, `dto::CreateProjectReq`, `dto::UpdateProjectReq`, `dto::SetCodeReq`, `dto::DeployReq`, `dto::LoginReq` — tous `#[derive(Serialize, Deserialize, ToSchema)]`.
  - Conversions : `dto::to_list_item(&projects::Model) -> ProjectListItem`, `dto::to_detail(projects::Model, Vec<versions::Model>) -> ProjectDetail`.

- [ ] **Step 1 : Ajouter la dépendance utoipa**

Résoudre la version via Context7 (cible lignée 5.x), puis dans `backend/Cargo.toml`, sous `[dependencies]`, après la ligne `chrono = { version = "0.4" }` :

```toml
utoipa = { version = "5", features = ["chrono"] }
```

(La feature `chrono` n'est pas requise par nos DTO — les dates sont des `String` — mais l'aligne si un champ chrono est exposé plus tard ; la retirer si `cargo deny`/clippy s'en plaint. Pas de feature `axum` nécessaire : approche code-first manuelle.)

- [ ] **Step 2 : Créer `backend/src/dto/mod.rs`**

Contenu complet (types migrés depuis `latch-dto/src/lib.rs` + conversions migrées depuis `controllers/dto.rs`, avec `ToSchema` ajouté) :

```rust
//! Contrat de fil de l'API admin : DTO requête/réponse + conversions `Model → DTO`.
//! Source de vérité des shapes sérialisées (le schéma OpenAPI en dérive, cf. `openapi.rs`).
//! `ProjectListItem` n'a structurellement pas de `pin` (invariant §9.2). Dates = `String` RFC 3339.

use serde::de;
use serde::{Deserialize, Deserializer, Serialize};
use utoipa::ToSchema;

use crate::models::_entities::{projects, versions};

/// Item de liste — **sans PIN** (invariant §9.2 : structurellement absent).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ProjectListItem {
    pub id: i32,
    pub slug: String,
    pub name: String,
    pub code_enabled: bool,
    pub brand_name: Option<String>,
    pub active_version_id: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct VersionItem {
    pub id: i32,
    pub n: i32,
    pub created_at: String,
    pub is_active: bool,
}

/// Détail — expose le PIN (copiable en admin uniquement, invariant §9.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ProjectDetail {
    pub id: i32,
    pub slug: String,
    pub name: String,
    pub code_enabled: bool,
    pub pin: Option<String>,
    pub brand_name: Option<String>,
    pub active_version_id: Option<i32>,
    pub versions: Vec<VersionItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CreateProjectReq {
    pub name: String,
    #[serde(default)]
    pub brand_name: Option<String>,
    #[serde(default = "default_true")]
    pub code_enabled: bool,
    #[serde(default)]
    pub pin: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Deserialize un `Option<Option<String>>` en distinguant absent / null / valeur.
fn deserialize_optional_optional_string<'de, D>(
    deserializer: D,
) -> Result<Option<Option<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OptionalOptionalString;

    impl<'de> de::Visitor<'de> for OptionalOptionalString {
        type Value = Option<Option<String>>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("null or a string")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(None))
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            String::deserialize(deserializer).map(|s| Some(Some(s)))
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(None))
        }
    }

    deserializer.deserialize_option(OptionalOptionalString)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct UpdateProjectReq {
    #[serde(default)]
    pub name: Option<String>,
    /// `Option<Option<String>>` : absent ⇒ inchangé ; `null` ⇒ effacer ; valeur ⇒ définir.
    /// Vu par OpenAPI comme une string nullable (`value_type` force le schéma).
    #[serde(default, deserialize_with = "deserialize_optional_optional_string")]
    #[schema(value_type = Option<String>, nullable)]
    pub brand_name: Option<Option<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SetCodeReq {
    pub pin: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct DeployReq {
    pub html: String,
    #[serde(default)]
    pub activate: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct LoginReq {
    pub user: String,
    pub pass: String,
}

/// Projet → item de liste (sans PIN).
pub fn to_list_item(m: &projects::Model) -> ProjectListItem {
    ProjectListItem {
        id: m.id,
        slug: m.slug.clone(),
        name: m.name.clone(),
        code_enabled: m.code_enabled,
        brand_name: m.brand_name.clone(),
        active_version_id: m.active_version_id,
    }
}

/// Projet + ses versions → détail (avec PIN).
pub fn to_detail(m: projects::Model, vers: Vec<versions::Model>) -> ProjectDetail {
    let active = m.active_version_id;
    let versions = vers
        .into_iter()
        .map(|v| VersionItem {
            id: v.id,
            n: v.n,
            created_at: v.created_at.to_rfc3339(),
            is_active: Some(v.id) == active,
        })
        .collect();
    ProjectDetail {
        id: m.id,
        slug: m.slug,
        name: m.name,
        code_enabled: m.code_enabled,
        pin: m.pin,
        brand_name: m.brand_name,
        active_version_id: m.active_version_id,
        versions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_model() -> projects::Model {
        projects::Model {
            id: 1,
            slug: "mon-projet-k7Qp2maZ".to_string(),
            name: "Mon Projet".to_string(),
            code_enabled: true,
            pin: Some("424242".to_string()),
            brand_name: None,
            active_version_id: None,
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
        }
    }

    #[test]
    fn list_item_never_serializes_pin() {
        let json = serde_json::to_string(&to_list_item(&sample_model())).unwrap();
        assert!(!json.contains("424242"), "le PIN ne doit JAMAIS apparaître en liste (§9.2)");
        assert!(!json.contains("\"pin\""), "le champ pin ne doit pas exister en liste (§9.2)");
    }

    #[test]
    fn detail_does_serialize_pin() {
        let json = serde_json::to_string(&to_detail(sample_model(), vec![])).unwrap();
        assert!(json.contains("424242"), "le détail doit exposer le PIN (copiable en admin)");
    }

    #[test]
    fn create_req_defaults_code_enabled_true() {
        let req: CreateProjectReq = serde_json::from_str(r#"{"name":"X"}"#).unwrap();
        assert!(req.code_enabled, "code_enabled défaut = true (contrat §3)");
        assert_eq!(req.name, "X");
    }

    #[test]
    fn update_req_brand_name_absent_vs_null() {
        let absent: UpdateProjectReq = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(absent.brand_name, None, "champ absent = pas de changement");
        let cleared: UpdateProjectReq = serde_json::from_str(r#"{"brand_name":null}"#).unwrap();
        assert_eq!(cleared.brand_name, Some(None), "null = effacer le brand_name");
        let set: UpdateProjectReq = serde_json::from_str(r#"{"brand_name":"ACME"}"#).unwrap();
        assert_eq!(set.brand_name, Some(Some("ACME".to_string())), "valeur = définir");
    }
}
```

- [ ] **Step 3 : Déclarer le module dans `backend/src/lib.rs`**

Ajouter `pub mod dto;` (ordre alphabétique, après `pub mod data;`) :

```rust
pub mod app;
pub mod controllers;
pub mod data;
pub mod dto;
pub mod initializers;
pub mod models;
pub mod services;
pub mod tasks;
pub mod views;
pub mod web;
```

- [ ] **Step 4 : Rebrancher les imports puis supprimer `controllers/dto.rs`**

Dans `backend/src/controllers/admin.rs`, remplacer l'import (lignes 13-15) :

```rust
use crate::dto::{CreateProjectReq, DeployReq, ProjectListItem, SetCodeReq, UpdateProjectReq};
```

et remplacer les trois appels `crate::controllers::dto::to_list_item` / `crate::controllers::dto::to_detail` par `crate::dto::to_list_item` / `crate::dto::to_detail` (lignes ~29, ~56, ~77, ~127, ~175, ~187).

Dans `backend/src/controllers/auth.rs`, remplacer la ligne 16 :

```rust
use crate::dto::LoginReq;
```

Dans `backend/src/controllers/mod.rs`, retirer la ligne `pub mod dto;`.

Puis supprimer le fichier :

```bash
git rm backend/src/controllers/dto.rs
```

- [ ] **Step 5 : Vérifier — fmt, clippy, tests**

Run :
```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo nextest run
```
Expected : compile OK ; tests `dto::tests::*` PASS (4 tests) ; toute la suite (≈86) verte. `latch-dto` est encore en dépendance mais plus importée — c'est attendu (nettoyé en Task 2).

- [ ] **Step 6 : Commit**

```bash
git add backend/src/dto/mod.rs backend/src/lib.rs backend/Cargo.toml \
        backend/src/controllers/mod.rs backend/src/controllers/admin.rs \
        backend/src/controllers/auth.rs
git rm backend/src/controllers/dto.rs 2>/dev/null; git add -A
git commit -m "$(cat <<'EOF'
♻️ refactor(dto): inline latch-dto dans backend/src/dto + dérive utoipa::ToSchema

Les DTO du contrat de fil et les conversions Model→DTO emménagent dans
crate::dto (ToSchema ajouté), prérequis de la génération OpenAPI. La crate
latch-dto devient orpheline (retirée en Task 2).

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01XhGRMj78xsAunbwfpyFUbR
EOF
)"
```

---

## Task 2 : Retirer la crate `latch-dto`

`latch-dto` n'a plus aucun consommateur (Task 1 a tout rebranché). On la supprime du workspace et des dépendances.

**Files:**
- Modify: `Cargo.toml` (racine) — retirer `latch-dto` de `members` et `default-members`
- Modify: `backend/Cargo.toml` — retirer `latch-dto = { path = "../latch-dto" }`
- Delete: `latch-dto/` (toute la crate)

**Interfaces:**
- Consumes : rien de neuf (les types vivent désormais dans `crate::dto`, fournis par Task 1).
- Produces : workspace à 2 membres, plus aucune référence à `latch_dto`.

- [ ] **Step 1 : Retirer la dépendance dans `backend/Cargo.toml`**

Supprimer la ligne (≈13) :
```toml
latch-dto = { path = "../latch-dto" }
```

- [ ] **Step 2 : Retirer du workspace racine `Cargo.toml`**

Remplacer les lignes `members` / `default-members` par :
```toml
members = ["backend", "backend/migration"]
default-members = ["backend", "backend/migration"]
```
(Mettre à jour le commentaire au-dessus pour refléter que `latch-dto` est désormais inliné dans `backend/src/dto/`.)

- [ ] **Step 3 : Supprimer la crate**

```bash
git rm -r latch-dto
```

- [ ] **Step 4 : Vérifier**

Run :
```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo nextest run
```
Expected : compile OK (aucune référence `latch_dto` résiduelle) ; suite verte. Si une référence subsiste, l'erreur `unresolved import latch_dto` indique le fichier à corriger.

- [ ] **Step 5 : Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
🔥 chore(dto): retrait de la crate latch-dto (inlinée dans backend/src/dto)

Plus de consommateur Rust externe : le contrat de fil vit dans crate::dto et
sera partagé au front via le schéma OpenAPI généré, pas une crate partagée.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01XhGRMj78xsAunbwfpyFUbR
EOF
)"
```

---

## Task 3 : Réponses ad-hoc → DTO typés (`OkResponse`, `DeployResponse`, `ActivateResponse`)

Les handlers renvoient aujourd'hui des `serde_json::json!({...})` non typés. On les remplace par des structs `ToSchema` pour que le client front (Plan 2) ait des types pour **toutes** les réponses. Le JSON produit est **identique** (mêmes clés) → les tests d'intégration restent verts.

**Files:**
- Modify: `backend/src/dto/mod.rs` (ajouter 3 structs de réponse)
- Modify: `backend/src/controllers/admin.rs` (`delete`, `deploy`, `activate_version`, `delete_version`)
- Modify: `backend/src/controllers/auth.rs` (`login`, `logout`)

**Interfaces:**
- Produces : `dto::OkResponse { ok: bool }`, `dto::DeployResponse { id: i32, n: i32 }`, `dto::ActivateResponse { ok: bool, active_version_id: i32 }` — tous `Serialize + ToSchema`.

- [ ] **Step 1 : Ajouter les structs de réponse dans `backend/src/dto/mod.rs`**

Après `LoginReq` (avant le bloc `to_list_item`) :

```rust
/// Réponse générique « succès » (`{"ok": true}`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct OkResponse {
    pub ok: bool,
}

impl OkResponse {
    pub fn ok() -> Self {
        Self { ok: true }
    }
}

/// Réponse de déploiement : identifiant et numéro de la version créée.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct DeployResponse {
    pub id: i32,
    pub n: i32,
}

/// Réponse d'activation : confirme la bascule et renvoie le pointeur actif.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ActivateResponse {
    pub ok: bool,
    pub active_version_id: i32,
}
```

- [ ] **Step 2 : Remplacer les `json!` dans `admin.rs`**

- `delete` (≈162) : `format::json(serde_json::json!({"ok": true}))` → `format::json(crate::dto::OkResponse::ok())`
- `delete_version` (≈282) : idem → `format::json(crate::dto::OkResponse::ok())`
- `deploy` (≈205) : `format::json(serde_json::json!({"id": version.id, "n": version.n}))` → `format::json(crate::dto::DeployResponse { id: version.id, n: version.n })`
- `activate_version` (≈241) : `format::json(serde_json::json!({"ok": true, "active_version_id": version.id}))` → `format::json(crate::dto::ActivateResponse { ok: true, active_version_id: version.id })`

- [ ] **Step 3 : Remplacer les `json!` dans `auth.rs`**

- `login` (≈66) et `logout` (≈75) : `format::json(serde_json::json!({"ok": true}))` → `format::json(crate::dto::OkResponse::ok())`

- [ ] **Step 4 : Vérifier (le shape JSON est inchangé → tests d'intégration verts)**

Run :
```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo nextest run
```
Expected : suite verte, **notamment** `admin_api::*` et `security_invariants::*` (ils asservissent le JSON de réponse — `{"ok":true}`, `{"id":..,"n":..}` — qui n'a pas changé).

- [ ] **Step 5 : Commit**

```bash
git add backend/src/dto/mod.rs backend/src/controllers/admin.rs backend/src/controllers/auth.rs
git commit -m "$(cat <<'EOF'
✨ feat(dto): réponses typées OkResponse/DeployResponse/ActivateResponse

Remplace les serde_json::json! ad-hoc par des structs ToSchema (JSON identique)
pour que le client front ait des types sur toutes les réponses.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01XhGRMj78xsAunbwfpyFUbR
EOF
)"
```

---

## Task 4 : Annoter chaque handler avec `#[utoipa::path]`

On documente chaque route `/api/*` de façon déclarative. `#[utoipa::path]` génère un type compagnon `__path_<fn>` consommé par `ApiDoc` (Task 5). Placer l'attribut **avant** `#[debug_handler]`. Pas de test runtime ici : le deliverable est la **compilation** (une annotation mal formée casse le build — utoipa valide à la macro-expansion).

**Files:**
- Modify: `backend/src/controllers/admin.rs` (11 handlers)
- Modify: `backend/src/controllers/auth.rs` (2 handlers)

**Interfaces:**
- Consumes : types `dto::*` (Task 1, 3).
- Produces : symboles `__path_list`, `__path_detail`, `__path_create`, `__path_update`, `__path_delete`, `__path_set_code`, `__path_clear_code`, `__path_deploy`, `__path_activate_version`, `__path_delete_version`, `__path_preview_version` (admin) ; `__path_login`, `__path_logout` (auth). Consommés par `ApiDoc` (Task 5).

- [ ] **Step 1 : Annoter les handlers de `admin.rs`**

Ajouter l'attribut au-dessus de chaque `#[debug_handler]`. Bloc par handler (le `path` reprend le préfixe `/api` réel) :

```rust
#[utoipa::path(
    get, path = "/api/projects", tag = "projects",
    responses((status = 200, description = "Liste des projets (sans PIN)", body = Vec<ProjectListItem>),
              (status = 401, description = "Non authentifié"))
)]
// au-dessus de: async fn list(...)

#[utoipa::path(
    get, path = "/api/projects/{id}", tag = "projects",
    params(("id" = i32, Path, description = "Identifiant du projet")),
    responses((status = 200, description = "Détail du projet (avec PIN)", body = ProjectDetail),
              (status = 404, description = "Projet inconnu"),
              (status = 401, description = "Non authentifié"))
)]
// au-dessus de: async fn detail(...)

#[utoipa::path(
    post, path = "/api/projects", tag = "projects",
    request_body = CreateProjectReq,
    responses((status = 200, description = "Projet créé", body = ProjectDetail),
              (status = 400, description = "Requête invalide"),
              (status = 401, description = "Non authentifié"),
              (status = 403, description = "Origin invalide (CSRF)"))
)]
// au-dessus de: async fn create(...)

#[utoipa::path(
    put, path = "/api/projects/{id}", tag = "projects",
    params(("id" = i32, Path, description = "Identifiant du projet")),
    request_body = UpdateProjectReq,
    responses((status = 200, description = "Projet mis à jour", body = ProjectDetail),
              (status = 400, description = "Requête invalide"),
              (status = 404, description = "Projet inconnu"),
              (status = 401, description = "Non authentifié"),
              (status = 403, description = "Origin invalide (CSRF)"))
)]
// au-dessus de: async fn update(...)

#[utoipa::path(
    delete, path = "/api/projects/{id}", tag = "projects",
    params(("id" = i32, Path, description = "Identifiant du projet")),
    responses((status = 200, description = "Projet supprimé", body = OkResponse),
              (status = 404, description = "Projet inconnu"),
              (status = 401, description = "Non authentifié"),
              (status = 403, description = "Origin invalide (CSRF)"))
)]
// au-dessus de: async fn delete(...)

#[utoipa::path(
    post, path = "/api/projects/{id}/code", tag = "projects",
    params(("id" = i32, Path, description = "Identifiant du projet")),
    request_body = SetCodeReq,
    responses((status = 200, description = "Code activé", body = ProjectDetail),
              (status = 401, description = "Non authentifié"),
              (status = 403, description = "Origin invalide (CSRF)"))
)]
// au-dessus de: async fn set_code(...)

#[utoipa::path(
    delete, path = "/api/projects/{id}/code", tag = "projects",
    params(("id" = i32, Path, description = "Identifiant du projet")),
    responses((status = 200, description = "Code désactivé", body = ProjectDetail),
              (status = 401, description = "Non authentifié"),
              (status = 403, description = "Origin invalide (CSRF)"))
)]
// au-dessus de: async fn clear_code(...)

#[utoipa::path(
    post, path = "/api/projects/{id}/deploy", tag = "versions",
    params(("id" = i32, Path, description = "Identifiant du projet")),
    request_body = DeployReq,
    responses((status = 200, description = "Version déployée", body = DeployResponse),
              (status = 401, description = "Non authentifié"),
              (status = 403, description = "Origin invalide (CSRF)"))
)]
// au-dessus de: async fn deploy(...)

#[utoipa::path(
    post, path = "/api/projects/{id}/versions/{n}/activate", tag = "versions",
    params(("id" = i32, Path, description = "Identifiant du projet"),
           ("n" = i32, Path, description = "Numéro de version")),
    responses((status = 200, description = "Version activée", body = ActivateResponse),
              (status = 404, description = "Version inconnue"),
              (status = 401, description = "Non authentifié"),
              (status = 403, description = "Origin invalide (CSRF)"))
)]
// au-dessus de: async fn activate_version(...)

#[utoipa::path(
    delete, path = "/api/projects/{id}/versions/{n}", tag = "versions",
    params(("id" = i32, Path, description = "Identifiant du projet"),
           ("n" = i32, Path, description = "Numéro de version")),
    responses((status = 200, description = "Version supprimée", body = OkResponse),
              (status = 400, description = "Version active : suppression refusée"),
              (status = 404, description = "Version inconnue"),
              (status = 401, description = "Non authentifié"),
              (status = 403, description = "Origin invalide (CSRF)"))
)]
// au-dessus de: async fn delete_version(...)

#[utoipa::path(
    get, path = "/api/projects/{id}/versions/{n}/preview", tag = "versions",
    params(("id" = i32, Path, description = "Identifiant du projet"),
           ("n" = i32, Path, description = "Numéro de version")),
    responses((status = 200, description = "HTML brut de la version", content_type = "text/html"),
              (status = 404, description = "Version inconnue"),
              (status = 401, description = "Non authentifié"))
)]
// au-dessus de: async fn preview_version(...)
```

- [ ] **Step 2 : Annoter les handlers de `auth.rs`**

```rust
#[utoipa::path(
    post, path = "/api/login", tag = "auth",
    request_body = LoginReq,
    responses((status = 200, description = "Authentifié (cookie de session posé)", body = OkResponse),
              (status = 401, description = "Identifiants invalides"))
)]
// au-dessus de: async fn login(...)

#[utoipa::path(
    post, path = "/api/logout", tag = "auth",
    responses((status = 200, description = "Session détruite", body = OkResponse),
              (status = 403, description = "Origin invalide (CSRF)"))
)]
// au-dessus de: async fn logout(...)
```

- [ ] **Step 3 : Vérifier la compilation (les macros valident le schéma)**

Run :
```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings
```
Expected : compile OK. Une erreur typique `body = Foo` avec `Foo` non `ToSchema` indique un type non dérivé — vérifier Task 1/3. Les symboles `__path_*` sont générés mais pas encore agrégés (Task 5).

- [ ] **Step 4 : Commit**

```bash
git add backend/src/controllers/admin.rs backend/src/controllers/auth.rs
git commit -m "$(cat <<'EOF'
✨ feat(openapi): annotations #[utoipa::path] sur toutes les routes /api

Documente méthode, params, request_body et réponses de chaque endpoint admin
et auth (déclaratif). Agrégé en ApiDoc à la tâche suivante.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01XhGRMj78xsAunbwfpyFUbR
EOF
)"
```

---

## Task 5 : `ApiDoc` — agréger paths + schemas + test de structure

Crée le document OpenAPI agrégé et un test inline qui vérifie qu'il contient bien tous les chemins et schémas attendus.

**Files:**
- Create: `backend/src/openapi.rs`
- Modify: `backend/src/lib.rs` (ajouter `pub mod openapi;`)

**Interfaces:**
- Consumes : symboles `__path_*` (Task 4), types `dto::*` (Task 1, 3).
- Produces : `latch::openapi::ApiDoc` implémentant `utoipa::OpenApi` → `ApiDoc::openapi() -> utoipa::openapi::OpenApi`. Consommé par le test de drift (Task 6).

- [ ] **Step 1 : Créer `backend/src/openapi.rs`**

```rust
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
            assert!(paths.contains_key(expected), "chemin manquant dans l'OpenAPI : {expected}");
        }
    }

    #[test]
    fn document_contains_core_schemas() {
        // Le JSON sérialisé doit référencer les schémas clés du contrat.
        let json = ApiDoc::openapi().to_pretty_json().unwrap();
        for schema in ["ProjectListItem", "ProjectDetail", "CreateProjectReq", "DeployResponse"] {
            assert!(json.contains(schema), "schéma manquant dans l'OpenAPI : {schema}");
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
        assert!(!window.contains("\"pin\""), "ProjectListItem ne doit pas exposer pin (§9.2)");
    }
}
```

> Note : `__path_*` ne sont pas importés explicitement — la macro `#[openapi(paths(auth::login, ...))]` les résout via le chemin de la fonction. `auth::login`/`admin::list` etc. doivent être visibles depuis `openapi.rs` ; ils sont `async fn` privés au module. La macro utoipa accède au type compagnon `__path_login` généré **dans le même module** que `login` ; référencer `auth::login` suffit (utoipa dérive le chemin du `__path`). Si la macro se plaint de visibilité, rendre les handlers `pub(crate)` (changer `async fn login` → `pub(crate) async fn login`, idem pour chaque handler annoté).

- [ ] **Step 2 : Déclarer le module dans `backend/src/lib.rs`**

Ajouter `pub mod openapi;` (après `pub mod models;`) :

```rust
pub mod app;
pub mod controllers;
pub mod data;
pub mod dto;
pub mod initializers;
pub mod models;
pub mod openapi;
pub mod services;
pub mod tasks;
pub mod views;
pub mod web;
```

- [ ] **Step 3 : Vérifier**

Run :
```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo nextest run
```
Expected : tests `openapi::tests::document_contains_all_paths`, `document_contains_core_schemas`, `list_schema_has_no_pin_field` PASS. Si erreur de visibilité des handlers, appliquer la note du Step 1 (`pub(crate) async fn ...`) et relancer.

- [ ] **Step 4 : Commit**

```bash
git add backend/src/openapi.rs backend/src/lib.rs backend/src/controllers/admin.rs backend/src/controllers/auth.rs
git commit -m "$(cat <<'EOF'
✨ feat(openapi): ApiDoc agrège paths + schemas + tests de structure

Document OpenAPI code-first agrégé ; tests : tous les chemins présents, schémas
clés présents, ProjectListItem sans champ pin (§9.2 reflété dans le contrat).

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01XhGRMj78xsAunbwfpyFUbR
EOF
)"
```

---

## Task 6 : `openapi.json` commité + test de drift

Génère le schéma figé à la racine du repo (input du build front, Plan 2/3) et le verrouille par un test qui échoue si un DTO change sans régénération.

**Files:**
- Create: `backend/tests/openapi_drift.rs`
- Create: `openapi.json` (racine du repo, généré)

**Interfaces:**
- Consumes : `latch::openapi::ApiDoc` (Task 5).
- Produces : `openapi.json` commité ; convention de régénération `UPDATE_OPENAPI=1 cargo test --test openapi_drift`.

- [ ] **Step 1 : Écrire le test de drift**

`backend/tests/openapi_drift.rs` :

```rust
//! Verrou anti-drift : `openapi.json` (racine) doit toujours refléter `ApiDoc`.
//! Régénérer après tout changement de DTO/route : `UPDATE_OPENAPI=1 cargo test --test openapi_drift`.

use std::path::PathBuf;

use latch::openapi::ApiDoc;
use utoipa::OpenApi;

fn openapi_json_path() -> PathBuf {
    // CARGO_MANIFEST_DIR = .../backend ; le schéma vit à la racine du repo.
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../openapi.json")
}

#[test]
fn openapi_json_is_in_sync() {
    let generated = ApiDoc::openapi()
        .to_pretty_json()
        .expect("sérialisation OpenAPI");
    let path = openapi_json_path();

    if std::env::var("UPDATE_OPENAPI").is_ok() {
        std::fs::write(&path, format!("{generated}\n")).expect("écriture openapi.json");
        return;
    }

    let on_disk = std::fs::read_to_string(&path).expect(
        "openapi.json manquant — générer avec: UPDATE_OPENAPI=1 cargo test --test openapi_drift",
    );
    assert_eq!(
        on_disk.trim_end(),
        generated.trim_end(),
        "openapi.json périmé — régénérer: UPDATE_OPENAPI=1 cargo test --test openapi_drift"
    );
}
```

- [ ] **Step 2 : Générer le fichier initial**

Run :
```bash
UPDATE_OPENAPI=1 cargo test --test openapi_drift
```
Expected : le test PASS (mode écriture) et crée `openapi.json` à la racine. Vérifier qu'il existe et est non vide :
```bash
rtk ls openapi.json && head -c 200 openapi.json
```

- [ ] **Step 3 : Vérifier le drift en mode lecture (CI)**

Run :
```bash
cargo nextest run
```
Expected : `openapi_drift::openapi_json_is_in_sync` PASS (le fichier sur disque == génération). Toute la suite verte.

- [ ] **Step 4 : Commit**

```bash
git add backend/tests/openapi_drift.rs openapi.json
git commit -m "$(cat <<'EOF'
✅ test(openapi): openapi.json commité + verrou anti-drift

Le schéma figé à la racine est l'input du build front. Le test régénère et
compare (UPDATE_OPENAPI=1 pour réécrire) — garde-fou qui remplace le partage
latch-dto.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01XhGRMj78xsAunbwfpyFUbR
EOF
)"
```

---

## Task 7 : Swagger UI en dev uniquement (confort)

Expose le document OpenAPI + une UI Swagger sous `/api-docs`, **uniquement** hors production (contrat §4 : absent de l'image distroless). Tâche de confort dev ; sautable sans impact sur le front (qui consomme `openapi.json` au build).

**Files:**
- Modify: `backend/Cargo.toml` (ajouter `utoipa-swagger-ui`)
- Modify: `backend/src/app.rs` (`after_routes` : monter Swagger si non-prod)

**Interfaces:**
- Consumes : `latch::openapi::ApiDoc` (Task 5).
- Produces : route `/api-docs` (dev/test seulement).

- [ ] **Step 1 : Ajouter la dépendance**

Résoudre la version compatible via Context7 (alignée sur utoipa 5.x), puis dans `backend/Cargo.toml` :

```toml
utoipa-swagger-ui = { version = "8", features = ["axum"] }
```

- [ ] **Step 2 : Monter Swagger en non-prod dans `after_routes`**

Dans `backend/src/app.rs`, dans `after_routes`, **avant** le `Ok(router)` final et après le montage SPA, insérer :

```rust
        // Swagger UI : confort dev uniquement. Jamais en production (surface + poids).
        // Fail-secure : exclure Production via le même critère que le cookie Secure.
        let is_prod = !matches!(
            ctx.environment,
            loco_rs::environment::Environment::Development | loco_rs::environment::Environment::Test
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
```

(Adapter l'import : `SwaggerUi` est ré-exporté par `utoipa_swagger_ui`. Le `merge` opère sur l'`axum::Router` reçu par `after_routes`.)

- [ ] **Step 3 : Vérifier**

Run :
```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo nextest run
```
Expected : compile + suite verte. (La présence de la route dev n'est pas testée automatiquement ici ; vérification manuelle optionnelle : `cd backend && cargo loco start` puis ouvrir `http://127.0.0.1:5150/api-docs`.)

- [ ] **Step 4 : Vérifier que cargo-deny passe (nouvelles deps)**

Si `cargo-deny` est installé localement :
```bash
cargo deny check licenses advisories
```
Sinon, déléguer à la CI. Si une licence d'une dépendance transitive d'`utoipa`/`utoipa-swagger-ui` manque dans `deny.toml`, l'ajouter à `allow = [...]` (cf. QUIRKS « cargo-deny = liste blanche stricte »).

- [ ] **Step 5 : Commit**

```bash
git add backend/Cargo.toml backend/src/app.rs Cargo.lock deny.toml
git commit -m "$(cat <<'EOF'
✨ feat(openapi): Swagger UI sous /api-docs en dev uniquement

Monté dans after_routes hors Production (fail-secure). Absent de l'image
distroless de prod. Confort de debug du contrat API.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01XhGRMj78xsAunbwfpyFUbR
EOF
)"
```

---

## Task 8 : Clôture mémoire du Plan 1

Met à jour la base de connaissances (règle non-négociable du `CLAUDE.md`). Pas de code ; documentation uniquement.

**Files:**
- Modify: `docs/INDEX.md`, `docs/HANDOFF.md`, `docs/CONVENTIONS.md`, `docs/QUIRKS.md`

- [ ] **Step 1 : `docs/INDEX.md` — livrables Backend OpenAPI**

Dans la section « Backend (cœur + adaptateurs) », ajouter :
```markdown
- [x] DTO inlinés dans `backend/src/dto/` (ex-`latch-dto`) + dérivation `utoipa::ToSchema` — Migration React Plan 1 — 2026-06-25
- [x] Réponses typées `OkResponse`/`DeployResponse`/`ActivateResponse` (fin des `serde_json::json!` ad-hoc) — Migration React Plan 1 — 2026-06-25
- [x] `#[utoipa::path]` sur toutes les routes `/api/*` + `openapi::ApiDoc` (paths + schemas) — Migration React Plan 1 — 2026-06-25
- [x] `openapi.json` commité (racine) + test de drift `backend/tests/openapi_drift.rs` — Migration React Plan 1 — 2026-06-25
- [x] Swagger UI sous `/api-docs` en dev uniquement — Migration React Plan 1 — 2026-06-25
```
Et marquer la crate `latch-dto` comme retirée (annoter la ligne existante si présente, ou ajouter une note).

- [ ] **Step 2 : `docs/CONVENTIONS.md` — pattern d'annotation OpenAPI**

Ajouter une section « Annoter un endpoint pour OpenAPI (utoipa) » :
```markdown
## Endpoint OpenAPI (utoipa) type

Chaque handler `/api/*` porte un `#[utoipa::path(...)]` (avant `#[debug_handler]`) décrivant
méthode, `path` (préfixe `/api` inclus), `params` (path params typés), `request_body`, et
`responses` (avec `body = <DTO ToSchema>`). Les réponses non-DTO (`{ok:true}`, `{id,n}`) sont
des structs `ToSchema` dédiées dans `crate::dto`, pas des `serde_json::json!`. Le handler est
ajouté à `openapi::ApiDoc` (`paths(...)`), son DTO de réponse à `components(schemas(...))`.
Après tout changement : `UPDATE_OPENAPI=1 cargo test --test openapi_drift` pour régénérer
`openapi.json`. Les handlers annotés sont `pub(crate)` si la macro exige la visibilité.
```

- [ ] **Step 3 : `docs/QUIRKS.md` — pièges utoipa rencontrés**

Ajouter toute embûche réelle (ex. visibilité `pub(crate)` des handlers, `#[schema(value_type = Option<String>)]` pour `Option<Option<String>>`, ordre `#[utoipa::path]` avant `#[debug_handler]`, résolution CWD du test de drift via `CARGO_MANIFEST_DIR`). N'écrire que ce qui a effectivement mordu pendant l'implémentation.

- [ ] **Step 4 : `docs/HANDOFF.md` — entrée datée**

Ajouter en haut (sous le H1) une entrée `## 2026-06-25 — Migration React Plan 1 : Backend OpenAPI livré` avec : `Dernière chose faite`, `Trucs en suspens` (Plan 2 = front React), `Prochaine chose à creuser` (écrire le Plan 2), `Notes pour future Claude` (régénération `openapi.json`, Swagger en dev).

- [ ] **Step 5 : Commit**

```bash
git add docs/INDEX.md docs/HANDOFF.md docs/CONVENTIONS.md docs/QUIRKS.md
git commit -m "$(cat <<'EOF'
📝 docs: clôture mémoire Plan 1 (backend OpenAPI livré)

INDEX (livrables), CONVENTIONS (pattern annotation utoipa), QUIRKS (pièges),
HANDOFF (entrée datée).

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01XhGRMj78xsAunbwfpyFUbR
EOF
)"
```

---

## Critères de sortie du Plan 1

- `cargo fmt --all` clean, `cargo clippy --all-targets -- -D warnings` 0 warning.
- `cargo nextest run` vert, **dont** : `dto::tests::*`, `openapi::tests::*`, `openapi_drift::openapi_json_is_in_sync`, et les tests d'intégration existants (`admin_api`, `security_invariants`, `spa_serving`, `architecture`) toujours verts.
- `latch-dto` supprimée ; workspace à 2 membres ; aucune référence `latch_dto`.
- `openapi.json` commité à la racine, à jour avec `ApiDoc`.
- Garde d'architecture verte (services sans axum/loco — inchangée).
- Docs mémoire à jour (INDEX, HANDOFF, CONVENTIONS, QUIRKS).
- Aucun nom de client réel.

## Hors périmètre (Plans suivants)

- **Plan 2** : app React (scaffold Vite, `openapi-typescript` depuis `openapi.json`, client `openapi-fetch`, Query/RHF+zod/i18next/sonner, pages+panels, tests Vitest+MSW).
- **Plan 3** : CI en pistes (reusable workflows) + supply-chain front, Docker stage Node/pnpm, `.nvmrc`, smoke e2e Playwright, alignement docs (BOOTSTRAP, contrat, ROADMAP, ENVIRONMENT, BACKLOG, README).
