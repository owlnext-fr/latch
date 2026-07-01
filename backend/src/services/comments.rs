//! Service commentaires — cœur métier (contrat §1, agnostique HTTP). Suppose
//! l'appelant déjà autorisé : l'`owner_token` est fourni par l'adaptateur (qui
//! gère le cookie signé), jamais minté ici. Corps en texte brut (jamais d'HTML).

use std::collections::HashMap;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set, TransactionTrait,
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
/// Identité de propriété de l'unique compte admin (jamais sérialisée : voir `is_admin`).
/// Non-collision avec un ULID visiteur (26 chars Crockford base32, sans underscore).
pub const ADMIN_OWNER_TOKEN: &str = "__admin__";
/// `author_name` stocké pour les messages admin — jamais affiché (l'UI rend un libellé i18n via `is_admin`).
pub const ADMIN_AUTHOR: &str = "admin";

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

    /// Ajoute une réponse admin à **n'importe quel** pin du projet `project_id`
    /// (sans owner-check — l'admin ne possède pas les pins des visiteurs).
    /// Vérifie pin → version → projet avant d'insérer (NotFound sinon).
    pub async fn admin_add_reply(
        &self,
        project_id: i32,
        pin_id: i32,
        body: &str,
    ) -> Result<comments::Model, CoreError> {
        use crate::models::_entities::versions;
        let body = validate_body(body)?;
        let pin = comment_pins::Entity::find_by_id(pin_id)
            .filter(comment_pins::Column::DeletedAt.is_null())
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
        Ok(comments::ActiveModel {
            pin_id: Set(pin.id),
            owner_token: Set(ADMIN_OWNER_TOKEN.to_string()),
            author_name: Set(ADMIN_AUTHOR.to_string()),
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
        let pins = q
            .order_by_asc(comment_pins::Column::Id)
            .all(&self.db)
            .await?;

        if pins.is_empty() {
            return Ok(vec![]);
        }

        // Requête unique pour tous les messages — pas de N+1.
        let pin_ids: Vec<i32> = pins.iter().map(|p| p.id).collect();
        let all_msgs = comments::Entity::find()
            .filter(comments::Column::PinId.is_in(pin_ids))
            .filter(comments::Column::DeletedAt.is_null())
            .order_by_asc(comments::Column::Id)
            .all(&self.db)
            .await?;

        // Regroupement en mémoire : pin_id → Vec<comments::Model> (ordre id-asc préservé).
        let mut msgs_by_pin: HashMap<i32, Vec<comments::Model>> = HashMap::new();
        for msg in all_msgs {
            msgs_by_pin.entry(msg.pin_id).or_default().push(msg);
        }

        let mut out = Vec::with_capacity(pins.len());
        for pin in pins {
            // Un pin sans message vivant n'est pas montré (cohérent avec le soft-delete du dernier message).
            if let Some(messages) = msgs_by_pin.remove(&pin.id) {
                if !messages.is_empty() {
                    out.push(PinWithMessages { pin, messages });
                }
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
        let pin_to_version: HashMap<i32, i32> = pins.iter().map(|p| (p.id, p.version_id)).collect();
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
}

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
            .create_pin(
                v.id,
                OWNER_A,
                "Léa",
                "Le bouton est trop petit",
                r#"{"v":1}"#,
            )
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
        let pwm = svc
            .create_pin(v.id, OWNER_A, "Léa", "un", "{}")
            .await
            .unwrap();

        let reply = svc
            .add_reply(pwm.pin.id, OWNER_A, "Léa", "deux")
            .await
            .unwrap();
        assert_eq!(reply.pin_id, pwm.pin.id);
        assert_eq!(reply.body, "deux");
    }

    #[tokio::test]
    async fn add_reply_to_foreign_pin_is_not_found() {
        let db = test_db().await;
        let v = version(&db).await;
        let svc = CommentsService::new(db);
        let pwm = svc
            .create_pin(v.id, OWNER_A, "Léa", "un", "{}")
            .await
            .unwrap();

        let err = svc
            .add_reply(pwm.pin.id, OWNER_B, "Max", "intrus")
            .await
            .unwrap_err();
        assert!(matches!(err, CoreError::NotFound));
    }

    #[tokio::test]
    async fn admin_add_reply_appends_to_any_pin_with_sentinel_owner() {
        let db = test_db().await;
        let v = version(&db).await;
        let svc = CommentsService::new(db);
        // Un visiteur crée un fil ; l'admin y répond sans le posséder.
        let pwm = svc
            .create_pin(v.id, OWNER_A, "Léa", "un", "{}")
            .await
            .unwrap();

        let reply = svc
            .admin_add_reply(v.project_id, pwm.pin.id, "réponse admin")
            .await
            .unwrap();

        assert_eq!(reply.pin_id, pwm.pin.id);
        assert_eq!(reply.body, "réponse admin");
        assert_eq!(reply.owner_token, ADMIN_OWNER_TOKEN);
    }

    #[tokio::test]
    async fn admin_add_reply_wrong_project_is_not_found() {
        let db = test_db().await;
        let v = version(&db).await;
        let svc = CommentsService::new(db);
        let pwm = svc
            .create_pin(v.id, OWNER_A, "Léa", "un", "{}")
            .await
            .unwrap();

        // project_id qui ne possède pas ce pin → NotFound (ne révèle pas l'existence).
        let err = svc
            .admin_add_reply(v.project_id + 999, pwm.pin.id, "intrus")
            .await
            .unwrap_err();
        assert!(matches!(err, CoreError::NotFound));
    }

    #[tokio::test]
    async fn list_for_owner_only_returns_own_pins() {
        let db = test_db().await;
        let v = version(&db).await;
        let svc = CommentsService::new(db);
        svc.create_pin(v.id, OWNER_A, "Léa", "a", "{}")
            .await
            .unwrap();
        svc.create_pin(v.id, OWNER_B, "Max", "b", "{}")
            .await
            .unwrap();

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
            svc.create_pin(v.id, OWNER_A, "Léa", "   ", "{}")
                .await
                .unwrap_err(),
            CoreError::Validation(_)
        ));
        let long = "x".repeat(MAX_BODY_LEN + 1);
        assert!(matches!(
            svc.create_pin(v.id, OWNER_A, "Léa", &long, "{}")
                .await
                .unwrap_err(),
            CoreError::Validation(_)
        ));
    }

    #[tokio::test]
    async fn create_pin_rejects_empty_author_and_anchor() {
        let db = test_db().await;
        let v = version(&db).await;
        let svc = CommentsService::new(db);
        assert!(matches!(
            svc.create_pin(v.id, OWNER_A, "  ", "ok", "{}")
                .await
                .unwrap_err(),
            CoreError::Validation(_)
        ));
        assert!(matches!(
            svc.create_pin(v.id, OWNER_A, "Léa", "ok", "")
                .await
                .unwrap_err(),
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
        svc.create_pin(v.id, OWNER_A, "Léa", "a", "{}")
            .await
            .unwrap();
        svc.create_pin(v.id, OWNER_B, "Max", "b", "{}")
            .await
            .unwrap();

        let counts = svc.count_comments_by_version(&[v.id]).await.unwrap();
        assert_eq!(counts.get(&v.id).copied(), Some(2));
    }

    #[tokio::test]
    async fn create_pin_rejects_when_cap_reached() {
        let db = test_db().await;
        let v = version(&db).await;
        let svc = CommentsService::new(db);

        for i in 0..MAX_PINS_PER_VERSION_PER_OWNER {
            svc.create_pin(v.id, OWNER_A, "Léa", &format!("message {i}"), "{}")
                .await
                .unwrap();
        }

        let err = svc
            .create_pin(v.id, OWNER_A, "Léa", "un de trop", "{}")
            .await
            .unwrap_err();
        assert!(
            matches!(err, CoreError::Validation(_)),
            "attendu Validation, obtenu {err:?}"
        );
    }

    #[tokio::test]
    async fn edit_message_updates_own_body() {
        let db = test_db().await;
        let v = version(&db).await;
        let svc = CommentsService::new(db);
        let pwm = svc
            .create_pin(v.id, OWNER_A, "Léa", "avant", "{}")
            .await
            .unwrap();
        let id = pwm.messages[0].id;

        let edited = svc.edit_message(id, OWNER_A, "après").await.unwrap();
        assert_eq!(edited.body, "après");
    }

    #[tokio::test]
    async fn edit_message_of_other_is_not_found() {
        let db = test_db().await;
        let v = version(&db).await;
        let svc = CommentsService::new(db);
        let pwm = svc
            .create_pin(v.id, OWNER_A, "Léa", "x", "{}")
            .await
            .unwrap();
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
        let pwm = svc
            .create_pin(v.id, OWNER_A, "Léa", "seul", "{}")
            .await
            .unwrap();

        svc.delete_message(pwm.messages[0].id, OWNER_A)
            .await
            .unwrap();

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
        let pwm = svc
            .create_pin(v.id, OWNER_A, "Léa", "un", "{}")
            .await
            .unwrap();
        svc.add_reply(pwm.pin.id, OWNER_A, "Léa", "deux")
            .await
            .unwrap();

        svc.delete_message(pwm.messages[0].id, OWNER_A)
            .await
            .unwrap();
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
        let pwm = svc
            .create_pin(v.id, OWNER_A, "Léa", "un", "{}")
            .await
            .unwrap();
        svc.add_reply(pwm.pin.id, OWNER_A, "Léa", "deux")
            .await
            .unwrap();

        svc.delete_pin(pwm.pin.id, OWNER_A).await.unwrap();
        assert!(svc
            .list_for_version_and_owner(v.id, OWNER_A)
            .await
            .unwrap()
            .is_empty());
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
        let pwm = svc
            .create_pin(v.id, OWNER_A, "Léa", "x", "{}")
            .await
            .unwrap();
        let mid = pwm.messages[0].id;

        // Mauvais projet → NotFound (ne supprime pas).
        assert!(matches!(
            svc.moderate_delete_message(other.id, mid)
                .await
                .unwrap_err(),
            CoreError::NotFound
        ));
        // Bon projet → OK.
        svc.moderate_delete_message(v.project_id, mid)
            .await
            .unwrap();
        assert!(svc.list_for_version(v.id).await.unwrap().is_empty());
    }
}
