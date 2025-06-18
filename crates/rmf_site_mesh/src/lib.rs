pub mod shapes;
pub mod primitives;

pub enum Angle {
    Deg(f32),
    Rad(f32),
}

impl Angle {
    pub fn radians(&self) -> f32 {
        match self {
            Angle::Deg(v) => v.to_radians(),
            Angle::Rad(v) => *v,
        }
    }

    pub fn degrees(&self) -> f32 {
        match self {
            Angle::Deg(v) => *v,
            Angle::Rad(v) => v.to_degrees(),
        }
    }

    pub fn match_variant(self, other: Angle) -> Self {
        match other {
            Angle::Deg(_) => Angle::Deg(self.degrees()),
            Angle::Rad(_) => Angle::Rad(self.radians()),
        }
    }

    pub fn is_radians(&self) -> bool {
        matches!(self, Angle::Rad(_))
    }

    pub fn is_degrees(&self) -> bool {
        matches!(self, Angle::Deg(_))
    }
}