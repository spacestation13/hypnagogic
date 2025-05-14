use dmi::icon::IconState;
use image::{DynamicImage, GenericImageView};
use tracing::debug;
use crate::util::color::Color;

// Removes duplicate frames from the icon state's animation, if it has any
#[must_use]
pub fn dedupe_frames(icon_state: IconState) -> IconState {
    struct AccumulatedAnim {
        delays: Vec<f32>,
        frames: Vec<Vec<DynamicImage>>,
        current_frame: usize,
    }

    if icon_state.frames as usize <= 1 {
        return icon_state;
    }
    let Some(current_delays) = &icon_state.delay else {
        return icon_state;
    };


    // We're just gonna wrap these up into chunks so we can work on them in groups
    let delay_bucket = icon_state.images.chunks_exact(icon_state.dirs as usize)
        .map(|full_direction| full_direction.to_vec());
    // As we walk through the frames (chunks of pixels) in this icon state, we're going to keep track
    // of the ones that are duplicates, and "dedupe" them by simply adding extra
    // frame delay and removing the extra frame
    let deduped_anim = current_delays.into_iter().zip(delay_bucket).fold(
    AccumulatedAnim {
            delays: Vec::new(),
            // [[1, 1, 1, 1], [2, 2, 2, 2], ...]
            frames: Vec::new(),
            current_frame: 0,
        },
        |mut acc, (current_delay, images)| {
            if acc.current_frame != 0 && acc.frames[acc.current_frame - 1].iter().eq(images.iter())  {
                acc.delays[acc.current_frame - 1] += current_delay;
                return acc;
            }
            acc.delays.push(*current_delay);
            let count = images.len();
            acc.frames.push(images);
            acc.current_frame += 1;
            acc
        },
    );
    // now we just need to flatten out our chunks and we're good to go
    let fixed_frames = deduped_anim.frames.into_iter().flatten().collect::<Vec<DynamicImage>>();

    IconState {
        frames: deduped_anim.current_frame as u32,
        images: fixed_frames,
        delay: Some(deduped_anim.delays),
        ..icon_state
    }
}

#[must_use]
pub fn colors_in_image(image: &DynamicImage) -> Vec<Color> {
    let mut colors = Vec::new();
    for pixel in image.pixels() {
        let color = pixel.2;
        if !colors.contains(&color) {
            colors.push(color);
        }
    }
    colors
        .iter()
        .map(|c| Color::new(c.0[0], c.0[1], c.0[2], c.0[3]))
        .collect()
}

pub fn sort_colors_by_luminance(colors: &mut [Color]) {
    colors.sort_by(|a, b| a.luminance().partial_cmp(&b.luminance()).unwrap());
}

#[must_use]
pub fn pick_contrasting_colors(colors: &[Color]) -> (Color, Color) {
    let mut sorted_colors = colors.to_vec();
    sort_colors_by_luminance(&mut sorted_colors);
    let len_as_f32 = colors.len() as f32;
    let first = 0.10 * len_as_f32;
    let first_index = (first.floor() as usize).saturating_sub(1);
    let second = 0.90 * len_as_f32;
    let second_index = (second.floor() as usize).saturating_sub(1);
    (sorted_colors[first_index], sorted_colors[second_index])
}
