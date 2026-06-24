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
        let tmp = dest.with_extension("tmp");
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
}

#[cfg(test)]
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
    async fn write_overwrites_atomically() {
        let dir = tempdir().unwrap();
        let storage = FsStorage::new(dir.path().to_path_buf());
        storage.write("1/1.html", b"old").await.unwrap();
        storage.write("1/1.html", b"new").await.unwrap();
        assert_eq!(storage.read("1/1.html").await.unwrap(), "new");
        // pas de fichier .tmp résiduel
        assert!(!dir.path().join("1/1.tmp").exists());
    }
}
