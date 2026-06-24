//! DTO de l'API admin. Le découpage liste/détail fait respecter l'invariant §9.2 :
//! le PIN n'est sérialisé QUE par `ProjectDetail`. `ProjectListItem` ne le porte
//! même pas comme champ — impossible de le fuiter par erreur dans une liste.

use serde::{Deserialize, Serialize};

use crate::models::_entities::{projects, versions};

#[derive(Debug, Serialize)]
pub struct ProjectListItem {
    pub id: i32,
    pub slug: String,
    pub name: String,
    pub code_enabled: bool,
    pub brand_name: Option<String>,
    pub active_version_id: Option<i32>,
    // PAS de `pin` ici. Volontaire (invariant §9.2).
}

impl From<&projects::Model> for ProjectListItem {
    fn from(m: &projects::Model) -> Self {
        Self {
            id: m.id,
            slug: m.slug.clone(),
            name: m.name.clone(),
            code_enabled: m.code_enabled,
            brand_name: m.brand_name.clone(),
            active_version_id: m.active_version_id,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct VersionItem {
    pub id: i32,
    pub n: i32,
    pub created_at: String,
    pub is_active: bool,
}

#[derive(Debug, Serialize)]
pub struct ProjectDetail {
    pub id: i32,
    pub slug: String,
    pub name: String,
    pub code_enabled: bool,
    /// Exposé UNIQUEMENT sur le détail (invariant §9.2). Copiable en admin.
    pub pin: Option<String>,
    pub brand_name: Option<String>,
    pub active_version_id: Option<i32>,
    pub versions: Vec<VersionItem>,
}

impl ProjectDetail {
    pub fn from_model(m: projects::Model, versions: Vec<versions::Model>) -> Self {
        let active = m.active_version_id;
        let versions = versions
            .into_iter()
            .map(|v| VersionItem {
                id: v.id,
                n: v.n,
                created_at: v.created_at.to_rfc3339(),
                is_active: Some(v.id) == active,
            })
            .collect();
        Self {
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
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectReq {
    pub name: String,
    pub brand_name: Option<String>,
    #[serde(default = "default_true")]
    pub code_enabled: bool,
    pub pin: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectReq {
    pub name: Option<String>,
    pub brand_name: Option<Option<String>>,
}

#[derive(Debug, Deserialize)]
pub struct SetCodeReq {
    pub pin: String,
}

#[derive(Debug, Deserialize)]
pub struct DeployReq {
    pub html: String,
    #[serde(default)]
    pub activate: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::_entities::projects;

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
        let item = ProjectListItem::from(&sample_model());
        let json = serde_json::to_string(&item).unwrap();
        assert!(
            !json.contains("424242"),
            "le PIN ne doit JAMAIS apparaître en liste"
        );
        assert!(
            !json.contains("\"pin\""),
            "le champ pin ne doit pas exister en liste"
        );
    }

    #[test]
    fn detail_does_serialize_pin() {
        let detail = ProjectDetail::from_model(sample_model(), vec![]);
        let json = serde_json::to_string(&detail).unwrap();
        assert!(
            json.contains("424242"),
            "le détail doit exposer le PIN (copiable en admin)"
        );
    }
}
