use std::collections::HashMap;

use dmi::icon::IconState;
use image::{DynamicImage, GenericImage};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::config::blocks::cutters::StringMap;
use crate::operations::error::{ProcessorError, ProcessorResult};
use crate::operations::format_converter::error::{
    InconsistentDelay,
    InconsistentDirs,
    RestrorationError,
};
use crate::operations::{IconOperationConfig, InputIcon, OperationMode, ProcessorPayload};
use crate::util::adjacency::Adjacency;
use crate::util::delays::text_delays;
use crate::util::directions::DirectionStrategy;

#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct BitmaskSliceReconstruct {
    // List of icon states to extract
    pub extract: Vec<String>,
    // Map of state name -> state to insert as
    pub bespoke: Option<StringMap>,
    // Map of key -> value to set on the created config
    // Exists to let you set arbitrary values
    pub set: Option<StringMap>,
}

impl IconOperationConfig for BitmaskSliceReconstruct {
    #[tracing::instrument(skip(input))]
    fn perform_operation(
        &self,
        input: &InputIcon,
        mode: OperationMode,
    ) -> ProcessorResult<ProcessorPayload> {
        debug!("Starting bitmask slice reconstruction");
        let InputIcon::Dmi(icon) = input else {
            return Err(ProcessorError::DMINotFound);
        };

        // First, pull out icon states from DMI
        let states = icon.states.clone();

        let bespoke = match self.bespoke.as_ref() {
            Some(bespoke) => bespoke.clone(),
            None => StringMap::default(),
        };

        // Try and work out the output prefix by pulling from the first frame
        let mut problem_entries: Vec<String> = vec![];
        let output_prefix = states.first().and_then(|first_frame| {
            let mut split_up = first_frame.name.split('-');
            let first_entry = split_up.next();
            if split_up.next().is_none() {
                None
            } else {
                first_entry
            }
        });

        // Next, check if anything conflicts, if it does we'll error
        let frames_drop_prefix = states
            .clone()
            .into_iter()
            .map(|state| {
                let full_name = state.name.clone();
                let mut split_name = full_name.split('-');
                let prefix = split_name.next().unwrap_or_default();
                let suffix = split_name
                    .map(|elem| elem.to_string())
                    .reduce(|acc, elm| format!("{acc}-{elm}"));
                if suffix.is_some() && prefix != output_prefix.unwrap_or_default() {
                    problem_entries.push(full_name.clone());
                }
                (state, suffix.unwrap_or(prefix.to_string()))
            })
            .collect::<Vec<(IconState, String)>>();

        if let Some(troublesome_states) = problem_entries
            .into_iter()
            .reduce(|acc, elem| format!("{acc}, {elem}"))
        {
            return Err(ProcessorError::from(
                RestrorationError::InconsistentPrefixes(troublesome_states),
            ));
        }
        // Now, we remove the "core" frames, and dump them out
        let mut bespoke_found: Vec<String> = vec![];
        // Extract just the bits we care about
        let mut trimmed_frames = frames_drop_prefix
            .clone()
            .into_iter()
            .filter_map(|(mut state, suffix)| {
                state.name.clone_from(&suffix);
                if bespoke.get(suffix.as_str()).is_some() {
                    bespoke_found.push(suffix);
                    Some(state)
                } else if self.extract.contains(&suffix) {
                    Some(state)
                } else {
                    None
                }
            })
            .collect::<Vec<IconState>>();

        // Check for any states that aren't extracted and aren't entirely numbers
        // If we find any, error (cause we're dropping them here)
        let strings_caught = trimmed_frames
            .clone()
            .into_iter()
            .map(|state| state.name.clone())
            .collect::<Vec<String>>();
        let ignored_states = frames_drop_prefix
            .into_iter()
            .filter_map(|(_, suffix)| {
                if suffix.parse::<Adjacency>().is_ok()
                    || strings_caught.iter().any(|caught| *caught == suffix)
                {
                    None
                } else {
                    Some(format!("({suffix})"))
                }
            })
            .reduce(|acc, elem| {
                format! {"{acc}, {elem}"}
            });

        if let Some(missed_suffixes) = ignored_states {
            return Err(ProcessorError::from(RestrorationError::DroppedStates(
                missed_suffixes,
            )));
        }

        // Alright next we're gonna work out the order of our insertion into the png
        // based off the order of the extract/bespoke maps Extract first, then
        // bespoke
        let position_map = self
            .extract
            .clone()
            .into_iter()
            .chain(bespoke_found.clone().into_iter())
            .enumerate()
            .fold(HashMap::new(), |mut acc, (index, name)| {
                acc.insert(name, index);
                acc
            });

        // I don't like all these clones but position() mutates and I don't want that so
        // I'm not sure what else to do
        let get_pos = |search_for: &String| position_map.get(search_for);

        trimmed_frames.sort_by(|a, b| {
            let a_pos = get_pos(&a.name);
            let b_pos = get_pos(&b.name);
            a_pos.cmp(&b_pos)
        });

        let frame_count = trimmed_frames.len();
        let longest_frame = trimmed_frames
            .clone()
            .into_iter()
            .map(|state| state.frames)
            .max()
            .unwrap_or(1);
        let most_directions = trimmed_frames
            .clone()
            .into_iter()
            .map(|state| state.dirs)
            .max()
            .unwrap_or(1);

        // We now have a set of frames that we want to draw, ordered as requested
        // So all we gotta do is make that png
        // We assume all states have the same animation length,
        let mut output_image = DynamicImage::new_rgba8(
            icon.width * frame_count as u32 * most_directions as u32,
            icon.height * longest_frame,
        );
        let delays: Option<Vec<f32>> = trimmed_frames
            .iter()
            .filter_map(|elem| elem.delay.clone())
            .reduce(|acc, elem| if acc.len() > elem.len() { acc } else { elem });

        let delay_count = delays.clone().unwrap_or_default().iter().sum();
        let rewind = trimmed_frames
            .first()
            .and_then(|first_frame| Some(first_frame.rewind))
            .unwrap_or(false);

        let input_count: u32 = position_map.keys().len() as u32;
        let mut delay_problem_states: Vec<InconsistentDelay> = vec![];
        let mut dir_problem_states: Vec<InconsistentDirs> = vec![];
        for (x, state) in trimmed_frames.into_iter().enumerate() {
            if state.dirs != most_directions {
                dir_problem_states.push(InconsistentDirs {
                    state: state.name,
                    dirs: state.dirs,
                });
                continue;
            }
            let chunk_size = state.dirs as usize;

            // [1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3] -> [[1, 1, 1, 1], ...]
            // but also duplicate compacted frames to deal with bullshit
            let grouped_frames = if delays != state.delay && delays.is_some() {
                let frame_delays = state.delay.clone().unwrap_or(vec![delay_count]);
                let frame_count: f32 = frame_delays.iter().sum();
                if (delay_count - frame_count).abs() < 0.001 {
                    delay_problem_states.push(InconsistentDelay {
                        state: state.name,
                        delays: state.delay.unwrap_or_default(),
                    });
                    continue;
                }
                let real_delays = delays.as_ref().unwrap();
                // alright now we have to extend out our existing frames to account for the fact
                // that we may have dropped some
                let mut delay_index = 0;
                let mut new_frames = vec![];
                let mut broken_child = false;
                for (mut delay_to_process, frame) in frame_delays
                    .into_iter()
                    .zip(state.images.chunks(chunk_size))
                {
                    while delay_to_process > 0.0 && delay_index <= real_delays.len() {
                        delay_to_process -= real_delays[delay_index];
                        delay_index += 1;
                        new_frames.push(frame);
                    }
                    if delay_to_process.round() != 0.0 {
                        broken_child = true;
                        break;
                    }
                }
                if broken_child {
                    delay_problem_states.push(InconsistentDelay {
                        state: state.name,
                        delays: state.delay.unwrap_or_default(),
                    });
                    continue;
                }
                new_frames
            } else {
                state
                    .images
                    .chunks(chunk_size)
                    .collect::<Vec<&[DynamicImage]>>()
            };
            for (y, frame_directions) in grouped_frames.into_iter().enumerate() {
                // now we place!
                frame_directions.iter().cloned().enumerate().for_each(
                    |(direction_index, frame)| {
                        let direction_multiple = direction_index as u32;
                        debug!("{} {} {} {}", state.name, x, y, direction_multiple);
                        let x_pos =
                            (x as u32) * icon.width + direction_multiple * input_count * icon.width;
                        let y_pos = (y as u32) * icon.height;
                        output_image
                            .copy_from(&frame, x_pos, y_pos)
                            .unwrap_or_else(|_| {
                                panic!(
                                    "Failed to copy frame (bad dmi?): {} #{} {}",
                                    state.name, x_pos, y_pos
                                )
                            });
                    },
                );
            }
        }
        if !delay_problem_states.is_empty() {
            return Err(ProcessorError::from(
                RestrorationError::InconsistentDelays {
                    expected: delays.unwrap_or_default(),
                    problems: delay_problem_states,
                },
            ));
        }
        if !dir_problem_states.is_empty() {
            return Err(ProcessorError::from(RestrorationError::InconsistentDirs {
                expected: most_directions,
                problems: dir_problem_states,
            }));
        }

        let mut config: Vec<String> = vec![];
        if let Some(prefix_name) = output_prefix {
            config.push(format!("output_name = \"{prefix_name}\""));
        }
        if let Some(map) = &self.set {
            map.0.clone().into_iter().for_each(|entry| {
                config.push(format!("{} = {}", entry.0, entry.1));
            });
            config.push(String::new());
        }
        let strategy = DirectionStrategy::count_to_strategy(most_directions).unwrap();
        if strategy != DirectionStrategy::Standard {
            config.push(format!("direction_strategy = \"{strategy}\"").to_string());
            config.push(String::new());
        }
        let mut count = frame_count - bespoke_found.len();
        if let Some(map) = &self.bespoke {
            config.push("[prefabs]".to_string());
            map.0.clone().into_iter().for_each(|entry| {
                config.push(format!("{} = {}", entry.1, count));
                count += 1;
            });
            config.push(String::new());
        }
        if let Some(actual_delay) = delays {
            config.push("[animation]".to_string());
            config.push(format!("delays = {}", text_delays(&actual_delay, "")));
            if rewind {
                config.push(format!("rewind = {rewind}"));
            }
            config.push(String::new());
        }
        config.push("[icon_size]".to_string());
        config.push(format!("x = {}", icon.width));
        config.push(format!("y = {}", icon.height));
        config.push(String::new());
        config.push("[output_icon_size]".to_string());
        config.push(format!("x = {}", icon.width));
        config.push(format!("y = {}", icon.height));
        config.push(String::new());
        config.push("[cut_pos]".to_string());
        config.push(format!("x = {}", icon.width / 2));
        config.push(format!("y = {}", icon.height / 2));
        // Newline gang
        config.push(String::new());
        Ok(ProcessorPayload::wrap_png_config(
            ProcessorPayload::from_image(output_image),
            config.join("\n"),
        ))
    }

    fn verify_config(&self) -> ProcessorResult<()> {
        // TODO: Actual verification
        Ok(())
    }
}
