//! The models Watershed can download (`models.toml`, embedded in the binary).

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use serde::Deserialize;

use crate::bsroformer::LoadOptions;
use crate::download::{self, DownloadError, Expected};
use crate::{GgufSeparator, SeparationError};

#[derive(Debug, Deserialize)]
struct CatalogFile {
    model: Vec<ModelEntry>,
}

static CATALOG: LazyLock<Vec<ModelEntry>> = LazyLock::new(|| {
    toml::from_str::<CatalogFile>(include_str!("../models.toml"))
        .expect("models.toml is checked by the tests")
        .model
});

/// One downloadable model. See `models.toml` for what each field means.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ModelEntry {
    pub id: String,
    pub name: String,
    pub file: String,
    pub size: u64,
    pub sha256: String,
    pub urls: Vec<String>,
    pub stems: Vec<String>,
    pub license: String,
    #[serde(default)]
    pub default: bool,
}

/// All models, in catalog order.
pub fn models() -> &'static [ModelEntry] {
    &CATALOG
}

pub fn find(id: &str) -> Option<&'static ModelEntry> {
    CATALOG.iter().find(|m| m.id == id)
}

/// The model used unless the user picks another.
pub fn default_model() -> &'static ModelEntry {
    CATALOG
        .iter()
        .find(|m| m.default)
        .expect("models.toml has a default model")
}

impl ModelEntry {
    /// Where this model lives inside the models folder.
    pub fn path_in(&self, models_dir: &Path) -> PathBuf {
        models_dir.join(&self.file)
    }

    /// Whether the model file is there with the right size. The hash is only
    /// checked when downloading: hashing 200 MB at every start would be slow.
    pub fn is_installed(&self, models_dir: &Path) -> bool {
        std::fs::metadata(self.path_in(models_dir)).is_ok_and(|m| m.len() == self.size)
    }

    /// Downloads the model into `models_dir`, unless it is already installed, and
    /// returns its path. See [`download::download`] for resuming and verification.
    pub fn install(
        &self,
        models_dir: &Path,
        progress: &dyn Fn(u64, u64),
        cancel: &dyn Fn() -> bool,
    ) -> Result<PathBuf, DownloadError> {
        let path = self.path_in(models_dir);
        if self.is_installed(models_dir) {
            return Ok(path);
        }
        std::fs::create_dir_all(models_dir).map_err(|source| DownloadError::Io {
            path: models_dir.to_path_buf(),
            source,
        })?;
        download::download(
            &self.urls,
            &path,
            Expected {
                size: self.size,
                sha256: &self.sha256,
            },
            progress,
            cancel,
        )?;
        Ok(path)
    }

    /// Loads the installed model as a separator with this entry's stem names.
    pub fn load(
        &self,
        models_dir: &Path,
        options: &LoadOptions,
    ) -> Result<GgufSeparator, SeparationError> {
        GgufSeparator::load(&self.path_in(models_dir), self.stems.clone(), options)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn catalog_is_valid() {
        let models = models();
        assert!(!models.is_empty());
        assert_eq!(models.iter().filter(|m| m.default).count(), 1);
        let unique =
            |values: Vec<&str>| values.iter().collect::<HashSet<_>>().len() == values.len();
        assert!(unique(models.iter().map(|m| m.id.as_str()).collect()));
        assert!(unique(models.iter().map(|m| m.file.as_str()).collect()));
        for m in models {
            assert!(m.size > 0, "{}", m.id);
            assert_eq!(m.sha256.len(), 64, "{}", m.id);
            assert!(
                m.sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
                "{}: sha256 must be lower-case hex",
                m.id
            );
            assert!(!m.urls.is_empty(), "{}", m.id);
            assert!(m.urls.iter().all(|u| u.starts_with("https://")), "{}", m.id);
            assert!(!m.stems.is_empty() && unique(m.stems.iter().map(String::as_str).collect()));
            // Stem names become file names.
            assert!(
                m.stems
                    .iter()
                    .all(|s| s.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')),
                "{}",
                m.id
            );
            assert_eq!(find(&m.id), Some(m));
        }
    }

    #[test]
    fn installed_means_the_right_size() {
        let dir = std::env::temp_dir().join(format!("watershed-catalog-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let model = ModelEntry {
            size: 4,
            ..default_model().clone()
        };
        assert!(!model.is_installed(&dir));
        std::fs::write(model.path_in(&dir), b"abc").unwrap();
        assert!(!model.is_installed(&dir));
        std::fs::write(model.path_in(&dir), b"abcd").unwrap();
        assert!(model.is_installed(&dir));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
