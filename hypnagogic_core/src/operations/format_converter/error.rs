use thiserror::Error;
use user_error::UFE;

use crate::util::delays::text_delays;

#[derive(Debug)]
pub struct InconsistentDelay {
    pub state: String,
    pub delays: Vec<f32>,
}

#[derive(Debug, Error)]
pub enum RestrorationError {
    #[error("Inconsistent Prefixes")]
    InconsistentPrefixes(String),
    #[error("Dropped States")]
    DroppedStates(String),
    #[error("Inconsistent Delays")]
    InconsistentDelays {
        expected: Vec<f32>,
        problems: Vec<InconsistentDelay>,
    },
}

impl UFE for RestrorationError {
    fn summary(&self) -> String {
        format!("{self}")
    }

    fn reasons(&self) -> Option<Vec<String>> {
        match self {
            RestrorationError::InconsistentPrefixes(reason) => {
                Some(vec![format!(
                    "The following icon states are named with inconsistent prefixes (with the \
                     rest of the file) [{reason}]"
                )])
            }
            RestrorationError::DroppedStates(states) => {
                Some(vec![format!(
                    "Restoration would fail to properly capture the following icon states: \
                     [{states}]"
                )])
            }
            RestrorationError::InconsistentDelays { expected, problems } => {
                let mut hand_back: Vec<String> = vec![];
                hand_back.push(format!(
                    "The default strings are {}",
                    text_delays(expected, "ds")
                ));
                for problem in problems {
                    hand_back.push(format!(
                        "Icon state {}'s delays {} do not match",
                        problem.state,
                        text_delays(&problem.delays, "ds")
                    ));
                }
                Some(hand_back)
            }
        }
    }

    fn helptext(&self) -> Option<String> {
        match self {
            RestrorationError::InconsistentPrefixes(_) => {
                Some("Make sure you don't have two sets of cut icons in one file".to_string())
            }
            RestrorationError::DroppedStates(_) => {
                Some(
                    "You likely have a set of basically \"additional\" uncut icon states. \
                     Consider moving them to their own dmi"
                        .to_string(),
                )
            }
            RestrorationError::InconsistentDelays {
                expected: _,
                problems: _,
            } => {
                Some(
                    "Did someone make these by hand? You may need to just go through and set them \
                     to be consistent"
                        .to_string(),
                )
            }
        }
    }
}
