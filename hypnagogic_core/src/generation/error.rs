use thiserror::Error;
use user_error::UFE;

#[derive(Debug, Error)]
pub enum GenerationError {
    #[error("Text Length Error")]
    TextTooLong(String, u32, u32),
    #[error("Text has too many lines: {0}; max lines for size is {1}")]
    TooManyLines(String, u32, u32),
}

impl UFE for GenerationError {
    fn summary(&self) -> String {
       format!("{}", self)
   }

   fn reasons(&self) -> Option<Vec<String>> {
       match self {
            GenerationError::TextTooLong(text, length, max) => Some(vec![format!("Text ({text}) is tooo long to render ({length} pixels wide), max length for this size is around {max}")]),
            GenerationError::TooManyLines(text, height, max) => Some(vec![format!("Text ({text}) has too many lines ({height} pixels tall), max height for this size is around {max}")]),
       }
   }

   fn helptext(&self) -> Option<String> {
       match self {
            GenerationError::TextTooLong(_, _, _) => Some("Try reducing the length of the text (no duh)".to_string()),
            GenerationError::TooManyLines(_, _, _) => Some("Consider using LESS newlines".to_string()),
       }
   }
}

