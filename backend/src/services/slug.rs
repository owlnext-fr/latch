//! Génération de slug : base lisible dérivée du nom + suffixe aléatoire.
//! Pur (aucune I/O, aucune DB). Le suffixe (8 base62 ≈ 47 bits) est la part
//! quasi non-énumérable du slug — décision actée 2026-06-24 (QUIRKS).

use rand::Rng;

const SUFFIX_LEN: usize = 8;
const BASE62: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

/// Base lisible : minuscules, ascii alphanumérique, tirets simples, sans tiret
/// en bordure. Fallback `"projet"` si rien d'exploitable.
pub fn slugify_base(name: &str) -> String {
    let mut out = String::new();
    let mut pending_sep = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            if pending_sep && !out.is_empty() {
                out.push('-');
            }
            out.push(c.to_ascii_lowercase());
            pending_sep = false;
        } else if c.is_ascii() && !c.is_ascii_alphanumeric() && !out.is_empty() {
            // ASCII non-alphanumeric (space, punctuation, etc.) marks separator
            pending_sep = true;
        }
        // Non-ASCII characters are silently dropped, don't set pending_sep
    }
    if out.is_empty() {
        out.push_str("projet");
    }
    out
}

/// Suffixe aléatoire de 8 caractères base62.
pub fn random_suffix() -> String {
    let mut rng = rand::thread_rng();
    (0..SUFFIX_LEN)
        .map(|_| BASE62[rng.gen_range(0..BASE62.len())] as char)
        .collect()
}

/// Slug complet : `{base}-{suffixe}`.
pub fn generate_slug(name: &str) -> String {
    format!("{}-{}", slugify_base(name), random_suffix())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugifies_spaces_and_case() {
        assert_eq!(slugify_base("Mon Projet"), "mon-projet");
    }

    #[test]
    fn collapses_and_trims_separators() {
        assert_eq!(slugify_base("  Hello!!  World  "), "hello-world");
    }

    #[test]
    fn drops_non_ascii() {
        // Les accents (non-ascii) sont retirés : la base est cosmétique,
        // l'unicité vient du suffixe. (Deburr = backlog.)
        assert_eq!(slugify_base("Café Déjà"), "caf-dj");
    }

    #[test]
    fn empty_name_falls_back() {
        assert_eq!(slugify_base("***"), "projet");
        assert_eq!(slugify_base(""), "projet");
    }

    #[test]
    fn suffix_is_8_base62_chars() {
        let s = random_suffix();
        assert_eq!(s.len(), 8);
        assert!(s.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn suffix_varies() {
        // Collision sur 62^8 ≈ 2e14 : pratiquement impossible.
        assert_ne!(random_suffix(), random_suffix());
    }

    #[test]
    fn generate_slug_combines_base_and_suffix() {
        let slug = generate_slug("Mon Projet");
        let (base, suffix) = slug.rsplit_once('-').unwrap();
        assert_eq!(base, "mon-projet");
        assert_eq!(suffix.len(), 8);
    }
}
