//! Registre central de validation de FORME des entrées (contrat §1 : la validation
//! de forme vit à la frontière ; ce module en est la source de vérité). Les invariants
//! métier restent dans les services propriétaires.

use std::sync::OnceLock;
use validator::ValidationError;

use crate::services::pin;

/// Longueurs max en CARACTÈRES (Unicode), cohérent avec #13.
pub const MAX_NAME_LEN: usize = 128;
pub const MAX_BODY_LEN: usize = 2000;
pub const MAX_AUTHOR_NAME_LEN: usize = 80;
pub const MAX_RELEASE_NOTES_LEN: usize = 10_000;

/// Tailles max en OCTETS (env-configurables — limites opérationnelles).
pub const DEFAULT_MAX_HTML_BYTES: u64 = 5_242_880; // 5 Mo
pub const DEFAULT_MAX_ANCHOR_BYTES: u64 = 8_192; // 8 Ko

fn env_bytes(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

pub fn max_html_bytes() -> u64 {
    static C: OnceLock<u64> = OnceLock::new();
    *C.get_or_init(|| env_bytes("LATCH_MAX_HTML_BYTES", DEFAULT_MAX_HTML_BYTES))
}
pub fn max_anchor_bytes() -> u64 {
    static C: OnceLock<u64> = OnceLock::new();
    *C.get_or_init(|| env_bytes("LATCH_MAX_ANCHOR_BYTES", DEFAULT_MAX_ANCHOR_BYTES))
}

/// Logique pure « non-vide + ≤ max octets » (testable sans env).
pub(crate) fn bytes_within(v: &str, max: u64) -> Result<(), ValidationError> {
    if v.is_empty() {
        return Err(ValidationError::new("required"));
    }
    if v.len() as u64 > max {
        return Err(ValidationError::new("too_large"));
    }
    Ok(())
}

fn chars_within(v: &str, max: usize, code: &'static str) -> Result<(), ValidationError> {
    if v.chars().count() > max {
        return Err(ValidationError::new(code));
    }
    Ok(())
}

pub fn validate_name(v: &str) -> Result<(), ValidationError> {
    if v.trim().is_empty() {
        return Err(ValidationError::new("name_required"));
    }
    chars_within(v, MAX_NAME_LEN, "name_too_long")
}

pub fn validate_optional_name(v: &Option<String>) -> Result<(), ValidationError> {
    match v {
        Some(s) => validate_name(s),
        None => Ok(()),
    }
}

pub fn validate_optional_brand(v: &Option<String>) -> Result<(), ValidationError> {
    match v {
        Some(s) => chars_within(s, MAX_NAME_LEN, "brand_name_too_long"),
        None => Ok(()),
    }
}

/// `Option<Option<String>>` (UpdateProjectReq.brand_name) : valide l'inner si présent.
pub fn validate_opt_opt_brand(v: &Option<Option<String>>) -> Result<(), ValidationError> {
    match v {
        Some(Some(s)) => chars_within(s, MAX_NAME_LEN, "brand_name_too_long"),
        _ => Ok(()),
    }
}

pub fn validate_body(v: &str) -> Result<(), ValidationError> {
    if v.trim().is_empty() {
        return Err(ValidationError::new("body_required"));
    }
    chars_within(v, MAX_BODY_LEN, "body_too_long")
}

pub fn validate_author_name(v: &str) -> Result<(), ValidationError> {
    if v.trim().is_empty() {
        return Err(ValidationError::new("author_required"));
    }
    chars_within(v, MAX_AUTHOR_NAME_LEN, "author_too_long")
}

pub fn validate_optional_release_notes(v: &Option<String>) -> Result<(), ValidationError> {
    match v {
        Some(s) => chars_within(s, MAX_RELEASE_NOTES_LEN, "release_notes_too_long"),
        None => Ok(()),
    }
}

pub fn validate_pin(v: &str) -> Result<(), ValidationError> {
    if pin::is_valid_pin(v) {
        Ok(())
    } else {
        Err(ValidationError::new("pin_must_be_6_digits"))
    }
}

pub fn validate_html(v: &str) -> Result<(), ValidationError> {
    bytes_within(v, max_html_bytes())
}

pub fn validate_anchor(v: &str) -> Result<(), ValidationError> {
    bytes_within(v, max_anchor_bytes())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn name_rejects_empty_and_too_long() {
        assert!(validate_name("").is_err());
        assert!(validate_name("   ").is_err());
        assert!(validate_name(&"x".repeat(MAX_NAME_LEN + 1)).is_err());
        assert!(validate_name(&"x".repeat(MAX_NAME_LEN)).is_ok());
    }

    #[test]
    fn body_rejects_empty_and_over_max() {
        assert!(validate_body("").is_err());
        assert!(validate_body(&"x".repeat(MAX_BODY_LEN + 1)).is_err());
        assert!(validate_body("ok").is_ok());
    }

    #[test]
    fn author_rejects_over_max() {
        assert!(validate_author_name("").is_err());
        assert!(validate_author_name(&"x".repeat(MAX_AUTHOR_NAME_LEN + 1)).is_err());
        assert!(validate_author_name("Léa").is_ok());
    }

    #[test]
    fn pin_requires_six_digits() {
        assert!(validate_pin("424242").is_ok());
        assert!(validate_pin("42").is_err());
        assert!(validate_pin("abcdef").is_err());
    }

    /// Forme pure préférée (cf. brief §Step 2, résolution d'ambiguïté #1) :
    /// teste `bytes_within` directement plutôt que de muter `std::env` (fragile,
    /// `OnceLock` fige la 1re lecture — un test env-based polluerait les autres
    /// tests du même binaire).
    #[test]
    fn html_len_logic_pure() {
        assert!(bytes_within("", 10).is_err()); // vide
        assert!(bytes_within("12345678901", 10).is_err()); // 11 > 10
        assert!(bytes_within("hello", 10).is_ok());
    }

    #[test]
    fn anchor_rejects_empty_and_over_default() {
        assert!(validate_anchor("").is_err());
        assert!(validate_anchor(&"x".repeat((DEFAULT_MAX_ANCHOR_BYTES + 1) as usize)).is_err());
        assert!(validate_anchor("{}").is_ok());
    }

    #[test]
    fn default_bounds_are_positive() {
        // `max_html_bytes`/`max_anchor_bytes` lisent l'env via `OnceLock` (figé au 1er
        // appel) : on ne peut pas tester leur bornage env-based ici sans risquer de
        // polluer d'autres tests du même binaire. On vérifie juste qu'ils renvoient
        // une valeur positive cohérente avec les defaults documentés.
        assert!(max_html_bytes() > 0);
        assert!(max_anchor_bytes() > 0);
    }
}
