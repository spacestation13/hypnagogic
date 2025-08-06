use std::fmt::{Display, Formatter};

use enum_iterator::Sequence;
use fixed_map::Key;
use serde::{Deserialize, Serialize};

use crate::util::adjacency::Adjacency;

/// Represents a "side" of a given tile. Directions correspond to unrotated
/// cardinal directions, with "North" pointing "upwards."
#[derive(
    Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Debug, Sequence, Serialize, Deserialize, Key,
)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    North,
    South,
    East,
    West,
}

impl From<&str> for Side {
    fn from(s: &str) -> Self {
        match s {
            "north" => Self::North,
            "south" => Self::South,
            "east" => Self::East,
            "west" => Self::West,
            _ => panic!("Invalid side: {s}"),
        }
    }
}

impl Display for Side {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Side::North => write!(f, "north"),
            Side::South => write!(f, "south"),
            Side::East => write!(f, "east"),
            Side::West => write!(f, "west"),
        }
    }
}

impl Side {
    /// Matches enum variants to Byond bitfield directions
    #[must_use]
    pub const fn byond_dir(&self) -> u8 {
        match self {
            Side::North => 0b0000_0001,
            Side::South => 0b0000_0010,
            Side::East => 0b0000_0100,
            Side::West => 0b0000_1000,
        }
    }

    /// Returns an array of directions in the order that byond specifies
    /// directions. Yes, it is correct that "South" is done before North
    #[must_use]
    pub const fn dmi_cardinals() -> [Self; 4] {
        [Self::South, Self::North, Self::East, Self::West]
    }

    /// Returns a boolean determining whether a Side is a "vertical" side.
    /// "North" and "South" return true and vice versa. Maybe this is
    /// reversed, depends on whether you think of the side as the line
    /// making it up or not.
    #[must_use]
    pub const fn is_vertical(self) -> bool {
        match self {
            Self::North | Self::South => true,
            Self::East | Self::West => false,
        }
    }
}

#[derive(
    Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Debug, Sequence, Serialize, Deserialize, Key,
)]
#[serde(rename_all = "snake_case")]
pub enum Corner {
    NorthEast,
    SouthEast,
    SouthWest,
    NorthWest,
}

impl Corner {
    /// Returns the two sides that make up a given corner
    /// Order is always (horizontal, vertical)
    #[must_use]
    pub const fn sides_of_corner(self) -> (Side, Side) {
        match self {
            Corner::NorthEast => (Side::East, Side::North),
            Corner::SouthEast => (Side::East, Side::South),
            Corner::SouthWest => (Side::West, Side::South),
            Corner::NorthWest => (Side::West, Side::North),
        }
    }

    /// Generates a Byond bitfield direction given the corner
    #[must_use]
    pub const fn byond_dir(self) -> u8 {
        let (horizontal, vertical) = self.sides_of_corner();
        horizontal.byond_dir() | vertical.byond_dir()
    }
}

/// Represents the five possible given states for a corner to be in when bitmask
/// smoothing
#[derive(Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Debug, Deserialize, Serialize, Key)]
#[serde(rename_all = "snake_case")]
pub enum CornerType {
    Convex,
    Concave,
    Horizontal,
    Vertical,
    Flat,
    // Corner diagonals bullllshit
    BottomRightInner,
    BottomLeftInner,
    TopRightInner,
    TopLeftInner,
    BottomRightOuter,
    BottomLeftOuter,
    TopRightOuter,
    TopLeftOuter,
}

impl From<&str> for CornerType {
    fn from(value: &str) -> Self {
        match value {
            "convex" => Self::Convex,
            "concave" => Self::Concave,
            "horizontal" => Self::Horizontal,
            "vertical" => Self::Vertical,
            "flat" => Self::Flat,
            "bottom_right_inner" => Self::BottomRightInner,
            "bottom_left_inner" => Self::BottomLeftInner,
            "top_right_inner" => Self::TopRightInner,
            "top_left_inner" => Self::TopLeftInner,
            "bottom_right_outer" => Self::BottomRightOuter,
            "bottom_left_outer" => Self::BottomLeftOuter,
            "top_right_outer" => Self::TopRightOuter,
            "top_left_outer" => Self::TopLeftOuter,
            _ => panic!("Invalid String: {value}"),
        }
    }
}

impl Display for CornerType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Convex => write!(f, "convex"),
            Self::Concave => write!(f, "concave"),
            Self::Horizontal => write!(f, "horizontal"),
            Self::Vertical => write!(f, "vertical"),
            Self::Flat => write!(f, "flat"),
            Self::BottomRightInner => write!(f, "bottom_right_inner"),
            Self::BottomLeftInner => write!(f, "bottom_left_inner"),
            Self::TopRightInner => write!(f, "top_right_inner"),
            Self::TopLeftInner => write!(f, "top_left_inner"),
            Self::BottomRightOuter => write!(f, "bottom_right_outer"),
            Self::BottomLeftOuter => write!(f, "bottom_left_outer"),
            Self::TopRightOuter => write!(f, "top_right_outer"),
            Self::TopLeftOuter => write!(f, "top_left_outer"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum CornerSet {
    #[default]
    // Produces cardinal smoothing, so smoothing with your 4 neighbors
    Cardinal,
    // Produces standard diagonal smoothing, so smoothing with your 8 neighbors.
    // This requires an extra "flat" input which represents smoothing with all 8 at once
    StandardDiagonal,
    // Produces vornered diagonal smoothing, which is like diagonal smoothing but it takes 8
    // Additional inputs to use as corners for L sides, one for when there's a turf on the inside
    // and one for when there is not
    CornerDiagonal,
}

impl Display for CornerSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cardinal => write!(f, "Cardinal"),
            Self::StandardDiagonal => write!(f, "StandardDiagonal"),
            Self::CornerDiagonal => write!(f, "CornerDiagonal"),
        }
    }
}

impl CornerSet {
    pub fn possible_bit_states(&self) -> u16 {
        match self {
            Self::Cardinal => usize::pow(2, 4) as u16, /* we need 16 bits for this guy, since we
                                                         * have 4 dirs to care about */
            Self::StandardDiagonal => usize::pow(2, 8) as u16, // 255 for you, since we have the 8
            Self::CornerDiagonal => usize::pow(2, 8) as u16, /* and 255 for you, since the
                                                              * diagonals are done as a suffix */
        }
    }

    pub fn output_adjacencies(&self) -> Vec<Adjacency> {
        match self {
            Self::CornerDiagonal => {
                let inner_corners = Adjacency::diagonal_cardinals();
                let outer_corners = Adjacency::filled_diagonals();
                (0..self.possible_bit_states())
                    .flat_map(|bits| {
                        let adjacency = Adjacency::from_bits(bits).unwrap();
                        if inner_corners.contains(&adjacency) {
                            vec![adjacency, adjacency.clone() | Adjacency::INNER_EDGE]
                        } else if outer_corners.contains(&adjacency) {
                            vec![adjacency, adjacency.clone() | Adjacency::OUTER_EDGE]
                        } else {
                            vec![adjacency]
                        }
                    })
                    .collect::<Vec<Adjacency>>()
            }
            _ => {
                (0..self.possible_bit_states())
                    .map(|bits| Adjacency::from_bits(bits).unwrap())
                    .collect::<Vec<Adjacency>>()
            }
        }
    }

    #[must_use]
    pub fn corners_used(&self) -> Vec<CornerType> {
        match self {
            Self::Cardinal => {
                vec![
                    CornerType::Convex,
                    CornerType::Concave,
                    CornerType::Horizontal,
                    CornerType::Vertical,
                ]
            }
            Self::StandardDiagonal => {
                vec![
                    CornerType::Convex,
                    CornerType::Concave,
                    CornerType::Horizontal,
                    CornerType::Vertical,
                    CornerType::Flat,
                ]
            }
            Self::CornerDiagonal => {
                vec![
                    CornerType::Convex,
                    CornerType::Concave,
                    CornerType::Horizontal,
                    CornerType::Vertical,
                    CornerType::Flat,
                    CornerType::BottomRightInner,
                    CornerType::BottomLeftInner,
                    CornerType::TopRightInner,
                    CornerType::TopLeftInner,
                    CornerType::BottomRightOuter,
                    CornerType::BottomLeftOuter,
                    CornerType::TopRightOuter,
                    CornerType::TopLeftOuter,
                ]
            }
        }
    }
}
