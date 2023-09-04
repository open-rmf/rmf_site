use super::{rbmf::*, PortingError, Result};
use crate::{
    Angle, Door as SiteDoor, DoubleSlidingDoor, DoubleSwingDoor, NameInSite, Side,
    SingleSlidingDoor, SingleSwingDoor, Swing,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Display};

#[derive(Deserialize, Serialize, Clone)]
pub struct DoorProperties {
    pub right_left_ratio: RbmfFloat,
    pub motion_axis: RbmfString,
    pub motion_degrees: RbmfFloat,
    pub motion_direction: RbmfInt,
    pub name: RbmfString,
    pub plugin: RbmfString,
    #[serde(rename = "type")]
    pub type_: RbmfString,
}

impl Default for DoorProperties {
    fn default() -> Self {
        Self {
            right_left_ratio: RbmfFloat::from(1.),
            motion_axis: RbmfString::from("start"),
            motion_degrees: RbmfFloat::from(90.),
            motion_direction: RbmfInt::from(1),
            name: RbmfString::from("new_door"),
            plugin: RbmfString::from("normal"),
            type_: RbmfString::from("hinged"),
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Door(pub usize, pub usize, pub DoorProperties);

impl Door {
    pub fn to_pivot_on(&self) -> Result<Side> {
        if self.2.motion_axis.1 == "start" {
            Ok(Side::Left)
        } else if self.2.motion_axis.1 == "end" {
            Ok(Side::Right)
        } else {
            Err(PortingError::InvalidMotionAxis(
                self.2.motion_axis.1.clone(),
            ))
        }
    }

    pub fn to_swing(&self) -> Result<Swing> {
        match self.to_pivot_on()? {
            Side::Left => {
                if self.2.motion_direction.1 < 0 {
                    Ok(Swing::Forward(Angle::Deg(self.2.motion_degrees.1 as f32)))
                } else {
                    Ok(Swing::Backward(Angle::Deg(self.2.motion_degrees.1 as f32)))
                }
            }
            Side::Right => {
                if self.2.motion_direction.1 < 0 {
                    Ok(Swing::Backward(Angle::Deg(self.2.motion_degrees.1 as f32)))
                } else {
                    Ok(Swing::Forward(Angle::Deg(self.2.motion_degrees.1 as f32)))
                }
            }
        }
    }

    pub fn to_site(&self, vertex_to_anchor_id: &HashMap<usize, u32>) -> Result<SiteDoor<u32>> {
        let type_: DoorType = self.2.type_.1.as_str().into();
        let kind = match type_ {
            DoorType::SingleSliding => SingleSlidingDoor {
                towards: Side::Right,
            }
            .into(),
            DoorType::DoubleSliding => DoubleSlidingDoor {
                left_right_ratio: 1. / self.2.right_left_ratio.1 as f32,
            }
            .into(),
            DoorType::SingleTelescope => {
                return Err(PortingError::DeprecatedType(
                    "porting telescope door type is not supported".to_string(),
                ));
            }
            DoorType::DoubleTelescope => {
                return Err(PortingError::DeprecatedType(
                    "porting double_telescope type is not supported".to_string(),
                ));
            }
            DoorType::SingleSwing | DoorType::SingleHinged => SingleSwingDoor {
                pivot_on: self.to_pivot_on()?,
                swing: self.to_swing()?,
            }
            .into(),
            DoorType::DoubleSwing | DoorType::DoubleHinged => DoubleSwingDoor {
                swing: self.to_swing()?,
                left_right_ratio: 1. / self.2.right_left_ratio.1 as f32,
            }
            .into(),
            DoorType::Unknown => return Err(PortingError::InvalidType(self.2.type_.1.clone())),
        };

        let left_anchor = vertex_to_anchor_id
            .get(&self.0)
            .ok_or(PortingError::InvalidVertex(self.0))?;
        let right_anchor = vertex_to_anchor_id
            .get(&self.1)
            .ok_or(PortingError::InvalidVertex(self.1))?;

        Ok(SiteDoor {
            anchors: [*left_anchor, *right_anchor].into(),
            name: NameInSite(self.2.name.1.clone()),
            kind,
            marker: Default::default(),
        })
    }
}

pub enum DoorType {
    Unknown,
    SingleSliding,
    DoubleSliding,
    SingleTelescope,
    DoubleTelescope,
    SingleSwing,
    DoubleSwing,
    SingleHinged,
    DoubleHinged,
}

impl DoorType {
    pub fn to_value(&self) -> String {
        match self {
            DoorType::Unknown => "unknown".to_string(),
            DoorType::SingleSliding => "sliding".to_string(),
            DoorType::DoubleSliding => "double_sliding".to_string(),
            DoorType::SingleTelescope => "telescope".to_string(),
            DoorType::DoubleTelescope => "double_telescope".to_string(),
            DoorType::SingleSwing => "swing".to_string(),
            DoorType::DoubleSwing => "double_swing".to_string(),
            DoorType::SingleHinged => "hinged".to_string(),
            DoorType::DoubleHinged => "double_hinged".to_string(),
        }
    }
}

impl From<&str> for DoorType {
    fn from(s: &str) -> Self {
        match s {
            "sliding" => DoorType::SingleSliding,
            "double_sliding" => DoorType::DoubleSliding,
            "telescope" => DoorType::SingleTelescope,
            "double_telescope" => DoorType::DoubleTelescope,
            "swing" => DoorType::SingleSwing,
            "double_swing" => DoorType::DoubleSwing,
            "hinged" => DoorType::SingleHinged,
            "double_hinged" => DoorType::DoubleHinged,
            _ => DoorType::Unknown,
        }
    }
}

impl Display for DoorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DoorType::Unknown => write!(f, "Unknown"),
            DoorType::SingleSliding => write!(f, "Sliding"),
            DoorType::DoubleSliding => write!(f, "Double Sliding"),
            DoorType::SingleTelescope => write!(f, "Telescope"),
            DoorType::DoubleTelescope => write!(f, "Double Telescope"),
            DoorType::SingleSwing => write!(f, "Swing"),
            DoorType::DoubleSwing => write!(f, "Double Swing"),
            DoorType::SingleHinged => write!(f, "Hinged"),
            DoorType::DoubleHinged => write!(f, "Double Hinged"),
        }
    }
}

pub static DOOR_TYPES: &[DoorType] = &[
    DoorType::SingleSliding,
    DoorType::DoubleSliding,
    DoorType::SingleTelescope,
    DoorType::DoubleTelescope,
    DoorType::SingleSwing,
    DoorType::DoubleSwing,
    DoorType::SingleHinged,
    DoorType::DoubleHinged,
];
