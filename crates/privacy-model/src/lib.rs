//! The model's artifact contract. Networking is opt-in and separate from inference.

use std::path::{Path, PathBuf};
use strum::{AsRefStr, EnumIter, IntoEnumIterator};

pub const DEFAULT_DIRECTORY: &str = "models/privacy-filter";

#[cfg(feature = "download")]
mod download;
#[cfg(feature = "download")]
pub use download::*;

/// Every consumer uses this type for the required files, names and pinned contents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, AsRefStr, EnumIter)]
pub enum ModelFile {
    #[strum(serialize = "config.json")]
    Config,
    #[strum(serialize = "tokenizer.json")]
    Tokenizer,
    #[strum(serialize = "model.safetensors")]
    Weights,
}

impl ModelFile {
    pub fn all() -> impl Iterator<Item = Self> {
        Self::iter()
    }

    pub fn path(self, directory: impl AsRef<Path>) -> PathBuf {
        directory.as_ref().join(self.as_ref())
    }

    pub fn artifact(self) -> Artifact {
        let (sha256, size) = match self {
            Self::Config => (
                "b2b26a4a4a000639ad30b0c264adbefe365bdb567fbd7bb27303b8c438375bd1",
                3039,
            ),
            Self::Tokenizer => (
                "0614fe83cadab421296e664e1f48f4261fa8fef6e03e63bb75c20f38e37d07d3",
                27868174,
            ),
            Self::Weights => (
                "06f66b87650b988b04e218285f9fe3df6a4943416b6ffa8171f07bc56cf12a9d",
                2798989498,
            ),
        };
        Artifact {
            file: self,
            sha256: sha256.into(),
            size,
        }
    }
}

/// Contents expected for a model file; alternate releases can supply their own descriptors.
#[derive(Debug, Clone)]
pub struct Artifact {
    pub file: ModelFile,
    pub sha256: String,
    pub size: u64,
}
