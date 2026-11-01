//! The models Watershed can download (`models.toml`, embedded in the binary).

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use serde::Deserialize;

use crate::bsroformer::LoadOptions;
use crate::download::{self, DownloadError, Expected};
use crate::{BurnSeparator, GgufSeparator, SeparationError, Separator};

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
    /// Present on models for the native backend, which `file` is a checkpoint for.
    /// The others are GGUF files.
    pub native: Option<NativeModel>,
}

/// What the native backend needs beside the checkpoint.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct NativeModel {
    pub architecture: Architecture,
    /// The checkpoint's `.yaml`.
    pub config: ConfigFile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Architecture {
    BsRoformer,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ConfigFile {
    pub file: String,
    pub size: u64,
    pub sha256: String,
    pub urls: Vec<String>,
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

    /// Whether the model's files are there with the right size. The hash is only
    /// checked when downloading: hashing 200 MB at every start would be slow.
    pub fn is_installed(&self, models_dir: &Path) -> bool {
        let has = |file: &str, size| {
            std::fs::metadata(models_dir.join(file)).is_ok_and(|m| m.len() == size)
        };
        has(&self.file, self.size)
            && self
                .native
                .as_ref()
                .is_none_or(|n| has(&n.config.file, n.config.size))
    }

    /// Downloads the model into `models_dir`, unless it is already installed, and
    /// returns its path. See [`download::download`] for resuming and verification.
    /// `progress` follows the main file: a config is a few KB.
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
        if let Some(native) = &self.native {
            let config = &native.config;
            let path = models_dir.join(&config.file);
            if !std::fs::metadata(&path).is_ok_and(|m| m.len() == config.size) {
                download::download(
                    &config.urls,
                    &path,
                    Expected {
                        size: config.size,
                        sha256: &config.sha256,
                    },
                    &|_, _| {},
                    cancel,
                )?;
            }
        }
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
    /// `options` only applies to GGUF models.
    pub fn load(
        &self,
        models_dir: &Path,
        options: &LoadOptions,
    ) -> Result<Box<dyn Separator>, SeparationError> {
        let path = self.path_in(models_dir);
        let stems = self.stems.clone();
        Ok(match &self.native {
            None => Box::new(GgufSeparator::load(&path, stems, options)?),
            Some(native) => {
                let Architecture::BsRoformer = native.architecture;
                let config = models_dir.join(&native.config.file);
                Box::new(BurnSeparator::load(&path, &config, stems)?)
            }
        })
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
        assert!(default_model().native.is_none());
        assert!(models.iter().any(|m| m.native.is_some()));
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
            if let Some(native) = &m.native {
                let config = &native.config;
                assert!(config.size > 0, "{}", m.id);
                assert_eq!(config.sha256.len(), 64, "{}", m.id);
                assert!(config.urls.iter().all(|u| u.starts_with("https://")));
                assert_ne!(config.file, m.file);
            }
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

    #[test]
    fn a_native_model_also_needs_its_config() {
        let dir = std::env::temp_dir().join(format!("watershed-native-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut model = models()
            .iter()
            .find(|m| m.native.is_some())
            .unwrap()
            .clone();
        model.size = 4;
        let config = &mut model.native.as_mut().unwrap().config;
        config.size = 2;
        let config_path = dir.join(&config.file);
        std::fs::write(model.path_in(&dir), b"abcd").unwrap();
        assert!(!model.is_installed(&dir));
        std::fs::write(config_path, b"ab").unwrap();
        assert!(model.is_installed(&dir));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
