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
    /// Numéro (`n`) de la version active, ou `None` si aucun déploiement.
    /// On n'expose PAS `active_version_id` (PK interne trompeuse) dans la liste.
    pub active_version_n: Option<i32>,
    /// Nombre total de versions du projet.
    pub version_count: i32,
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
        let json = serde_json::to_string(&to_detail(sample_model(), vec![])).unwrap();
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
}
