//! Cœur (contrat §1, agnostique HTTP) : jeton de déverrouillage client.
//! Le jeton porté par le cookie signé lie le **PIN courant** du projet :
//! roter le PIN invalide les jetons déjà émis (révocation §6), et l'expiration
//! borne leur durée de vie. La signature du *transport* (anti-falsification) est
//! assurée par le `SignedCookieJar` côté adaptateur ; ici on ne gère que le lien
//! au PIN + l'expiration, en valeurs pures et testables.

use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::services::security::secure_compare;

type HmacSha256 = Hmac<Sha256>;

/// Empreinte one-way du PIN, scopée au slug. Sûre à exposer dans la valeur du
/// cookie (un cookie signé n'est pas chiffré — sa valeur est lisible).
fn fingerprint(secret: &[u8], slug: &str, pin: &str) -> String {
    // `new_from_slice` ne peut pas échouer pour HMAC (accepte toute longueur de clé ≥ 0) :
    // l'erreur `InvalidLength` n'est levée que pour des longueurs hors-spec, inapplicable ici.
    #[allow(clippy::expect_used)]
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC accepte toute clé");
    mac.update(slug.as_bytes());
    mac.update(b":");
    mac.update(pin.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Valeur du cookie : `"<exp_unix>:<fp_hex>"`.
pub fn issue_token(secret: &[u8], slug: &str, pin: &str, exp_unix: i64) -> String {
    format!("{exp_unix}:{}", fingerprint(secret, slug, pin))
}

/// `true` ssi le jeton est bien formé, non expiré (`now <= exp`), et son empreinte
/// correspond au PIN **courant** (comparaison à temps constant).
pub fn verify_token(secret: &[u8], slug: &str, pin: &str, token: &str, now_unix: i64) -> bool {
    let Some((exp_str, fp)) = token.split_once(':') else {
        return false;
    };
    let Ok(exp) = exp_str.parse::<i64>() else {
        return false;
    };
    if now_unix > exp {
        return false;
    }
    secure_compare(fp, &fingerprint(secret, slug, pin))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &[u8] = b"unit-test-secret-key-please-override-0123456789abcdef0123456789";

    #[test]
    fn valid_token_roundtrips() {
        let t = issue_token(SECRET, "demo-abc", "123456", 1000);
        assert!(verify_token(SECRET, "demo-abc", "123456", &t, 999));
    }

    #[test]
    fn rotated_pin_invalidates_token() {
        // Jeton émis sous l'ancien PIN ; le projet a roté vers un nouveau PIN.
        let t = issue_token(SECRET, "demo-abc", "123456", 1000);
        assert!(!verify_token(SECRET, "demo-abc", "654321", &t, 999));
    }

    #[test]
    fn expired_token_rejected() {
        let t = issue_token(SECRET, "demo-abc", "123456", 1000);
        assert!(!verify_token(SECRET, "demo-abc", "123456", &t, 1001));
    }

    #[test]
    fn tampered_fingerprint_rejected() {
        let t = issue_token(SECRET, "demo-abc", "123456", 1000);
        let tampered = format!("{}0", t); // fp altéré
        assert!(!verify_token(SECRET, "demo-abc", "123456", &tampered, 999));
    }

    #[test]
    fn malformed_token_rejected() {
        assert!(!verify_token(SECRET, "demo-abc", "123456", "garbage", 999));
        assert!(!verify_token(
            SECRET,
            "demo-abc",
            "123456",
            "notanint:abc",
            999
        ));
    }

    #[test]
    fn fingerprint_is_slug_scoped() {
        // Même PIN, slug différent → empreinte différente (un cookie ne vaut que pour son slug).
        let t = issue_token(SECRET, "demo-abc", "123456", 1000);
        assert!(!verify_token(SECRET, "autre-slug", "123456", &t, 999));
    }
}
