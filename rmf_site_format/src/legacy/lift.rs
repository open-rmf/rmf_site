use super::{PortingError, Result};
use crate::{
    DoorType, DoubleSlidingDoor, LiftCabin, LiftCabinDoor, RectangularLiftCabin,
    LiftCabinDoorPlacement, Level, Lift as SiteLift, LiftProperties, Anchor,
    Categorized, LevelDoors,
    DEFAULT_CABIN_WALL_THICKNESS, DEFAULT_CABIN_DOOR_THICKNESS,
};
use serde::{Deserialize, Serialize};
use glam::Vec2;
use std::{
    collections::{BTreeMap, HashMap},
    ops::RangeFrom,
};

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

enum CabinFace {
    Front,
    Back,
    Left,
    Right,
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

    pub fn to_site(
        &self,
        lift_name: &String,
        site_id: &mut RangeFrom<u32>,
        site_anchors: &mut BTreeMap<u32, [f32; 2]>,
        levels: &BTreeMap<u32, Level>,
        level_name_to_id: &BTreeMap<String, u32>,
        all_lift_cabin_anchors: &BTreeMap<String, Vec<(u32, [f32; 2])>>,
    ) -> Result<SiteLift<u32>> {
        let ref_anchor_positions = lift.calculate_anchors();
        let reference_anchors = {
            let left = site_id.next().unwrap();
            let right = site_id.next().unwrap();
            site_anchors.insert(left, ref_anchor_positions[0]);
            site_anchors.insert(right, ref_anchor_positions[1]);
            [left, right]
        };

        if self.doors.len() > 4 {
            return Err(PortingError::InvalidLiftCabinDoorCount {
                lift: lift_name.clone(),
                door_count: self.doors.len(),
            });
        }

        let mut cabin_doors = BTreeMap::new();
        let mut cabin_door_name_to_id = HashMap::new();
        let mut front_door = None;
        let mut back_door = None;
        let mut left_door = None;
        let mut right_door = None;

        for (door_name, door) in &self.doors {
            let id = site_id.next().unwrap();
            cabin_door_name_to_id.insert(door_name.clone(), id);
            cabin_doors.insert(
                id,
                LiftCabinDoor {
                    kind: DoorType::DoubleSliding(DoubleSlidingDoor { left_right_ratio: 0.5 }),
                    marker: Default::default(),
                }
            );

            let dx = door.x;
            let dy = door.y;

            let cabin_face = if dx.abs() < 1e-3 {
                // Very small x value means the door must be on the left or right face
                if dy >= self.width/2.0 {
                    // Positive y means left door
                    CabinFace::Left
                } else if dy <= -self.width/2.0 {
                    // Negative y means right door
                    CabinFace::Right
                } else {
                    return Err(PortingError::InvalidLiftCabinDoorPlacement { lift: lift_name.clone(), door: door_name.clone() });
                }
            } else {
                let m = dy/dx;
                let y_intercept = m * self.depth/2.0;
                if y_intercept.abs() <= self.width/2.0 {
                    // The door must be on the front or back face
                    if dx >= self.depth/2.0 {
                        // Positive x means front door
                        CabinFace::Front
                    } else if dx <= -self.depth/2.0 {
                        CabinFace::Back
                    } else {
                        return Err(PortingError::InvalidLiftCabinDoorPlacement { lift: lift_name.clone(), door: door_name.clone() });
                    }
                } else {
                    // The door must be on the left or right face
                    if dy >= self.width/2.0 {
                        CabinFace::Left
                    } else if dy <= self.width/2.0 {
                        CabinFace::Right
                    } else {
                        return Err(PortingError::InvalidLiftCabinDoorPlacement { lift: lift_name.clone(), door: door_name.clone() });
                    }
                }
            };

            match cabin_face {
                CabinFace::Front => {
                    if front_door.is_some() {
                        return Err(PortingError::DuplicateLiftCabinDoor { lift: lift_name.clone(), face: "front" });
                    }
                    front_door = Some(LiftCabinDoorPlacement {
                        door: id,
                        width: door.width,
                        thickness: None,
                        shifted: Some(dy),
                        custom_gap: Some(dx - self.depth/2.0),
                    });
                }
                CabinFace::Back => {
                    if back_door.is_some() {
                        return Err(PortingError::DuplicateLiftCabinDoor { lift: lift_name.clone(), face: "back" });
                    }
                    back_door = Some(LiftCabinDoorPlacement {
                        door: id,
                        width: door.width,
                        thickness: None,
                        shifted: Some(-dy),
                        custom_gap: Some(-dx - self.depth/2.0),
                    })
                }
                CabinFace::Left => {
                    if left_door.is_some() {
                        return Err(PortingError::DuplicateLiftCabinDoor { lift: lift_name.clone(), face: "left" });
                    }
                    left_door = Some(LiftCabinDoorPlacement {
                        door: id,
                        width: door.width,
                        thickness: None,
                        shifted: Some(-dx),
                        custom_gap: Some((dy - self.width/2.0) as f32),
                    });
                }
                CabinFace::Right => {
                    if right_door.is_some() {
                        return Err(PortingError::DuplicateLiftCabinDoor { lift: lift_name.clone(), face: "right" });
                    }
                    right_door = Some(LiftCabinDoorPlacement {
                        door: id,
                        width: door.width,
                        thickness: None,
                        shifted: Some(dx),
                        custom_gap: Some((-dy - self.width/2.0))
                    });
                }
            }
        }

        let width = self.width as f32;
        let depth = self.depth as f32;
        let cabin = LiftCabin::Rect(RectangularLiftCabin {
            width,
            depth,
            wall_thickness: None,
            gap: None,
            shift: None,
            front_door,
            back_door,
            left_door,
            right_door,
        });

        let level_visit_doors = self.level_doors
            .iter()
            .map(|(level_name, door_names)| {
                (
                    level_name_to_id.get(level_name).ok_or(
                        PortingError::InvalidLevelName(level_name.clone())
                    )?,
                    door_names.iter().map(
                        |door_name| {
                            cabin_door_name_to_id.get(door_name).ok_or(
                                PortingError::InvalidLiftCabinDoorName {
                                    lift: lift_name.clone(),
                                    door: door_name.clone()
                                }
                            )
                        }
                    ).collect()
                )
            }).collect();

        let cabin_anchors: BTreeMap<u32, [f32; 2]> = [all_lift_cabin_anchors.get(name)]
            .into_iter()
            .filter_map(|x| x)
            .flat_map(|x| x)
            .copied()
            .collect();

        let level_door_anchors = {
            let doors = [
                (front_door, Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0)),
                (back_door, Vec2::new(-1.0, 0.0), Vec2::new(0.0, -1.0)),
                (left_door, Vec2::new(0.0, 1.0), Vec2::new(-1.0, 0.0)),
                (right_door, Vec2::new(0.0, -1.0), Vec2::new(1.0, 0.0)),
            ];

            let mut level_door_anchors = BTreeMap::new();
            for (placement, u, v) in doors.into_iter().filter_map(|(p, g, s)| p.map(|p| (p, g, s))) {
                let gap = placement.custom_gap.unwrap();
                let forward = Vec2::new(
                    depth/2.0 + DEFAULT_CABIN_WALL_THICKNESS + gap,
                    width/2.0 + DEFAULT_CABIN_WALL_THICKNESS + gap,
                );
                let center = forward.dot(u) * u + placement.shifted.unwrap_or(0.0) * v;
                let l_anchor = center + placement.width/2.0*v;
                let r_anchor = center - placement.width/2.0*v;
                let d_floor = DEFAULT_CABIN_DOOR_THICKNESS/2.0 * u;

                let left_id = site_id.next().unwrap();
                cabin_anchors.insert(left_id, Anchor::CategorizedTranslate2D(
                    Categorized::new(l_anchor)
                    .with_floor(l_anchor - d_floor)
                ));

                let right_id = site_id.next().unwrap();
                cabin_anchors.insert(right_id, Anchor::CategorizedTranslate2D(
                    Categorized::new(r_anchor)
                    .with_floor(r_anchor - d_floor)
                ));

                level_door_anchors.insert(placement.door, [left_id, right_id].into());
            }

            level_door_anchors
        };

        Ok(SiteLift {
            cabin_doors,
            properties: LiftProperties {
                name: lift_name,
                reference_anchors,
                cabin,
                level_doors: LevelDoors {
                    visit: level_visit_doors,
                    reference_anchors: level_door_anchors,
                },
                is_static: !self.plugins,
                initial_level: level_name_to_id.get(&self.initial_floor_name),
            },
            cabin_anchors,
        })
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
