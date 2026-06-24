# Phase 1 — Cœur (services) + modèle + migrations — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Construire le cœur métier de `latch` (agnostique HTTP) et son modèle de données — migrations `projects`/`versions`, services `slug`/`security`/`pin`/`storage`/`projects`/`deploy`, le tout couvert par des tests unitaires verts.

**Architecture:** Couche service hexagonale légère (contrat §1). Le cœur vit dans `backend/src/services/`, ne dépend **ni d'axum ni de loco**, rend un `CoreError` (thiserror), et suppose son appelant déjà autorisé. Les entités SeaORM (`models/_entities/`) sont **générées** depuis les migrations, puis consommées directement par les services. Le `deploy()` respecte l'ordre fichier→DB du contrat §8, dans une transaction.

**Tech Stack:** Rust 2021, Loco 0.16 (squelette), SeaORM 1.1 (`sqlx-sqlite`, `bundled`), `thiserror` 2, `rand` 0.8, `subtle` 2, `chrono`, `async-trait`, `tokio` (`fs`). Tests : `tokio::test` sur SQLite in-memory + `tempfile`.

## Global Constraints

- **Le cœur ne voit jamais axum/loco.** Aucun `use axum::` ni `use loco_rs::` dans `backend/src/services/`. Violation = bug d'architecture (contrat §1, QUIRKS). Les services dépendent de `sea_orm`, `std`, `tokio::fs` — pas du framework.
- **Pas d'`unwrap`/`expect`** hors `#[cfg(test)]`. Erreurs propagées via `CoreError` (BOOTSTRAP §4).
- **Versions épinglées** (déjà au lock, ne pas bump) : Loco 0.16, sea-orm 1.1, libsqlite3-sys `bundled`. Nouvelles deps : `thiserror = "2"`, `rand = "0.8"`, `subtle = "2"`, dev `tempfile = "3"`, + feature `fs` sur `tokio`.
- **Suffixe de slug = 8 chars base62** `[A-Za-z0-9]` (≈47 bits), décision actée 2026-06-24 (QUIRKS, contrat §6).
- **PIN = 6 chiffres, stocké en clair** (récupérable, contrat §3). Aucun hash nulle part (invariant sécu contrat §9). Comparaison du PIN **à temps constant** (`subtle`).
- **Table `sessions` hors périmètre Phase 1** (reportée Phase 2, décision 2026-06-24 — ROADMAP).
- **Commandes Loco depuis `backend/`** (Loco lit `./config` au CWD — QUIRKS) ; `fmt`/`clippy`/`test` depuis la racine. En pratique, préfixer les commandes par `rtk`.
- **Commits gitmoji + conventionnel** : `<gitmoji> <type>: <desc>` (BOOTSTRAP §4).
- **Définition de terminé** (CLAUDE.md) : `cargo fmt` + `cargo clippy --all-targets -- -D warnings` verts, tests verts, doc à jour.

---

## File Structure

| Fichier | Responsabilité |
|---|---|
| `backend/Cargo.toml` | + deps `thiserror`/`rand`/`subtle`, dev `tempfile`, feature `fs` sur tokio |
| `backend/src/lib.rs` | + `pub mod services;` |
| `backend/src/services/mod.rs` | déclare les sous-modules + `pub use errors::CoreError;` + `test_support` (cfg test) |
| `backend/src/services/errors.rs` | `CoreError` (thiserror) — `NotFound`, `Validation`, `Db`, `Io` |
| `backend/src/services/slug.rs` | pur : `slugify_base`, `random_suffix`, `generate_slug` |
| `backend/src/services/security.rs` | pur : `secure_compare` (temps constant) |
| `backend/src/services/pin.rs` | pur : `generate_pin`, `is_valid_pin` |
| `backend/src/services/storage.rs` | trait `Storage` + `FsStorage` (write atomique tmp→rename, read) |
| `backend/src/services/projects.rs` | `ProjectsService` (DB) : create/list/get_by_slug/set_code/clear_code/verify_code + `CreateProject` |
| `backend/src/services/deploy.rs` | `DeployService` (DB+Storage) : `deploy()` (ordre fichier→tx, flip pointeur) |
| `backend/src/services/test_support.rs` | `#[cfg(test)]` : `test_db()` SQLite in-memory + `Migrator::up` |
| `backend/migration/src/m20260624_000001_create_projects.rs` | table `projects` |
| `backend/migration/src/m20260624_000002_create_versions.rs` | table `versions` + index unique `(project_id, n)` |
| `backend/migration/src/lib.rs` | enregistre les 2 migrations |
| `backend/src/models/_entities/{projects,versions}.rs` | **générés** par `cargo loco db entities` |

**Décisions de modélisation actées dans ce plan :**
- `projects.active_version_id` = **integer nullable, FK *logique* non contrainte au niveau DB** (référence circulaire `projects ⇄ versions` ; SQLite exige que la table cible existe à la création de la FK — impossible des deux côtés). L'intégrité est garantie applicativement par `deploy()`. À consigner dans QUIRKS à l'implémentation.
- `versions.project_id` = **vraie FK** → `projects.id`, `ON DELETE CASCADE`.
- `UNIQUE(project_id, n)` = index unique composite (backstop d'intégrité du compteur `n`).

---

## Task 1: Dépendances + `CoreError` + squelette `services/`

**Files:**
- Modify: `backend/Cargo.toml`
- Modify: `backend/src/lib.rs`
- Create: `backend/src/services/mod.rs`
- Create: `backend/src/services/errors.rs`

**Interfaces:**
- Produces: `crate::services::CoreError` — enum `{ NotFound, Validation(String), Db(sea_orm::DbErr), Io(std::io::Error) }`, `impl std::error::Error`, `From<DbErr>`, `From<io::Error>`.

- [ ] **Step 1: Ajouter les dépendances**

Dans `backend/Cargo.toml`, section `[dependencies]`, ajouter la feature `fs` à tokio et les trois crates :

```toml
tokio = { version = "1.45", default-features = false, features = [
  "rt-multi-thread",
  "fs",
] }
thiserror = { version = "2" }
rand = { version = "0.8" }
subtle = { version = "2" }
```

Dans `[dev-dependencies]`, ajouter :

```toml
tempfile = { version = "3" }
```

- [ ] **Step 2: Wirer le module et écrire `CoreError`**

`backend/src/lib.rs` — ajouter la ligne `pub mod services;` (ordre alphabétique, après `pub mod models;`).

`backend/src/services/mod.rs` :

```rust
pub mod errors;

pub use errors::CoreError;
```

`backend/src/services/errors.rs` :

```rust
//! Erreur du cœur métier — agnostique HTTP (contrat §1).
//! Chaque adaptateur (web, MCP) mappe `CoreError` vers son propre type de réponse.

use sea_orm::DbErr;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    /// Ressource absente (projet/slug/version inconnu).
    #[error("resource not found")]
    NotFound,

    /// Entrée invalide (nom vide, PIN mal formé…).
    #[error("validation error: {0}")]
    Validation(String),

    /// Erreur de la couche ORM/DB.
    #[error(transparent)]
    Db(#[from] DbErr),

    /// Erreur d'I/O (couche `Storage`).
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
```

- [ ] **Step 3: Écrire le test (qui échoue)**

Ajouter en bas de `backend/src/services/errors.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_from_io_error() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let err: CoreError = io.into();
        assert!(matches!(err, CoreError::Io(_)));
    }

    #[test]
    fn not_found_displays_message() {
        assert_eq!(CoreError::NotFound.to_string(), "resource not found");
    }
}
```

- [ ] **Step 4: Lancer les tests, vérifier qu'ils passent**

Run: `cargo test -p latch services::errors`
Expected: 2 tests PASS.

- [ ] **Step 5: fmt + clippy**

Run: `cargo fmt --all && cargo clippy --all-targets -- -D warnings`
Expected: aucun warning.

- [ ] **Step 6: Commit**

```bash
git add backend/Cargo.toml backend/Cargo.lock backend/src/lib.rs backend/src/services/
git commit -m "✨ feat: CoreError + squelette de la couche service (cœur)"
```

---

## Task 2: Service `slug` (pur)

**Files:**
- Create: `backend/src/services/slug.rs`
- Modify: `backend/src/services/mod.rs`

**Interfaces:**
- Produces:
  - `pub fn slugify_base(name: &str) -> String` — base lisible ascii, minuscules, tirets simples, fallback `"projet"` si vide.
  - `pub fn random_suffix() -> String` — 8 chars base62.
  - `pub fn generate_slug(name: &str) -> String` — `"{base}-{suffix}"`.

- [ ] **Step 1: Déclarer le module**

Dans `backend/src/services/mod.rs`, ajouter `pub mod slug;` (après `pub mod errors;`).

- [ ] **Step 2: Écrire les tests (qui échouent)**

`backend/src/services/slug.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugifies_spaces_and_case() {
        assert_eq!(slugify_base("Mon Projet"), "mon-projet");
    }

    #[test]
    fn collapses_and_trims_separators() {
        assert_eq!(slugify_base("  Hello!!  World  "), "hello-world");
    }

    #[test]
    fn drops_non_ascii() {
        // Les accents (non-ascii) sont retirés : la base est cosmétique,
        // l'unicité vient du suffixe. (Deburr = backlog.)
        assert_eq!(slugify_base("Café Déjà"), "caf-dj");
    }

    #[test]
    fn empty_name_falls_back() {
        assert_eq!(slugify_base("***"), "projet");
        assert_eq!(slugify_base(""), "projet");
    }

    #[test]
    fn suffix_is_8_base62_chars() {
        let s = random_suffix();
        assert_eq!(s.len(), 8);
        assert!(s.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn suffix_varies() {
        // Collision sur 62^8 ≈ 2e14 : pratiquement impossible.
        assert_ne!(random_suffix(), random_suffix());
    }

    #[test]
    fn generate_slug_combines_base_and_suffix() {
        let slug = generate_slug("Mon Projet");
        let (base, suffix) = slug.rsplit_once('-').unwrap();
        assert_eq!(base, "mon-projet");
        assert_eq!(suffix.len(), 8);
    }
}
```

- [ ] **Step 3: Vérifier que ça échoue (ne compile pas)**

Run: `cargo test -p latch services::slug`
Expected: FAIL — `cannot find function slugify_base`.

- [ ] **Step 4: Implémenter**

En tête de `backend/src/services/slug.rs` (au-dessus du `mod tests`) :

```rust
//! Génération de slug : base lisible dérivée du nom + suffixe aléatoire.
//! Pur (aucune I/O, aucune DB). Le suffixe (8 base62 ≈ 47 bits) est la part
//! quasi non-énumérable du slug — décision actée 2026-06-24 (QUIRKS).

use rand::Rng;

const SUFFIX_LEN: usize = 8;
const BASE62: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

/// Base lisible : minuscules, ascii alphanumérique, tirets simples, sans tiret
/// en bordure. Fallback `"projet"` si rien d'exploitable.
pub fn slugify_base(name: &str) -> String {
    let mut out = String::new();
    let mut pending_sep = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            if pending_sep && !out.is_empty() {
                out.push('-');
            }
            out.push(c.to_ascii_lowercase());
            pending_sep = false;
        } else {
            pending_sep = true;
        }
    }
    if out.is_empty() {
        out.push_str("projet");
    }
    out
}

/// Suffixe aléatoire de 8 caractères base62.
pub fn random_suffix() -> String {
    let mut rng = rand::thread_rng();
    (0..SUFFIX_LEN)
        .map(|_| BASE62[rng.gen_range(0..BASE62.len())] as char)
        .collect()
}

/// Slug complet : `{base}-{suffixe}`.
pub fn generate_slug(name: &str) -> String {
    format!("{}-{}", slugify_base(name), random_suffix())
}
```

- [ ] **Step 5: Vérifier que les tests passent**

Run: `cargo test -p latch services::slug`
Expected: 7 tests PASS.

- [ ] **Step 6: fmt + clippy + commit**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings
git add backend/src/services/
git commit -m "✨ feat: service slug (base lisible + suffixe 8 base62)"
```

---

## Task 3: Service `security` (comparaison à temps constant)

**Files:**
- Create: `backend/src/services/security.rs`
- Modify: `backend/src/services/mod.rs`

**Interfaces:**
- Produces: `pub fn secure_compare(a: &str, b: &str) -> bool` — égalité à temps constant (longueurs égales requises pour le match).

- [ ] **Step 1: Déclarer le module**

Dans `backend/src/services/mod.rs`, ajouter `pub mod security;`.

- [ ] **Step 2: Écrire les tests (qui échouent)**

`backend/src/services/security.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_strings_match() {
        assert!(secure_compare("123456", "123456"));
    }

    #[test]
    fn different_same_length_no_match() {
        assert!(!secure_compare("123456", "123457"));
    }

    #[test]
    fn different_length_no_match() {
        assert!(!secure_compare("123456", "12345"));
        assert!(!secure_compare("", "x"));
    }
}
```

- [ ] **Step 3: Vérifier l'échec**

Run: `cargo test -p latch services::security`
Expected: FAIL — `cannot find function secure_compare`.

- [ ] **Step 4: Implémenter**

En tête de `backend/src/services/security.rs` :

```rust
//! Primitive de comparaison à temps constant, partagée par la vérif du PIN
//! (Phase 1) et la validation du `deploy_token` côté adaptateur MCP (Phase 5).
//! L'auth elle-même vit dans l'adaptateur (contrat §1) ; ceci n'est que la
//! primitive sans état.

use subtle::ConstantTimeEq;

/// `true` ssi `a == b`, en temps constant pour des entrées de même longueur.
/// Une différence de longueur renvoie `false` immédiatement (acceptable :
/// nos secrets — PIN 6 chiffres, token de taille fixe — ont une longueur connue).
pub fn secure_compare(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    a.ct_eq(b).into()
}
```

- [ ] **Step 5: Vérifier que les tests passent**

Run: `cargo test -p latch services::security`
Expected: 3 tests PASS.

- [ ] **Step 6: fmt + clippy + commit**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings
git add backend/src/services/
git commit -m "✨ feat: service security (secure_compare temps constant)"
```

---

## Task 4: Service `pin` (pur)

**Files:**
- Create: `backend/src/services/pin.rs`
- Modify: `backend/src/services/mod.rs`

**Interfaces:**
- Produces:
  - `pub fn generate_pin() -> String` — 6 chiffres, zero-paddé (`"000000".."999999"`).
  - `pub fn is_valid_pin(s: &str) -> bool` — exactement 6 chiffres ascii.

- [ ] **Step 1: Déclarer le module**

Dans `backend/src/services/mod.rs`, ajouter `pub mod pin;`.

- [ ] **Step 2: Écrire les tests (qui échouent)**

`backend/src/services/pin.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_pin_is_six_digits() {
        for _ in 0..100 {
            let p = generate_pin();
            assert_eq!(p.len(), 6, "pin {p:?} should be 6 chars");
            assert!(p.chars().all(|c| c.is_ascii_digit()), "pin {p:?} digits only");
        }
    }

    #[test]
    fn validates_six_digit_pins() {
        assert!(is_valid_pin("000000"));
        assert!(is_valid_pin("123456"));
    }

    #[test]
    fn rejects_malformed_pins() {
        assert!(!is_valid_pin("12345"));   // trop court
        assert!(!is_valid_pin("1234567")); // trop long
        assert!(!is_valid_pin("12a456"));  // non chiffre
        assert!(!is_valid_pin(""));
    }
}
```

- [ ] **Step 3: Vérifier l'échec**

Run: `cargo test -p latch services::pin`
Expected: FAIL — `cannot find function generate_pin`.

- [ ] **Step 4: Implémenter**

En tête de `backend/src/services/pin.rs` :

```rust
//! Génération/validation du PIN (6 chiffres). Pur. Stocké en clair (contrat §3) ;
//! la vérification à temps constant passe par `services::security::secure_compare`.

use rand::Rng;

/// PIN aléatoire à 6 chiffres, zero-paddé.
pub fn generate_pin() -> String {
    let n: u32 = rand::thread_rng().gen_range(0..1_000_000);
    format!("{n:06}")
}

/// `true` ssi `s` est exactement 6 chiffres ascii.
pub fn is_valid_pin(s: &str) -> bool {
    s.len() == 6 && s.chars().all(|c| c.is_ascii_digit())
}
```

- [ ] **Step 5: Vérifier que les tests passent**

Run: `cargo test -p latch services::pin`
Expected: 3 tests PASS.

- [ ] **Step 6: fmt + clippy + commit**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings
git add backend/src/services/
git commit -m "✨ feat: service pin (génération + validation 6 chiffres)"
```

---

## Task 5: Trait `Storage` + `FsStorage`

**Files:**
- Create: `backend/src/services/storage.rs`
- Modify: `backend/src/services/mod.rs`

**Interfaces:**
- Produces:
  - `pub trait Storage: Send + Sync` avec `async fn write(&self, rel_path: &str, contents: &[u8]) -> Result<(), CoreError>` et `async fn read(&self, rel_path: &str) -> Result<String, CoreError>`.
  - `pub struct FsStorage` + `pub fn new(root: PathBuf) -> Self`.

- [ ] **Step 1: Déclarer le module**

Dans `backend/src/services/mod.rs`, ajouter `pub mod storage;`.

- [ ] **Step 2: Écrire les tests (qui échouent)**

`backend/src/services/storage.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn write_then_read_roundtrip() {
        let dir = tempdir().unwrap();
        let storage = FsStorage::new(dir.path().to_path_buf());

        storage.write("42/1.html", b"<h1>hi</h1>").await.unwrap();
        let got = storage.read("42/1.html").await.unwrap();
        assert_eq!(got, "<h1>hi</h1>");
    }

    #[tokio::test]
    async fn write_creates_nested_dirs() {
        let dir = tempdir().unwrap();
        let storage = FsStorage::new(dir.path().to_path_buf());
        // le sous-dossier "7" n'existe pas encore
        storage.write("7/3.html", b"x").await.unwrap();
        assert!(dir.path().join("7/3.html").exists());
    }

    #[tokio::test]
    async fn read_missing_is_not_found() {
        let dir = tempdir().unwrap();
        let storage = FsStorage::new(dir.path().to_path_buf());
        let err = storage.read("nope.html").await.unwrap_err();
        assert!(matches!(err, CoreError::NotFound));
    }

    #[tokio::test]
    async fn write_overwrites_atomically() {
        let dir = tempdir().unwrap();
        let storage = FsStorage::new(dir.path().to_path_buf());
        storage.write("1/1.html", b"old").await.unwrap();
        storage.write("1/1.html", b"new").await.unwrap();
        assert_eq!(storage.read("1/1.html").await.unwrap(), "new");
        // pas de fichier .tmp résiduel
        assert!(!dir.path().join("1/1.tmp").exists());
    }
}
```

- [ ] **Step 3: Vérifier l'échec**

Run: `cargo test -p latch services::storage`
Expected: FAIL — `cannot find type FsStorage`.

- [ ] **Step 4: Implémenter**

En tête de `backend/src/services/storage.rs` :

```rust
//! Adaptateur sortant "fichiers" (contrat §1). Le HTML des versions vit dans
//! le volume. `write` est atomique (tmp → rename) pour ne jamais exposer un
//! fichier à moitié écrit (contrat §8). Injectable : le cœur dépend du trait,
//! les tests utilisent un tempdir, jamais le disque de prod.

use std::path::PathBuf;

use async_trait::async_trait;
use tokio::fs;

use crate::services::errors::CoreError;

#[async_trait]
pub trait Storage: Send + Sync {
    /// Écrit `contents` à `rel_path` (relatif à la racine), en créant les
    /// dossiers parents. Atomique : écrit un `.tmp` puis `rename` en place.
    async fn write(&self, rel_path: &str, contents: &[u8]) -> Result<(), CoreError>;

    /// Lit le contenu UTF-8 à `rel_path`. `CoreError::NotFound` si absent.
    async fn read(&self, rel_path: &str) -> Result<String, CoreError>;
}

/// Implémentation sur système de fichiers, ancrée à `root` (le volume).
pub struct FsStorage {
    root: PathBuf,
}

impl FsStorage {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

#[async_trait]
impl Storage for FsStorage {
    async fn write(&self, rel_path: &str, contents: &[u8]) -> Result<(), CoreError> {
        let dest = self.root.join(rel_path);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).await?;
        }
        let tmp = dest.with_extension("tmp");
        fs::write(&tmp, contents).await?;
        fs::rename(&tmp, &dest).await?;
        Ok(())
    }

    async fn read(&self, rel_path: &str) -> Result<String, CoreError> {
        let dest = self.root.join(rel_path);
        match fs::read_to_string(&dest).await {
            Ok(s) => Ok(s),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(CoreError::NotFound),
            Err(e) => Err(CoreError::Io(e)),
        }
    }
}
```

- [ ] **Step 5: Vérifier que les tests passent**

Run: `cargo test -p latch services::storage`
Expected: 4 tests PASS.

- [ ] **Step 6: fmt + clippy + commit**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings
git add backend/src/services/
git commit -m "✨ feat: trait Storage + FsStorage (write atomique, read)"
```

---

## Task 6: Migrations `projects`/`versions` + génération des entités + `test_support`

**Files:**
- Create: `backend/migration/src/m20260624_000001_create_projects.rs`
- Create: `backend/migration/src/m20260624_000002_create_versions.rs`
- Modify: `backend/migration/src/lib.rs`
- Create (générés): `backend/src/models/_entities/projects.rs`, `backend/src/models/_entities/versions.rs` (+ `_entities/mod.rs` mis à jour par la génération)
- Create: `backend/src/services/test_support.rs`
- Modify: `backend/src/services/mod.rs`

**Interfaces:**
- Produces:
  - Entités `crate::models::_entities::projects::{Entity, Model, ActiveModel, Column}` avec champs `id, slug, name, code_enabled, pin (Option), brand_name (Option), active_version_id (Option), created_at, updated_at`.
  - Entités `crate::models::_entities::versions::{Entity, Model, ActiveModel, Column}` avec champs `id, project_id, n, html_path, created_at`.
  - `#[cfg(test)] pub(crate) async fn test_support::test_db() -> sea_orm::DatabaseConnection`.

- [ ] **Step 1: Écrire la migration `projects`**

`backend/migration/src/m20260624_000001_create_projects.rs` :

```rust
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Projects::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Projects::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Projects::Slug).string().not_null().unique_key())
                    .col(ColumnDef::new(Projects::Name).string().not_null())
                    .col(
                        ColumnDef::new(Projects::CodeEnabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(ColumnDef::new(Projects::Pin).string().null())
                    .col(ColumnDef::new(Projects::BrandName).string().null())
                    // FK *logique* vers versions.id (réf. circulaire : pas de
                    // contrainte DB possible, la cible n'existe pas encore).
                    .col(ColumnDef::new(Projects::ActiveVersionId).integer().null())
                    .col(
                        ColumnDef::new(Projects::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Projects::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Projects::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Projects {
    Table,
    Id,
    Slug,
    Name,
    CodeEnabled,
    Pin,
    BrandName,
    ActiveVersionId,
    CreatedAt,
    UpdatedAt,
}
```

- [ ] **Step 2: Écrire la migration `versions`**

`backend/migration/src/m20260624_000002_create_versions.rs` :

```rust
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Versions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Versions::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Versions::ProjectId).integer().not_null())
                    .col(ColumnDef::new(Versions::N).integer().not_null())
                    .col(ColumnDef::new(Versions::HtmlPath).string().not_null())
                    .col(
                        ColumnDef::new(Versions::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_versions_project_id")
                            .from(Versions::Table, Versions::ProjectId)
                            .to(Projects::Table, Projects::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Backstop d'intégrité : un seul `n` par projet (compteur v1, v2…).
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_versions_project_n")
                    .table(Versions::Table)
                    .col(Versions::ProjectId)
                    .col(Versions::N)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Versions::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Versions {
    Table,
    Id,
    ProjectId,
    N,
    HtmlPath,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Projects {
    Table,
    Id,
}
```

- [ ] **Step 3: Enregistrer les migrations**

`backend/migration/src/lib.rs` — remplacer le corps :

```rust
#![allow(elided_lifetimes_in_paths)]
#![allow(clippy::wildcard_imports)]
pub use sea_orm_migration::prelude::*;

mod m20260624_000001_create_projects;
mod m20260624_000002_create_versions;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260624_000001_create_projects::Migration),
            Box::new(m20260624_000002_create_versions::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
```

- [ ] **Step 4: Appliquer les migrations puis générer les entités**

Depuis `backend/` (QUIRKS : Loco lit `./config` au CWD). Repartir d'une DB de dev propre pour une génération nette :

```bash
cd backend
rm -f latch_development.sqlite
cargo loco db migrate
cargo loco db entities
cd ..
```

Expected : `db migrate` applique 2 migrations sans erreur ; `db entities` écrit `src/models/_entities/projects.rs` et `src/models/_entities/versions.rs` et met à jour `src/models/_entities/mod.rs` (+ `prelude.rs`).

- [ ] **Step 5: Vérifier les champs générés**

Run: `cargo build -p latch`
Ouvrir `backend/src/models/_entities/projects.rs` et `versions.rs` et vérifier que les champs/Colonnes correspondent aux Interfaces ci-dessus (`code_enabled: bool`, `pin: Option<String>`, `active_version_id: Option<i32>`, `created_at: DateTimeWithTimeZone`, etc.). Si un type diffère (ex. `DateTimeUtc` au lieu de `DateTimeWithTimeZone`), ajuster les `Set(...)` des tâches 7 et 8 en conséquence.
Expected: compile OK.

- [ ] **Step 6: Écrire `test_support` + le test de migration (qui échoue d'abord)**

Dans `backend/src/services/mod.rs`, ajouter en bas :

```rust
#[cfg(test)]
pub(crate) mod test_support;
```

`backend/src/services/test_support.rs` :

```rust
//! Helpers de test du cœur : une base SQLite **in-memory** isolée par test,
//! migrée via `Migrator`. Jamais le disque de prod (ROADMAP Phase 1).

use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};

/// Connexion SQLite in-memory migrée. `max_connections(1)` est **load-bearing** :
/// chaque connexion `sqlite::memory:` a sa propre base ; avec un pool > 1, les
/// requêtes taperaient des bases vides différentes (QUIRK).
pub(crate) async fn test_db() -> DatabaseConnection {
    let mut opt = ConnectOptions::new("sqlite::memory:");
    opt.max_connections(1);
    let db = Database::connect(opt)
        .await
        .expect("connect in-memory sqlite");
    Migrator::up(&db, None).await.expect("run migrations");
    db
}
```

Ajouter le test de schéma dans `backend/migration/src/lib.rs` n'est pas idéal (le crate migration ne dépend pas des entités). Le placer côté backend, dans un nouveau bloc test de `backend/src/services/mod.rs` :

```rust
#[cfg(test)]
mod migration_tests {
    use crate::models::_entities::{projects, versions};
    use crate::services::test_support::test_db;
    use sea_orm::{ActiveModelTrait, Set};

    #[tokio::test]
    async fn unique_project_n_is_enforced() {
        let db = test_db().await;

        let p = projects::ActiveModel {
            slug: Set("a-aaaaaaaa".to_string()),
            name: Set("A".to_string()),
            code_enabled: Set(false),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();

        versions::ActiveModel {
            project_id: Set(p.id),
            n: Set(1),
            html_path: Set(format!("{}/1.html", p.id)),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();

        let dup = versions::ActiveModel {
            project_id: Set(p.id),
            n: Set(1),
            html_path: Set(format!("{}/1-bis.html", p.id)),
            ..Default::default()
        }
        .insert(&db)
        .await;

        assert!(dup.is_err(), "UNIQUE(project_id, n) doit rejeter le doublon");
    }
}
```

- [ ] **Step 7: Vérifier que le test passe**

Run: `cargo test -p latch migration_tests`
Expected: 1 test PASS.

- [ ] **Step 8: fmt + clippy + commit**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings
git add backend/migration/ backend/src/models/ backend/src/services/
git commit -m "✨ feat: migrations projects/versions (+ unique project_id,n) + entités générées"
```

---

## Task 7: `ProjectsService` (CRUD + codes)

**Files:**
- Create: `backend/src/services/projects.rs`
- Modify: `backend/src/services/mod.rs`

**Interfaces:**
- Consumes: `slug::generate_slug`, `pin::{generate_pin, is_valid_pin}`, `security::secure_compare`, entités `projects`, `CoreError`.
- Produces:
  - `pub struct CreateProject { pub name: String, pub brand_name: Option<String>, pub code_enabled: bool, pub pin: Option<String> }`
  - `pub struct ProjectsService` + `pub fn new(db: DatabaseConnection) -> Self`
  - `async fn create(&self, input: CreateProject) -> Result<projects::Model, CoreError>`
  - `async fn list(&self) -> Result<Vec<projects::Model>, CoreError>`
  - `async fn get_by_slug(&self, slug: &str) -> Result<projects::Model, CoreError>`
  - `async fn set_code(&self, id: i32, pin: &str) -> Result<projects::Model, CoreError>`
  - `async fn clear_code(&self, id: i32) -> Result<projects::Model, CoreError>`
  - `async fn verify_code(&self, slug: &str, provided: &str) -> Result<bool, CoreError>`

- [ ] **Step 1: Déclarer le module**

Dans `backend/src/services/mod.rs`, ajouter `pub mod projects;`.

- [ ] **Step 2: Écrire les tests (qui échouent)**

`backend/src/services/projects.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::test_support::test_db;

    fn svc(db: sea_orm::DatabaseConnection) -> ProjectsService {
        ProjectsService::new(db)
    }

    #[tokio::test]
    async fn create_defaults_to_code_enabled_with_generated_pin() {
        let s = svc(test_db().await);
        let p = s
            .create(CreateProject {
                name: "Mon Projet".to_string(),
                brand_name: None,
                code_enabled: true,
                pin: None,
            })
            .await
            .unwrap();

        assert!(p.code_enabled);
        let pin = p.pin.expect("pin généré");
        assert_eq!(pin.len(), 6);
        assert!(p.slug.starts_with("mon-projet-"));
        assert!(p.active_version_id.is_none());
    }

    #[tokio::test]
    async fn create_without_code_has_no_pin() {
        let s = svc(test_db().await);
        let p = s
            .create(CreateProject {
                name: "Libre".to_string(),
                brand_name: Some("ACME".to_string()),
                code_enabled: false,
                pin: None,
            })
            .await
            .unwrap();
        assert!(!p.code_enabled);
        assert!(p.pin.is_none());
        assert_eq!(p.brand_name.as_deref(), Some("ACME"));
    }

    #[tokio::test]
    async fn create_rejects_empty_name() {
        let s = svc(test_db().await);
        let err = s
            .create(CreateProject {
                name: "   ".to_string(),
                brand_name: None,
                code_enabled: true,
                pin: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation(_)));
    }

    #[tokio::test]
    async fn get_by_slug_missing_is_not_found() {
        let s = svc(test_db().await);
        let err = s.get_by_slug("nope-xxxxxxxx").await.unwrap_err();
        assert!(matches!(err, CoreError::NotFound));
    }

    #[tokio::test]
    async fn set_and_clear_code() {
        let s = svc(test_db().await);
        let p = s
            .create(CreateProject {
                name: "P".to_string(),
                brand_name: None,
                code_enabled: false,
                pin: None,
            })
            .await
            .unwrap();

        let p = s.set_code(p.id, "424242").await.unwrap();
        assert!(p.code_enabled);
        assert_eq!(p.pin.as_deref(), Some("424242"));

        let p = s.clear_code(p.id).await.unwrap();
        assert!(!p.code_enabled);
        assert!(p.pin.is_none());
    }

    #[tokio::test]
    async fn set_code_rejects_bad_pin() {
        let s = svc(test_db().await);
        let p = s
            .create(CreateProject {
                name: "P".to_string(),
                brand_name: None,
                code_enabled: false,
                pin: None,
            })
            .await
            .unwrap();
        let err = s.set_code(p.id, "12ab").await.unwrap_err();
        assert!(matches!(err, CoreError::Validation(_)));
    }

    #[tokio::test]
    async fn verify_code_matches_and_rejects() {
        let s = svc(test_db().await);
        let p = s
            .create(CreateProject {
                name: "P".to_string(),
                brand_name: None,
                code_enabled: true,
                pin: Some("135790".to_string()),
            })
            .await
            .unwrap();

        assert!(s.verify_code(&p.slug, "135790").await.unwrap());
        assert!(!s.verify_code(&p.slug, "000000").await.unwrap());
    }

    #[tokio::test]
    async fn verify_code_open_project_always_true() {
        let s = svc(test_db().await);
        let p = s
            .create(CreateProject {
                name: "Open".to_string(),
                brand_name: None,
                code_enabled: false,
                pin: None,
            })
            .await
            .unwrap();
        assert!(s.verify_code(&p.slug, "whatever").await.unwrap());
    }
}
```

- [ ] **Step 3: Vérifier l'échec**

Run: `cargo test -p latch services::projects`
Expected: FAIL — `cannot find type ProjectsService`.

- [ ] **Step 4: Implémenter**

En tête de `backend/src/services/projects.rs` :

```rust
//! Service projets — cœur métier (contrat §1, agnostique HTTP). Suppose
//! l'appelant déjà autorisé : aucune notion de session/token/cookie ici.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};

use crate::models::_entities::projects;
use crate::services::errors::CoreError;
use crate::services::{pin, security, slug};

/// Entrée de création d'un projet.
#[derive(Debug, Clone)]
pub struct CreateProject {
    pub name: String,
    pub brand_name: Option<String>,
    pub code_enabled: bool,
    /// PIN explicite ; si `None` et `code_enabled`, un PIN est auto-généré.
    pub pin: Option<String>,
}

pub struct ProjectsService {
    db: DatabaseConnection,
}

impl ProjectsService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn create(&self, input: CreateProject) -> Result<projects::Model, CoreError> {
        if input.name.trim().is_empty() {
            return Err(CoreError::Validation("name is required".to_string()));
        }

        let pin_value = if input.code_enabled {
            let p = input.pin.unwrap_or_else(pin::generate_pin);
            if !pin::is_valid_pin(&p) {
                return Err(CoreError::Validation("pin must be 6 digits".to_string()));
            }
            Some(p)
        } else {
            None
        };

        let model = projects::ActiveModel {
            slug: Set(slug::generate_slug(&input.name)),
            name: Set(input.name),
            code_enabled: Set(input.code_enabled),
            pin: Set(pin_value),
            brand_name: Set(input.brand_name),
            ..Default::default()
        }
        .insert(&self.db)
        .await?;

        Ok(model)
    }

    pub async fn list(&self) -> Result<Vec<projects::Model>, CoreError> {
        Ok(projects::Entity::find()
            .order_by_desc(projects::Column::Id)
            .all(&self.db)
            .await?)
    }

    pub async fn get_by_slug(&self, slug: &str) -> Result<projects::Model, CoreError> {
        projects::Entity::find()
            .filter(projects::Column::Slug.eq(slug))
            .one(&self.db)
            .await?
            .ok_or(CoreError::NotFound)
    }

    pub async fn set_code(&self, id: i32, pin: &str) -> Result<projects::Model, CoreError> {
        if !pin::is_valid_pin(pin) {
            return Err(CoreError::Validation("pin must be 6 digits".to_string()));
        }
        let mut m: projects::ActiveModel = self.get_by_id(id).await?.into();
        m.code_enabled = Set(true);
        m.pin = Set(Some(pin.to_string()));
        m.updated_at = Set(chrono::Utc::now().into());
        Ok(m.update(&self.db).await?)
    }

    pub async fn clear_code(&self, id: i32) -> Result<projects::Model, CoreError> {
        let mut m: projects::ActiveModel = self.get_by_id(id).await?.into();
        m.code_enabled = Set(false);
        m.pin = Set(None);
        m.updated_at = Set(chrono::Utc::now().into());
        Ok(m.update(&self.db).await?)
    }

    pub async fn verify_code(&self, slug: &str, provided: &str) -> Result<bool, CoreError> {
        let project = self.get_by_slug(slug).await?;
        if !project.code_enabled {
            return Ok(true);
        }
        match project.pin {
            Some(pin) => Ok(security::secure_compare(&pin, provided)),
            None => Ok(false),
        }
    }

    async fn get_by_id(&self, id: i32) -> Result<projects::Model, CoreError> {
        projects::Entity::find_by_id(id)
            .one(&self.db)
            .await?
            .ok_or(CoreError::NotFound)
    }
}
```

- [ ] **Step 5: Vérifier que les tests passent**

Run: `cargo test -p latch services::projects`
Expected: 8 tests PASS.

- [ ] **Step 6: fmt + clippy + commit**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings
git add backend/src/services/
git commit -m "✨ feat: ProjectsService (CRUD + set/clear/verify code)"
```

---

## Task 8: `DeployService` (transaction fichier→DB, flip pointeur)

**Files:**
- Create: `backend/src/services/deploy.rs`
- Modify: `backend/src/services/mod.rs`

**Interfaces:**
- Consumes: trait `Storage`, entités `projects`/`versions`, `CoreError`.
- Produces:
  - `pub struct DeployService` + `pub fn new(db: DatabaseConnection, storage: Arc<dyn Storage>) -> Self`
  - `async fn deploy(&self, project_id: i32, html: &str, activate: bool) -> Result<versions::Model, CoreError>`

- [ ] **Step 1: Déclarer le module**

Dans `backend/src/services/mod.rs`, ajouter `pub mod deploy;`.

- [ ] **Step 2: Écrire les tests (qui échouent)**

`backend/src/services/deploy.rs` :

```rust
#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use sea_orm::EntityTrait;
    use tempfile::TempDir;

    use super::*;
    use crate::models::_entities::projects;
    use crate::services::projects::{CreateProject, ProjectsService};
    use crate::services::storage::FsStorage;
    use crate::services::test_support::test_db;

    async fn make_project(db: &sea_orm::DatabaseConnection) -> projects::Model {
        ProjectsService::new(db.clone())
            .create(CreateProject {
                name: "P".to_string(),
                brand_name: None,
                code_enabled: false,
                pin: None,
            })
            .await
            .unwrap()
    }

    fn storage(dir: &TempDir) -> Arc<dyn Storage> {
        Arc::new(FsStorage::new(dir.path().to_path_buf()))
    }

    #[tokio::test]
    async fn first_deploy_is_version_one_and_writes_html() {
        let db = test_db().await;
        let dir = tempfile::tempdir().unwrap();
        let p = make_project(&db).await;

        let svc = DeployService::new(db.clone(), storage(&dir));
        let v = svc.deploy(p.id, "<h1>hi</h1>", true).await.unwrap();

        assert_eq!(v.n, 1);
        assert_eq!(v.project_id, p.id);
        // HTML écrit dans le storage
        let written = std::fs::read_to_string(dir.path().join(&v.html_path)).unwrap();
        assert_eq!(written, "<h1>hi</h1>");
        // pointeur flippé
        let p = projects::Entity::find_by_id(p.id).one(&db).await.unwrap().unwrap();
        assert_eq!(p.active_version_id, Some(v.id));
    }

    #[tokio::test]
    async fn second_deploy_increments_n() {
        let db = test_db().await;
        let dir = tempfile::tempdir().unwrap();
        let p = make_project(&db).await;
        let svc = DeployService::new(db.clone(), storage(&dir));

        let v1 = svc.deploy(p.id, "a", true).await.unwrap();
        let v2 = svc.deploy(p.id, "b", true).await.unwrap();
        assert_eq!(v1.n, 1);
        assert_eq!(v2.n, 2);
    }

    #[tokio::test]
    async fn deploy_without_activate_leaves_pointer() {
        let db = test_db().await;
        let dir = tempfile::tempdir().unwrap();
        let p = make_project(&db).await;
        let svc = DeployService::new(db.clone(), storage(&dir));

        let v = svc.deploy(p.id, "x", false).await.unwrap();
        let p = projects::Entity::find_by_id(p.id).one(&db).await.unwrap().unwrap();
        assert!(p.active_version_id.is_none());
        assert_eq!(v.n, 1);
    }
}
```

- [ ] **Step 3: Vérifier l'échec**

Run: `cargo test -p latch services::deploy`
Expected: FAIL — `cannot find type DeployService`.

- [ ] **Step 4: Implémenter**

En tête de `backend/src/services/deploy.rs` :

```rust
//! Service de déploiement — cœur (contrat §1, §8). Même point d'entrée pour
//! l'admin (upload manuel) et le tool MCP. Ordre **imposé** : écrire le HTML
//! d'abord (atomique), puis la transaction DB (insert version + flip pointeur).
//! Un échec DB après écriture ne laisse qu'un fichier orphelin inoffensif ;
//! l'ordre inverse laisserait un pointeur actif vers un fichier absent.

use std::sync::Arc;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};

use crate::models::_entities::{projects, versions};
use crate::services::errors::CoreError;
use crate::services::storage::Storage;

pub struct DeployService {
    db: DatabaseConnection,
    storage: Arc<dyn Storage>,
}

impl DeployService {
    pub fn new(db: DatabaseConnection, storage: Arc<dyn Storage>) -> Self {
        Self { db, storage }
    }

    /// Déploie `html` comme nouvelle version de `project_id`. Si `activate`,
    /// repointe `projects.active_version_id`. Renvoie la version créée.
    pub async fn deploy(
        &self,
        project_id: i32,
        html: &str,
        activate: bool,
    ) -> Result<versions::Model, CoreError> {
        // 1. n = max(n)+1 pour ce projet (hors transaction ; UNIQUE(project_id,n)
        //    est le backstop si deux deploys concurrents calculaient le même n).
        let last = versions::Entity::find()
            .filter(versions::Column::ProjectId.eq(project_id))
            .order_by_desc(versions::Column::N)
            .one(&self.db)
            .await?;
        let n = last.map_or(1, |v| v.n + 1);
        let html_path = format!("{project_id}/{n}.html");

        // 2. Écrire le HTML d'abord (atomique).
        self.storage.write(&html_path, html.as_bytes()).await?;

        // 3. Transaction : insérer la version, puis flipper le pointeur si demandé.
        let txn = self.db.begin().await?;

        let inserted = versions::ActiveModel {
            project_id: Set(project_id),
            n: Set(n),
            html_path: Set(html_path),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        if activate {
            let mut project: projects::ActiveModel = projects::Entity::find_by_id(project_id)
                .one(&txn)
                .await?
                .ok_or(CoreError::NotFound)?
                .into();
            project.active_version_id = Set(Some(inserted.id));
            project.updated_at = Set(chrono::Utc::now().into());
            project.update(&txn).await?;
        }

        txn.commit().await?;
        Ok(inserted)
    }
}
```

- [ ] **Step 5: Vérifier que les tests passent**

Run: `cargo test -p latch services::deploy`
Expected: 3 tests PASS.

- [ ] **Step 6: Suite complète + fmt + clippy**

Run: `cargo nextest run` (ou `cargo test -p latch`) puis `cargo fmt --all && cargo clippy --all-targets -- -D warnings`
Expected: tous les tests verts (cœur complet), zéro warning.

- [ ] **Step 7: Commit**

```bash
git add backend/src/services/
git commit -m "✨ feat: DeployService (ordre fichier→tx, flip pointeur transactionnel)"
```

---

## Task 9: Garde d'architecture + mise à jour de la mémoire projet

**Files:**
- Create: `backend/tests/architecture.rs` (ou bloc test dans `services/mod.rs`)
- Modify: `docs/INDEX.md`, `docs/HANDOFF.md`, `docs/CONVENTIONS.md`, `docs/QUIRKS.md`

**Interfaces:** aucune (test + doc).

- [ ] **Step 1: Test-garde « le cœur ne voit pas axum/loco »**

Le contrat §1 et QUIRKS posent l'invariant : aucun `use axum`/`use loco_rs` dans `src/services/`. Le matérialiser par un test qui lit les sources et casse le build si violé. `backend/tests/architecture.rs` :

```rust
//! Garde d'architecture (contrat §1) : le cœur `src/services/` est agnostique
//! HTTP. Aucune dépendance à axum ou loco ne doit y apparaître.

use std::fs;
use std::path::Path;

#[test]
fn services_do_not_depend_on_axum_or_loco() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/services");
    let mut offenders = Vec::new();

    for entry in fs::read_dir(&dir).expect("read src/services") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let src = fs::read_to_string(&path).unwrap();
        for (i, line) in src.lines().enumerate() {
            let t = line.trim_start();
            if t.starts_with("use axum") || t.starts_with("use loco_rs") {
                offenders.push(format!("{}:{}", path.display(), i + 1));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "le cœur ne doit pas dépendre d'axum/loco (contrat §1) : {offenders:?}"
    );
}
```

- [ ] **Step 2: Vérifier qu'il passe**

Run: `cargo test -p latch --test architecture`
Expected: 1 test PASS (le cœur est propre).

- [ ] **Step 3: Mettre à jour la mémoire projet** (règle de fin d'implémentation, CLAUDE.md)

- `docs/INDEX.md` : cocher les livrables Phase 1 (migrations projects/versions, services slug/security/pin/storage/projects/deploy, garde d'archi) ; passer la ligne `[ ] Phase 1` → `[x]` si tous critères de sortie verts.
- `docs/HANDOFF.md` : entrée datée en haut (`Dernière chose faite` / `Trucs en suspens` / `Prochaine chose à creuser` = Phase 2 / `Notes pour future Claude`).
- `docs/CONVENTIONS.md` : remplir « Service (cœur) type » avec le squelette réel (`DeployService { db, storage: Arc<dyn Storage> }`, méthode rendant `Result<_, CoreError>`) et « Test d'intégration type » (helper `test_db()` in-memory).
- `docs/QUIRKS.md` : ajouter (a) `sqlite::memory:` exige `max_connections(1)` en test ; (b) `active_version_id` = FK logique non contrainte (réf. circulaire SQLite) ; (c) FK SQLite non enforced sans `PRAGMA foreign_keys=ON` (le cascade `versions→projects` est best-effort).

- [ ] **Step 4: Commit final**

```bash
git add backend/tests/ docs/
git commit -m "✅ test: garde d'archi (cœur sans axum/loco) + 📝 docs: clôture Phase 1"
```

---

## Self-Review (effectué)

**Couverture spec (ROADMAP Phase 1) :**
- Migrations `projects`, `versions` → Task 6 (sessions reportée Phase 2, décision actée).
- `services/projects` (create/list/get_by_slug/set_code/clear_code/verify_code) → Task 7.
- `services/deploy` (tx insert + flip, ordre fichier→DB §8) → Task 8.
- `services/slug` (base + suffixe) → Task 2.
- trait `Storage` + `FsStorage` → Task 5.
- `CoreError` → Task 1.
- `verify_code` temps constant → Task 3 (`secure_compare`) + Task 7.
- PIN auto-généré 6 chiffres → Task 4 + Task 7.
- `deploy_token` (primitive de validation) → Task 3 (`secure_compare`, réutilisée par l'adaptateur MCP en Phase 5 ; l'auth ne vit pas dans le cœur — contrat §1).
- Tests unit slug/code/bascule/deploy avec `Storage` sur tempdir → Tasks 2,3,4,7,8.
- Aucun `use axum`/`use loco_rs` dans `src/services/` → Task 9 (test-garde).

**Cohérence des types :** `ProjectsService::new(DatabaseConnection)`, `DeployService::new(DatabaseConnection, Arc<dyn Storage>)`, `deploy(...) -> versions::Model`, `verify_code(...) -> bool`, `CreateProject` — utilisés de manière identique dans tâches 7/8 et leurs tests.

**Placeholders :** aucun — chaque step porte le code/commande réels.

**Point de vigilance unique (Task 6 step 5) :** les types de date générés (`DateTimeWithTimeZone` attendu) sont à confirmer sur la sortie réelle de `cargo loco db entities` ; ajuster les `Set(chrono::Utc::now().into())` si la génération produit `DateTimeUtc`.
