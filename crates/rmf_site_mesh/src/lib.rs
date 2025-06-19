pub mod shapes;
use std::{cmp, ops::{Add, Mul, Neg, Sub}};

use bevy_derive::Deref;
pub use shapes::*;

pub mod primitives;
pub use primitives::*;
pub enum Angle {
    Deg(Degrees),
    Rad(Radians),
}

#[derive(Deref, Clone, Copy, PartialEq, PartialOrd)]
pub struct Radians(pub f32);

impl From<Radians> for Angle {
    fn from(value: Radians) -> Self {
        Angle::Rad(value)
    }
}

impl From<Angle> for Radians {
    fn from(value: Angle) -> Self {
        match value {
            Angle::Deg(degrees) => Radians(degrees.to_radians()),
            Angle::Rad(radians) => radians,
        }
    }
}

impl PartialEq<f32> for Radians {
    fn eq(&self, other: &f32) -> bool {
        &self.0 == other
    }
}

impl PartialOrd<f32> for Radians {
    fn partial_cmp(&self, other: &f32) -> Option<cmp::Ordering> {
       self.0.partial_cmp(other)
    }
}

impl Neg for Radians {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl Add for Radians {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}


#[derive(Deref, PartialEq, PartialOrd)]
pub struct Degrees(pub f32);

impl From<Degrees> for Angle {
    fn from(value: Degrees) -> Self {
        Angle::Deg(value)
    }
}

// impl From<Degrees> for u32 {
//     fn from(value: Degrees) -> Self {
//         value.0 as u32
//     }
// }

impl Mul for Degrees {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}

impl Mul<f32> for Degrees {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl From<Degrees> for Radians {
    fn from(value: Degrees) -> Self {
       Self(value.to_radians())
    }
}

impl From<Radians> for Degrees {
    fn from(value: Radians) -> Self {
        Self(value.to_degrees())
    }
}

impl From<Angle> for Degrees {
    fn from(value: Angle) -> Self {
        match value {
            Angle::Deg(degrees) => degrees,
            Angle::Rad(radians) => Degrees(radians.to_degrees()),
        }
    }
}


// impl Angle {
//     pub fn from_degrees(degrees: f32) -> Self {
//         Self::Deg(degrees.to_degrees())
//     }
//     pub fn from_radians(radians: f32) -> Self{

//     }
// }

// impl Angle {
//     pub fn radians(&self) -> Radians {
//         match self {
//             Angle::Deg(v) => Radians(v.to_radians()),
//             Angle::Rad(v) => Radians(**v),
//         }
//     }

//     pub fn degrees(&self) -> Degrees {
//         match self {
//             Angle::Deg(v) => Degrees(**v),
//             Angle::Rad(v) => Degrees(v.to_degrees()),
//         }
//     }
// }