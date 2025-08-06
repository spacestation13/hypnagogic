use std::collections::{BTreeMap, HashMap};

use dmi::icon::{Icon, IconState};
use enum_iterator::all;
use fixed_map::Map;
use image::{imageops, DynamicImage, GenericImageView};
use serde::{Deserialize, Serialize};
use tracing::{debug, trace};

use crate::config::blocks::cutters::{
    Animation,
    CutPosition,
    IconSize,
    OutputIconPosition,
    OutputIconSize,
    Positions,
    PrefabOverlays,
    Prefabs,
};
use crate::config::blocks::generators::MapIcon;
use crate::generation::icon::generate_map_icon;
use crate::operations::error::{ProcessorError, ProcessorResult};
use crate::operations::{
    IconOperationConfig,
    InputIcon,
    NamedIcon,
    OperationMode,
    OutputImage,
    ProcessorPayload,
};
use crate::util::adjacency::Adjacency;
use crate::util::corners::{Corner, CornerSet, CornerType, Side};
use crate::util::directions::{Direction, DirectionStrategy};
use crate::util::icon_ops::dedupe_frames;
use crate::util::repeat_for;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct SideSpacing {
    pub start: u32,
    pub end: u32,
}

impl SideSpacing {
    #[must_use]
    pub fn step(self) -> u32 {
        self.end - self.start
    }
}

#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct BitmaskSlice {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub output_name: Option<String>,
    pub output_type: CornerSet,
    #[serde(default)]
    pub direction_strategy: DirectionStrategy,
    pub icon_size: IconSize,
    pub output_icon_pos: OutputIconPosition,
    pub output_icon_size: OutputIconSize,
    pub positions: Positions,
    pub cut_pos: CutPosition,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub animation: Option<Animation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub prefabs: Option<Prefabs>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub prefab_overlays: Option<PrefabOverlays>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub map_icon: Option<MapIcon>,
}

impl IconOperationConfig for BitmaskSlice {
    #[tracing::instrument(skip(input))]
    fn perform_operation(
        &self,
        input: &InputIcon,
        mode: OperationMode,
    ) -> ProcessorResult<ProcessorPayload> {
        debug!("Starting bitmask slice icon op");
        let InputIcon::DynamicImage(img) = input else {
            return Err(ProcessorError::ImageNotFound);
        };
        let (in_x, in_y) = img.dimensions();
        let num_frames = in_y / self.icon_size.y;

        // I want to shitcheck our image against how large we KNOW it has to be
        let position_count = self.positions.count() as u32;
        let direction_count = self.direction_strategy.input_vec().len() as u32;
        let prefab_count = if let Some(prefab) = &self.prefabs {
            prefab.0.values().count() as u32
        } else {
            0
        };
        let expected_width = direction_count * (position_count + prefab_count) * self.icon_size.x;
        let actual_width = in_x;
        if expected_width != actual_width {
            // If we're 1 slot off, then it's either prefabs or an extra position
            if expected_width.abs_diff(actual_width) == direction_count * self.icon_size.x {
                let expected_inputs = expected_width / self.icon_size.x;
                let reality_inputs = actual_width / self.icon_size.x;
                return Err(ProcessorError::ImageWidthOffByOne(
                    expected_width,
                    actual_width,
                    expected_inputs,
                    reality_inputs,
                ));
            }
            // Otherwise, if we're a multiple of the direction count off then the source is
            // pretty obvious
            let direction_width = (expected_width / direction_count) as f32;
            if (actual_width as f32 / direction_width).fract().abs() < 0.001 {
                let actual_direction =
                    actual_width / ((position_count + prefab_count) * self.icon_size.x);
                return Err(ProcessorError::ImageWidthOffByDirection(
                    expected_width,
                    actual_width,
                    direction_count,
                    actual_direction,
                ));
            }
            // If not, it's off by a small value and we can't deduce the cause
            return Err(ProcessorError::ImproperImageWidth(
                expected_width,
                actual_width,
            ));
        }

        let (corners, prefabs) = self.generate_corners(img)?;

        let possible_adjacencies = self.output_type.output_adjacencies();

        // First phase: generate icons
        let assembled_map =
            self.generate_icons(&corners, &prefabs, num_frames, &possible_adjacencies);

        // Second phase: map to byond icon states and produce dirs if need
        // Even though this is the same loop as what happens in generate_icons,
        // all states need to be generated first for the
        // Rotation to work correctly, so it must be done as a second loop.
        let mut icon_states = vec![];

        let output_directions = self.direction_strategy.output_vec();
        let dir_count = output_directions.len() as u8;

        let delay = self
            .animation
            .clone()
            .map(|x| repeat_for(&x.delays, num_frames as usize));
        let rewind = self
            .animation
            .as_ref()
            .and_then(|animation| animation.rewind)
            .unwrap_or(false);

        let states_to_gen = possible_adjacencies
            .into_iter()
            .filter(Adjacency::ref_has_no_orphaned_corner);
        for adjacency in states_to_gen {
            let mut animated_blocks = vec![vec![]; num_frames as usize];
            for direction in &output_directions {
                let next_frame = match self.direction_strategy {
                    DirectionStrategy::CardinalsRotated => {
                        let rotated_sig: Adjacency = adjacency.rotate_to(*direction);
                        trace!(sig = ?direction, rotated_sig = ?rotated_sig, "Rotated");
                        assembled_map.get(Direction::STANDARD).unwrap()[&rotated_sig].clone()
                    }
                    _ => assembled_map.get(*direction).unwrap()[&adjacency].clone(),
                };
                next_frame
                    .into_iter()
                    .enumerate()
                    .for_each(|(index, image)| animated_blocks[index].push(image));
            }
            let icon_state_frames = animated_blocks
                .into_iter()
                .flatten()
                .collect::<Vec<DynamicImage>>();

            let signature = adjacency.pretty_print();

            let name = if let Some(prefix_name) = &self.output_name {
                format!("{prefix_name}-{signature}")
            } else {
                format!("{signature}")
            };
            icon_states.push(dedupe_frames(IconState {
                name,
                dirs: dir_count,
                frames: num_frames,
                images: icon_state_frames,
                delay: delay.clone(),
                rewind,
                ..Default::default()
            }));
        }

        if let Some(map_icon) = &self.map_icon {
            let icon =
                generate_map_icon(self.output_icon_size.x, self.output_icon_size.y, map_icon)?;
            icon_states.push(IconState {
                name: map_icon.icon_state_name.clone(),
                dirs: 1,
                frames: 1,
                images: vec![icon],
                ..Default::default()
            });
        }

        let output_icon = Icon {
            version: dmi::icon::DmiVersion::default(),
            width: self.output_icon_size.x,
            height: self.output_icon_size.y,
            states: icon_states,
        };

        if mode == OperationMode::Debug {
            debug!("Starting debug output");
            let mut out = self.generate_debug_icons(&corners);

            out.push(NamedIcon::from_icon(output_icon));
            Ok(ProcessorPayload::MultipleNamed(out))
        } else {
            Ok(ProcessorPayload::from_icon(output_icon))
        }
    }

    fn verify_config(&self) -> ProcessorResult<()> {
        // TODO: Actual verification
        Ok(())
    }
}

type CornerPayload = Map<Direction, Map<CornerType, Map<Corner, Vec<DynamicImage>>>>;
type PrefabPayload = Map<Direction, HashMap<Adjacency, Vec<DynamicImage>>>;


impl BitmaskSlice {
    #[tracing::instrument(skip(img))]
    pub fn build_corner(
        &self,
        img: &DynamicImage,
        position: u32,
        position_count: u32,
        dir_index: u32,
        num_frames: u32,
        prefab_count: u32,
    ) -> Map<Corner, Vec<DynamicImage>> {
        let mut out = Map::new();

        for corner in all::<Corner>() {
            out.insert(corner, vec![]);
            for frame_num in 0..num_frames {
                let frame_vec = out.get_mut(corner).unwrap();

                let (x_side, y_side) = corner.sides_of_corner();

                let x_spacing = self.get_side_info(x_side);
                let y_spacing = self.get_side_info(y_side);
                let x_offset = x_spacing.start;
                let y_offset = y_spacing.start;
                let index = dir_index * (position_count + prefab_count) + position;
                let x = index * self.icon_size.x + x_offset;
                let y = (frame_num * self.icon_size.y) + y_offset;

                let width = x_spacing.step();
                let height = y_spacing.step();
                trace!(
                    corner = ?corner,
                    x = ?x,
                    y = ?y,
                    width = ?width,
                    height = ?height,
                    "Ready to generate image"
                );
                let corner_img = img.crop_imm(x, y, width, height);
                frame_vec.push(corner_img);
            }
        }
        out
    }

    /// Generates corners
    /// # Errors
    /// Errors on malformed image
    /// # Panics
    /// Shouldn't panic
    #[tracing::instrument(skip(img))]
    pub fn generate_corners(
        &self,
        img: &DynamicImage,
    ) -> ProcessorResult<(CornerPayload, PrefabPayload)> {
        let (_width, height) = img.dimensions();

        let num_frames = height / self.icon_size.y;

        let corner_types = self.output_type.corners_used();

        let direction_positions = self.direction_strategy.input_positions();

        let prefab_count = if let Some(prefab) = &self.prefabs {
            prefab.count() as u32
        } else {
            0
        };
        let position_count = self.positions.count() as u32;

        let mut corner_directions: CornerPayload = Map::new();
        for direction in self.direction_strategy.input_vec() {
            let dir_index = *direction_positions.get(direction).unwrap();
            let mut corner_map = Map::new();
            for corner_type in &corner_types[..] {
                let position = self.positions.get(*corner_type).unwrap();

                let corners = self.build_corner(
                    img,
                    position,
                    position_count,
                    dir_index,
                    num_frames,
                    prefab_count,
                );

                corner_map.insert(*corner_type, corners);
            }
            corner_directions.insert(direction, corner_map);
        }

        let mut prefab_directions: PrefabPayload = Map::new();
        for direction in self.direction_strategy.input_vec() {
            let dir_index = *direction_positions.get(direction).unwrap();
            let mut prefabs = HashMap::new();
            if let Some(prefabs_config) = &self.prefabs {
                for (adjacency, position) in &prefabs_config.0 {
                    let mut frame_vector = vec![];
                    for frame in 0..num_frames {
                        debug!(
                            "prefab inputs: idx {} position {} position count {} prefab count {} \
                             frame {}",
                            dir_index, position, prefab_count, position_count, frame
                        );
                        let input_index = dir_index * (position_count + prefab_count) + position;
                        let x = input_index * self.icon_size.x;
                        let y = frame * self.icon_size.y;
                        debug!("prefab at {} {}", x, y);
                        let img = img.crop_imm(x, y, self.icon_size.x, self.icon_size.y);

                        frame_vector.push(img);
                    }
                    prefabs.insert(*adjacency, frame_vector);
                }
            }
            prefab_directions.insert(direction, prefabs);
        }

        Ok((corner_directions, prefab_directions))
    }

    /// Blah
    /// # Panics
    /// Whatever
    #[must_use]
    pub fn generate_icons(
        &self,
        corners: &CornerPayload,
        prefabs: &PrefabPayload,
        num_frames: u32,
        possible_adjacencies: &Vec<Adjacency>,
    ) -> Map<Direction, BTreeMap<Adjacency, Vec<DynamicImage>>> {
        let mut assembled_map = Map::new();

        for direction in self.direction_strategy.input_vec() {
            let corner_map = corners.get(direction).unwrap();
            let prefab_map = prefabs.get(direction).unwrap();
            let mut assembled: BTreeMap<Adjacency, Vec<DynamicImage>> = BTreeMap::new();
            for adjacency in possible_adjacencies {
                let mut icon_state_images = vec![];
                for frame in 0..num_frames {
                    if prefab_map.contains_key(&adjacency) {
                        debug!("prefab found! {}", adjacency.pretty_print());
                        let mut frame_image = DynamicImage::new_rgba8(
                            self.output_icon_size.x,
                            self.output_icon_size.y,
                        );
                        imageops::replace(
                            &mut frame_image,
                            prefab_map
                                .get(&adjacency)
                                .unwrap()
                                .get(frame as usize)
                                .unwrap(),
                            self.output_icon_pos.x as i64,
                            self.output_icon_pos.y as i64,
                        );

                        icon_state_images.push(frame_image);
                    } else {
                        let mut frame_image = DynamicImage::new_rgba8(
                            self.output_icon_size.x,
                            self.output_icon_size.y,
                        );

                        for corner in all::<Corner>() {
                            let corner_type = adjacency.get_corner_type(corner);
                            let corner_img = &corner_map
                                .get(corner_type)
                                .unwrap()
                                .get(corner)
                                .unwrap()
                                .get(frame as usize)
                                .unwrap();

                            let (horizontal, vertical) = corner.sides_of_corner();
                            let horizontal = self.get_side_info(horizontal);
                            let vertical = self.get_side_info(vertical);

                            imageops::overlay(
                                &mut frame_image,
                                *corner_img,
                                horizontal.start as i64,
                                vertical.start as i64,
                            );
                        }
                        icon_state_images.push(frame_image);
                    }
                }
                assembled.insert(adjacency.clone(), icon_state_images);
            }
            assembled_map.insert(direction, assembled);
        }
        assembled_map
    }

    /// Generates debug outputs for bitmask slice
    /// # Panics
    /// Shouldn't panic, unless the passed in corners are malformed
    #[must_use]
    pub fn generate_debug_icons(&self, corners: &CornerPayload) -> Vec<NamedIcon> {
        let mut out = vec![];

        let directions: Vec<Direction> = self.direction_strategy.input_vec();
        let mut corners_image = DynamicImage::new_rgba8(
            directions.len() as u32 * corners.len() as u32 * self.icon_size.x,
            self.icon_size.y,
        );

        let prefab_count = if let Some(prefab) = &self.prefabs {
            prefab.count() as u32
        } else {
            0
        };
        let position_count = self.positions.count() as u32;

        let direction_positions = self.direction_strategy.input_positions();
        for direction in directions {
            let corner_map = corners.get(direction).unwrap();
            let dir_index = direction_positions.get(direction).unwrap();
            for (corner_type, map) in corner_map.iter() {
                let position = self.positions.get(corner_type).unwrap();
                for (corner, vec) in map.iter() {
                    let input_index = dir_index * (position_count + prefab_count) + position;
                    // output each corner as it's own file
                    out.push(NamedIcon::new(
                        "DEBUGOUT/CORNERS/",
                        &format!(
                            "CORNER-{dir_index}{direction:?}-{input_index}-{corner_type:?\
                             }-{corner:?}"
                        ),
                        OutputImage::Png(vec.first().unwrap().clone()),
                    ));
                    // Reassemble the input image from corners (minus prefabs and frames)
                    let (horizontal, vertical) = corner.sides_of_corner();
                    let horizontal = self.get_side_info(horizontal);
                    let vertical = self.get_side_info(vertical);
                    let frame = vec.first().unwrap();
                    imageops::replace(
                        &mut corners_image,
                        frame,
                        (input_index * self.icon_size.x + horizontal.start) as i64,
                        vertical.start as i64,
                    );
                }
            }
        }
        out.push(NamedIcon::new(
            "DEBUGOUT",
            "ASSEMBLED-CORNERS",
            OutputImage::Png(corners_image),
        ));
        out
    }

    #[must_use]
    pub fn get_side_info(&self, side: Side) -> SideSpacing {
        match side {
            Side::North => {
                SideSpacing {
                    start: 0,
                    end: self.cut_pos.y,
                }
            }
            Side::South => {
                SideSpacing {
                    start: self.cut_pos.y,
                    end: self.icon_size.y,
                }
            }
            Side::East => {
                SideSpacing {
                    start: self.cut_pos.x,
                    end: self.icon_size.x,
                }
            }
            Side::West => {
                SideSpacing {
                    start: 0,
                    end: self.cut_pos.x,
                }
            }
        }
    }
}
