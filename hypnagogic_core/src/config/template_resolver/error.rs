use std::path::PathBuf;

use thiserror::Error;
use toml::Value;

#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("Template dir not found while creating FileResolver {0}")]
    NoTemplateDir(PathBuf),
    #[error("Failed to find template: `{0}`, expected `{1}`")]
    FailedToFindTemplate(String, PathBuf),
    #[error("Generic toml parse error while resolving template: {0}")]
    TOMLError(#[from] toml::de::Error),
    #[error("Generic IO Error when attempting to resolve template: {0}")]
    IOError(#[from] std::io::Error),
}

pub type TemplateResult = Result<Value, TemplateError>;
