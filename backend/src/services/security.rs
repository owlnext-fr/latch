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
