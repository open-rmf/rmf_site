use super::{PortingError, Result};
use crate::{DoorType, DoubleSlidingDoor, LiftCabin, LiftCabinDoor, ParameterizedLiftCabin};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Deserialize, Serialize, Clone)]
pub struct LiftDoor {
    pub door_type: i32,
    pub motion_axis_orientation: i32,
    pub width: f64,
    pub x: f64,
    pub y: f64,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Lift {
    pub depth: f64,
    pub doors: BTreeMap<String, LiftDoor>,
    pub lowest_floor: String,
    pub highest_floor: String,
    pub initial_floor_name: String,
    pub level_doors: BTreeMap<String, Vec<String>>,
    pub plugins: bool,
    pub reference_floor_name: String,
    pub width: f64,
    pub x: f64,
    pub y: f64,
    pub yaw: f64,
}

impl Lift {
    pub fn calculate_anchors(&self) -> [[f32; 2]; 2] {
        let x = self.x as f32;
        let y = self.y as f32;
        let d = self.depth as f32 / 2.0;
        let w = self.width as f32 / 2.0;
        let theta = self.yaw as f32;
        let rotate = |x, y| {
            (
                x * theta.cos() - y * theta.sin(),
                x * theta.sin() + y * theta.cos(),
            )
        };
        let (dx_0, dy_0) = rotate(d, w);
        let (dx_1, dy_1) = rotate(d, -w);
        return [[x + dx_0, y + dy_0], [x + dx_1, y + dy_1]];
    }

    pub fn make_cabin(&self, name: &String) -> Result<LiftCabin> {
        if self.doors.len() > 1 {
            return Err(PortingError::InvalidLiftCabinDoors {
                lift: name.clone(),
                door_count: self.doors.len(),
            });
        }

        let door_width = self
            .doors
            .iter()
            .next()
            .map(|(_, door)| door.width as f32)
            .unwrap_or(self.width as f32 * 0.75);

        Ok(LiftCabin::Params(ParameterizedLiftCabin {
            width: self.width as f32,
            depth: self.depth as f32,
            door: LiftCabinDoor {
                width: door_width,
                kind: DoorType::DoubleSliding(DoubleSlidingDoor::default()),
                shifted: None,
            },
            wall_thickness: None,
            gap: None,
            shift: None,
        }))
    }
}

impl Default for Lift {
    fn default() -> Self {
        Self {
            depth: 1.0,
            doors: BTreeMap::new(),
            lowest_floor: "L1".to_string(),
            highest_floor: "L1".to_string(),
            initial_floor_name: "L1".to_string(),
            level_doors: BTreeMap::new(),
            plugins: false,
            reference_floor_name: "L1".to_string(),
            width: 1.0,
            x: 0.0,
            y: 0.0,
            yaw: 0.0,
        }
    }
}
