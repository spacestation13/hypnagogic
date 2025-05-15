use std::fmt::{Display, Formatter};

use fixed_map::{Key, Map};
use serde::{Deserialize, Serialize};

use super::adjacency::Adjacency;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Key)]
pub enum Direction {
    N,
    S,
    E,
    W,
    NE,
    SE,
    SW,
    NW,
}

impl Direction {   
    pub const STANDARD: Direction = Direction::S;

    /// Returns an array of the cardinal directions in the order used by DMI
    #[must_use]
    pub const fn dmi_cardinals() -> [Direction; 4] {
        [Direction::S, Direction::N, Direction::E, Direction::W]
    }

    /// Returns an array of all directions in the order used by DMI
    #[must_use]
    pub const fn dmi_all() -> [Direction; 8] {
        [Direction::S, Direction::N, Direction::E, Direction::W, Direction::SE, Direction::SW, Direction::NE, Direction::NW]
    }

}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum DirectionStrategy {
    #[default]
    // No directions, produces a dmi with 1 dir per icon state, expects standard input
    Standard,
    // Produces a dmi with 4 dirs per icon state, expects 4x the amount of input
    Cardinals,
    // Produces a dmi with 4 dirs per icon state, you can think of each direction as a "rotation" of the existing ADJACENCY (not the icon state)
    CardinalsRotated,
    // Produces a dmi with 8 dirs per icon state, expects 8x the amount of input
    All,
}

impl Display for DirectionStrategy {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DirectionStrategy::Standard => write!(f, "Standard"),
            DirectionStrategy::Cardinals => write!(f, "Cardinals"),
            DirectionStrategy::CardinalsRotated => write!(f, "CardinalsRotated"),
            DirectionStrategy::All => write!(f, "All"),
        }
    }
}

impl DirectionStrategy {

    #[must_use]
    pub const fn count_to_strategy(count: u8) -> Option<DirectionStrategy> {
        match count {
            1 => Some(DirectionStrategy::Standard),
            4 => Some(DirectionStrategy::Cardinals),
            8 => Some(DirectionStrategy::All),
            _ => None
        }
    }

    #[must_use]
    pub fn input_vec(&self) -> Vec<Direction> {
        match self {
            DirectionStrategy::Standard => vec![Direction::STANDARD],
            DirectionStrategy::Cardinals => Direction::dmi_cardinals().to_vec(),
            DirectionStrategy::CardinalsRotated => vec![Direction::STANDARD],
            DirectionStrategy::All => Direction::dmi_all().to_vec(),
        }
    }

    #[must_use]
    pub fn output_vec(&self) -> Vec<Direction> {
        match self {
            DirectionStrategy::CardinalsRotated => Direction::dmi_cardinals().to_vec(),
            _ => self.input_vec()
        }
    }

    #[must_use]
    pub fn input_positions(&self) -> Map<Direction, u32> {
        self.input_vec()
            .into_iter()
            .enumerate()
            .fold(Map::new(), |mut acc, (position, direction)| {
            acc.insert(direction, position as u32); 
            acc 
        })
    }

    #[must_use]
    pub fn rotate_adjacency(&self, adjacency: Adjacency, direction: Direction) -> Adjacency {
        match self {
            DirectionStrategy::CardinalsRotated => adjacency.rotate_to(direction),
            _ => adjacency,
        }
    }
}

