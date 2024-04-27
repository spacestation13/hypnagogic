use thiserror::Error;
use user_error::UFE;

#[derive(Debug, Error)]
pub enum ProcessorError {
    #[error("Image Error")]
    ImageNotFound,
    #[error("DMI Error")]
    DMINotFound,
    #[error("Image Processing Error")]
    ImageError(#[from] image::error::ImageError),
    #[error("Restoration Error")]
    RestorationFailed(#[from] crate::operations::format_converter::error::RestrorationError),
    #[error("Generation Error")]
    GenerationFailed(#[from] crate::generation::error::GenerationError),
    #[error("Error within image config:\n{0}")]
    ConfigError(String),
}

pub type ProcessorResult<T> = Result<T, ProcessorError>;

impl UFE for ProcessorError {
    fn summary(&self) -> String {
        format!("{}", self)
    }

    fn reasons(&self) -> Option<Vec<String>> {
        match self {
            ProcessorError::ImageNotFound => {
                Some(vec!["This operation only accepts raw images".to_string()])
            }
            ProcessorError::DMINotFound => {
                Some(vec!["This operation only accepts DMIs".to_string()])
            }
            ProcessorError::ImageError(error) => Some(vec![format!("{}", error)]),
            ProcessorError::RestorationFailed(error) => error.reasons(),
            ProcessorError::GenerationFailed(error) => error.reasons(),
            ProcessorError::ConfigError(config) => Some(vec![format!("{}", config)]),
        }
    }

    fn helptext(&self) -> Option<String> {
        match self {
            ProcessorError::ImageNotFound => {
                Some(
                    "Check to make sure you're using the right type of image (png is a safe bet)"
                        .to_string(),
                )
            }
            ProcessorError::DMINotFound => {
                Some(
                    "Check to make sure you're using a dmi and not like a png or something"
                        .to_string(),
                )
            }
            ProcessorError::ImageError(_) => None,
            ProcessorError::RestorationFailed(error) => error.helptext(),
            ProcessorError::GenerationFailed(error) => error.helptext(),
            ProcessorError::ConfigError(_config) => {
                Some("TBH this needs to be its own error type".to_string())
            }
        }
    }
}
