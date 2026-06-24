//! Contrat de fil partagé entre le backend (`latch`) et la SPA (`latch-ui`).
//! Une seule source de vérité : pas de drift possible. serde uniquement → wasm-safe.
//! Les dates sont des `String` (RFC 3339). Aucune dépendance sea-orm ici.

use serde::{Deserialize, Serialize};

/// Item de liste — **sans PIN** (invariant §9.2 : structurellement absent).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectListItem {
    pub id: i32,
    pub slug: String,
    pub name: String,
    pub code_enabled: bool,
    pub brand_name: Option<String>,
    pub active_version_id: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionItem {
    pub id: i32,
    pub n: i32,
    pub created_at: String,
    pub is_active: bool,
}

/// Détail — expose le PIN (copiable en admin uniquement, invariant §9.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateProjectReq {
    #[serde(default)]
    pub name: Option<String>,
    /// `Option<Option<String>>` : champ absent ⇒ pas de changement ; `null` ⇒ effacer.
    #[serde(default)]
    pub brand_name: Option<Option<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetCodeReq {
    pub pin: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeployReq {
    pub html: String,
    #[serde(default)]
    pub activate: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoginReq {
    pub user: String,
    pub pass: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_list_item() -> ProjectListItem {
        ProjectListItem {
            id: 1,
            slug: "mon-projet-k7Qp2maZ".into(),
            name: "Mon Projet".into(),
            code_enabled: true,
            brand_name: None,
            active_version_id: None,
        }
    }

    #[test]
    fn list_item_never_serializes_pin() {
        let json = serde_json::to_string(&sample_list_item()).unwrap();
        assert!(
            !json.contains("\"pin\""),
            "le champ pin ne doit pas exister en liste"
        );
        assert!(!json.contains("424242"));
    }

    #[test]
    fn detail_roundtrips_with_pin() {
        let detail = ProjectDetail {
            id: 1,
            slug: "mon-projet-k7Qp2maZ".into(),
            name: "Mon Projet".into(),
            code_enabled: true,
            pin: Some("424242".into()),
            brand_name: None,
            active_version_id: Some(3),
            versions: vec![VersionItem {
                id: 3,
                n: 3,
                created_at: "2026-06-24T00:00:00+00:00".into(),
                is_active: true,
            }],
        };
        let json = serde_json::to_string(&detail).unwrap();
        assert!(json.contains("424242"), "le détail expose le PIN");
        let back: ProjectDetail = serde_json::from_str(&json).unwrap();
        assert_eq!(detail, back, "round-trip stable (contrat de fil)");
    }

    #[test]
    fn create_req_defaults_code_enabled_true() {
        let req: CreateProjectReq = serde_json::from_str(r#"{"name":"X"}"#).unwrap();
        assert!(req.code_enabled, "code_enabled défaut = true (contrat §3)");
        assert_eq!(req.name, "X");
    }
}
