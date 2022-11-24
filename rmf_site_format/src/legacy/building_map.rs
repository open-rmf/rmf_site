use super::{crowd_sim::CrowdSim, level::Level, lift::Lift, PortingError, Result};
use crate::{
    legacy::optimization::align_building, Anchor, Angle, AssetSource, AssociatedGraphs, Category,
    DisplayColor, Dock as SiteDock, Drawing as SiteDrawing, DrawingMarker,
    Fiducial as SiteFiducial, FiducialMarker, IsStatic, Label, Lane as SiteLane, LaneMarker,
    Level as SiteLevel, LevelProperties as SiteLevelProperties, Lift as SiteLift, LiftProperties,
    Motion, NameInSite, NavGraph, OrientationConstraint, PixelsPerMeter, Pose, ReverseLane,
    Rotation, Site, SiteProperties,
};
use glam::{DAffine2, DMat3, DQuat, DVec2, DVec3, EulerRot};
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
        let alignments = align_building(&map);

        let get_delta_yaw = |tf: &DAffine2| {
            DQuat::from_mat3(&DMat3::from_cols(
                tf.matrix2.col(0).extend(0.0).normalize(),
                tf.matrix2.col(1).extend(0.0).normalize(),
                DVec3::Z,
            ))
            .to_euler(EulerRot::ZYX)
            .0
        };

        for (level_name, level) in map.levels.iter_mut() {
            let alignment = alignments.get(level_name).unwrap();
            let tf = alignment.to_affine();
            level.alignment = Some(alignment.clone());
            for v in &mut level.vertices {
                let p = tf.transform_point2(v.to_vec());
                v.0 = p.x as f64;
                v.1 = -p.y as f64;
            }

            let delta_yaw = get_delta_yaw(&tf);

            for model in &mut level.models {
                let p = tf.transform_point2(model.to_vec());
                model.x = p.x;
                model.y = -p.y;
                model.yaw -= delta_yaw;
            }

            for camera in &mut level.physical_cameras {
                let p = tf.transform_point2(camera.to_vec());
                camera.x = p.x;
                camera.y = -p.y;
                camera.yaw -= delta_yaw;
            }

            for light in &mut level.lights {
                let p = tf.transform_point2(DVec2::new(
                    light.pose.trans[0] as f64,
                    light.pose.trans[1] as f64,
                ));
                light.pose.trans[0] = p.x as f32;
                light.pose.trans[1] = -p.y as f32;
                light.pose.rot.apply_yaw(Angle::Rad(-delta_yaw as f32));
            }

            for fiducial in &mut level.fiducials {
                let p = tf.transform_point2(fiducial.to_vec());
                fiducial.0 = p.x;
                fiducial.1 = -p.y;
            }
        }

        for (lift_name, lift) in map.lifts.iter_mut() {
            let tf = alignments
                .get(&lift.reference_floor_name)
                .unwrap()
                .to_affine();
            let p = tf.transform_point2(lift.to_vec());
            lift.x = p.x;
            lift.y = -p.y;
            lift.yaw -= get_delta_yaw(&tf);
        }

        map.coordinate_system = CoordinateSystem::CartesianMeters;
        map
    }

    pub fn to_site(&self) -> Result<Site> {
        let mut site_id = 0_u32..;
        let mut site_anchors = BTreeMap::new();
        let mut levels = BTreeMap::new();
        let mut level_name_to_id = BTreeMap::new();
        let mut lanes = BTreeMap::<u32, SiteLane<u32>>::new();
        let mut locations = BTreeMap::new();

        let mut lift_cabin_anchors: BTreeMap<String, Vec<(u32, Anchor)>> = BTreeMap::new();

        let mut building_id_to_nav_graph_id = HashMap::new();

        for (name, level) in &self.levels {
            let mut vertex_to_anchor_id: HashMap<usize, u32> = Default::default();
            let mut anchors: BTreeMap<u32, Anchor> = BTreeMap::new();
            for (i, v) in level.vertices.iter().enumerate() {
                let anchor_id = if v.4.lift_cabin.is_empty() {
                    // This is a regular level anchor, not inside a lift cabin
                    let anchor_id = site_id.next().unwrap();
                    let anchor = [v.0 as f32, v.1 as f32];
                    anchors.insert(anchor_id, anchor.into());
                    anchor_id
                } else {
                    let lift = self
                        .lifts
                        .get(&v.4.lift_cabin.1)
                        .ok_or(PortingError::InvalidLiftName(v.4.lift_cabin.1.clone()))?;
                    let lift_cabin_anchors = lift_cabin_anchors
                        .entry(v.4.lift_cabin.1.clone())
                        .or_default();
                    let x = v.0 as f32 - lift.x as f32;
                    let y = v.1 as f32 - lift.y as f32;
                    if let Some(duplicate) = lift_cabin_anchors.iter().next() {
                        // This is a duplicate cabin anchor so we return its
                        // existing ID
                        duplicate.0
                    } else {
                        // This is a new cabin anchor so we need to create an
                        // ID for it
                        let anchor_id = site_id.next().unwrap();
                        lift_cabin_anchors.push((anchor_id, [x, y].into()));
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
                let (pose, pixels_per_meter) = if let Some(a) = level.alignment {
                    let p = a.translation;
                    let pose = Pose {
                        trans: [p.x as f32, -p.y as f32, 0.0001 as f32],
                        rot: Rotation::Yaw(Angle::Rad(a.rotation as f32)),
                    };
                    (pose, PixelsPerMeter((1.0 / a.scale) as f32))
                } else {
                    (Pose::default(), PixelsPerMeter::default())
                };
                drawings.insert(
                    site_id.next().unwrap(),
                    SiteDrawing {
                        source: AssetSource::Local(level.drawing.filename.clone()),
                        pose,
                        pixels_per_meter,
                        marker: DrawingMarker,
                    },
                );
            }

            let mut fiducials = BTreeMap::new();
            for fiducial in &level.fiducials {
                let anchor_id = site_id.next().unwrap();
                anchors.insert(anchor_id, [fiducial.0 as f32, fiducial.1 as f32].into());
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

            let mut lights = BTreeMap::new();
            for light in &level.lights {
                lights.insert(site_id.next().unwrap(), light.clone());
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
                    lights,
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

                let graph_id = building_id_to_nav_graph_id
                    .entry(lane.2.graph_idx.1)
                    .or_insert(site_id.next().unwrap());

                let site_lane = SiteLane {
                    anchors: [left, right].into(),
                    forward: motion,
                    reverse,
                    graphs: AssociatedGraphs::Only([*graph_id].into()),
                    marker: LaneMarker,
                };

                lanes.insert(site_id.next().unwrap(), site_lane);
            }
        }

        let mut lifts = BTreeMap::new();
        for (name, lift) in &self.lifts {
            let lift_id = site_id.next().unwrap();
            lifts.insert(
                lift_id,
                lift.to_site(
                    name,
                    &mut site_id,
                    &mut site_anchors,
                    &levels,
                    &level_name_to_id,
                    &lift_cabin_anchors,
                )?,
            );
        }

        let mut nav_graphs = BTreeMap::new();
        let default_nav_graph_colors = [
            [1.0, 0.5, 0.3, 1.0],
            [0.6, 1.0, 0.5, 1.0],
            [0.6, 0.8, 1.0, 1.0],
            [0.6, 0.2, 0.3, 1.0],
            [0.1, 0.0, 1.0, 1.0],
            [0.8, 0.4, 0.5, 1.0],
            [0.9, 1.0, 0.0, 1.0],
            [0.7, 0.5, 0.1, 1.0],
        ];
        for (i, (_, graph_id)) in building_id_to_nav_graph_id.iter().enumerate() {
            let color_index = i % default_nav_graph_colors.len();
            nav_graphs.insert(
                *graph_id,
                NavGraph {
                    name: NameInSite("unnamed_graph_#".to_string() + &i.to_string()),
                    color: DisplayColor(default_nav_graph_colors[color_index]),
                    marker: Default::default(),
                },
            );
        }

        Ok(Site {
            format_version: Default::default(),
            anchors: site_anchors,
            properties: SiteProperties {
                name: self.name.clone(),
            },
            levels,
            lifts,
            lanes,
            locations,
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
