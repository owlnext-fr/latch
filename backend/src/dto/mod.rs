//! Contrat de fil de l'API admin : DTO requête/réponse + conversions `Model → DTO`.
//! Source de vérité des shapes sérialisées (le schéma OpenAPI en dérive, cf. `openapi.rs`).
//! `ProjectListItem` n'a structurellement pas de `pin` (invariant §9.2). Dates = `String` RFC 3339.

use serde::de;
use serde::{Deserialize, Deserializer, Serialize};
use utoipa::ToSchema;

use crate::models::_entities::{comment_pins, comments, projects, versions};

/// Item de liste — **sans PIN** (invariant §9.2 : structurellement absent).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ProjectListItem {
    pub id: i32,
    pub slug: String,
    pub name: String,
    pub code_enabled: bool,
    pub brand_name: Option<String>,
    /// Numéro (`n`) de la version active, ou `None` si aucun déploiement.
    /// On n'expose PAS `active_version_id` (PK interne trompeuse) dans la liste.
    pub active_version_n: Option<i32>,
    /// Nombre total de versions du projet.
    pub version_count: i32,
    pub comments_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct VersionItem {
    pub id: i32,
    pub n: i32,
    pub created_at: String,
    pub is_active: bool,
    pub release_notes: Option<String>,
    pub comment_count: i32,
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
    pub comments_enabled: bool,
    pub versions: Vec<VersionItem>,
}

/// Meta publique servie à la page de déverrouillage (`GET /api/public/{slug}`).
/// **Sans PIN** (invariant §9.2 : structurellement absent) — `brand_name` est fait
/// pour être affiché publiquement sur la page de code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct PublicMeta {
    pub brand_name: Option<String>,
    pub code_enabled: bool,
    pub comments_enabled: bool,
}

/// Réponse de `GET /c/{slug}/notes` — notes de la version active, rendues côté client.
/// `notes_md` est du markdown brut ; le rendu restreint (sans HTML/lien/image) vit côté shell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ReleaseNotes {
    pub n: i32,
    pub notes_md: String,
}

/// Corps de `POST /c/{slug}/unlock`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct UnlockReq {
    pub pin: String,
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
    #[serde(default)]
    pub comments_enabled: Option<bool>,
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
    #[serde(default)]
    pub comments_enabled: Option<bool>,
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
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct LoginReq {
    pub user: String,
    pub pass: String,
}

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

/// Réponse de `GET /api/settings` — infos de branchement MCP pour l'admin.
/// Expose `deploy_token` (secret applicatif) à un admin AUTHENTIFIÉ uniquement
/// (même logique que le PIN au détail). Jamais via MCP, jamais en liste de projets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SettingsResponse {
    pub deploy_token: String,
    pub mcp_url: String,
    pub public_base_url: String,
}

/// Projet + ses versions → item de liste (sans PIN).
/// `active_version_n` = le `n` de la version pointée par `active_version_id` (jamais le PK).
pub fn to_list_item(m: &projects::Model, vers: &[versions::Model]) -> ProjectListItem {
    let active_version_n = m
        .active_version_id
        .and_then(|aid| vers.iter().find(|v| v.id == aid).map(|v| v.n));
    ProjectListItem {
        id: m.id,
        slug: m.slug.clone(),
        name: m.name.clone(),
        code_enabled: m.code_enabled,
        brand_name: m.brand_name.clone(),
        active_version_n,
        version_count: vers.len() as i32,
        comments_enabled: m.comments_enabled,
    }
}

/// Projet + ses versions → détail (avec PIN).
/// `counts` : nombre de commentaires par `version_id` (issu de `CommentsService`).
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

/// Projet → meta publique (sans PIN, sans version).
pub fn to_public_meta(m: &projects::Model) -> PublicMeta {
    PublicMeta {
        brand_name: m.brand_name.clone(),
        code_enabled: m.code_enabled,
        comments_enabled: m.comments_enabled,
    }
}

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

/// Champs communs à `CommentMessage` et `AdminCommentMessage` : `(id, author_name, body, created_at, updated_at)`.
fn message_base_fields(m: &comments::Model) -> (i32, String, String, String, String) {
    (
        m.id,
        m.author_name.clone(),
        m.body.clone(),
        m.created_at.to_rfc3339(),
        m.updated_at.to_rfc3339(),
    )
}

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
            .map(|m| {
                let (id, author_name, body, created_at, updated_at) = message_base_fields(m);
                CommentMessage {
                    id,
                    author_name,
                    body,
                    created_at,
                    updated_at,
                    editable: m.owner_token == caller_owner_token,
                }
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
            .map(|m| {
                let (id, author_name, body, created_at, updated_at) = message_base_fields(m);
                AdminCommentMessage {
                    id,
                    author_name,
                    body,
                    created_at,
                    updated_at,
                }
            })
            .collect(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn sample_pin(owner: &str) -> crate::models::_entities::comment_pins::Model {
        let now = chrono::Utc::now();
        crate::models::_entities::comment_pins::Model {
            id: 7,
            version_id: 1,
            owner_token: owner.to_string(),
            anchor: "{}".to_string(),
            status: "open".to_string(),
            created_at: now.into(),
            updated_at: now.into(),
            deleted_at: None,
        }
    }

    fn sample_msg(owner: &str, author: &str) -> crate::models::_entities::comments::Model {
        let now = chrono::Utc::now();
        crate::models::_entities::comments::Model {
            id: 9,
            pin_id: 7,
            owner_token: owner.to_string(),
            author_name: author.to_string(),
            body: "hi".to_string(),
            created_at: now.into(),
            updated_at: now.into(),
            deleted_at: None,
        }
    }

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
            comments_enabled: true,
        }
    }

    #[test]
    fn list_item_never_serializes_pin() {
        let json = serde_json::to_string(&to_list_item(&sample_model(), &[])).unwrap();
        assert!(
            !json.contains("424242"),
            "le PIN ne doit JAMAIS apparaître en liste (§9.2)"
        );
        assert!(
            !json.contains("\"pin\""),
            "le champ pin ne doit pas exister en liste (§9.2)"
        );
    }

    #[test]
    fn detail_does_serialize_pin() {
        let json = serde_json::to_string(&to_detail(
            sample_model(),
            vec![],
            &std::collections::HashMap::new(),
        ))
        .unwrap();
        assert!(
            json.contains("424242"),
            "le détail doit exposer le PIN (copiable en admin)"
        );
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
        assert_eq!(
            cleared.brand_name,
            Some(None),
            "null = effacer le brand_name"
        );
        let set: UpdateProjectReq = serde_json::from_str(r#"{"brand_name":"ACME"}"#).unwrap();
        assert_eq!(
            set.brand_name,
            Some(Some("ACME".to_string())),
            "valeur = définir"
        );
    }

    #[test]
    fn public_meta_never_serializes_pin() {
        let json = serde_json::to_string(&to_public_meta(&sample_model())).unwrap();
        assert!(
            !json.contains("424242") && !json.contains("\"pin\""),
            "PublicMeta ne doit jamais exposer le PIN (§9.2)"
        );
        assert!(json.contains("code_enabled"));
    }

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
        let detail = to_detail(sample_model(), vec![v], &std::collections::HashMap::new());
        assert_eq!(detail.versions[0].release_notes.as_deref(), Some("# Notes"));
    }

    #[test]
    fn create_req_comments_enabled_defaults_none() {
        let req: CreateProjectReq = serde_json::from_str(r#"{"name":"X"}"#).unwrap();
        assert_eq!(
            req.comments_enabled, None,
            "absent ⇒ None (handler dérive du code)"
        );
    }

    #[test]
    fn detail_carries_comments_enabled() {
        let json = serde_json::to_string(&to_detail(
            sample_model(),
            vec![],
            &std::collections::HashMap::new(),
        ))
        .unwrap();
        assert!(json.contains("comments_enabled"));
    }

    #[test]
    fn comment_pin_hides_owner_token_and_computes_editable() {
        let pin = sample_pin("01OWNERAAAAAAAAAAAAAAAAAAA");
        let msg = sample_msg("01OWNERAAAAAAAAAAAAAAAAAAA", "Léa");
        let dto = to_comment_pin(&pin, &[msg], "01OWNERAAAAAAAAAAAAAAAAAAA");
        let json = serde_json::to_string(&dto).unwrap();
        assert!(
            !json.contains("owner_token"),
            "owner_token ne doit jamais sortir"
        );
        assert!(!json.contains("01OWNERAAAAAAAAAAAAAAAAAAA"));
        assert!(dto.messages[0].editable, "auteur courant ⇒ editable");
    }

    #[test]
    fn comment_pin_not_editable_for_other_caller() {
        let pin = sample_pin("A");
        let msg = sample_msg("A", "Léa");
        let dto = to_comment_pin(&pin, &[msg], "B");
        assert!(!dto.messages[0].editable);
    }

    #[test]
    fn admin_comment_pin_hides_owner_token() {
        let pin = sample_pin("SECRET");
        let msg = sample_msg("SECRET", "Max");
        let json = serde_json::to_string(&to_admin_comment_pin(&pin, &[msg])).unwrap();
        assert!(!json.contains("SECRET") && !json.contains("owner_token"));
        assert!(!json.contains("editable"), "pas d'editable côté admin");
    }
}
