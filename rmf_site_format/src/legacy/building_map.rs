use super::{crowd_sim::CrowdSim, level::Level, lift::Lift, PortingError, Result};
use crate::{
    Dock as SiteDock, Drawing as SiteDrawing, DrawingMarker, DrawingSource,
    Fiducial as SiteFiducial, FiducialMarker, IsStatic, Label, Lane as SiteLane, LaneMarker,
    Level as SiteLevel, LevelDoors, LevelProperties as SiteLevelProperties, Lift as SiteLift,
    LiftProperties, Motion, NameInSite, NavGraph, NavGraphProperties, OrientationConstraint, Pose,
    ReverseLane, Site, SiteProperties,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum CoordinateSystem {
    ReferenceImage,
    CartesianMeters,
}

impl Default for CoordinateSystem {
    fn default() -> Self {
        CoordinateSystem::ReferenceImage
    }
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct BuildingMap {
    pub name: String,
    #[serde(default)]
    pub coordinate_system: CoordinateSystem,
    pub levels: BTreeMap<String, Level>,
    #[serde(default)]
    pub crowd_sim: CrowdSim,
    #[serde(default)]
    pub lifts: BTreeMap<String, Lift>,
}

impl BuildingMap {
    pub fn from_bytes(data: &[u8]) -> serde_yaml::Result<BuildingMap> {
        let map: BuildingMap = serde_yaml::from_slice(data)?;
        match map.coordinate_system {
            CoordinateSystem::ReferenceImage => Ok(BuildingMap::from_pixel_coordinates(map)),
            CoordinateSystem::CartesianMeters => Ok(map),
        }
    }

    /// Converts a map from the oldest legacy format, which uses pixel coordinates.
    fn from_pixel_coordinates(mut map: BuildingMap) -> BuildingMap {
        for (_, level) in map.levels.iter_mut() {
            // todo: calculate scale and inter-level alignment
            let mut ofs_x = 0.0;
            let mut ofs_y = 0.0;
            let mut num_v = 0;
            for v in &level.vertices {
                ofs_x += v.0;
                ofs_y += -v.1;
                num_v += 1;
            }
            ofs_x /= num_v as f64;
            ofs_y /= num_v as f64;

            // try to guess the scale by averaging the measurement distances.
            let mut n_dist = 0;
            let mut sum_dist = 0.;
            for meas in &level.measurements {
                let dx_raw = level.vertices[meas.0].0 - level.vertices[meas.1].0;
                let dy_raw = level.vertices[meas.0].1 - level.vertices[meas.1].1;
                let dist_raw = (dx_raw * dx_raw + dy_raw * dy_raw).sqrt();
                let dist_meters = *meas.2.distance;
                sum_dist += dist_meters / dist_raw;
                n_dist += 1;
            }
            let scale = match n_dist {
                0 => 1.0,
                _ => sum_dist / n_dist as f64,
            };

            // convert to meters
            for v in level.vertices.iter_mut() {
                v.0 = (v.0 - ofs_x) * scale;
                v.1 = (-v.1 - ofs_y) * scale;
            }

            for m in level.models.iter_mut() {
                m.x = (m.x - ofs_x) * scale;
                m.y = (-m.y - ofs_y) * scale;
            }
        }
        map.coordinate_system = CoordinateSystem::CartesianMeters;
        map
    }

    pub fn to_site(&self) -> Result<Site> {
        let mut site_id = 0_u32..;
        let mut levels = BTreeMap::new();
        let mut level_name_to_id = BTreeMap::new();
        let mut nav_graph_lanes = HashMap::<i64, Vec<SiteLane<u32>>>::new();
        // Note: In the old format, all Locations are effectively "visible" to
        // all nav graphs, but may be unreachable to some, and that is figured
        // out at RMF runtime.
        let mut locations = BTreeMap::new();

        let mut lift_cabin_anchors: BTreeMap<String, Vec<(u32, [f32; 2])>> = BTreeMap::new();

        for (name, level) in &self.levels {
            let mut vertex_to_anchor_id: HashMap<usize, u32> = Default::default();
            let mut anchors = BTreeMap::new();
            for (i, v) in level.vertices.iter().enumerate() {
                let anchor_id = if v.4.lift_cabin.is_empty() {
                    // This is a regular level anchor, not inside a lift cabin
                    let anchor_id = site_id.next().unwrap();
                    let anchor = [v.0 as f32, v.1 as f32];
                    anchors.insert(anchor_id, anchor);
                    anchor_id
                } else {
                    let lift_cabin_anchors = lift_cabin_anchors
                        .entry(v.4.lift_cabin.1.clone())
                        .or_default();
                    if let Some(duplicate) = lift_cabin_anchors.iter().find(|(_, [x, y])| {
                        let dx = v.0 as f32 - *x;
                        let dy = v.1 as f32 - *y;
                        (dx * dx + dy * dy).sqrt() < 0.01
                    }) {
                        // This is a duplicate cabin anchor so we return its
                        // existing ID
                        duplicate.0
                    } else {
                        // This is a new cabin anchor so we need to create an
                        // ID for it
                        let anchor_id = site_id.next().unwrap();
                        lift_cabin_anchors.push((anchor_id, [v.0 as f32, v.1 as f32]));
                        anchor_id
                    }
                };

                vertex_to_anchor_id.insert(i, anchor_id);
                if let Some(location) = v.make_location(anchor_id) {
                    locations.insert(site_id.next().unwrap(), location);
                }
            }

            let mut doors = BTreeMap::new();
            for door in &level.doors {
                let site_door = door.to_site(&vertex_to_anchor_id)?;
                doors.insert(site_id.next().unwrap(), site_door);
            }

            let mut drawings = BTreeMap::new();
            if !level.drawing.filename.is_empty() {
                drawings.insert(
                    site_id.next().unwrap(),
                    SiteDrawing {
                        source: DrawingSource::Filename(level.drawing.filename.clone()),
                        pose: Pose::default(),
                        marker: DrawingMarker,
                    },
                );
            }

            let mut fiducials = BTreeMap::new();
            for fiducial in &level.fiducials {
                let anchor_id = site_id.next().unwrap();
                anchors.insert(anchor_id, [fiducial.0 as f32, fiducial.1 as f32]);
                // Do not add this anchor to the vertex_to_anchor_id map because
                // this fiducial is not really recognized as a vertex to the
                // building format.
                fiducials.insert(
                    site_id.next().unwrap(),
                    SiteFiducial {
                        label: if fiducial.2.is_empty() {
                            Label(None)
                        } else {
                            Label(Some(fiducial.2.clone()))
                        },
                        anchor: anchor_id.into(),
                        marker: FiducialMarker,
                    },
                );
            }

            let mut floors = BTreeMap::new();
            for floor in &level.floors {
                let site_floor = floor.to_site(&vertex_to_anchor_id)?;
                floors.insert(site_id.next().unwrap(), site_floor);
            }

            let mut measurements = BTreeMap::new();
            for measurement in &level.measurements {
                let site_measurement = measurement.to_site(&vertex_to_anchor_id)?;
                measurements.insert(site_id.next().unwrap(), site_measurement);
            }

            let mut models = BTreeMap::new();
            for model in &level.models {
                models.insert(site_id.next().unwrap(), model.to_site());
            }

            let mut physical_cameras = BTreeMap::new();
            for cam in &level.physical_cameras {
                physical_cameras.insert(site_id.next().unwrap(), cam.to_site());
            }

            let mut walls = BTreeMap::new();
            for wall in &level.walls {
                let site_wall = wall.to_site(&vertex_to_anchor_id)?;
                walls.insert(site_id.next().unwrap(), site_wall);
            }

            let elevation = level.elevation as f32;

            let level_id = site_id.next().unwrap();
            level_name_to_id.insert(name.clone(), level_id);
            levels.insert(
                level_id,
                SiteLevel {
                    properties: SiteLevelProperties {
                        name: name.clone(),
                        elevation,
                    },
                    anchors,
                    doors,
                    drawings,
                    fiducials,
                    floors,
                    lights: Default::default(),
                    measurements,
                    models,
                    physical_cameras,
                    walls,
                },
            );

            for lane in &level.lanes {
                let left = *vertex_to_anchor_id
                    .get(&lane.0)
                    .ok_or(PortingError::InvalidVertex(lane.0))?;
                let right = *vertex_to_anchor_id
                    .get(&lane.1)
                    .ok_or(PortingError::InvalidVertex(lane.1))?;

                let get_dock = |v: usize| {
                    let dock_name = &level.vertices.get(v).unwrap().4.dock_name.1;
                    if dock_name.is_empty() {
                        return None;
                    } else {
                        return Some(SiteDock {
                            name: dock_name.clone(),
                            duration: None,
                        });
                    }
                };

                let left_dock = get_dock(lane.0);
                let right_dock = get_dock(lane.1);

                let motion = Motion {
                    orientation_constraint: if lane.2.orientation.1 == "forward" {
                        OrientationConstraint::Forwards
                    } else if lane.2.orientation.1 == "backward" {
                        OrientationConstraint::Backwards
                    } else {
                        OrientationConstraint::None
                    },
                    speed_limit: None,
                    dock: left_dock,
                };

                let reverse = if !lane.2.bidirectional.1 {
                    ReverseLane::Disable
                } else if right_dock != motion.dock {
                    ReverseLane::Different(Motion {
                        dock: right_dock,
                        ..motion.clone()
                    })
                } else {
                    ReverseLane::Same
                };

                let site_lane = SiteLane {
                    anchors: [left, right].into(),
                    forward: motion,
                    reverse,
                    marker: LaneMarker,
                };

                nav_graph_lanes
                    .entry(lane.2.graph_idx.1)
                    .or_insert(Default::default())
                    .push(site_lane);
            }
        }

        let mut nav_graphs = BTreeMap::new();
        for (idx, lanes) in nav_graph_lanes {
            let lanes: BTreeMap<_, _> = lanes
                .into_iter()
                .map(|lane| (site_id.next().unwrap(), lane))
                .collect();

            nav_graphs.insert(
                site_id.next().unwrap(),
                NavGraph {
                    properties: NavGraphProperties {
                        name: idx.to_string(),
                    },
                    lanes,
                    locations: locations.clone(),
                },
            );
        }

        let mut lifts = BTreeMap::new();
        for (name, lift) in &self.lifts {
            let anchors = lift.calculate_anchors();
            let anchor_level_id = level_name_to_id.get(&lift.reference_floor_name).ok_or(
                PortingError::InvalidLevelName(lift.reference_floor_name.clone()),
            )?;
            let level_anchors = &mut levels.get_mut(anchor_level_id).unwrap().anchors;
            let anchors = {
                let left = site_id.next().unwrap();
                let right = site_id.next().unwrap();
                level_anchors.insert(left, anchors[0]);
                level_anchors.insert(right, anchors[1]);
                [left, right]
            };

            let cabin = lift.make_cabin(name)?;
            let mut level_doors = BTreeMap::new();
            for (level, doors) in &lift.level_doors {
                let level_id = level_name_to_id
                    .get(level)
                    .ok_or(PortingError::InvalidLevelName(level.clone()))?;

                if doors.len() != 1 {
                    return Err(PortingError::InvalidLiftLevelDoorCount {
                        lift: name.clone(),
                        level: level.clone(),
                        door_count: doors.len(),
                    });
                }

                let door_name = doors.iter().last().unwrap();
                let door_id = levels
                    .get(level_id)
                    .unwrap()
                    .doors
                    .iter()
                    .find(|(_, door)| door.name.0 == *door_name)
                    .ok_or(PortingError::InvalidLiftLevelDoorName {
                        lift: name.clone(),
                        level: level.clone(),
                        door: door_name.clone(),
                    })?
                    .0;

                level_doors.insert(*level_id, *door_id);
            }
            let level_doors = LevelDoors(level_doors);

            let cabin_anchors: BTreeMap<u32, [f32; 2]> = [lift_cabin_anchors.get(name)]
                .into_iter()
                .filter_map(|x| x)
                .flat_map(|x| x)
                .copied()
                .collect();

            lifts.insert(
                site_id.next().unwrap(),
                SiteLift {
                    properties: LiftProperties {
                        name: NameInSite(name.clone()),
                        reference_anchors: anchors.into(),
                        cabin,
                        level_doors,
                        corrections: Default::default(),
                        is_static: IsStatic(!lift.plugins),
                    },
                    cabin_anchors,
                },
            );
        }

        Ok(Site {
            format_version: Default::default(),
            properties: SiteProperties {
                name: self.name.clone(),
            },
            levels,
            lifts,
            nav_graphs,
            agents: Default::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn building_map_serialization() -> std::result::Result<(), Box<dyn Error>> {
        let data = std::fs::read("../assets/demo_maps/office.building.yaml")?;
        let map = BuildingMap::from_bytes(&data)?;
        std::fs::create_dir_all("test_output")?;
        let out_file = std::fs::File::create("test_output/office.building.yaml")?;
        serde_yaml::to_writer(out_file, &map)?;
        Ok(())
    }

    #[test]
    fn site_conversion() {
        let data = std::fs::read("../assets/demo_maps/office.building.yaml").unwrap();
        let map = BuildingMap::from_bytes(&data).unwrap();
        println!("{}", map.to_site().unwrap().to_string().unwrap());
    }

    #[test]
    fn site_yaml() {
        let data = std::fs::read("../assets/demo_maps/office.building.yaml").unwrap();
        let map = BuildingMap::from_bytes(&data).unwrap();
        println!(
            "{}",
            serde_json::to_string_pretty(&map.to_site().unwrap()).unwrap()
        );
    }
}
