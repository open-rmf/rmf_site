use super::{PortingError, Result};
use crate::{
    Anchor, DoorType, DoubleSlidingDoor, Edge, InitialLevel, IsStatic, LevelVisits,
    Lift as SiteLift, LiftCabin, LiftCabinDoor, LiftCabinDoorPlacement, LiftProperties, NameInSite,
    RectFace, RectangularLiftCabin, SiteID,
};
use glam::DVec2;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    ops::RangeFrom,
};

#[derive(Deserialize, Serialize, Clone)]
pub struct LiftDoor {
    pub door_type: i32,
    pub motion_axis_orientation: f32,
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
    pub fn to_vec(&self) -> DVec2 {
        DVec2::new(self.x, self.y)
    }

    pub fn calculate_anchors(&self) -> [[f32; 2]; 2] {
        // TODO(MXG): Rewrite this with glam now that we've accepted it as a dependency
        let x = self.x as f32;
        let y = self.y as f32;
        // NOTE: Coordinate axes changed between the legacy format and the
        // new format. Width used to be along the local x axis, and depth
        // used to be along the local y axis, but now that is flipped to
        // better align with the robotics convention of the local x axis
        // being forward/backward while the local y axis is the lateral
        // direction (side-to-side).
        let d = self.width as f32 / 2.0;
        let w = self.depth as f32 / 2.0;
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
        site_anchors: &mut BTreeMap<SiteID, Anchor>,
        level_name_to_id: &BTreeMap<String, SiteID>,
        all_lift_cabin_anchors: &BTreeMap<String, Vec<(SiteID, Anchor)>>,
    ) -> Result<SiteLift> {
        let ref_anchor_positions = self.calculate_anchors();
        let reference_anchors = {
            let left = SiteID::from(site_id.next().unwrap());
            let right = SiteID::from(site_id.next().unwrap());
            site_anchors.insert(left, ref_anchor_positions[0].into());
            site_anchors.insert(right, ref_anchor_positions[1].into());
            Edge::new(left, right)
        };

        if self.doors.len() > 4 {
            return Err(PortingError::InvalidLiftCabinDoorCount {
                lift: lift_name.clone(),
                door_count: self.doors.len(),
            });
        }

        let mut cabin_door_name_to_id = HashMap::new();
        let mut front_door = None;
        let mut back_door = None;
        let mut left_door = None;
        let mut right_door = None;

        for (door_name, door) in &self.doors {
            let id = SiteID::from(site_id.next().unwrap());
            cabin_door_name_to_id.insert(door_name.clone(), id);

            let dx = door.x as f32;
            let dy = door.y as f32;
            // NOTE: Coordinate axes changed between the legacy format and the
            // new format. Width used to be along the local x axis, and depth
            // used to be along the local y axis, but now that is flipped to
            // better align with the robotics convention of the local x axis
            // being forward/backward while the local y axis is the lateral
            // direction (side-to-side).
            let half_width = self.depth as f32 / 2.0;
            let half_depth = self.width as f32 / 2.0;

            let cabin_face = if dx.abs() < 1e-3 {
                // Very small x value means the door must be on the left or right face
                if dy >= half_width {
                    // Positive y means left door
                    RectFace::Left
                } else if dy <= -half_width {
                    // Negative y means right door
                    RectFace::Right
                } else {
                    return Err(PortingError::InvalidLiftCabinDoorPlacement {
                        lift: lift_name.clone(),
                        door: door_name.clone(),
                    });
                }
            } else {
                let m = dy / dx;
                let y_intercept = m * half_depth;
                if y_intercept.abs() <= half_width {
                    // The door must be on the front or back face
                    if dx >= half_depth {
                        // Positive x means front door
                        RectFace::Front
                    } else if dx <= -half_depth {
                        RectFace::Back
                    } else {
                        return Err(PortingError::InvalidLiftCabinDoorPlacement {
                            lift: lift_name.clone(),
                            door: door_name.clone(),
                        });
                    }
                } else {
                    // The door must be on the left or right face
                    if dy >= half_width {
                        RectFace::Left
                    } else if dy <= half_width {
                        RectFace::Right
                    } else {
                        return Err(PortingError::InvalidLiftCabinDoorPlacement {
                            lift: lift_name.clone(),
                            door: door_name.clone(),
                        });
                    }
                }
            };

            let width = door.width as f32;
            match cabin_face {
                RectFace::Front => {
                    if front_door.is_some() {
                        return Err(PortingError::DuplicateLiftCabinDoor {
                            lift: lift_name.clone(),
                            face: "front",
                        });
                    }
                    front_door = Some(LiftCabinDoorPlacement {
                        door: id,
                        width,
                        thickness: None,
                        shifted: Some(dy),
                        custom_gap: Some(dx - half_depth),
                    });
                }
                RectFace::Back => {
                    if back_door.is_some() {
                        return Err(PortingError::DuplicateLiftCabinDoor {
                            lift: lift_name.clone(),
                            face: "back",
                        });
                    }
                    back_door = Some(LiftCabinDoorPlacement {
                        door: id,
                        width,
                        thickness: None,
                        shifted: Some(-dy),
                        custom_gap: Some(-dx - half_depth),
                    })
                }
                RectFace::Left => {
                    if left_door.is_some() {
                        return Err(PortingError::DuplicateLiftCabinDoor {
                            lift: lift_name.clone(),
                            face: "left",
                        });
                    }
                    left_door = Some(LiftCabinDoorPlacement {
                        door: id,
                        width,
                        thickness: None,
                        shifted: Some(-dx),
                        custom_gap: Some((dy - half_width) as f32),
                    });
                }
                RectFace::Right => {
                    if right_door.is_some() {
                        return Err(PortingError::DuplicateLiftCabinDoor {
                            lift: lift_name.clone(),
                            face: "right",
                        });
                    }
                    right_door = Some(LiftCabinDoorPlacement {
                        door: id,
                        width,
                        thickness: None,
                        shifted: Some(dx),
                        custom_gap: Some(-dy - half_width),
                    });
                }
            }
        }

        // NOTE: Coordinate axes changed between the legacy format and the
        // new format. Width used to be along the local x axis, and depth
        // used to be along the local y axis, but now that is flipped to
        // better align with the robotics convention of the local x axis
        // being forward/backward while the local y axis is the lateral
        // direction (side-to-side).
        let width = self.depth as f32;
        let depth = self.width as f32;
        let cabin = RectangularLiftCabin {
            width,
            depth,
            wall_thickness: None,
            gap: None,
            shift: None,
            front_door,
            back_door,
            left_door,
            right_door,
        };

        let door_level_visits = {
            let mut door_level_visits: BTreeMap<SiteID, LevelVisits> = BTreeMap::new();
            for (level_name, door_names) in &self.level_doors {
                let level = *level_name_to_id
                    .get(level_name)
                    .ok_or(PortingError::InvalidLevelName(level_name.clone()))?;

                for door_name in door_names {
                    let door = cabin_door_name_to_id.get(door_name).ok_or(
                        PortingError::InvalidLiftCabinDoorName {
                            lift: lift_name.clone(),
                            door: door_name.clone(),
                        },
                    )?;

                    door_level_visits.entry(*door).or_default().0.insert(level);
                }
            }
            door_level_visits
        };

        let mut cabin_anchors: BTreeMap<SiteID, Anchor> = [all_lift_cabin_anchors.get(lift_name)]
            .into_iter()
            .filter_map(|x| x)
            .flat_map(|x| x)
            .cloned()
            .collect();

        let cabin_doors = {
            let mut cabin_doors = BTreeMap::new();
            for face in RectFace::iter_all() {
                if let (Some([left, right]), Some(p)) =
                    (cabin.level_door_anchors(face), cabin.door(face))
                {
                    let left_id = SiteID::from(site_id.next().unwrap());
                    let right_id = SiteID::from(site_id.next().unwrap());
                    cabin_anchors.insert(left_id, left);
                    cabin_anchors.insert(right_id, right);
                    cabin_doors.insert(
                        p.door,
                        LiftCabinDoor {
                            kind: DoorType::DoubleSliding(DoubleSlidingDoor::default()),
                            reference_anchors: [left_id, right_id].into(),
                            visits: door_level_visits
                                .get(&p.door)
                                .cloned()
                                .unwrap_or(LevelVisits::default()),
                            marker: Default::default(),
                        },
                    );
                }
            }
            cabin_doors
        };

        let cabin = LiftCabin::Rect(cabin);
        Ok(SiteLift {
            cabin_doors,
            properties: LiftProperties {
                name: NameInSite(lift_name.clone()),
                reference_anchors,
                cabin,
                is_static: IsStatic(!self.plugins),
                initial_level: InitialLevel(
                    level_name_to_id.get(&self.initial_floor_name).copied(),
                ),
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
