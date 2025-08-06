use dmi::icon::{Icon, IconState};
use enum_iterator::all;
use fixed_map::Map;
use image::{imageops, DynamicImage, GenericImageView};
use serde::{Deserialize, Serialize};
use tracing::trace;

use crate::config::blocks::cutters::SlicePoint;
use crate::generation::icon::generate_map_icon;
use crate::operations::cutters::bitmask_slice::{
    BitmaskSlice,
    SideSpacing,
};
use crate::operations::error::{ProcessorError, ProcessorResult};
use crate::operations::{
    IconOperationConfig,
    InputIcon,
    NamedIcon,
    OperationMode,
    ProcessorPayload,
};
use crate::util::adjacency::Adjacency;
use crate::util::corners::{Corner, Side};
use crate::util::directions::{Direction, DirectionStrategy};
use crate::util::icon_ops::dedupe_frames;
use crate::util::repeat_for;

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct BitmaskDirectionalVis {
    #[serde(flatten)]
    pub bitmask_slice_config: BitmaskSlice,
    pub slice_point: SlicePoint,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub mask_color: Option<String>,
}

impl IconOperationConfig for BitmaskDirectionalVis {
    fn perform_operation(
        &self,
        input: &InputIcon,
        mode: OperationMode,
    ) -> ProcessorResult<ProcessorPayload> {
        let InputIcon::DynamicImage(img) = input else {
            return Err(ProcessorError::ImageNotFound);
        };
        let (corners, prefabs) = self.bitmask_slice_config.generate_corners(img)?;

        let (_in_x, in_y) = img.dimensions();
        let num_frames = in_y / self.bitmask_slice_config.icon_size.y;

        let possible_adjacencies = self.bitmask_slice_config.output_type.output_adjacencies();

        let output_directions = self.bitmask_slice_config.direction_strategy.output_vec();
        let dir_count = output_directions.len() as u8;
        let assembled_map = self.bitmask_slice_config.generate_icons(
            &corners,
            &prefabs,
            num_frames,
            &possible_adjacencies,
        );

        let delay: Option<Vec<f32>> = self
            .bitmask_slice_config
            .animation
            .clone()
            .map(|x| repeat_for(&x.delays, num_frames as usize));
        let rewind = self
            .bitmask_slice_config
            .animation
            .as_ref()
            .and_then(|animation| animation.rewind)
            .unwrap_or(false);

        let mut icon_states = vec![];

        let states_to_gen = possible_adjacencies.into_iter().filter(Adjacency::ref_has_no_orphaned_corner);

        for adjacency in states_to_gen {
            for side in Side::dmi_cardinals() {
                let mut icon_state_frames = vec![];
                let slice_info = self.get_side_cuts(side);

                let (x, y, width, height) = if side.is_vertical() {
                    (
                        0,
                        slice_info.start,
                        self.bitmask_slice_config.icon_size.x,
                        slice_info.step(),
                    )
                } else {
                    (
                        slice_info.start,
                        0,
                        slice_info.step(),
                        self.bitmask_slice_config.icon_size.y,
                    )
                };

                for direction in &output_directions {
                    let images = match self.bitmask_slice_config.direction_strategy {
                        DirectionStrategy::CardinalsRotated => {
                            let rotated_sig: Adjacency = adjacency.rotate_to(*direction);
                            trace!(sig = ?direction, rotated_sig = ?rotated_sig, "Rotated");
                            assembled_map
                                .get(Direction::STANDARD)
                                .unwrap()
                                .get(&adjacency)
                                .unwrap()
                        }
                        _ => {
                            assembled_map
                                .get(*direction)
                                .unwrap()
                                .get(&adjacency)
                                .unwrap()
                        }
                    };
                    for image in images {
                        let mut cut_img = DynamicImage::new_rgba8(
                            self.bitmask_slice_config.icon_size.x,
                            self.bitmask_slice_config.icon_size.y,
                        );

                        let crop = image.crop_imm(x, y, width, height);

                        imageops::overlay(&mut cut_img, &crop, x as i64, y as i64);
                        icon_state_frames.push(cut_img);
                    }
                }
                icon_states.push(dedupe_frames(IconState {
                    name: format!("{}-{}", adjacency.pretty_print(), side.byond_dir()),

                    dirs: dir_count,
                    frames: num_frames,
                    images: icon_state_frames,
                    delay: delay.clone(),
                    rewind,
                    ..Default::default()
                }));
            }
        }

        let convex_images = self
            .bitmask_slice_config
            .direction_strategy
            .output_vec()
            .iter()
            .fold(Map::new(), |mut acc, direction| {
                let input_dir = match self.bitmask_slice_config.direction_strategy {
                    // Rotation doesn't DO anything to cardinals, we just need to ensure we only
                    // PULL using the standard dir here
                    DirectionStrategy::CardinalsRotated => Direction::STANDARD,
                    _ => *direction,
                };
                let just_cardinals = assembled_map
                    .get(input_dir)
                    .unwrap()
                    .get(&Adjacency::CARDINALS)
                    .unwrap();
                acc.insert(*direction, just_cardinals);
                acc
            });
        for corner in all::<Corner>() {
            let mut icon_state_frames = vec![];

            let (horizontal, vertical) = corner.sides_of_corner();

            let horizontal_side_info = self.bitmask_slice_config.get_side_info(horizontal);
            let x = horizontal_side_info.start;
            let width = horizontal_side_info.step();

            // todo: This is awful, maybe a better way to do this?
            let (y, height) = if vertical == Side::North {
                (0, self.slice_point.get(vertical).unwrap())
            } else {
                let slice_point = self.slice_point.get(vertical).unwrap();
                let end = self.bitmask_slice_config.icon_size.y;
                (slice_point, end - slice_point)
            };

            for direction in &output_directions {
                let images = *convex_images.get(*direction).unwrap();
                for image in images {
                    let mut cut_img = DynamicImage::new_rgba8(
                        self.bitmask_slice_config.icon_size.x,
                        self.bitmask_slice_config.icon_size.y,
                    );

                    let crop_img = image.crop_imm(x, y, width, height);

                    imageops::overlay(&mut cut_img, &crop_img, x as i64, y as i64);
                    icon_state_frames.push(cut_img);
                }
            }

            icon_states.push(dedupe_frames(IconState {
                name: format!("innercorner-{}", corner.byond_dir()),
                dirs: dir_count,
                frames: num_frames,
                images: icon_state_frames,
                delay: delay.clone(),
                rewind,

                ..Default::default()
            }));
        }

        if let Some(map_icon) = &self.bitmask_slice_config.map_icon {
            let icon = generate_map_icon(
                self.bitmask_slice_config.output_icon_size.x,
                self.bitmask_slice_config.output_icon_size.y,
                map_icon,
            )?;
            icon_states.push(IconState {
                name: map_icon.icon_state_name.clone(),
                dirs: 1,
                frames: 1,
                images: vec![icon],
                ..Default::default()
            });
        }

        let out_icon = Icon {
            version: dmi::icon::DmiVersion::default(),
            width: self.bitmask_slice_config.output_icon_size.x,
            height: self.bitmask_slice_config.output_icon_size.y,
            states: icon_states,
        };

        if mode == OperationMode::Debug {
            let mut out = self.bitmask_slice_config.generate_debug_icons(&corners);

            out.push(NamedIcon::from_icon(out_icon));
            Ok(ProcessorPayload::MultipleNamed(out))
        } else {
            Ok(ProcessorPayload::from_icon(out_icon))
        }
    }

    fn verify_config(&self) -> ProcessorResult<()> {
        // TODO: actually verify config
        Ok(())
    }
}

impl BitmaskDirectionalVis {
    /// Gets the side cutter info for a given side based on the slice point
    /// # Panics
    /// Can panic if the `slice_point` map is unpopulated, which shouldn't
    /// happen if initialized correctly Generally indicates a bad
    /// implementation of `BitmaskDirectionalVis`
    #[must_use]
    pub fn get_side_cuts(&self, side: Side) -> SideSpacing {
        match side {
            Side::North => {
                SideSpacing {
                    start: 0,
                    end: self.slice_point.get(Side::North).unwrap(),
                }
            }
            Side::South => {
                SideSpacing {
                    start: self.slice_point.get(Side::South).unwrap(),
                    end: self.bitmask_slice_config.icon_size.y,
                }
            }
            Side::East => {
                SideSpacing {
                    start: self.slice_point.get(Side::East).unwrap(),
                    end: self.bitmask_slice_config.icon_size.x,
                }
            }
            Side::West => {
                SideSpacing {
                    start: 0,
                    end: self.slice_point.get(Side::West).unwrap(),
                }
            }
        }
    }
}
