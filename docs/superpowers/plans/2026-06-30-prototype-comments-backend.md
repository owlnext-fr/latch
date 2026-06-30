# Prototype Comments — Backend Implementation Plan (Plan 1/3)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the entire backend for element-anchored prototype comments — schema, core `CommentsService`, public visitor endpoints, anonymous identity, admin read + moderation — with no frontend.

**Architecture:** Layered/hexagonal on Loco (contrat §1). New core service `CommentsService` (agnostic, no axum/loco) holds all comment logic; thin adapters in `serve.rs` (public, unlock-gated) and `admin.rs` (AdminAuth) translate HTTP → service. Comments live in two new tables (`comment_pins` + `comments`) bound to `versions`. Visitor identity is an opaque ULID `owner_token` carried in a signed `latch_comment` cookie (reusing the existing `UNLOCK_COOKIE_SECRET` / `SignedCookieJar`). Comment bodies are **plain text** (no markdown, no server HTML).

**Tech Stack:** Rust, Loco 0.16, SeaORM 1.1, axum 0.8, `axum-extra` `SignedCookieJar`, `tower_governor`, `ulid`, `utoipa`/`openapi.json` drift, `cargo nextest`, `axum-test` harness.

## Global Constraints

- **No `unwrap`/`expect`** outside `#[cfg(test)]` and boot init. Propagate errors. (BOOTSTRAP §4)
- **Core layer** (`src/services/`) must NOT `use axum` or `use loco_rs` — enforced by `backend/tests/architecture.rs`. Services return `CoreError` (thiserror). (contrat §1)
- **Security invariants (build-breaking):** no response contains a hash; the cleartext PIN appears only on admin project detail; **`owner_token` is NEVER serialized** in any response (public or admin) — expose a per-caller `editable: bool` instead. (contrat §9)
- **No server-side HTML** for comment bodies — plain text only.
- **Gating:** every comment route is behind `unlock_ok` **and** a `comments_enabled` check (locked project → 403; comments disabled → 404). Free projects pass `unlock_ok` without a cookie (intended).
- **Confidentialité:** fixtures use `demo` / `ACME` / `mon-projet` only — never a real client name. (CLAUDE.md)
- **Commits:** conventional + gitmoji, e.g. `✨ feat(comments): …`, `🧱 chore:`, `📝 docs:`.
- **Limits (verbatim):** body ≤ **2000** chars (`chars()`); `author_name` ≤ **80** chars, control chars stripped; max **200** pins per `(version, owner_token)`.
- **OpenAPI drift:** after any DTO/route change, regenerate with `UPDATE_OPENAPI=1 cargo test --test openapi_drift` **and** `cd frontend && pnpm gen:api`.
- **Run all backend commands from the repo root** (`/srv/owlnext/latch`). `cargo nextest run` runs the suite.

---

### Task 1: Doc-first — amend the contract

**Files:**
- Modify: `docs/contrat-deploy.md` (§3 model, §6 `/c` surface, §7 admin, §9 invariants)
- Modify: `docs/BACKLOG.md` (orphan-rows note)

**Interfaces:**
- Produces: the normative contract for everything below. No code.

- [ ] **Step 1: Add the data-model deltas to `docs/contrat-deploy.md` §3**

Under §3, after the `versions` block, add:

```markdown
**`projects`** gains `comments_enabled` (bool, NOT NULL). Défaut sécurité-aware posé à la
création (code activé → `true`, libre → `false`) ; modifiable indépendamment ensuite.

**`comment_pins`** — point d'ancrage d'un fil de commentaires, lié à une version.
- `id` (PK) · `version_id` (FK → `versions.id`, ON DELETE CASCADE)
- `owner_token` (opaque, **jamais sérialisé** en réponse) · `anchor` (TEXT, descripteur JSON)
- `status` (TEXT, `open`/`resolved`, défaut `open`, réservé — pas d'UI v1)
- `created_at` · `updated_at` · `deleted_at` (NULL, soft-delete)

**`comments`** — message d'un fil.
- `id` (PK) · `pin_id` (FK → `comment_pins.id`, ON DELETE CASCADE)
- `owner_token` (opaque, jamais sérialisé) · `author_name` (≤ 80, auto-déclaré)
- `body` (TEXT, texte brut, ≤ 2000 caractères) · `created_at` · `updated_at` · `deleted_at` (NULL)

Le `anchor` est un descripteur JSON opaque côté serveur (jamais interprété) : `{ v, selector,
fingerprint, textQuote, offset, fallbackPoint }`. Suppression = soft-delete (`deleted_at`).
```

- [ ] **Step 2: Add the `/c` comment surface to `docs/contrat-deploy.md` §6**

Add a subsection §6.4:

```markdown
### 6.4 Commentaires ancrés (`/c/<slug>/comments`)

Toutes en `Cache-Control: no-store`, **gardées par `unlock_ok` + `comments_enabled`**
(projet à code non déverrouillé → 403 ; commentaires désactivés → 404). Les **écritures**
portent en plus : garde **Origin** same-origin, header **`X-Comment-Client`** exigé,
cookie d'identité `latch_comment` (ULID opaque, signé, `HttpOnly`/`Secure`/`SameSite=Lax`,
`Path=/c/<slug>`, secret = `UNLOCK_COOKIE_SECRET` réutilisé), et un rate-limit dédié
(`LATCH_COMMENT_RL_*`).

- `GET    /c/<slug>/comments` — mes pins+fils de la version active (filtré `owner_token`).
- `POST   /c/<slug>/comments` — crée un pin + 1ᵉʳ message ; pose le cookie d'identité si absent.
- `POST   /c/<slug>/comments/pins/<pin>/replies` — ajoute un message à mon pin.
- `PUT    /c/<slug>/comments/messages/<id>` — édite mon message.
- `DELETE /c/<slug>/comments/messages/<id>` — supprime mon message (soft ; si dernier → pin soft-deleted).
- `DELETE /c/<slug>/comments/pins/<pin>` — supprime mon pin entier.

Réponses : `owner_token` **jamais** présent ; chaque message porte `editable: bool` (calculé
par appelant). MCP `deploy_prototype` inchangé (ne touche pas aux commentaires).
```

- [ ] **Step 3: Add admin + invariants to §7 and §9**

In §7, add a bullet:

```markdown
- **Commentaires** : toggle `comments_enabled` par projet (défaut sécurité-aware) ; vue admin
  par version (liste lecture seule `GET /api/projects/<id>/versions/<n>/comments` + modération
  `DELETE /api/projects/<id>/comments/messages/<id>`, vérifie l'appartenance au projet).
```

In §9, add invariant 7:

```markdown
7. **`owner_token` jamais sérialisé** (réponse publique ou admin) — `editable: bool` à la place.
   Le gate `unlock_ok` + `comments_enabled` couvre toutes les routes commentaires.
```

- [ ] **Step 4: Note the orphan-rows limitation in `docs/BACKLOG.md`**

Append:

```markdown
## Nettoyage des commentaires sur hard-delete projet/version (2026-06-30)
SQLite n'enforce pas les FK (pas de PRAGMA) → supprimer un projet/version laisse des
`comment_pins`/`comments` orphelins. Inoffensifs (inaccessibles : le lookup projet/version
404 avant). Même posture que les fichiers HTML orphelins sur `delete_version`. Amélioration
future : purge explicite en transaction dans `delete_project`/`delete_version`.
```

- [ ] **Step 5: Commit**

```bash
rtk git add docs/contrat-deploy.md docs/BACKLOG.md
rtk git commit -m "📝 docs(contrat): commentaires ancrés — modèle, surface /c, invariant owner_token"
```

---

### Task 2: Schema — migrations + entities

**Files:**
- Create: `backend/migration/src/m20260630_000005_add_comments_enabled_to_projects.rs`
- Create: `backend/migration/src/m20260630_000006_create_comment_tables.rs`
- Modify: `backend/migration/src/lib.rs`
- Create: `backend/src/models/_entities/comment_pins.rs`
- Create: `backend/src/models/_entities/comments.rs`
- Modify: `backend/src/models/_entities/projects.rs` (add `comments_enabled` field)
- Modify: `backend/src/models/_entities/mod.rs`, `backend/src/models/_entities/prelude.rs`
- Modify: `backend/src/dto/mod.rs` (fix `sample_model()` literal)
- Test: `backend/src/services/mod.rs` (`migration_tests` module)

**Interfaces:**
- Produces: tables `comment_pins`, `comments`; column `projects.comments_enabled`; entities
  `comment_pins::{Entity,Model,ActiveModel,Column}`, `comments::{...}`; `projects::Model.comments_enabled: bool`.

- [ ] **Step 1: Write the migration that adds `comments_enabled` to `projects`**

Create `backend/migration/src/m20260630_000005_add_comments_enabled_to_projects.rs`:

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
                    .table(Projects::Table)
                    .add_column(
                        ColumnDef::new(Projects::CommentsEnabled)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;
        // Backfill sécurité-aware : les projets existants suivent leur code d'accès.
        manager
            .get_connection()
            .execute_unprepared("UPDATE projects SET comments_enabled = code_enabled")
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Projects::Table)
                    .drop_column(Projects::CommentsEnabled)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Projects {
    Table,
    CommentsEnabled,
}
```

- [ ] **Step 2: Write the migration that creates `comment_pins` + `comments`**

Create `backend/migration/src/m20260630_000006_create_comment_tables.rs`:

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
                    .table(CommentPins::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CommentPins::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CommentPins::VersionId).integer().not_null())
                    .col(ColumnDef::new(CommentPins::OwnerToken).string().not_null())
                    .col(ColumnDef::new(CommentPins::Anchor).text().not_null())
                    .col(
                        ColumnDef::new(CommentPins::Status)
                            .string()
                            .not_null()
                            .default("open"),
                    )
                    .col(
                        ColumnDef::new(CommentPins::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(CommentPins::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(CommentPins::DeletedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_comment_pins_version_id")
                            .from(CommentPins::Table, CommentPins::VersionId)
                            .to(Versions::Table, Versions::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_comment_pins_version_id")
                    .table(CommentPins::Table)
                    .col(CommentPins::VersionId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Comments::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Comments::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Comments::PinId).integer().not_null())
                    .col(ColumnDef::new(Comments::OwnerToken).string().not_null())
                    .col(ColumnDef::new(Comments::AuthorName).string().not_null())
                    .col(ColumnDef::new(Comments::Body).text().not_null())
                    .col(
                        ColumnDef::new(Comments::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Comments::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Comments::DeletedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_comments_pin_id")
                            .from(Comments::Table, Comments::PinId)
                            .to(CommentPins::Table, CommentPins::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_comments_pin_id")
                    .table(Comments::Table)
                    .col(Comments::PinId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Comments::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(CommentPins::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum CommentPins {
    Table,
    Id,
    VersionId,
    OwnerToken,
    Anchor,
    Status,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

#[derive(DeriveIden)]
enum Comments {
    Table,
    Id,
    PinId,
    OwnerToken,
    AuthorName,
    Body,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

#[derive(DeriveIden)]
enum Versions {
    Table,
    Id,
}
```

- [ ] **Step 3: Register both migrations in `backend/migration/src/lib.rs`**

Add the `mod` declarations after the existing ones and the `Box::new(...)` entries before the `// inject-above` comment:

```rust
mod m20260630_000005_add_comments_enabled_to_projects;
mod m20260630_000006_create_comment_tables;
```

```rust
            Box::new(m20260630_000005_add_comments_enabled_to_projects::Migration),
            Box::new(m20260630_000006_create_comment_tables::Migration),
            // inject-above (do not remove this comment)
```

- [ ] **Step 4: Add the `comment_pins` and `comments` entities**

Create `backend/src/models/_entities/comment_pins.rs`:

```rust
//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.20

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "comment_pins")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub version_id: i32,
    pub owner_token: String,
    #[sea_orm(column_type = "Text")]
    pub anchor: String,
    pub status: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::versions::Entity",
        from = "Column::VersionId",
        to = "super::versions::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Versions,
    #[sea_orm(has_many = "super::comments::Entity")]
    Comments,
}

impl Related<super::versions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Versions.def()
    }
}

impl Related<super::comments::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Comments.def()
    }
}
```

Create `backend/src/models/_entities/comments.rs`:

```rust
//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.20

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "comments")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub pin_id: i32,
    pub owner_token: String,
    pub author_name: String,
    #[sea_orm(column_type = "Text")]
    pub body: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::comment_pins::Entity",
        from = "Column::PinId",
        to = "super::comment_pins::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    CommentPins,
}

impl Related<super::comment_pins::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CommentPins.def()
    }
}
```

> These match `sea-orm-cli` output for the schema in Step 2. They can be regenerated later with
> `cargo loco db entities` (run from `backend/`); the content above is the expected result.

- [ ] **Step 5: Add `comments_enabled` to the generated `projects` entity**

In `backend/src/models/_entities/projects.rs`, add the field **after** `updated_at` (ALTER appends
at the end of the table, so codegen places it last):

```rust
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub comments_enabled: bool,
}
```

Add `comment_pins`/`comments` to `backend/src/models/_entities/mod.rs`:

```rust
pub mod prelude;

pub mod comment_pins;
pub mod comments;
pub mod projects;
pub mod versions;
```

And to `backend/src/models/_entities/prelude.rs`:

```rust
pub use super::comment_pins::Entity as CommentPins;
pub use super::comments::Entity as Comments;
pub use super::projects::Entity as Projects;
pub use super::versions::Entity as Versions;
```

- [ ] **Step 6: Fix every literal `projects::Model { … }` construction**

The new non-`Option` field breaks literal constructions. Run:

```bash
cargo check -p latch 2>&1 | grep -A2 "missing field .comments_enabled"
```

Fix each hit by adding `comments_enabled: true,`. The known one is `sample_model()` in
`backend/src/dto/mod.rs` (add after `updated_at: chrono::Utc::now().into(),`):

```rust
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
            comments_enabled: true,
        }
```

- [ ] **Step 7: Write the migration/schema test**

In `backend/src/services/mod.rs`, inside the existing `migration_tests` module, add:

```rust
    #[tokio::test]
    async fn comment_tables_accept_inserts_and_cascade() {
        use crate::models::_entities::{comment_pins, comments};
        let db = test_db().await;

        let p = projects::ActiveModel {
            slug: Set("demo-aaaaaaaa".to_string()),
            name: Set("Demo".to_string()),
            code_enabled: Set(false),
            comments_enabled: Set(true),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();

        let v = versions::ActiveModel {
            project_id: Set(p.id),
            n: Set(1),
            html_path: Set(format!("{}/1.html", p.id)),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();

        let pin = comment_pins::ActiveModel {
            version_id: Set(v.id),
            owner_token: Set("01OWNERTOKEN0000000000000".to_string()),
            anchor: Set(r#"{"v":1,"selector":"button"}"#.to_string()),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        assert_eq!(pin.status, "open", "status défaut = open");
        assert!(pin.deleted_at.is_none());

        comments::ActiveModel {
            pin_id: Set(pin.id),
            owner_token: Set("01OWNERTOKEN0000000000000".to_string()),
            author_name: Set("Léa".to_string()),
            body: Set("Top".to_string()),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();

        let count = comments::Entity::find().all(&db).await.unwrap().len();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn projects_have_comments_enabled_column() {
        let db = test_db().await;
        let p = projects::ActiveModel {
            slug: Set("demo-bbbbbbbb".to_string()),
            name: Set("Demo".to_string()),
            code_enabled: Set(true),
            comments_enabled: Set(true),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        assert!(p.comments_enabled);
    }
```

- [ ] **Step 8: Run the tests**

Run: `cargo nextest run -p latch migration_tests`
Expected: PASS (`comment_tables_accept_inserts_and_cascade`, `projects_have_comments_enabled_column`).

- [ ] **Step 9: Commit**

```bash
rtk git add backend/migration/src backend/src/models/_entities backend/src/services/mod.rs backend/src/dto/mod.rs
rtk git commit -m "🧱 feat(comments): schéma comment_pins/comments + projects.comments_enabled"
```

---

### Task 3: CommentsService — create & list

**Files:**
- Create: `backend/src/services/comments.rs`
- Modify: `backend/src/services/mod.rs` (declare `pub mod comments;`)
- Test: inline `#[cfg(test)]` in `comments.rs`

**Interfaces:**
- Consumes: `comment_pins`, `comments`, `versions` entities; `CoreError`.
- Produces:
  - `pub struct PinWithMessages { pub pin: comment_pins::Model, pub messages: Vec<comments::Model> }`
  - `CommentsService::new(db: DatabaseConnection) -> Self`
  - `create_pin(&self, version_id: i32, owner_token: &str, author_name: &str, body: &str, anchor: &str) -> Result<PinWithMessages, CoreError>`
  - `add_reply(&self, pin_id: i32, owner_token: &str, author_name: &str, body: &str) -> Result<comments::Model, CoreError>`
  - `list_for_version(&self, version_id: i32) -> Result<Vec<PinWithMessages>, CoreError>`
  - `list_for_version_and_owner(&self, version_id: i32, owner_token: &str) -> Result<Vec<PinWithMessages>, CoreError>`
  - `count_comments_by_version(&self, version_ids: &[i32]) -> Result<std::collections::HashMap<i32, i32>, CoreError>`
  - consts `MAX_BODY_LEN`, `MAX_AUTHOR_NAME_LEN`, `MAX_PINS_PER_VERSION_PER_OWNER`
  - helpers `validate_body`, `sanitize_author_name` (module-private)

- [ ] **Step 1: Declare the module**

In `backend/src/services/mod.rs` add `pub mod comments;` in alphabetical position (after `pub mod comment` would be `comments`; place after `pub mod analytics`? there is none — put it right after the first line):

```rust
pub mod comments;
pub mod deploy;
```

- [ ] **Step 2: Write the failing tests for create + list**

Create `backend/src/services/comments.rs` with this test module at the bottom (the impl follows in Step 3):

```rust
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::models::_entities::{projects, versions};
    use crate::services::test_support::test_db;
    use sea_orm::{ActiveModelTrait, Set};

    const OWNER_A: &str = "01OWNERAAAAAAAAAAAAAAAAAAA";
    const OWNER_B: &str = "01OWNERBBBBBBBBBBBBBBBBBBB";

    async fn version(db: &sea_orm::DatabaseConnection) -> versions::Model {
        let p = projects::ActiveModel {
            slug: Set("demo-aaaaaaaa".to_string()),
            name: Set("Demo".to_string()),
            code_enabled: Set(false),
            comments_enabled: Set(true),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
        versions::ActiveModel {
            project_id: Set(p.id),
            n: Set(1),
            html_path: Set(format!("{}/1.html", p.id)),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn create_pin_stores_pin_and_first_message() {
        let db = test_db().await;
        let v = version(&db).await;
        let svc = CommentsService::new(db);

        let pwm = svc
            .create_pin(v.id, OWNER_A, "Léa", "Le bouton est trop petit", r#"{"v":1}"#)
            .await
            .unwrap();

        assert_eq!(pwm.pin.version_id, v.id);
        assert_eq!(pwm.pin.anchor, r#"{"v":1}"#);
        assert_eq!(pwm.messages.len(), 1);
        assert_eq!(pwm.messages[0].author_name, "Léa");
        assert_eq!(pwm.messages[0].body, "Le bouton est trop petit");
    }

    #[tokio::test]
    async fn add_reply_appends_to_owned_pin() {
        let db = test_db().await;
        let v = version(&db).await;
        let svc = CommentsService::new(db);
        let pwm = svc.create_pin(v.id, OWNER_A, "Léa", "un", "{}").await.unwrap();

        let reply = svc.add_reply(pwm.pin.id, OWNER_A, "Léa", "deux").await.unwrap();
        assert_eq!(reply.pin_id, pwm.pin.id);
        assert_eq!(reply.body, "deux");
    }

    #[tokio::test]
    async fn add_reply_to_foreign_pin_is_not_found() {
        let db = test_db().await;
        let v = version(&db).await;
        let svc = CommentsService::new(db);
        let pwm = svc.create_pin(v.id, OWNER_A, "Léa", "un", "{}").await.unwrap();

        let err = svc.add_reply(pwm.pin.id, OWNER_B, "Max", "intrus").await.unwrap_err();
        assert!(matches!(err, CoreError::NotFound));
    }

    #[tokio::test]
    async fn list_for_owner_only_returns_own_pins() {
        let db = test_db().await;
        let v = version(&db).await;
        let svc = CommentsService::new(db);
        svc.create_pin(v.id, OWNER_A, "Léa", "a", "{}").await.unwrap();
        svc.create_pin(v.id, OWNER_B, "Max", "b", "{}").await.unwrap();

        let mine = svc.list_for_version_and_owner(v.id, OWNER_A).await.unwrap();
        assert_eq!(mine.len(), 1);
        assert_eq!(mine[0].messages[0].author_name, "Léa");

        let all = svc.list_for_version(v.id).await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn create_pin_rejects_empty_and_too_long_body() {
        let db = test_db().await;
        let v = version(&db).await;
        let svc = CommentsService::new(db);

        assert!(matches!(
            svc.create_pin(v.id, OWNER_A, "Léa", "   ", "{}").await.unwrap_err(),
            CoreError::Validation(_)
        ));
        let long = "x".repeat(MAX_BODY_LEN + 1);
        assert!(matches!(
            svc.create_pin(v.id, OWNER_A, "Léa", &long, "{}").await.unwrap_err(),
            CoreError::Validation(_)
        ));
    }

    #[tokio::test]
    async fn create_pin_rejects_empty_author_and_anchor() {
        let db = test_db().await;
        let v = version(&db).await;
        let svc = CommentsService::new(db);
        assert!(matches!(
            svc.create_pin(v.id, OWNER_A, "  ", "ok", "{}").await.unwrap_err(),
            CoreError::Validation(_)
        ));
        assert!(matches!(
            svc.create_pin(v.id, OWNER_A, "Léa", "ok", "").await.unwrap_err(),
            CoreError::Validation(_)
        ));
    }

    #[tokio::test]
    async fn author_name_control_chars_are_stripped() {
        let db = test_db().await;
        let v = version(&db).await;
        let svc = CommentsService::new(db);
        let pwm = svc
            .create_pin(v.id, OWNER_A, "Lé\u{0007}a\n", "ok", "{}")
            .await
            .unwrap();
        assert_eq!(pwm.messages[0].author_name, "Léa");
    }

    #[tokio::test]
    async fn count_comments_by_version_groups() {
        let db = test_db().await;
        let v = version(&db).await;
        let svc = CommentsService::new(db);
        svc.create_pin(v.id, OWNER_A, "Léa", "a", "{}").await.unwrap();
        svc.create_pin(v.id, OWNER_B, "Max", "b", "{}").await.unwrap();

        let counts = svc.count_comments_by_version(&[v.id]).await.unwrap();
        assert_eq!(counts.get(&v.id).copied(), Some(2));
    }
}
```

- [ ] **Step 3: Write the implementation (top of `comments.rs`, above the test module)**

```rust
//! Service commentaires — cœur métier (contrat §1, agnostique HTTP). Suppose
//! l'appelant déjà autorisé : l'`owner_token` est fourni par l'adaptateur (qui
//! gère le cookie signé), jamais minté ici. Corps en texte brut (jamais d'HTML).

use std::collections::HashMap;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};

use crate::models::_entities::{comment_pins, comments};
use crate::services::errors::CoreError;
use crate::services::security::secure_compare;

/// Longueur max du corps d'un message (caractères).
pub const MAX_BODY_LEN: usize = 2000;
/// Longueur max du nom auto-déclaré (caractères).
pub const MAX_AUTHOR_NAME_LEN: usize = 80;
/// Plafond anti-flood : pins par (version, owner_token).
pub const MAX_PINS_PER_VERSION_PER_OWNER: usize = 200;

/// Un pin et ses messages non supprimés, triés du plus ancien au plus récent.
#[derive(Debug, Clone)]
pub struct PinWithMessages {
    pub pin: comment_pins::Model,
    pub messages: Vec<comments::Model>,
}

/// Valide le corps : non vide après trim, ≤ MAX_BODY_LEN caractères.
fn validate_body(body: &str) -> Result<String, CoreError> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return Err(CoreError::Validation("body is required".to_string()));
    }
    if trimmed.chars().count() > MAX_BODY_LEN {
        return Err(CoreError::Validation(format!(
            "body too long (max {MAX_BODY_LEN} chars)"
        )));
    }
    Ok(trimmed.to_string())
}

/// Nettoie le nom : retire les caractères de contrôle, trim, non vide, ≤ MAX_AUTHOR_NAME_LEN.
fn sanitize_author_name(name: &str) -> Result<String, CoreError> {
    let cleaned: String = name.chars().filter(|c| !c.is_control()).collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        return Err(CoreError::Validation("author_name is required".to_string()));
    }
    if trimmed.chars().count() > MAX_AUTHOR_NAME_LEN {
        return Err(CoreError::Validation(format!(
            "author_name too long (max {MAX_AUTHOR_NAME_LEN} chars)"
        )));
    }
    Ok(trimmed.to_string())
}

pub struct CommentsService {
    db: DatabaseConnection,
}

impl CommentsService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Crée un pin + son premier message dans une transaction.
    pub async fn create_pin(
        &self,
        version_id: i32,
        owner_token: &str,
        author_name: &str,
        body: &str,
        anchor: &str,
    ) -> Result<PinWithMessages, CoreError> {
        let body = validate_body(body)?;
        let author = sanitize_author_name(author_name)?;
        if anchor.trim().is_empty() {
            return Err(CoreError::Validation("anchor is required".to_string()));
        }

        // Plafond anti-flood : pins vivants pour ce (version, owner).
        let existing = comment_pins::Entity::find()
            .filter(comment_pins::Column::VersionId.eq(version_id))
            .filter(comment_pins::Column::OwnerToken.eq(owner_token))
            .filter(comment_pins::Column::DeletedAt.is_null())
            .count(&self.db)
            .await?;
        if existing as usize >= MAX_PINS_PER_VERSION_PER_OWNER {
            return Err(CoreError::Validation("too many pins".to_string()));
        }

        let txn = self.db.begin().await?;
        let pin = comment_pins::ActiveModel {
            version_id: Set(version_id),
            owner_token: Set(owner_token.to_string()),
            anchor: Set(anchor.to_string()),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
        let message = comments::ActiveModel {
            pin_id: Set(pin.id),
            owner_token: Set(owner_token.to_string()),
            author_name: Set(author),
            body: Set(body),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
        txn.commit().await?;

        Ok(PinWithMessages {
            pin,
            messages: vec![message],
        })
    }

    /// Ajoute un message à un pin **possédé** par `owner_token`. Pin étranger/absent → NotFound.
    pub async fn add_reply(
        &self,
        pin_id: i32,
        owner_token: &str,
        author_name: &str,
        body: &str,
    ) -> Result<comments::Model, CoreError> {
        let body = validate_body(body)?;
        let author = sanitize_author_name(author_name)?;
        let pin = self.owned_live_pin(pin_id, owner_token).await?;

        Ok(comments::ActiveModel {
            pin_id: Set(pin.id),
            owner_token: Set(owner_token.to_string()),
            author_name: Set(author),
            body: Set(body),
            ..Default::default()
        }
        .insert(&self.db)
        .await?)
    }

    /// Tous les pins vivants d'une version (tous auteurs) — usage admin.
    pub async fn list_for_version(
        &self,
        version_id: i32,
    ) -> Result<Vec<PinWithMessages>, CoreError> {
        self.list_pins(version_id, None).await
    }

    /// Les pins vivants d'une version appartenant à `owner_token` — usage visiteur.
    pub async fn list_for_version_and_owner(
        &self,
        version_id: i32,
        owner_token: &str,
    ) -> Result<Vec<PinWithMessages>, CoreError> {
        self.list_pins(version_id, Some(owner_token)).await
    }

    async fn list_pins(
        &self,
        version_id: i32,
        owner: Option<&str>,
    ) -> Result<Vec<PinWithMessages>, CoreError> {
        let mut q = comment_pins::Entity::find()
            .filter(comment_pins::Column::VersionId.eq(version_id))
            .filter(comment_pins::Column::DeletedAt.is_null());
        if let Some(token) = owner {
            q = q.filter(comment_pins::Column::OwnerToken.eq(token));
        }
        let pins = q.order_by_asc(comment_pins::Column::Id).all(&self.db).await?;

        let mut out = Vec::with_capacity(pins.len());
        for pin in pins {
            let messages = comments::Entity::find()
                .filter(comments::Column::PinId.eq(pin.id))
                .filter(comments::Column::DeletedAt.is_null())
                .order_by_asc(comments::Column::Id)
                .all(&self.db)
                .await?;
            // Un pin sans message vivant n'est pas montré (cohérent avec le soft-delete du dernier message).
            if !messages.is_empty() {
                out.push(PinWithMessages { pin, messages });
            }
        }
        Ok(out)
    }

    /// Compte les commentaires vivants par version (regroupé, pas de N+1).
    pub async fn count_comments_by_version(
        &self,
        version_ids: &[i32],
    ) -> Result<HashMap<i32, i32>, CoreError> {
        let mut counts: HashMap<i32, i32> = version_ids.iter().map(|id| (*id, 0)).collect();
        if version_ids.is_empty() {
            return Ok(counts);
        }
        let pins = comment_pins::Entity::find()
            .filter(comment_pins::Column::VersionId.is_in(version_ids.to_vec()))
            .filter(comment_pins::Column::DeletedAt.is_null())
            .all(&self.db)
            .await?;
        let pin_to_version: HashMap<i32, i32> =
            pins.iter().map(|p| (p.id, p.version_id)).collect();
        if pin_to_version.is_empty() {
            return Ok(counts);
        }
        let pin_ids: Vec<i32> = pin_to_version.keys().copied().collect();
        let msgs = comments::Entity::find()
            .filter(comments::Column::PinId.is_in(pin_ids))
            .filter(comments::Column::DeletedAt.is_null())
            .all(&self.db)
            .await?;
        for m in msgs {
            if let Some(vid) = pin_to_version.get(&m.pin_id) {
                *counts.entry(*vid).or_insert(0) += 1;
            }
        }
        Ok(counts)
    }

    /// Charge un pin vivant possédé par `owner_token`, ou NotFound (ne révèle pas l'existence).
    async fn owned_live_pin(
        &self,
        pin_id: i32,
        owner_token: &str,
    ) -> Result<comment_pins::Model, CoreError> {
        let pin = comment_pins::Entity::find_by_id(pin_id)
            .filter(comment_pins::Column::DeletedAt.is_null())
            .one(&self.db)
            .await?
            .ok_or(CoreError::NotFound)?;
        if !secure_compare(&pin.owner_token, owner_token) {
            return Err(CoreError::NotFound);
        }
        Ok(pin)
    }
}
```

> `count(...)` and `is_in(...)` require `use sea_orm::PaginatorTrait;` for `count`. Add it to the
> `use sea_orm::{…}` line: append `PaginatorTrait`.

- [ ] **Step 4: Add `PaginatorTrait` to the imports**

Update the `use sea_orm::{...}` in `comments.rs` to include `PaginatorTrait`:

```rust
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set, TransactionTrait,
};
```

- [ ] **Step 5: Run the tests**

Run: `cargo nextest run -p latch services::comments`
Expected: PASS (8 tests).

- [ ] **Step 6: Commit**

```bash
rtk git add backend/src/services/comments.rs backend/src/services/mod.rs
rtk git commit -m "✨ feat(comments): CommentsService — create_pin/add_reply/list/count"
```

---

### Task 4: CommentsService — edit, delete & moderation

**Files:**
- Modify: `backend/src/services/comments.rs`

**Interfaces:**
- Consumes: `CommentsService`, `PinWithMessages`, `owned_live_pin`.
- Produces:
  - `edit_message(&self, comment_id: i32, owner_token: &str, body: &str) -> Result<comments::Model, CoreError>`
  - `delete_message(&self, comment_id: i32, owner_token: &str) -> Result<(), CoreError>`
  - `delete_pin(&self, pin_id: i32, owner_token: &str) -> Result<(), CoreError>`
  - `moderate_delete_message(&self, project_id: i32, comment_id: i32) -> Result<(), CoreError>`

- [ ] **Step 1: Write the failing tests (append to the test module in `comments.rs`)**

```rust
    #[tokio::test]
    async fn edit_message_updates_own_body() {
        let db = test_db().await;
        let v = version(&db).await;
        let svc = CommentsService::new(db);
        let pwm = svc.create_pin(v.id, OWNER_A, "Léa", "avant", "{}").await.unwrap();
        let id = pwm.messages[0].id;

        let edited = svc.edit_message(id, OWNER_A, "après").await.unwrap();
        assert_eq!(edited.body, "après");
    }

    #[tokio::test]
    async fn edit_message_of_other_is_not_found() {
        let db = test_db().await;
        let v = version(&db).await;
        let svc = CommentsService::new(db);
        let pwm = svc.create_pin(v.id, OWNER_A, "Léa", "x", "{}").await.unwrap();
        let id = pwm.messages[0].id;
        assert!(matches!(
            svc.edit_message(id, OWNER_B, "hack").await.unwrap_err(),
            CoreError::NotFound
        ));
    }

    #[tokio::test]
    async fn delete_last_message_soft_deletes_pin() {
        let db = test_db().await;
        let v = version(&db).await;
        let svc = CommentsService::new(db.clone());
        let pwm = svc.create_pin(v.id, OWNER_A, "Léa", "seul", "{}").await.unwrap();

        svc.delete_message(pwm.messages[0].id, OWNER_A).await.unwrap();

        // plus aucun pin vivant visible
        let mine = svc.list_for_version_and_owner(v.id, OWNER_A).await.unwrap();
        assert!(mine.is_empty());
        // le pin porte un tombstone
        use crate::models::_entities::comment_pins;
        let pin = comment_pins::Entity::find_by_id(pwm.pin.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert!(pin.deleted_at.is_some());
    }

    #[tokio::test]
    async fn delete_one_of_two_messages_keeps_pin() {
        let db = test_db().await;
        let v = version(&db).await;
        let svc = CommentsService::new(db);
        let pwm = svc.create_pin(v.id, OWNER_A, "Léa", "un", "{}").await.unwrap();
        svc.add_reply(pwm.pin.id, OWNER_A, "Léa", "deux").await.unwrap();

        svc.delete_message(pwm.messages[0].id, OWNER_A).await.unwrap();
        let mine = svc.list_for_version_and_owner(v.id, OWNER_A).await.unwrap();
        assert_eq!(mine.len(), 1);
        assert_eq!(mine[0].messages.len(), 1);
        assert_eq!(mine[0].messages[0].body, "deux");
    }

    #[tokio::test]
    async fn delete_pin_hides_whole_thread() {
        let db = test_db().await;
        let v = version(&db).await;
        let svc = CommentsService::new(db);
        let pwm = svc.create_pin(v.id, OWNER_A, "Léa", "un", "{}").await.unwrap();
        svc.add_reply(pwm.pin.id, OWNER_A, "Léa", "deux").await.unwrap();

        svc.delete_pin(pwm.pin.id, OWNER_A).await.unwrap();
        assert!(svc.list_for_version_and_owner(v.id, OWNER_A).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn moderate_delete_checks_project_ownership() {
        use crate::models::_entities::projects;
        let db = test_db().await;
        let v = version(&db).await; // project of v
        let other = projects::ActiveModel {
            slug: Set("demo-cccccccc".to_string()),
            name: Set("Autre".to_string()),
            code_enabled: Set(false),
            comments_enabled: Set(true),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        let svc = CommentsService::new(db);
        let pwm = svc.create_pin(v.id, OWNER_A, "Léa", "x", "{}").await.unwrap();
        let mid = pwm.messages[0].id;

        // Mauvais projet → NotFound (ne supprime pas).
        assert!(matches!(
            svc.moderate_delete_message(other.id, mid).await.unwrap_err(),
            CoreError::NotFound
        ));
        // Bon projet → OK.
        svc.moderate_delete_message(v.project_id, mid).await.unwrap();
        assert!(svc.list_for_version(v.id).await.unwrap().is_empty());
    }
```

- [ ] **Step 2: Implement the four methods (add to `impl CommentsService`)**

```rust
    /// Édite le corps d'un message **possédé** par `owner_token`. Étranger/absent → NotFound.
    pub async fn edit_message(
        &self,
        comment_id: i32,
        owner_token: &str,
        body: &str,
    ) -> Result<comments::Model, CoreError> {
        let body = validate_body(body)?;
        let msg = self.owned_live_message(comment_id, owner_token).await?;
        let mut active: comments::ActiveModel = msg.into();
        active.body = Set(body);
        active.updated_at = Set(chrono::Utc::now().into());
        Ok(active.update(&self.db).await?)
    }

    /// Soft-delete d'un message possédé. Si c'était le dernier vivant du pin, soft-delete le pin.
    pub async fn delete_message(
        &self,
        comment_id: i32,
        owner_token: &str,
    ) -> Result<(), CoreError> {
        let msg = self.owned_live_message(comment_id, owner_token).await?;
        let pin_id = msg.pin_id;
        self.soft_delete_message(msg).await?;
        self.soft_delete_pin_if_empty(pin_id).await
    }

    /// Soft-delete d'un pin possédé (et de ses messages).
    pub async fn delete_pin(&self, pin_id: i32, owner_token: &str) -> Result<(), CoreError> {
        let pin = self.owned_live_pin(pin_id, owner_token).await?;
        self.soft_delete_pin(pin).await
    }

    /// Modération admin : supprime n'importe quel message **du projet `project_id`**.
    /// Vérifie message → pin → version → projet avant de supprimer (NotFound sinon).
    pub async fn moderate_delete_message(
        &self,
        project_id: i32,
        comment_id: i32,
    ) -> Result<(), CoreError> {
        use crate::models::_entities::versions;
        let msg = comments::Entity::find_by_id(comment_id)
            .filter(comments::Column::DeletedAt.is_null())
            .one(&self.db)
            .await?
            .ok_or(CoreError::NotFound)?;
        let pin = comment_pins::Entity::find_by_id(msg.pin_id)
            .one(&self.db)
            .await?
            .ok_or(CoreError::NotFound)?;
        let version = versions::Entity::find_by_id(pin.version_id)
            .one(&self.db)
            .await?
            .ok_or(CoreError::NotFound)?;
        if version.project_id != project_id {
            return Err(CoreError::NotFound);
        }
        let pin_id = msg.pin_id;
        self.soft_delete_message(msg).await?;
        self.soft_delete_pin_if_empty(pin_id).await
    }

    async fn owned_live_message(
        &self,
        comment_id: i32,
        owner_token: &str,
    ) -> Result<comments::Model, CoreError> {
        let msg = comments::Entity::find_by_id(comment_id)
            .filter(comments::Column::DeletedAt.is_null())
            .one(&self.db)
            .await?
            .ok_or(CoreError::NotFound)?;
        if !secure_compare(&msg.owner_token, owner_token) {
            return Err(CoreError::NotFound);
        }
        Ok(msg)
    }

    async fn soft_delete_message(&self, msg: comments::Model) -> Result<(), CoreError> {
        let mut active: comments::ActiveModel = msg.into();
        active.deleted_at = Set(Some(chrono::Utc::now().into()));
        active.update(&self.db).await?;
        Ok(())
    }

    async fn soft_delete_pin(&self, pin: comment_pins::Model) -> Result<(), CoreError> {
        let pin_id = pin.id;
        let mut active: comment_pins::ActiveModel = pin.into();
        active.deleted_at = Set(Some(chrono::Utc::now().into()));
        active.update(&self.db).await?;
        // Soft-delete des messages vivants du pin.
        let live = comments::Entity::find()
            .filter(comments::Column::PinId.eq(pin_id))
            .filter(comments::Column::DeletedAt.is_null())
            .all(&self.db)
            .await?;
        for m in live {
            self.soft_delete_message(m).await?;
        }
        Ok(())
    }

    async fn soft_delete_pin_if_empty(&self, pin_id: i32) -> Result<(), CoreError> {
        let remaining = comments::Entity::find()
            .filter(comments::Column::PinId.eq(pin_id))
            .filter(comments::Column::DeletedAt.is_null())
            .count(&self.db)
            .await?;
        if remaining == 0 {
            if let Some(pin) = comment_pins::Entity::find_by_id(pin_id)
                .filter(comment_pins::Column::DeletedAt.is_null())
                .one(&self.db)
                .await?
            {
                let mut active: comment_pins::ActiveModel = pin.into();
                active.deleted_at = Set(Some(chrono::Utc::now().into()));
                active.update(&self.db).await?;
            }
        }
        Ok(())
    }
```

- [ ] **Step 3: Run the tests**

Run: `cargo nextest run -p latch services::comments`
Expected: PASS (14 tests total).

- [ ] **Step 4: Commit**

```bash
rtk git add backend/src/services/comments.rs
rtk git commit -m "✨ feat(comments): edit/delete/delete_pin/moderation (soft-delete + owner check)"
```

---

### Task 5: `comments_enabled` plumbing (projects)

**Files:**
- Modify: `backend/src/services/projects.rs` (`CreateProject` + `create`)
- Modify: `backend/src/dto/mod.rs` (DTO fields + conversions)
- Modify: `backend/src/controllers/admin.rs` (`create`, `update`)
- Test: inline tests in `projects.rs` and `dto/mod.rs`

**Interfaces:**
- Consumes: `projects::Model.comments_enabled`.
- Produces: `CreateProject.comments_enabled: bool`; DTO fields `comments_enabled` on
  `ProjectListItem`/`ProjectDetail`/`PublicMeta` and `Option<bool>` on
  `CreateProjectReq`/`UpdateProjectReq`; `to_list_item`/`to_detail`/`to_public_meta` set them.

- [ ] **Step 1: Failing test — `ProjectsService::create` stores `comments_enabled`**

In `backend/src/services/projects.rs` test module, add:

```rust
    #[tokio::test]
    async fn create_stores_comments_enabled() {
        let s = svc(test_db().await);
        let p = s
            .create(CreateProject {
                name: "P".to_string(),
                brand_name: None,
                code_enabled: true,
                pin: Some("424242".to_string()),
                comments_enabled: true,
            })
            .await
            .unwrap();
        assert!(p.comments_enabled);
    }
```

- [ ] **Step 2: Add the field to `CreateProject` and set it in `create`**

In `projects.rs`, add to `struct CreateProject`:

```rust
    /// Active les commentaires sur le projet (défaut sécurité-aware posé par l'appelant).
    pub comments_enabled: bool,
```

In `create(...)`, add `comments_enabled` to the `ActiveModel`:

```rust
        let model = projects::ActiveModel {
            slug: Set(slug::generate_slug(&input.name)),
            name: Set(input.name),
            code_enabled: Set(input.code_enabled),
            pin: Set(pin_value),
            brand_name: Set(input.brand_name),
            comments_enabled: Set(input.comments_enabled),
            ..Default::default()
        }
```

Update the existing `create_*` tests in this file to pass `comments_enabled: false` (or `true`) in
their `CreateProject { … }` literals — run `cargo check -p latch` and fix each "missing field" hit.

- [ ] **Step 3: Add DTO fields + update conversions**

In `backend/src/dto/mod.rs`:

`ProjectListItem` — add after `version_count`:
```rust
    pub comments_enabled: bool,
```
`ProjectDetail` — add after `active_version_id`:
```rust
    pub comments_enabled: bool,
```
`PublicMeta` — add after `code_enabled`:
```rust
    pub comments_enabled: bool,
```
`CreateProjectReq` — add:
```rust
    #[serde(default)]
    pub comments_enabled: Option<bool>,
```
`UpdateProjectReq` — add:
```rust
    #[serde(default)]
    pub comments_enabled: Option<bool>,
```

In `to_list_item`, add `comments_enabled: m.comments_enabled,`.
In `to_detail`, add `comments_enabled: m.comments_enabled,`.
In `to_public_meta`, add `comments_enabled: m.comments_enabled,`.

- [ ] **Step 4: Wire the admin handlers**

In `backend/src/controllers/admin.rs` `create`:
```rust
    let svc = ProjectsService::new(ctx.db.clone());
    let comments_enabled = body.comments_enabled.unwrap_or(body.code_enabled);
    let project = svc
        .create(CreateProject {
            name: body.name,
            brand_name: body.brand_name,
            code_enabled: body.code_enabled,
            pin: body.pin,
            comments_enabled,
        })
        .await
        .map_err(into_response)?;
```

In `admin.rs` `update`, after the `brand_name` block and before `active.updated_at = …`:
```rust
    if let Some(ce) = body.comments_enabled {
        active.comments_enabled = Set(ce);
    }
```

- [ ] **Step 5: Update DTO test `sample_model` already has the field; add a default test**

In `dto/mod.rs` tests, add:
```rust
    #[test]
    fn create_req_comments_enabled_defaults_none() {
        let req: CreateProjectReq = serde_json::from_str(r#"{"name":"X"}"#).unwrap();
        assert_eq!(req.comments_enabled, None, "absent ⇒ None (handler dérive du code)");
    }

    #[test]
    fn detail_carries_comments_enabled() {
        let json = serde_json::to_string(&to_detail(sample_model(), vec![])).unwrap();
        assert!(json.contains("comments_enabled"));
    }
```

- [ ] **Step 6: Run the tests**

Run: `cargo nextest run -p latch services::projects dto`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
rtk git add backend/src/services/projects.rs backend/src/dto/mod.rs backend/src/controllers/admin.rs
rtk git commit -m "✨ feat(comments): toggle comments_enabled par projet (service + DTO + admin)"
```

---

### Task 6: Visitor identity helpers + `X-Comment-Client` guard

**Files:**
- Modify: `backend/Cargo.toml` (add `ulid`)
- Modify: `backend/src/controllers/serve.rs` (identity helpers + middleware)
- Test: inline `#[cfg(test)]` in `serve.rs`

**Interfaces:**
- Produces (all `pub(crate)` in `serve.rs`):
  - `const COMMENT_COOKIE_NAME: &str = "latch_comment"`
  - `fn mint_owner_token() -> String`
  - `fn read_owner_token(ctx: &AppContext, headers: &HeaderMap) -> Result<Option<String>>`
  - `fn comment_identity_cookie(ctx: &AppContext, slug: &str, token: &str) -> Cookie<'static>`
  - `async fn require_comment_client(req, next) -> Result<Response>` (403 if `X-Comment-Client` absent)

- [ ] **Step 1: Add the `ulid` dependency**

In `backend/Cargo.toml`, under `[dependencies]`, add (resolve the exact current version via Context7
`/dylanhart/ulid-rs` at implementation time; floor shown):

```toml
ulid = "1"
```

Run: `cargo build -p latch` — Expected: compiles with `ulid` available.

- [ ] **Step 2: Failing test — owner token mint + cookie roundtrip**

In `serve.rs`, add to its `#[cfg(test)]` module (create one if absent, mirroring other controllers):

```rust
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod comment_identity_tests {
    use super::*;

    #[test]
    fn minted_token_is_26_char_ulid() {
        let t = mint_owner_token();
        assert_eq!(t.len(), 26, "ULID Crockford base32 = 26 chars");
        assert!(t.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn two_tokens_differ() {
        assert_ne!(mint_owner_token(), mint_owner_token());
    }
}
```

- [ ] **Step 3: Implement the identity helpers in `serve.rs`**

Near the top constants (next to `UNLOCK_COOKIE_NAME`):

```rust
/// Cookie d'identité visiteur pour les commentaires (ULID opaque, signé). Scopé par slug.
pub(crate) const COMMENT_COOKIE_NAME: &str = "latch_comment";
/// Durée de vie du cookie d'identité (jours).
const COMMENT_IDENTITY_TTL_DAYS: i64 = 365;
```

Add the helpers (place near `unlock` handler):

```rust
/// Génère un `owner_token` opaque (ULID Crockford base32, 26 chars).
pub(crate) fn mint_owner_token() -> String {
    ulid::Ulid::new().to_string()
}

/// Lit l'`owner_token` du cookie signé `latch_comment`, s'il est présent et valide.
pub(crate) fn read_owner_token(
    ctx: &AppContext,
    headers: &HeaderMap,
) -> Result<Option<String>> {
    let key = crate::web::unlock_key(ctx)?;
    let jar = SignedCookieJar::from_headers(headers, key);
    Ok(jar.get(COMMENT_COOKIE_NAME).map(|c| c.value().to_string()))
}

/// Construit le cookie d'identité signé pour `slug` (réutilise la clé `UNLOCK_COOKIE_SECRET`).
pub(crate) fn comment_identity_cookie(
    ctx: &AppContext,
    slug: &str,
    token: &str,
) -> Cookie<'static> {
    Cookie::build((COMMENT_COOKIE_NAME, token.to_string()))
        .path(format!("/c/{slug}"))
        .http_only(true)
        .secure(crate::web::cookie_secure(ctx))
        .same_site(SameSite::Lax)
        .max_age(time::Duration::days(COMMENT_IDENTITY_TTL_DAYS))
        .build()
}
```

- [ ] **Step 4: Implement the `require_comment_client` middleware in `serve.rs`**

```rust
/// Middleware : exige le header `X-Comment-Client` sur les écritures de commentaires
/// (anti-CSRF complémentaire au SameSite + garde Origin). 403 si absent.
pub(crate) async fn require_comment_client(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> std::result::Result<Response, StatusCode> {
    if req.headers().contains_key("x-comment-client") {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}
```

> Imports needed in `serve.rs` (most already present): `axum::http::HeaderMap`,
> `axum_extra::extract::cookie::{Cookie, SameSite, SignedCookieJar}`, `axum::http::StatusCode`,
> `time`. Add any missing.

- [ ] **Step 5: Run the tests**

Run: `cargo nextest run -p latch comment_identity_tests`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
rtk git add backend/Cargo.toml backend/Cargo.lock backend/src/controllers/serve.rs
rtk git commit -m "✨ feat(comments): identité visiteur (cookie signé latch_comment) + garde X-Comment-Client"
```

---

### Task 7: Comment DTOs + conversions + `comment_count`

**Files:**
- Modify: `backend/src/dto/mod.rs`
- Test: inline tests in `dto/mod.rs`

**Interfaces:**
- Consumes: `comment_pins::Model`, `comments::Model`, `PinWithMessages` (via models).
- Produces DTOs + conversions:
  - `CreatePinReq { anchor, author_name, body }`, `ReplyReq { author_name, body }`, `EditMessageReq { body }`
  - `CommentMessage { id, author_name, body, created_at, updated_at, editable }`
  - `CommentPin { id, anchor, created_at, messages }`, `CommentList { version, pins }`
  - `AdminCommentMessage { id, author_name, body, created_at, updated_at }`
  - `AdminCommentPin { id, anchor, created_at, messages }`, `AdminCommentList { version, pins }`
  - `to_comment_pin(pin, msgs, caller_owner_token) -> CommentPin`
  - `to_admin_comment_pin(pin, msgs) -> AdminCommentPin`
  - `VersionItem.comment_count: i32` + `to_detail` gains a `counts: &HashMap<i32,i32>` param

- [ ] **Step 1: Failing test — owner_token never serialized; editable computed**

In `dto/mod.rs` tests:

```rust
    #[test]
    fn comment_pin_hides_owner_token_and_computes_editable() {
        use crate::models::_entities::{comment_pins, comments};
        let now = chrono::Utc::now();
        let pin = comment_pins::Model {
            id: 7,
            version_id: 1,
            owner_token: "01OWNERAAAAAAAAAAAAAAAAAAA".to_string(),
            anchor: r#"{"v":1}"#.to_string(),
            status: "open".to_string(),
            created_at: now.into(),
            updated_at: now.into(),
            deleted_at: None,
        };
        let msg = comments::Model {
            id: 9,
            pin_id: 7,
            owner_token: "01OWNERAAAAAAAAAAAAAAAAAAA".to_string(),
            author_name: "Léa".to_string(),
            body: "hi".to_string(),
            created_at: now.into(),
            updated_at: now.into(),
            deleted_at: None,
        };
        let dto = to_comment_pin(&pin, &[msg], "01OWNERAAAAAAAAAAAAAAAAAAA");
        let json = serde_json::to_string(&dto).unwrap();
        assert!(!json.contains("owner_token"), "owner_token ne doit jamais sortir");
        assert!(!json.contains("01OWNERAAAAAAAAAAAAAAAAAAA"));
        assert!(dto.messages[0].editable, "auteur courant ⇒ editable");
    }

    #[test]
    fn comment_pin_not_editable_for_other_caller() {
        use crate::models::_entities::{comment_pins, comments};
        let now = chrono::Utc::now();
        let pin = comment_pins::Model {
            id: 7, version_id: 1, owner_token: "A".to_string(), anchor: "{}".to_string(),
            status: "open".to_string(), created_at: now.into(), updated_at: now.into(), deleted_at: None,
        };
        let msg = comments::Model {
            id: 9, pin_id: 7, owner_token: "A".to_string(), author_name: "Léa".to_string(),
            body: "hi".to_string(), created_at: now.into(), updated_at: now.into(), deleted_at: None,
        };
        let dto = to_comment_pin(&pin, &[msg], "B");
        assert!(!dto.messages[0].editable);
    }

    #[test]
    fn admin_comment_pin_hides_owner_token() {
        use crate::models::_entities::{comment_pins, comments};
        let now = chrono::Utc::now();
        let pin = comment_pins::Model {
            id: 1, version_id: 1, owner_token: "SECRET".to_string(), anchor: "{}".to_string(),
            status: "open".to_string(), created_at: now.into(), updated_at: now.into(), deleted_at: None,
        };
        let msg = comments::Model {
            id: 2, pin_id: 1, owner_token: "SECRET".to_string(), author_name: "Max".to_string(),
            body: "y".to_string(), created_at: now.into(), updated_at: now.into(), deleted_at: None,
        };
        let json = serde_json::to_string(&to_admin_comment_pin(&pin, &[msg])).unwrap();
        assert!(!json.contains("SECRET") && !json.contains("owner_token"));
        assert!(!json.contains("editable"), "pas d'editable côté admin");
    }
```

- [ ] **Step 2: Add the DTOs (in `dto/mod.rs`)**

```rust
// ---- Commentaires ancrés (surface /c) -----------------------------------

/// Corps de `POST /c/{slug}/comments` — crée un pin + 1ᵉʳ message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CreatePinReq {
    /// Descripteur d'ancrage JSON opaque (le serveur ne l'interprète jamais).
    pub anchor: String,
    pub author_name: String,
    pub body: String,
}

/// Corps de `POST /c/{slug}/comments/pins/{pin}/replies`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ReplyReq {
    pub author_name: String,
    pub body: String,
}

/// Corps de `PUT /c/{slug}/comments/messages/{id}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct EditMessageReq {
    pub body: String,
}

/// Message d'un fil, vu par le visiteur (jamais d'`owner_token`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CommentMessage {
    pub id: i32,
    pub author_name: String,
    pub body: String,
    pub created_at: String,
    pub updated_at: String,
    /// `true` si l'appelant courant est l'auteur (peut éditer/supprimer).
    pub editable: bool,
}

/// Pin + son fil, vu par le visiteur.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CommentPin {
    pub id: i32,
    pub anchor: String,
    pub created_at: String,
    pub messages: Vec<CommentMessage>,
}

/// Réponse de `GET /c/{slug}/comments`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CommentList {
    pub version: i32,
    pub pins: Vec<CommentPin>,
}

/// Message vu par l'admin (lecture seule — pas d'`editable`, jamais d'`owner_token`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AdminCommentMessage {
    pub id: i32,
    pub author_name: String,
    pub body: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AdminCommentPin {
    pub id: i32,
    pub anchor: String,
    pub created_at: String,
    pub messages: Vec<AdminCommentMessage>,
}

/// Réponse de `GET /api/projects/{id}/versions/{n}/comments`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AdminCommentList {
    pub version: i32,
    pub pins: Vec<AdminCommentPin>,
}
```

- [ ] **Step 3: Add the conversions**

```rust
use crate::models::_entities::comment_pins;
// (add `comment_pins` / `comments` to the existing `use crate::models::_entities::{...}`)

/// Pin + messages → DTO visiteur. `editable` = l'appelant est l'auteur du message.
pub fn to_comment_pin(
    pin: &comment_pins::Model,
    messages: &[comments::Model],
    caller_owner_token: &str,
) -> CommentPin {
    CommentPin {
        id: pin.id,
        anchor: pin.anchor.clone(),
        created_at: pin.created_at.to_rfc3339(),
        messages: messages
            .iter()
            .map(|m| CommentMessage {
                id: m.id,
                author_name: m.author_name.clone(),
                body: m.body.clone(),
                created_at: m.created_at.to_rfc3339(),
                updated_at: m.updated_at.to_rfc3339(),
                editable: m.owner_token == caller_owner_token,
            })
            .collect(),
    }
}

/// Pin + messages → DTO admin (lecture seule).
pub fn to_admin_comment_pin(
    pin: &comment_pins::Model,
    messages: &[comments::Model],
) -> AdminCommentPin {
    AdminCommentPin {
        id: pin.id,
        anchor: pin.anchor.clone(),
        created_at: pin.created_at.to_rfc3339(),
        messages: messages
            .iter()
            .map(|m| AdminCommentMessage {
                id: m.id,
                author_name: m.author_name.clone(),
                body: m.body.clone(),
                created_at: m.created_at.to_rfc3339(),
                updated_at: m.updated_at.to_rfc3339(),
            })
            .collect(),
    }
}
```

> Update the top import: `use crate::models::_entities::{comment_pins, comments, projects, versions};`

- [ ] **Step 4: Add `comment_count` to `VersionItem` and thread counts through `to_detail`**

Add to `VersionItem`:
```rust
    pub comment_count: i32,
```

Change `to_detail` signature and body to accept a counts map:
```rust
pub fn to_detail(
    m: projects::Model,
    vers: Vec<versions::Model>,
    counts: &std::collections::HashMap<i32, i32>,
) -> ProjectDetail {
    let active = m.active_version_id;
    let versions = vers
        .into_iter()
        .map(|v| VersionItem {
            id: v.id,
            n: v.n,
            created_at: v.created_at.to_rfc3339(),
            is_active: Some(v.id) == active,
            release_notes: v.release_notes,
            comment_count: counts.get(&v.id).copied().unwrap_or(0),
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
        comments_enabled: m.comments_enabled,
        versions,
    }
}
```

Update all `to_detail(...)` callers to pass a counts map. In `dto/mod.rs` tests, replace
`to_detail(sample_model(), vec![])` with `to_detail(sample_model(), vec![], &std::collections::HashMap::new())`
(and the `version_item_carries_release_notes` test likewise). In `admin.rs`, every `to_detail(x, y)`
becomes `to_detail(x, y, &counts)`:
  - `create`, `set_code`, `clear_code`: `&std::collections::HashMap::new()` (no versions).
  - `update`, `detail`: build counts from `CommentsService::count_comments_by_version` (done in Task 10).
  For now (this task), pass `&std::collections::HashMap::new()` everywhere so it compiles; Task 10
  replaces the `update`/`detail` ones with real counts.

- [ ] **Step 5: Run the tests**

Run: `cargo nextest run -p latch dto`
Expected: PASS (incl. the 3 new comment DTO tests).

- [ ] **Step 6: Commit**

```bash
rtk git add backend/src/dto/mod.rs backend/src/controllers/admin.rs
rtk git commit -m "✨ feat(comments): DTOs commentaires + comment_count (owner_token jamais sérialisé)"
```

---

### Task 8: Public endpoints — gate, list, create

**Files:**
- Modify: `backend/src/controllers/serve.rs` (gate helper, `list_comments`, `create_comment`, routes + RL)
- Test: `backend/tests/comments_serve.rs` (new)

**Interfaces:**
- Consumes: `comments_gate`, identity helpers (Task 6), `CommentsService`, `load_active_version`,
  `to_comment_pin`, `CreatePinReq`, `CommentList`.
- Produces: `GET /c/{slug}/comments`, `POST /c/{slug}/comments`; helper
  `async fn comments_gate(ctx, headers, slug) -> Result<projects::Model>`.

- [ ] **Step 1: Failing integration tests**

Create `backend/tests/comments_serve.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]

use latch::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn create_then_list_returns_only_own_comment() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("LATCH_STORAGE_ROOT", tmp.path());
    let config = RequestConfigBuilder::new().save_cookies(true).build();
    request_with_config::<App, _, _>(config, |request, _ctx| async move {
        request
            .post("/api/login")
            .json(&serde_json::json!({"user": "admin", "pass": "s3cret"}))
            .await;
        let created = request
            .post("/api/projects")
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"name": "Demo", "code_enabled": false, "comments_enabled": true}))
            .await;
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();
        let slug = created.json::<serde_json::Value>()["slug"].as_str().unwrap().to_string();
        request
            .post(&format!("/api/projects/{id}/deploy"))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"html": "<h1>v1</h1>", "activate": true}))
            .await;

        // POST comment (visitor): needs Origin + X-Comment-Client
        let posted = request
            .post(&format!("/c/{slug}/comments"))
            .add_header("origin", "http://127.0.0.1")
            .add_header("x-comment-client", "1")
            .json(&serde_json::json!({"anchor": "{\"v\":1}", "author_name": "Léa", "body": "trop petit"}))
            .await;
        assert_eq!(posted.status_code(), 200);
        assert_eq!(posted.header("cache-control"), "no-store");
        let body = posted.text();
        assert!(!body.contains("owner_token"));

        // GET list (same cookie jar = same owner)
        let listed = request.get(&format!("/c/{slug}/comments")).await;
        assert_eq!(listed.status_code(), 200);
        let v = listed.json::<serde_json::Value>();
        assert_eq!(v["pins"].as_array().unwrap().len(), 1);
        assert_eq!(v["pins"][0]["messages"][0]["author_name"], "Léa");
        assert_eq!(v["pins"][0]["messages"][0]["editable"], true);
        assert!(!listed.text().contains("owner_token"));
    })
    .await;
    drop(tmp);
}

#[tokio::test]
#[serial]
async fn create_requires_comment_client_header() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("LATCH_STORAGE_ROOT", tmp.path());
    let config = RequestConfigBuilder::new().save_cookies(true).build();
    request_with_config::<App, _, _>(config, |request, _ctx| async move {
        request
            .post("/api/login")
            .json(&serde_json::json!({"user": "admin", "pass": "s3cret"}))
            .await;
        let created = request
            .post("/api/projects")
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"name": "Demo", "code_enabled": false, "comments_enabled": true}))
            .await;
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();
        let slug = created.json::<serde_json::Value>()["slug"].as_str().unwrap().to_string();
        request
            .post(&format!("/api/projects/{id}/deploy"))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"html": "<h1>v1</h1>", "activate": true}))
            .await;
        // No X-Comment-Client → 403
        let posted = request
            .post(&format!("/c/{slug}/comments"))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"anchor": "{}", "author_name": "Léa", "body": "x"}))
            .await;
        assert_eq!(posted.status_code(), 403);
    })
    .await;
    drop(tmp);
}

#[tokio::test]
#[serial]
async fn comments_disabled_project_returns_404() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("LATCH_STORAGE_ROOT", tmp.path());
    let config = RequestConfigBuilder::new().save_cookies(true).build();
    request_with_config::<App, _, _>(config, |request, _ctx| async move {
        request
            .post("/api/login")
            .json(&serde_json::json!({"user": "admin", "pass": "s3cret"}))
            .await;
        let created = request
            .post("/api/projects")
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"name": "Demo", "code_enabled": false, "comments_enabled": false}))
            .await;
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();
        let slug = created.json::<serde_json::Value>()["slug"].as_str().unwrap().to_string();
        request
            .post(&format!("/api/projects/{id}/deploy"))
            .add_header("origin", "http://127.0.0.1")
            .json(&serde_json::json!({"html": "<h1>v1</h1>", "activate": true}))
            .await;
        let listed = request.get(&format!("/c/{slug}/comments")).await;
        assert_eq!(listed.status_code(), 404);
    })
    .await;
    drop(tmp);
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo nextest run -p latch --test comments_serve`
Expected: FAIL (routes not defined → 404/405 mismatches).

- [ ] **Step 3: Implement the gate helper + handlers in `serve.rs`**

```rust
/// Gate commun aux routes commentaires. Renvoie le projet, ou une `Response` prête :
/// slug inconnu / `comments_enabled=false` → **404** ; projet verrouillé → **403**.
/// Pattern `Result<_, Response>` identique à `resolve_project_html` (statuts exacts,
/// pas de mapping via `loco_rs::Error` qui transformerait le 403 en 401).
async fn comments_gate(
    ctx: &AppContext,
    headers: &HeaderMap,
    slug: &str,
) -> std::result::Result<projects::Model, Response> {
    let svc = ProjectsService::new(ctx.db.clone());
    let project = match svc.get_by_slug(slug).await {
        Ok(p) => p,
        Err(_) => return Err(StatusCode::NOT_FOUND.into_response()),
    };
    if !project.comments_enabled {
        return Err(StatusCode::NOT_FOUND.into_response());
    }
    match unlock_ok(ctx, headers, slug, &project) {
        Ok(true) => Ok(project),
        Ok(false) => Err(StatusCode::FORBIDDEN.into_response()),
        Err(e) => Err(e.into_response()),
    }
}

/// En-têtes JSON `no-store` pour les réponses de commentaires.
fn comments_json_response(value: impl serde::Serialize) -> Result<Response> {
    let body = serde_json::to_string(&value)
        .map_err(|e| loco_rs::Error::Message(format!("serialize comments: {e}")))?;
    Ok((
        [
            (CACHE_CONTROL, HeaderValue::from_static("no-store")),
            (
                CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            ),
        ],
        body,
    )
        .into_response())
}

/// GET /c/{slug}/comments — mes pins+fils de la version active.
#[debug_handler]
pub(crate) async fn list_comments(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Result<Response> {
    let project = match comments_gate(&ctx, &headers, &slug).await {
        Ok(p) => p,
        Err(resp) => return Ok(resp),
    };
    let Some(version) = load_active_version(&ctx, &project).await? else {
        return Err(loco_rs::Error::NotFound);
    };
    let owner = read_owner_token(&ctx, &headers)?;
    let pins = match owner {
        Some(token) => {
            let svc = crate::services::comments::CommentsService::new(ctx.db.clone());
            let rows = svc
                .list_for_version_and_owner(version.id, &token)
                .await
                .map_err(into_response)?;
            rows.iter()
                .map(|pwm| crate::dto::to_comment_pin(&pwm.pin, &pwm.messages, &token))
                .collect()
        }
        None => vec![],
    };
    comments_json_response(crate::dto::CommentList {
        version: version.n,
        pins,
    })
}

/// POST /c/{slug}/comments — crée un pin + 1ᵉʳ message ; pose le cookie d'identité si absent.
#[debug_handler]
pub(crate) async fn create_comment(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    headers: HeaderMap,
    Json(body): Json<crate::dto::CreatePinReq>,
) -> Result<Response> {
    let project = match comments_gate(&ctx, &headers, &slug).await {
        Ok(p) => p,
        Err(resp) => return Ok(resp),
    };
    let Some(version) = load_active_version(&ctx, &project).await? else {
        return Err(loco_rs::Error::NotFound);
    };
    let (token, fresh) = match read_owner_token(&ctx, &headers)? {
        Some(t) => (t, false),
        None => (mint_owner_token(), true),
    };
    let svc = crate::services::comments::CommentsService::new(ctx.db.clone());
    let pwm = svc
        .create_pin(version.id, &token, &body.author_name, &body.body, &body.anchor)
        .await
        .map_err(into_response)?;
    let dto = crate::dto::to_comment_pin(&pwm.pin, &pwm.messages, &token);
    let json = comments_json_response(dto)?;
    if fresh {
        let key = crate::web::unlock_key(&ctx)?;
        let jar = SignedCookieJar::from_headers(&headers, key)
            .add(comment_identity_cookie(&ctx, &slug, &token));
        Ok((jar, json).into_response())
    } else {
        Ok(json)
    }
}
```

> Imports: ensure `projects` model, `CACHE_CONTROL`, `CONTENT_TYPE`, `HeaderValue` are in scope
> (they already are for `raw`/`notes`). `load_active_version` already exists (used by `raw`).

- [ ] **Step 4: Add the routes + comment rate-limit layers in `serve.rs` `routes()`**

Before `Routes::new()`, build comment RL layers (reuse the existing key extractors):

```rust
    let c_ip_burst: u32 = env_u32("LATCH_COMMENT_RL_IP_BURST", 10);
    let c_ip_per_sec: u64 = env_u64("LATCH_COMMENT_RL_IP_PER_SECOND", 1);
    let c_slug_burst: u32 = env_u32("LATCH_COMMENT_RL_SLUG_BURST", 60);
    let c_slug_period: u64 = env_u64("LATCH_COMMENT_RL_SLUG_PERIOD_SECS", 1);
    let comment_ip_layer = {
        #[allow(clippy::expect_used)]
        let config = Arc::new(
            GovernorConfigBuilder::default()
                .per_second(c_ip_per_sec)
                .burst_size(c_ip_burst)
                .key_extractor(IpSlugKeyExtractor)
                .finish()
                .expect("governor comment IP config valide"),
        );
        GovernorLayer { config }
    };
    let comment_slug_layer = {
        #[allow(clippy::expect_used)]
        let config = Arc::new(
            GovernorConfigBuilder::default()
                .period(Duration::from_secs(c_slug_period))
                .burst_size(c_slug_burst)
                .key_extractor(SlugKeyExtractor)
                .finish()
                .expect("governor comment slug config valide"),
        );
        GovernorLayer { config }
    };
```

Then add the routes (GET is gated but not rate-limited; POST stacks RL + Origin + client-header):

```rust
        .add("/c/{slug}/comments", get(list_comments))
        .add(
            "/c/{slug}/comments",
            post(create_comment)
                .layer(axum::middleware::from_fn(require_comment_client))
                .layer(axum::middleware::from_fn(
                    crate::controllers::middleware::origin::require_same_origin,
                ))
                .layer(comment_ip_layer.clone())
                .layer(comment_slug_layer.clone()),
        )
```

> `GovernorLayer { config }` is `Clone` (config is `Arc`) → `.clone()` lets later tasks reuse the same
> layers on the reply/edit/delete routes.

- [ ] **Step 5: Run the tests**

Run: `cargo nextest run -p latch --test comments_serve`
Expected: PASS (3 tests).

- [ ] **Step 6: Commit**

```bash
rtk git add backend/src/controllers/serve.rs backend/tests/comments_serve.rs
rtk git commit -m "✨ feat(comments): endpoints publics GET/POST + gate + identité + rate-limit"
```

---

### Task 9: Public endpoints — reply, edit, delete

**Files:**
- Modify: `backend/src/controllers/serve.rs` (handlers + routes)
- Modify: `backend/tests/comments_serve.rs`

**Interfaces:**
- Consumes: `comments_gate`, `read_owner_token`, `CommentsService` (edit/delete), `ReplyReq`, `EditMessageReq`.
- Produces routes: `POST …/pins/{pin}/replies`, `PUT …/messages/{id}`, `DELETE …/messages/{id}`, `DELETE …/pins/{pin}`.

- [ ] **Step 1: Failing tests (append to `comments_serve.rs`)**

```rust
#[tokio::test]
#[serial]
async fn reply_edit_delete_lifecycle() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("LATCH_STORAGE_ROOT", tmp.path());
    let config = RequestConfigBuilder::new().save_cookies(true).build();
    request_with_config::<App, _, _>(config, |request, _ctx| async move {
        request.post("/api/login").json(&serde_json::json!({"user":"admin","pass":"s3cret"})).await;
        let created = request.post("/api/projects").add_header("origin","http://127.0.0.1")
            .json(&serde_json::json!({"name":"Demo","code_enabled":false,"comments_enabled":true})).await;
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();
        let slug = created.json::<serde_json::Value>()["slug"].as_str().unwrap().to_string();
        request.post(&format!("/api/projects/{id}/deploy")).add_header("origin","http://127.0.0.1")
            .json(&serde_json::json!({"html":"<h1>v1</h1>","activate":true})).await;

        let pin = request.post(&format!("/c/{slug}/comments"))
            .add_header("origin","http://127.0.0.1").add_header("x-comment-client","1")
            .json(&serde_json::json!({"anchor":"{}","author_name":"Léa","body":"un"})).await;
        let pin_id = pin.json::<serde_json::Value>()["id"].as_i64().unwrap();
        let msg_id = pin.json::<serde_json::Value>()["messages"][0]["id"].as_i64().unwrap();

        // reply
        let reply = request.post(&format!("/c/{slug}/comments/pins/{pin_id}/replies"))
            .add_header("origin","http://127.0.0.1").add_header("x-comment-client","1")
            .json(&serde_json::json!({"author_name":"Léa","body":"deux"})).await;
        assert_eq!(reply.status_code(), 200);

        // edit first message
        let edit = request.put(&format!("/c/{slug}/comments/messages/{msg_id}"))
            .add_header("origin","http://127.0.0.1").add_header("x-comment-client","1")
            .json(&serde_json::json!({"body":"un-corrigé"})).await;
        assert_eq!(edit.status_code(), 200);
        assert_eq!(edit.json::<serde_json::Value>()["body"], "un-corrigé");

        // delete the reply message
        let reply_id = reply.json::<serde_json::Value>()["id"].as_i64().unwrap();
        let del = request.delete(&format!("/c/{slug}/comments/messages/{reply_id}"))
            .add_header("origin","http://127.0.0.1").add_header("x-comment-client","1").await;
        assert_eq!(del.status_code(), 200);

        let listed = request.get(&format!("/c/{slug}/comments")).await;
        assert_eq!(listed.json::<serde_json::Value>()["pins"][0]["messages"].as_array().unwrap().len(), 1);

        // delete the whole pin
        let delpin = request.delete(&format!("/c/{slug}/comments/pins/{pin_id}"))
            .add_header("origin","http://127.0.0.1").add_header("x-comment-client","1").await;
        assert_eq!(delpin.status_code(), 200);
        let after = request.get(&format!("/c/{slug}/comments")).await;
        assert_eq!(after.json::<serde_json::Value>()["pins"].as_array().unwrap().len(), 0);
    }).await;
    drop(tmp);
}
```

- [ ] **Step 2: Implement the handlers in `serve.rs`**

```rust
/// Lit l'owner_token requis (écritures sur ressource existante). 403 si pas d'identité.
fn require_owner(ctx: &AppContext, headers: &HeaderMap) -> Result<String> {
    read_owner_token(ctx, headers)?
        .ok_or_else(|| loco_rs::Error::Unauthorized("no identity".to_string()))
}

/// POST /c/{slug}/comments/pins/{pin}/replies — ajoute un message à mon pin.
#[debug_handler]
pub(crate) async fn reply_comment(
    State(ctx): State<AppContext>,
    Path((slug, pin)): Path<(String, i32)>,
    headers: HeaderMap,
    Json(body): Json<crate::dto::ReplyReq>,
) -> Result<Response> {
    if let Err(resp) = comments_gate(&ctx, &headers, &slug).await {
        return Ok(resp);
    }
    let token = require_owner(&ctx, &headers)?;
    let svc = crate::services::comments::CommentsService::new(ctx.db.clone());
    let msg = svc
        .add_reply(pin, &token, &body.author_name, &body.body)
        .await
        .map_err(into_response)?;
    comments_json_response(crate::dto::CommentMessage {
        id: msg.id,
        author_name: msg.author_name,
        body: msg.body,
        created_at: msg.created_at.to_rfc3339(),
        updated_at: msg.updated_at.to_rfc3339(),
        editable: true,
    })
}

/// PUT /c/{slug}/comments/messages/{id} — édite mon message.
#[debug_handler]
pub(crate) async fn edit_comment(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, i32)>,
    headers: HeaderMap,
    Json(body): Json<crate::dto::EditMessageReq>,
) -> Result<Response> {
    if let Err(resp) = comments_gate(&ctx, &headers, &slug).await {
        return Ok(resp);
    }
    let token = require_owner(&ctx, &headers)?;
    let svc = crate::services::comments::CommentsService::new(ctx.db.clone());
    let msg = svc.edit_message(id, &token, &body.body).await.map_err(into_response)?;
    comments_json_response(crate::dto::CommentMessage {
        id: msg.id,
        author_name: msg.author_name,
        body: msg.body,
        created_at: msg.created_at.to_rfc3339(),
        updated_at: msg.updated_at.to_rfc3339(),
        editable: true,
    })
}

/// DELETE /c/{slug}/comments/messages/{id} — supprime mon message.
#[debug_handler]
pub(crate) async fn delete_comment(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, i32)>,
    headers: HeaderMap,
) -> Result<Response> {
    if let Err(resp) = comments_gate(&ctx, &headers, &slug).await {
        return Ok(resp);
    }
    let token = require_owner(&ctx, &headers)?;
    let svc = crate::services::comments::CommentsService::new(ctx.db.clone());
    svc.delete_message(id, &token).await.map_err(into_response)?;
    comments_json_response(crate::dto::OkResponse::ok())
}

/// DELETE /c/{slug}/comments/pins/{pin} — supprime mon pin entier.
#[debug_handler]
pub(crate) async fn delete_comment_pin(
    State(ctx): State<AppContext>,
    Path((slug, pin)): Path<(String, i32)>,
    headers: HeaderMap,
) -> Result<Response> {
    if let Err(resp) = comments_gate(&ctx, &headers, &slug).await {
        return Ok(resp);
    }
    let token = require_owner(&ctx, &headers)?;
    let svc = crate::services::comments::CommentsService::new(ctx.db.clone());
    svc.delete_pin(pin, &token).await.map_err(into_response)?;
    comments_json_response(crate::dto::OkResponse::ok())
}
```

- [ ] **Step 3: Add the routes (in `serve.rs` `routes()`, after the POST `/comments` route)**

```rust
        .add(
            "/c/{slug}/comments/pins/{pin}/replies",
            post(reply_comment)
                .layer(axum::middleware::from_fn(require_comment_client))
                .layer(axum::middleware::from_fn(
                    crate::controllers::middleware::origin::require_same_origin,
                ))
                .layer(comment_ip_layer.clone())
                .layer(comment_slug_layer.clone()),
        )
        .add(
            "/c/{slug}/comments/messages/{id}",
            put(edit_comment)
                .layer(axum::middleware::from_fn(require_comment_client))
                .layer(axum::middleware::from_fn(
                    crate::controllers::middleware::origin::require_same_origin,
                ))
                .layer(comment_ip_layer.clone())
                .layer(comment_slug_layer.clone()),
        )
        .add(
            "/c/{slug}/comments/messages/{id}",
            axum::routing::delete(delete_comment)
                .layer(axum::middleware::from_fn(require_comment_client))
                .layer(axum::middleware::from_fn(
                    crate::controllers::middleware::origin::require_same_origin,
                ))
                .layer(comment_ip_layer.clone())
                .layer(comment_slug_layer.clone()),
        )
        .add(
            "/c/{slug}/comments/pins/{pin}",
            axum::routing::delete(delete_comment_pin)
                .layer(axum::middleware::from_fn(require_comment_client))
                .layer(axum::middleware::from_fn(
                    crate::controllers::middleware::origin::require_same_origin,
                ))
                .layer(comment_ip_layer.clone())
                .layer(comment_slug_layer.clone()),
        )
```

> Ensure `put` is imported: `use axum::routing::{get, post, put};` (extend the existing import).

- [ ] **Step 4: Run the tests**

Run: `cargo nextest run -p latch --test comments_serve`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
rtk git add backend/src/controllers/serve.rs backend/tests/comments_serve.rs
rtk git commit -m "✨ feat(comments): endpoints publics reply/edit/delete/delete-pin"
```

---

### Task 10: Admin endpoints + `comment_count` wiring + OpenAPI regen

**Files:**
- Modify: `backend/src/controllers/admin.rs` (`list_comments`, `moderate_delete_comment`, detail/update counts, routes)
- Modify: `backend/src/openapi.rs` (register paths + schemas)
- Modify: `openapi.json` (regenerated)
- Modify: `frontend/src/api/schema.d.ts` (regenerated)
- Test: `backend/tests/comments_admin.rs` (new)

**Interfaces:**
- Consumes: `CommentsService`, `to_admin_comment_pin`, `AdminCommentList`, `count_comments_by_version`.
- Produces: `GET /api/projects/{id}/versions/{n}/comments`, `DELETE /api/projects/{id}/comments/messages/{id}`.

- [ ] **Step 1: Failing admin integration tests**

Create `backend/tests/comments_admin.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]

use latch::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn admin_lists_all_comments_without_owner_token() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("LATCH_STORAGE_ROOT", tmp.path());
    let config = RequestConfigBuilder::new().save_cookies(true).build();
    request_with_config::<App, _, _>(config, |request, _ctx| async move {
        request.post("/api/login").json(&serde_json::json!({"user":"admin","pass":"s3cret"})).await;
        let created = request.post("/api/projects").add_header("origin","http://127.0.0.1")
            .json(&serde_json::json!({"name":"Demo","code_enabled":false,"comments_enabled":true})).await;
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();
        let slug = created.json::<serde_json::Value>()["slug"].as_str().unwrap().to_string();
        request.post(&format!("/api/projects/{id}/deploy")).add_header("origin","http://127.0.0.1")
            .json(&serde_json::json!({"html":"<h1>v1</h1>","activate":true})).await;
        request.post(&format!("/c/{slug}/comments")).add_header("origin","http://127.0.0.1")
            .add_header("x-comment-client","1")
            .json(&serde_json::json!({"anchor":"{}","author_name":"Léa","body":"coucou"})).await;

        let admin = request.get(&format!("/api/projects/{id}/versions/1/comments")).await;
        assert_eq!(admin.status_code(), 200);
        let v = admin.json::<serde_json::Value>();
        assert_eq!(v["pins"][0]["messages"][0]["author_name"], "Léa");
        assert_eq!(v["pins"][0]["messages"][0]["body"], "coucou");
        assert!(!admin.text().contains("owner_token"));
        assert!(!admin.text().contains("editable"));

        // comment_count visible dans le détail projet
        let detail = request.get(&format!("/api/projects/{id}")).await;
        let dv = detail.json::<serde_json::Value>();
        assert_eq!(dv["versions"][0]["comment_count"], 1);
    }).await;
    drop(tmp);
}

#[tokio::test]
#[serial]
async fn admin_moderates_a_message() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("LATCH_STORAGE_ROOT", tmp.path());
    let config = RequestConfigBuilder::new().save_cookies(true).build();
    request_with_config::<App, _, _>(config, |request, _ctx| async move {
        request.post("/api/login").json(&serde_json::json!({"user":"admin","pass":"s3cret"})).await;
        let created = request.post("/api/projects").add_header("origin","http://127.0.0.1")
            .json(&serde_json::json!({"name":"Demo","code_enabled":false,"comments_enabled":true})).await;
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();
        let slug = created.json::<serde_json::Value>()["slug"].as_str().unwrap().to_string();
        request.post(&format!("/api/projects/{id}/deploy")).add_header("origin","http://127.0.0.1")
            .json(&serde_json::json!({"html":"<h1>v1</h1>","activate":true})).await;
        let pin = request.post(&format!("/c/{slug}/comments")).add_header("origin","http://127.0.0.1")
            .add_header("x-comment-client","1")
            .json(&serde_json::json!({"anchor":"{}","author_name":"Léa","body":"spam"})).await;
        let mid = pin.json::<serde_json::Value>()["messages"][0]["id"].as_i64().unwrap();

        let del = request.delete(&format!("/api/projects/{id}/comments/messages/{mid}"))
            .add_header("origin","http://127.0.0.1").await;
        assert_eq!(del.status_code(), 200);
        let admin = request.get(&format!("/api/projects/{id}/versions/1/comments")).await;
        assert_eq!(admin.json::<serde_json::Value>()["pins"].as_array().unwrap().len(), 0);
    }).await;
    drop(tmp);
}
```

- [ ] **Step 2: Implement the admin handlers in `admin.rs`**

```rust
/// GET /api/projects/{id}/versions/{n}/comments — tous les fils de la version (lecture seule).
#[utoipa::path(
    get, path = "/api/projects/{id}/versions/{n}/comments", tag = "versions",
    params(("id" = i32, Path, description = "Identifiant du projet"),
           ("n" = i32, Path, description = "Numéro de version")),
    responses((status = 200, description = "Commentaires de la version", body = AdminCommentList),
              (status = 404, description = "Version inconnue"),
              (status = 401, description = "Non authentifié"))
)]
#[debug_handler]
async fn list_comments(
    _auth: AdminAuth,
    State(ctx): State<AppContext>,
    Path((id, n)): Path<(i32, i32)>,
) -> Result<Response> {
    let version = versions::Entity::find()
        .filter(versions::Column::ProjectId.eq(id))
        .filter(versions::Column::N.eq(n))
        .one(&ctx.db)
        .await
        .map_err(|e| into_response(e.into()))?
        .ok_or(loco_rs::Error::NotFound)?;
    let svc = crate::services::comments::CommentsService::new(ctx.db.clone());
    let rows = svc.list_for_version(version.id).await.map_err(into_response)?;
    let pins = rows
        .iter()
        .map(|pwm| crate::dto::to_admin_comment_pin(&pwm.pin, &pwm.messages))
        .collect();
    format::json(crate::dto::AdminCommentList { version: n, pins })
}

/// DELETE /api/projects/{id}/comments/messages/{cid} — modération (vérifie l'appartenance au projet).
#[utoipa::path(
    delete, path = "/api/projects/{id}/comments/messages/{cid}", tag = "versions",
    params(("id" = i32, Path, description = "Identifiant du projet"),
           ("cid" = i32, Path, description = "Identifiant du message")),
    responses((status = 200, description = "Message supprimé", body = OkResponse),
              (status = 404, description = "Message hors projet ou inconnu"),
              (status = 401, description = "Non authentifié"),
              (status = 403, description = "Origin invalide (CSRF)"))
)]
#[debug_handler]
async fn moderate_delete_comment(
    _auth: AdminAuth,
    State(ctx): State<AppContext>,
    Path((id, cid)): Path<(i32, i32)>,
) -> Result<Response> {
    let svc = crate::services::comments::CommentsService::new(ctx.db.clone());
    svc.moderate_delete_message(id, cid).await.map_err(into_response)?;
    format::json(crate::dto::OkResponse::ok())
}
```

> Extend the `use crate::dto::{...}` import with `AdminCommentList`.

- [ ] **Step 3: Wire real `comment_count` in `detail` and `update`**

In `admin.rs` `detail`, after loading `vers`, compute counts and pass them:

```rust
    let svc = crate::services::comments::CommentsService::new(ctx.db.clone());
    let version_ids: Vec<i32> = vers.iter().map(|v| v.id).collect();
    let counts = svc
        .count_comments_by_version(&version_ids)
        .await
        .map_err(into_response)?;
    format::json(crate::dto::to_detail(project, vers, &counts))
```

In `update`, do the same before the final `to_detail(saved, vers, …)`:

```rust
    let svc = crate::services::comments::CommentsService::new(ctx.db.clone());
    let version_ids: Vec<i32> = vers.iter().map(|v| v.id).collect();
    let counts = svc
        .count_comments_by_version(&version_ids)
        .await
        .map_err(into_response)?;
    format::json(crate::dto::to_detail(saved, vers, &counts))
```

(`create`, `set_code`, `clear_code` keep `&std::collections::HashMap::new()` from Task 7.)

- [ ] **Step 4: Add the admin routes**

In `admin.rs` `routes()`, before `preview_version`:

```rust
        .add("/projects/{id}/versions/{n}/comments", get(list_comments))
        .add(
            "/projects/{id}/comments/messages/{cid}",
            axum::routing::delete(moderate_delete_comment).layer(from_fn(require_same_origin)),
        )
```

- [ ] **Step 5: Register OpenAPI paths + schemas**

In `backend/src/openapi.rs`, add to `paths(...)`:

```rust
        admin::list_comments,
        admin::moderate_delete_comment,
        serve::list_comments,
        serve::create_comment,
        serve::reply_comment,
        serve::edit_comment,
        serve::delete_comment,
        serve::delete_comment_pin,
```

Add to `components(schemas(...))`:

```rust
        dto::CreatePinReq,
        dto::ReplyReq,
        dto::EditMessageReq,
        dto::CommentMessage,
        dto::CommentPin,
        dto::CommentList,
        dto::AdminCommentMessage,
        dto::AdminCommentPin,
        dto::AdminCommentList,
```

> For the `serve::*` handlers to be referenced by `utoipa::path`, add `#[utoipa::path(...)]` annotations
> to each public comment handler in `serve.rs` (mirror the admin annotations: method, path with `{slug}`
> params, `tag = "serving"`, 200/403/404 responses, request_body where applicable). Without the macro
> the symbol isn't a valid path item.

- [ ] **Step 6: Regenerate `openapi.json` and the TS schema**

Run:
```bash
UPDATE_OPENAPI=1 cargo test -p latch --test openapi_drift
cd frontend && pnpm gen:api && cd ..
```
Expected: `openapi.json` and `frontend/src/api/schema.d.ts` updated with the comment schemas/paths.

- [ ] **Step 7: Run the tests**

Run: `cargo nextest run -p latch --test comments_admin && cargo test -p latch --test openapi_drift`
Expected: PASS (admin tests + drift in sync).

- [ ] **Step 8: Commit**

```bash
rtk git add backend/src/controllers/admin.rs backend/src/openapi.rs backend/src/controllers/serve.rs openapi.json frontend/src/api/schema.d.ts backend/tests/comments_admin.rs
rtk git commit -m "✨ feat(comments): endpoints admin (liste + modération) + comment_count + openapi"
```

---

### Task 11: Security invariants + final gate

**Files:**
- Modify: `backend/tests/security_invariants.rs`
- Test: full backend suite + lint + drift + supply-chain

**Interfaces:**
- Consumes: everything above.
- Produces: build-breaking invariant tests + a green gate.

- [ ] **Step 1: Add the invariant tests**

Append to `backend/tests/security_invariants.rs`:

```rust
/// `owner_token` ne doit jamais apparaître dans une réponse de commentaire (public OU admin) — invariant §9.7.
#[tokio::test]
#[serial]
async fn owner_token_never_in_comment_responses() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("LATCH_STORAGE_ROOT", tmp.path());
    let config = RequestConfigBuilder::new().save_cookies(true).build();
    request_with_config::<App, _, _>(config, |request, _ctx| async move {
        request.post("/api/login").json(&serde_json::json!({"user":"admin","pass":"s3cret"})).await;
        let created = request.post("/api/projects").add_header("origin","http://127.0.0.1")
            .json(&serde_json::json!({"name":"Demo","code_enabled":false,"comments_enabled":true})).await;
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();
        let slug = created.json::<serde_json::Value>()["slug"].as_str().unwrap().to_string();
        request.post(&format!("/api/projects/{id}/deploy")).add_header("origin","http://127.0.0.1")
            .json(&serde_json::json!({"html":"<h1>v1</h1>","activate":true})).await;
        let posted = request.post(&format!("/c/{slug}/comments")).add_header("origin","http://127.0.0.1")
            .add_header("x-comment-client","1")
            .json(&serde_json::json!({"anchor":"{}","author_name":"Léa","body":"x"})).await;
        assert!(!posted.text().contains("owner_token"), "POST réponse fuite owner_token");
        let v = request.get(&format!("/c/{slug}/comments")).await;
        assert!(!v.text().contains("owner_token"), "GET visiteur fuite owner_token");
        let a = request.get(&format!("/api/projects/{id}/versions/1/comments")).await;
        assert!(!a.text().contains("owner_token"), "GET admin fuite owner_token");
    }).await;
    drop(tmp);
}

/// Un projet à code non déverrouillé refuse les commentaires (403) — invariant §9.7 (gate).
#[tokio::test]
#[serial]
async fn locked_project_forbids_comments() {
    std::env::set_var("ADMIN_USER", "admin");
    std::env::set_var("ADMIN_PASS", "s3cret");
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("LATCH_STORAGE_ROOT", tmp.path());
    let config = RequestConfigBuilder::new().save_cookies(true).build();
    request_with_config::<App, _, _>(config, |request, _ctx| async move {
        request.post("/api/login").json(&serde_json::json!({"user":"admin","pass":"s3cret"})).await;
        let created = request.post("/api/projects").add_header("origin","http://127.0.0.1")
            .json(&serde_json::json!({"name":"Demo","code_enabled":true,"pin":"424242","comments_enabled":true})).await;
        let id = created.json::<serde_json::Value>()["id"].as_i64().unwrap();
        let slug = created.json::<serde_json::Value>()["slug"].as_str().unwrap().to_string();
        request.post(&format!("/api/projects/{id}/deploy")).add_header("origin","http://127.0.0.1")
            .json(&serde_json::json!({"html":"<h1>v1</h1>","activate":true})).await;
        // pas de cookie unlock → 403 (note: la session admin n'est pas un cookie unlock)
        let listed = request.get(&format!("/c/{slug}/comments")).await;
        assert_eq!(listed.status_code(), 403, "projet verrouillé doit refuser (403)");
    }).await;
    drop(tmp);
}
```

- [ ] **Step 2: Run the full gate**

Run each, expect PASS / clean:
```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run -p latch
cargo test -p latch --test openapi_drift
cargo deny check
cd frontend && pnpm typecheck && cd ..
```
`pnpm typecheck` confirms the regenerated `schema.d.ts` still type-checks the frontend (no usage yet,
but the generated types must be valid).

- [ ] **Step 3: Commit**

```bash
rtk git add backend/tests/security_invariants.rs
rtk git commit -m "✅ test(comments): invariants owner_token jamais sérialisé + gate verrouillé"
```

---

## Self-Review checklist (run before handing off)

- **Spec coverage:** schema (Task 2) ✓ · service create/list/edit/delete/moderate (Tasks 3–4) ✓ ·
  comments_enabled toggle (Task 5) ✓ · identity cookie + X-Comment-Client (Task 6) ✓ · DTOs +
  comment_count + owner_token hidden (Task 7) ✓ · public endpoints (Tasks 8–9) ✓ · admin list +
  moderation (Task 10) ✓ · rate-limit (Task 8) ✓ · invariants (Task 11) ✓ · contract amended (Task 1) ✓.
  Frontend (shared module, visitor shell, admin Review) is **Plan 2 + Plan 3** — out of scope here.
- **Type consistency:** `CommentsService` method names match between Tasks 3/4 and their callers in
  Tasks 8/9/10; `to_comment_pin`/`to_admin_comment_pin` signatures match Task 7 → used in Tasks 8/9/10;
  `to_detail` 3-arg signature (Task 7) is honored by all `admin.rs` callers (Tasks 5/7/10).
- **Gate status codes:** `comments_gate` returns `Result<_, Response>` (404 disabled/unknown, 403 locked) —
  not `loco_rs::Error` (which would turn 403 into 401). All six public handlers use the match/`if let Err` form.

## What Plan 2 / Plan 3 will cover (not this plan)

- **Plan 2 (frontend shared + visitor):** `src/comments/` module (Picker seam, anchoring ladder,
  tracking controller, overlay, action bar, data adapter), lazy code-split into the shell, React Query.
- **Plan 3 (frontend admin + docs):** `ProjectForm` `comments_enabled` toggle (smart default), version
  list `comment_count` + `VersionCommentsPanel`, `/admin/.../review` overlay mount, `public_docs` pass.
