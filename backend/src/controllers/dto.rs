//! Adaptateur DTO : ré-exporte le contrat de fil partagé (`latch-dto`) et fournit
//! les conversions depuis les modèles sea-orm. Les conversions sont des FONCTIONS
//! LIBRES (orphan rule : on ne peut pas `impl From<&Model>` pour un type étranger).
//! L'invariant §9.2 reste structurel : `ProjectListItem` (latch-dto) n'a pas de `pin`.

pub use latch_dto::{
    CreateProjectReq, DeployReq, ProjectDetail, ProjectListItem, SetCodeReq, UpdateProjectReq,
    VersionItem,
};

use crate::models::_entities::{projects, versions};

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
        let json = serde_json::to_string(&to_list_item(&sample_model())).unwrap();
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
}
