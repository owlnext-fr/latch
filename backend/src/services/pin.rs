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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_pin_is_six_digits() {
        for _ in 0..100 {
            let p = generate_pin();
            assert_eq!(p.len(), 6, "pin {p:?} should be 6 chars");
            assert!(
                p.chars().all(|c| c.is_ascii_digit()),
                "pin {p:?} digits only"
            );
        }
    }

    #[test]
    fn validates_six_digit_pins() {
        assert!(is_valid_pin("000000"));
        assert!(is_valid_pin("123456"));
    }

    #[test]
    fn rejects_malformed_pins() {
        assert!(!is_valid_pin("12345")); // trop court
        assert!(!is_valid_pin("1234567")); // trop long
        assert!(!is_valid_pin("12a456")); // non chiffre
        assert!(!is_valid_pin(""));
    }
}
