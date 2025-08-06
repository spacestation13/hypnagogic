use std::str::FromStr;

use bitflags::{bitflags, bitflags_match};

use super::directions::Direction;
use crate::util::corners::{Corner, CornerType, Side};

bitflags! {
    #[allow(clippy::unsafe_derive_deserialize)]
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    pub struct Adjacency: u16 {
        const N = 0b00_0000_0001;
        const S = 0b00_0000_0010;
        const E = 0b00_0000_0100;
        const W = 0b00_0000_1000;
        const NE = 0b00_0001_0000;
        const SE = 0b00_0010_0000;
        const SW = 0b00_0100_0000;
        const NW = 0b00_1000_0000;
        const INNER_EDGE = 0b01_0000_0000;
        const OUTER_EDGE = 0b10_0000_0000;
        const N_S = Self::N.bits() | Self::S.bits();
        const E_W = Self::E.bits() | Self::W.bits();
        const EDGES = Self::INNER_EDGE.bits() | Self::OUTER_EDGE.bits();
        const CARDINALS = Self::N.bits() | Self::S.bits() | Self::E.bits() | Self::W.bits();
    }
}

impl From<Corner> for Adjacency {
    fn from(corner: Corner) -> Self {
        Adjacency::from_corner(corner)
    }
}

impl From<Side> for Adjacency {
    fn from(side: Side) -> Self {
        match side {
            Side::North => Adjacency::N,
            Side::South => Adjacency::S,
            Side::East => Adjacency::E,
            Side::West => Adjacency::W,
        }
    }
}

impl From<String> for Adjacency {
    fn from(text: String) -> Self {
        let mut parsing_text = text;
        if let Some(d_location) = parsing_text.find("d") {
            parsing_text.remove(d_location);
            parsing_text.remove(d_location - 1); // get the -
        };
        Adjacency::from_bits(parsing_text.parse::<u16>().unwrap()).unwrap()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct InvalidAdjacencyError;

impl FromStr for Adjacency {
    type Err = InvalidAdjacencyError;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut parsing_text = text.to_string();
        if let Some(d_location) = parsing_text.find("d") {
            parsing_text.remove(d_location);
            parsing_text.remove(d_location - 1); // get the -
        };
        let parsed_bytes = parsing_text.parse::<u16>().map_err(|_| InvalidAdjacencyError)?;
        if let Some(adjacency) = Adjacency::from_bits(parsed_bytes) {
            Ok(adjacency)
        } else {
            Err(InvalidAdjacencyError)
        }
    }
}

impl Adjacency {
    /// Returns an array of the cardinal directions in the order used by DMI
    #[must_use]
    pub const fn dmi_cardinals() -> [Adjacency; 4] {
        [Adjacency::S, Adjacency::N, Adjacency::E, Adjacency::W]
    }

    #[must_use]
    pub const fn diagonals() -> [Adjacency; 4] {
        [Adjacency::NE, Adjacency::SE, Adjacency::SW, Adjacency::NW]
    }

    #[must_use]
    pub fn diagonal_cardinals() -> Vec<Adjacency> {
        vec![Adjacency::N | Adjacency::E, Adjacency::S | Adjacency::E, Adjacency::S | Adjacency::W, Adjacency::N | Adjacency::W]
    }

    #[must_use]
    pub fn filled_diagonals() -> Vec<Adjacency> {
        Adjacency::diagonals().into_iter().map(|adjacency| {
            let (vertical, horizontal) = adjacency.corner_sides();
            adjacency | vertical | horizontal
        }).collect::<Vec<Adjacency>>()
    }

    /// Gets the sides for a given corner adjacency
    /// Adjacency is always returned in the format of `(Vertical, Horizontal)`
    /// # Panics
    /// Panics when a non-corner adjacency is passed in
    #[must_use]
    pub const fn corner_sides(self) -> (Adjacency, Adjacency) {
        match self.difference(Adjacency::EDGES) {
            Adjacency::NE => (Adjacency::N, Adjacency::E),
            Adjacency::SE => (Adjacency::S, Adjacency::E),
            Adjacency::SW => (Adjacency::S, Adjacency::W),
            Adjacency::NW => (Adjacency::N, Adjacency::W),
            _ => panic!("Not a corner!"),
        }
    }

    #[must_use]
    pub const fn adjacent_corners_filled(self, corner: Self) -> bool {
        let (first, second) = corner.corner_sides();
        self.contains(first) && self.contains(second)
    }

    #[must_use]
    pub const fn has_no_orphaned_corner(self) -> bool {
        // the loop here is manually unrolled because function is const
        let [first, second, third, fourth] = Self::diagonals();
        if self.contains(first) && !self.adjacent_corners_filled(first) {
            return false;
        }
        if self.contains(second) && !self.adjacent_corners_filled(second) {
            return false;
        }
        if self.contains(third) && !self.adjacent_corners_filled(third) {
            return false;
        }
        if self.contains(fourth) && !self.adjacent_corners_filled(fourth) {
            return false;
        }
        true
    }

    #[must_use]
    pub const fn ref_has_no_orphaned_corner(&self) -> bool {
        self.has_no_orphaned_corner()
    }

    // implemented as const for usage in get corner type
    const fn from_corner(corner: Corner) -> Self {
        match corner {
            Corner::NorthEast => Adjacency::NE,
            Corner::SouthEast => Adjacency::SE,
            Corner::SouthWest => Adjacency::SW,
            Corner::NorthWest => Adjacency::NW,
        }
    }

    #[must_use]
    pub fn set_flags_vec(self) -> Vec<Self> {
        let full: [Adjacency; 10] = [
            Adjacency::N,
            Adjacency::S,
            Adjacency::E,
            Adjacency::W,
            Adjacency::NE,
            Adjacency::SE,
            Adjacency::SW,
            Adjacency::NW,
            Adjacency::INNER_EDGE,
            Adjacency::OUTER_EDGE,
        ];
        full.into_iter().filter(|a| self.contains(*a)).collect()
    }
    #[must_use]
    pub fn get_corner_type(self, corner: Corner) -> CornerType {
        let adj_corner: Adjacency = Adjacency::from_corner(corner);
        let (vertical, horizontal) = adj_corner.corner_sides();
        // If we're an edge then it becomes stupid. Perhaps I should have done this as prefabs after all
        if self.intersects(Adjacency::EDGES) {
            bitflags_match!(self.difference(Adjacency::EDGES), {
                Adjacency::S | Adjacency::E => CornerType::BottomRightInner, 
                Adjacency::S | Adjacency::W => CornerType::BottomLeftInner, 
                Adjacency::N | Adjacency::E => CornerType::TopRightInner, 
                Adjacency::N | Adjacency::W => CornerType::TopLeftInner,
                Adjacency::S | Adjacency::E | Adjacency::SE => CornerType::BottomRightOuter, 
                Adjacency::S | Adjacency::W | Adjacency::SW => CornerType::BottomLeftOuter, 
                Adjacency::N | Adjacency::E | Adjacency::NE => CornerType::TopRightOuter, 
                Adjacency::N | Adjacency::W | Adjacency::NW => CornerType::TopLeftOuter, 
                _ => CornerType::Convex
            })
        // It should only flat smooth if cardinals are filled too
        } else if self.contains(vertical) && self.contains(horizontal) {
            if self.contains(adj_corner) {
                CornerType::Flat
            } else {
                CornerType::Concave
            }
        } else if self.contains(vertical) {
            // Since we don't have both, it must be exclusive meaning horizontal doesn't
            // need to be checked
            CornerType::Vertical
        } else if self.contains(horizontal) {
            // Ditto as above, but now for horizontal
            CornerType::Horizontal
        } else {
            CornerType::Convex
        }
    }

    #[must_use]
    pub fn pretty_print(self) -> String {
        let number_bits = self.bits() & !(Adjacency::EDGES.bits());
        let mut pretty_string = number_bits.to_string();
        if self.intersects(Adjacency::EDGES) {
            pretty_string = format!("{}-d", pretty_string);
        }
        pretty_string
    }

    #[must_use]
    pub fn rotate_dir(self, direction: Direction) -> Self {
        match direction {
            // 180 degree rotation
            Direction::N => {
                match self {
                    Adjacency::N => Adjacency::S,
                    Adjacency::S => Adjacency::N,
                    Adjacency::E => Adjacency::W,
                    Adjacency::W => Adjacency::E,
                    Adjacency::NE => Adjacency::SW,
                    Adjacency::SE => Adjacency::NW,
                    Adjacency::SW => Adjacency::NE,
                    Adjacency::NW => Adjacency::SE,
                    _ => unimplemented!("Only single allowed"),
                }
            }
            // No rotation needed!
            Direction::S => self,
            // Counter-clockwise 90 degrees
            Direction::E => {
                match self {
                    Adjacency::N => Adjacency::W,
                    Adjacency::S => Adjacency::E,
                    Adjacency::E => Adjacency::N,
                    Adjacency::W => Adjacency::S,
                    Adjacency::NE => Adjacency::NW,
                    Adjacency::SE => Adjacency::NE,
                    Adjacency::SW => Adjacency::SE,
                    Adjacency::NW => Adjacency::SW,
                    _ => unimplemented!("Only single allowed"),
                }
            }
            // Clockwise 90 degrees
            Direction::W => {
                match self {
                    Adjacency::N => Adjacency::E,
                    Adjacency::S => Adjacency::W,
                    Adjacency::E => Adjacency::S,
                    Adjacency::W => Adjacency::N,
                    Adjacency::NE => Adjacency::SE,
                    Adjacency::SE => Adjacency::SW,
                    Adjacency::SW => Adjacency::NW,
                    Adjacency::NW => Adjacency::NE,
                    _ => unimplemented!("Only single allowed"),
                }
            }
            _ => {
                unimplemented!(
                    "Rotating to diagonals doesn't make sense. This is a programming error."
                )
            }
        }
    }

    #[must_use]
    pub fn rotate_to(self, direction: Direction) -> Self {
        self.set_flags_vec()
            .into_iter()
            .map(|x| x.rotate_dir(direction))
            .reduce(|accum, item| accum | item)
            .unwrap_or(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_flags_vec_test() {
        let adj = Adjacency::N | Adjacency::S | Adjacency::W;

        let result = adj.set_flags_vec();

        let expected = [Adjacency::N, Adjacency::W, Adjacency::S];

        assert!(expected.iter().all(|item| result.contains(item)));
    }
}
