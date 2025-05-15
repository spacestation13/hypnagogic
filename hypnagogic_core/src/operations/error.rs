use thiserror::Error;
use user_error::UFE;

#[derive(Debug, Error)]
pub enum ProcessorError {
    #[error("Image Error")]
    ImageNotFound,
    #[error("DMI Error")]
    DMINotFound,
    #[error("Image Width Off By One Error")]
    ImageWidthOffByOne(u32, u32, u32, u32),
    #[error("Image Width Direction Error")]
    ImageWidthOffByDirection(u32, u32, u32, u32),
    #[error("Image Width Error")]
    ImproperImageWidth(u32, u32),
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
        format!("{self}")
    }

    fn reasons(&self) -> Option<Vec<String>> {
        match self {
            ProcessorError::ImageNotFound => {
                Some(vec!["This operation only accepts raw images".to_string()])
            }
            ProcessorError::DMINotFound => {
                Some(vec!["This operation only accepts DMIs".to_string()])
            }
            ProcessorError::ImageWidthOffByOne(
                expected,
                reality,
                expected_input,
                reality_input,
            ) => {
                Some(vec![format!(
                    "Expected a width of {expected}px, recieved a width of {reality}px. Expected \
                     {expected_input} inputs per direction, recieved {reality_input} "
                )])
            }
            ProcessorError::ImageWidthOffByDirection(
                expected,
                reality,
                expected_dir_count,
                dir_count,
            ) => {
                Some(vec![format!(
                    "Expected a width of {expected}px, recieved a width of {reality}px. Expected \
                     enough width for {expected_dir_count} directions, found {dir_count}"
                )])
            }
            ProcessorError::ImproperImageWidth(expected, reality) => {
                Some(vec![format!(
                    "Expected a width of {expected}px, recieved a width of {reality}px"
                )])
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
            ProcessorError::ImageWidthOffByOne(..) => {
                Some(
                    "Have you miscounted the amount of inputs you need? Remember it's 4 for \
                     cardinals, 5 for diagonals, and 1 extra for each prefab."
                        .to_string(),
                )
            }
            ProcessorError::ImageWidthOffByDirection(_, _, expected_dir_count, dir_count) => {
                if expected_dir_count > dir_count {
                    Some(
                        "Have you forgotten to add a set of inputs for some of your dirs?"
                            .to_string(),
                    )
                } else {
                    Some("Are you using the wrong direction strategy?".to_string())
                }
            }
            ProcessorError::ImproperImageWidth(..) => {
                Some("Have you made the image slightly the wrong width?".to_string())
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
