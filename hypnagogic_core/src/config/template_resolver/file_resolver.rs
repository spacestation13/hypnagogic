use core::default::Default;
use core::result::Result::{Err, Ok};
use std::fs;
use std::path::{Path, PathBuf};

use toml::Value;
use tracing::{debug, trace};

use crate::config::template_resolver::error::{TemplateError, TemplateResult};
use crate::config::template_resolver::TemplateResolver;

/// Loads templates from a folder on the filesystem.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FileResolver {
    path: PathBuf,
}

impl FileResolver {
    /// Creates a new `FileResolver` with the given path
    /// # Errors
    /// Returns an error if `path` does not exist.
    pub fn new(path: &Path) -> Result<Self, TemplateError> {
        let pathbuf = fs::canonicalize(path)
            .map_err(|_e| TemplateError::NoTemplateDir(path.to_path_buf()))?;
        Ok(FileResolver { path: pathbuf })
    }
}

impl Default for FileResolver {
    fn default() -> Self {
        FileResolver::new(Path::new("templates")).expect("templates folder does not exist")
    }
}

impl TemplateResolver for FileResolver {
    #[tracing::instrument(skip(input))]
    fn resolve(&self, input: &str) -> TemplateResult {
        let mut pathbuf = self.path.clone();
        pathbuf.push(Path::new(input));

        debug!(canon = ?pathbuf, "Full path parsed");

        let toml_path = pathbuf.with_extension("toml");

        pathbuf = if toml_path.exists() {
            toml_path
        } else {
            return Err(TemplateError::FailedToFindTemplate(
                input.to_string(),
                toml_path,
            ));
        };

        trace!("Found template at {:?}", pathbuf);

        let toml_string = fs::read_to_string(pathbuf.as_path())?;
        let deserialized: Value = toml::from_str(&toml_string)?;
        debug!(deserialized = ?deserialized, "Deserialized template");
        Ok(deserialized)
    }
}
