//! Adaptateur sortant "fichiers" (contrat §1). Le HTML des versions vit dans
//! le volume. `write` est atomique (tmp → rename) pour ne jamais exposer un
//! fichier à moitié écrit (contrat §8). Injectable : le cœur dépend du trait,
//! les tests utilisent un tempdir, jamais le disque de prod.

use std::path::PathBuf;

use async_trait::async_trait;
use tokio::fs;

use crate::services::errors::CoreError;

#[async_trait]
pub trait Storage: Send + Sync {
    /// Écrit `contents` à `rel_path` (relatif à la racine), en créant les
    /// dossiers parents. Atomique : écrit un `.tmp` puis `rename` en place.
    async fn write(&self, rel_path: &str, contents: &[u8]) -> Result<(), CoreError>;

    /// Lit le contenu UTF-8 à `rel_path`. `CoreError::NotFound` si absent.
    async fn read(&self, rel_path: &str) -> Result<String, CoreError>;

    /// Supprime le fichier à `rel_path`. **Idempotent** : un fichier déjà absent
    /// n'est PAS une erreur — le nettoyage d'un orphelin doit pouvoir se rejouer.
    async fn delete(&self, rel_path: &str) -> Result<(), CoreError>;
}

/// Implémentation sur système de fichiers, ancrée à `root` (le volume).
pub struct FsStorage {
    root: PathBuf,
}

impl FsStorage {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

#[async_trait]
impl Storage for FsStorage {
    async fn write(&self, rel_path: &str, contents: &[u8]) -> Result<(), CoreError> {
        let dest = self.root.join(rel_path);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).await?;
        }
        // Nom de tmp unique : appendé au nom de fichier complet (ne remplace pas
        // l'extension) + jeton aléatoire, pour que deux écritures concurrentes vers
        // le même chemin n'entrent jamais en collision sur le tmp.
        let file_name = dest
            .file_name()
            .ok_or_else(|| CoreError::Validation("invalid storage path".to_string()))?
            .to_string_lossy();
        let tmp = dest.with_file_name(format!("{file_name}.{:016x}.tmp", rand::random::<u64>()));
        fs::write(&tmp, contents).await?;
        fs::rename(&tmp, &dest).await?;
        Ok(())
    }

    async fn read(&self, rel_path: &str) -> Result<String, CoreError> {
        let dest = self.root.join(rel_path);
        match fs::read_to_string(&dest).await {
            Ok(s) => Ok(s),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(CoreError::NotFound),
            Err(e) => Err(CoreError::Io(e)),
        }
    }

    async fn delete(&self, rel_path: &str) -> Result<(), CoreError> {
        let dest = self.root.join(rel_path);
        match fs::remove_file(&dest).await {
            Ok(()) => Ok(()),
            // Idempotence : absent = déjà dans l'état voulu.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(CoreError::Io(e)),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn write_then_read_roundtrip() {
        let dir = tempdir().unwrap();
        let storage = FsStorage::new(dir.path().to_path_buf());

        storage.write("42/1.html", b"<h1>hi</h1>").await.unwrap();
        let got = storage.read("42/1.html").await.unwrap();
        assert_eq!(got, "<h1>hi</h1>");
    }

    #[tokio::test]
    async fn write_creates_nested_dirs() {
        let dir = tempdir().unwrap();
        let storage = FsStorage::new(dir.path().to_path_buf());
        // le sous-dossier "7" n'existe pas encore
        storage.write("7/3.html", b"x").await.unwrap();
        assert!(dir.path().join("7/3.html").exists());
    }

    #[tokio::test]
    async fn read_missing_is_not_found() {
        let dir = tempdir().unwrap();
        let storage = FsStorage::new(dir.path().to_path_buf());
        let err = storage.read("nope.html").await.unwrap_err();
        assert!(matches!(err, CoreError::NotFound));
    }

    #[tokio::test]
    async fn delete_removes_existing_file() {
        let dir = tempdir().unwrap();
        let storage = FsStorage::new(dir.path().to_path_buf());
        storage.write("5/2.html", b"bye").await.unwrap();
        assert!(dir.path().join("5/2.html").exists());

        storage.delete("5/2.html").await.unwrap();
        assert!(!dir.path().join("5/2.html").exists());
    }

    #[tokio::test]
    async fn delete_absent_is_idempotent() {
        let dir = tempdir().unwrap();
        let storage = FsStorage::new(dir.path().to_path_buf());
        // Supprimer un fichier jamais écrit ne doit pas être une erreur.
        storage.delete("never/existed.html").await.unwrap();
    }

    #[tokio::test]
    async fn write_overwrites_atomically() {
        let dir = tempdir().unwrap();
        let storage = FsStorage::new(dir.path().to_path_buf());
        storage.write("1/1.html", b"old").await.unwrap();
        storage.write("1/1.html", b"new").await.unwrap();
        assert_eq!(storage.read("1/1.html").await.unwrap(), "new");
        // aucun fichier .tmp résiduel dans le dossier du projet
        let leftovers: Vec<_> = std::fs::read_dir(dir.path().join("1"))
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("tmp"))
            .collect();
        assert!(leftovers.is_empty(), "tmp résiduel: {leftovers:?}");
    }
}
