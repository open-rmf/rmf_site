use super::{level::Level, lift::Lift, PortingError, Result};
use crate::{
    alignment::align_legacy_building, Affiliation, Anchor, Angle, AssetSource, AssociatedGraphs,
    Category, DisplayColor, Dock as SiteDock, Drawing as SiteDrawing, DrawingProperties,
    Fiducial as SiteFiducial, FiducialGroup, FiducialMarker, Guided, Lane as SiteLane, LaneMarker,
    Level as SiteLevel, LevelElevation, LevelProperties as SiteLevelProperties, Motion, NameInSite,
    NameOfSite, NavGraph, Navigation, OrientationConstraint, PixelsPerMeter, Pose,
    PreferredSemiTransparency, RankingsInLevel, ReverseLane, Rotation, Site, SiteProperties,
    DEFAULT_NAV_GRAPH_COLORS,
};
use glam::{DAffine2, DMat3, DQuat, DVec2, DVec3, EulerRot};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

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
    // TODO(MXG): Consider parsing legacy crowdsim data and converting it to
    // a format that will have future support.
    // #[serde(default)]
    // pub crowd_sim: CrowdSim,
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
        let alignments = align_legacy_building(&map);

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
            // We need to keep the vertices associated with measurements in image coordinates
            let mut drawing_vertices = HashSet::new();
            for measurement in &level.measurements {
                drawing_vertices.insert(measurement.0);
                drawing_vertices.insert(measurement.1);
            }
            for (idx, v) in level.vertices.iter_mut().enumerate() {
                if drawing_vertices.contains(&idx) {
                    v.1 = -v.1;
                } else {
                    let p = tf.transform_point2(v.to_vec());
                    v.0 = p.x as f64;
                    v.1 = -p.y as f64;
                }
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
                fiducial.1 = -fiducial.1;
            }

            for feature in &mut level.features {
                feature.y = -feature.y;
            }

            for (_, layer) in &mut level.layers {
                for feature in &mut layer.features {
                    feature.y = -feature.y;
                }
            }
        }

        for (_, lift) in map.lifts.iter_mut() {
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

        let mut fiducial_groups: BTreeMap<u32, FiducialGroup> = BTreeMap::new();
        let mut cartesian_fiducials: HashMap<u32, Vec<DVec2>> = HashMap::new();
        for (level_name, level) in &self.levels {
            let level_id = site_id.next().unwrap();
            let mut vertex_to_anchor_id: HashMap<usize, u32> = Default::default();
            let mut level_anchors: BTreeMap<u32, Anchor> = BTreeMap::new();
            for (i, v) in level.vertices.iter().enumerate() {
                let anchor_id = if v.4.lift_cabin.is_empty() {
                    // This is a regular level anchor, not inside a lift cabin
                    let anchor_id = site_id.next().unwrap();
                    let anchor = [v.0 as f32, v.1 as f32];
                    level_anchors.insert(anchor_id, anchor.into());
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

            let mut rankings = RankingsInLevel::default();
            let mut drawings = BTreeMap::new();
            let mut feature_info = HashMap::new();
            let mut primary_drawing_info = None;
            if !level.drawing.filename.is_empty() {
                let drawing_id = site_id.next().unwrap();
                let drawing_name = Path::new(&level.drawing.filename)
                    .file_stem()
                    .unwrap_or_default()
                    .to_str()
                    .unwrap()
                    .to_string();
                let (pose, pixels_per_meter) = if let Some(a) = level.alignment {
                    let p = a.translation;
                    let pose = Pose {
                        trans: [p.x as f32, -p.y as f32, 0.0 as f32],
                        rot: Rotation::Yaw(Angle::Rad(a.rotation as f32)),
                    };
                    (pose, PixelsPerMeter((1.0 / a.scale) as f32))
                } else {
                    (Pose::default(), PixelsPerMeter::default())
                };

                let mut drawing_anchors = BTreeMap::new();
                let mut drawing_fiducials = BTreeMap::new();

                // Use this transform to create anchors that pin the main
                // drawing to where it belongs in Cartesian coordinates.
                let drawing_tf = DAffine2::from_scale_angle_translation(
                    DVec2::splat(1.0 / pixels_per_meter.0 as f64),
                    pose.trans[2] as f64,
                    DVec2::new(pose.trans[0] as f64, pose.trans[1] as f64),
                );
                primary_drawing_info = Some((drawing_id, drawing_tf));

                for fiducial in &level.fiducials {
                    let anchor_id = site_id.next().unwrap();
                    // Do not add this anchor to the vertex_to_anchor_id map
                    // because this fiducial is not really recognized as a
                    // vertex to the building format.
                    drawing_anchors
                        .insert(anchor_id, [fiducial.0 as f32, fiducial.1 as f32].into());
                    let affiliation = if fiducial.2.is_empty() {
                        // We assume an empty reference name means this fiducial
                        // is not really being used.
                        Affiliation(None)
                    } else {
                        let name = &fiducial.2;
                        let group_id = if let Some((group_id, _)) = fiducial_groups
                            .iter()
                            .find(|(_, group)| group.name.0 == *name)
                        {
                            // The group already exists
                            *group_id
                        } else {
                            // The group does not exist yet, so let's create it
                            let group_id = site_id.next().unwrap();
                            fiducial_groups
                                .insert(group_id, FiducialGroup::new(NameInSite(name.clone())));
                            group_id
                        };
                        let drawing_coords = DVec2::new(fiducial.0, fiducial.1);
                        cartesian_fiducials
                            .entry(group_id)
                            .or_default()
                            .push(drawing_tf.transform_point2(drawing_coords));

                        Affiliation(Some(group_id))
                    };
                    drawing_fiducials.insert(
                        site_id.next().unwrap(),
                        SiteFiducial {
                            affiliation,
                            anchor: anchor_id.into(),
                            marker: FiducialMarker,
                        },
                    );
                }

                for feature in &level.features {
                    // Do not add this anchor to the vertex_to_anchor_id map because
                    // this fiducial is not really recognized as a vertex to the
                    // building format.
                    let anchor_id = site_id.next().unwrap();
                    let fiducial_id = site_id.next().unwrap();
                    drawing_anchors.insert(anchor_id, [feature.x as f32, feature.y as f32].into());

                    drawing_fiducials.insert(
                        fiducial_id,
                        SiteFiducial {
                            affiliation: Default::default(),
                            anchor: anchor_id.into(),
                            marker: FiducialMarker,
                        },
                    );

                    feature_info.insert(
                        feature.id.clone(),
                        FeatureInfo {
                            fiducial_id,
                            on_anchor: anchor_id,
                            in_drawing: drawing_id,
                            name: (!feature.name.is_empty()).then(|| feature.name.clone()),
                        },
                    );
                }

                let mut measurements = BTreeMap::new();
                for measurement in &level.measurements {
                    let mut site_measurement = measurement.to_site(&vertex_to_anchor_id)?;
                    let edge = &mut site_measurement.anchors;
                    let (start_anchor, end_anchor) = (
                        level_anchors.get(&edge.left()).unwrap(),
                        level_anchors.get(&edge.right()).unwrap(),
                    );
                    // Now get the anchors and duplicate them in the drawing
                    let anchor_id = site_id.next().unwrap();
                    drawing_anchors.insert(anchor_id, start_anchor.clone());
                    *edge.left_mut() = anchor_id;
                    let anchor_id = site_id.next().unwrap();
                    drawing_anchors.insert(anchor_id, end_anchor.clone());
                    *edge.right_mut() = anchor_id;
                    measurements.insert(site_id.next().unwrap(), site_measurement);
                    // TODO(luca) remove original anchors if they have no other dependents
                    // TODO(MXG): Have rankings for measurements
                }

                drawings.insert(
                    drawing_id,
                    SiteDrawing {
                        properties: DrawingProperties {
                            name: NameInSite(drawing_name),
                            source: AssetSource::Local(level.drawing.filename.clone()),
                            pose,
                            pixels_per_meter,
                            preferred_semi_transparency: PreferredSemiTransparency::for_drawing(),
                        },
                        anchors: drawing_anchors,
                        fiducials: drawing_fiducials,
                        measurements,
                    },
                );
                rankings.drawings.push(drawing_id);
            }

            for (name, layer) in &level.layers {
                // TODO(luca) coordinates in site and traffic editor might be different, use
                // optimization engine instead of parsing
                let drawing_id = site_id.next().unwrap();
                let pose = Pose {
                    trans: [
                        layer.transform.translation_x as f32,
                        layer.transform.translation_y as f32,
                        0.0 as f32,
                    ],
                    rot: Rotation::Yaw(Angle::Rad(layer.transform.yaw as f32)),
                };
                rankings.drawings.push(drawing_id);
                let mut drawing_anchors = BTreeMap::new();
                let mut drawing_fiducials = BTreeMap::new();
                for feature in &layer.features {
                    // Do not add this anchor to the vertex_to_anchor_id map because
                    // this fiducial is not really recognized as a vertex to the
                    // building format.
                    let anchor_id = site_id.next().unwrap();
                    let fiducial_id = site_id.next().unwrap();
                    drawing_anchors.insert(anchor_id, [feature.x as f32, feature.y as f32].into());

                    drawing_fiducials.insert(
                        fiducial_id,
                        SiteFiducial {
                            affiliation: Default::default(),
                            anchor: anchor_id.into(),
                            marker: FiducialMarker,
                        },
                    );

                    feature_info.insert(
                        feature.id.clone(),
                        FeatureInfo {
                            fiducial_id,
                            on_anchor: anchor_id,
                            in_drawing: drawing_id,
                            name: (!feature.name.is_empty()).then(|| feature.name.clone()),
                        },
                    );
                }

                drawings.insert(
                    drawing_id,
                    SiteDrawing {
                        properties: DrawingProperties {
                            name: NameInSite(name.clone()),
                            source: AssetSource::Local(layer.filename.clone()),
                            pose,
                            pixels_per_meter: PixelsPerMeter((1.0 / layer.transform.scale) as f32),
                            preferred_semi_transparency: PreferredSemiTransparency::for_drawing(),
                        },
                        anchors: drawing_anchors,
                        fiducials: drawing_fiducials,
                        measurements: Default::default(),
                    },
                );
            }

            for (i, constraint) in level.constraints.iter().enumerate() {
                let fiducial_group_id = site_id.next().unwrap();
                let group_name = constraint
                    .ids
                    .iter()
                    .find_map(|id| {
                        if let Some(info) = feature_info.get(id) {
                            info.name.clone()
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| format!("{}_constraint_{i}", level_name));
                fiducial_groups.insert(
                    fiducial_group_id,
                    FiducialGroup::new(NameInSite(group_name)),
                );

                for feature_id in &constraint.ids {
                    if let Some(info) = feature_info.get(feature_id) {
                        if let Some(drawing) = drawings.get_mut(&info.in_drawing) {
                            if let Some(fiducial) = drawing.fiducials.get_mut(&info.fiducial_id) {
                                fiducial.affiliation = Affiliation(Some(fiducial_group_id));
                            }
                            // Add a level anchor to pin this feature
                            if let Some((primary_drawing_id, drawing_tf)) = primary_drawing_info {
                                if info.in_drawing == primary_drawing_id {
                                    let anchor_tf = drawing
                                        .anchors
                                        .get(&info.on_anchor)
                                        .unwrap()
                                        .translation_for_category(Category::General);
                                    let drawing_coords =
                                        DVec2::new(anchor_tf[0] as f64, anchor_tf[1] as f64);
                                    cartesian_fiducials
                                        .entry(fiducial_group_id)
                                        .or_default()
                                        .push(drawing_tf.transform_point2(drawing_coords));
                                }
                            }
                        }
                    }
                }
            }

            let mut floors = BTreeMap::new();
            for floor in &level.floors {
                let site_floor = floor.to_site(&vertex_to_anchor_id)?;
                let id = site_id.next().unwrap();
                floors.insert(id, site_floor);
                rankings.floors.push(id);
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

            level_name_to_id.insert(level_name.clone(), level_id);
            levels.insert(
                level_id,
                SiteLevel {
                    properties: SiteLevelProperties {
                        name: NameInSite(level_name.clone()),
                        elevation: LevelElevation(elevation),
                        global_floor_visibility: Default::default(),
                        global_drawing_visibility: Default::default(),
                    },
                    anchors: level_anchors,
                    doors,
                    drawings,
                    floors,
                    lights,
                    models,
                    physical_cameras,
                    walls,
                    rankings,
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
                    &level_name_to_id,
                    &lift_cabin_anchors,
                )?,
            );
        }

        let mut nav_graphs = BTreeMap::new();
        for (i, (_, graph_id)) in building_id_to_nav_graph_id.iter().enumerate() {
            let color_index = i % DEFAULT_NAV_GRAPH_COLORS.len();
            nav_graphs.insert(
                *graph_id,
                NavGraph {
                    name: NameInSite("unnamed_graph_#".to_string() + &i.to_string()),
                    color: DisplayColor(DEFAULT_NAV_GRAPH_COLORS[color_index]),
                    marker: Default::default(),
                },
            );
        }

        let cartesian_fiducials: BTreeMap<u32, SiteFiducial<u32>> = cartesian_fiducials
            .into_iter()
            .map(|(group_id, locations)| {
                let p = locations
                    .iter()
                    .fold(DVec2::ZERO, |base, next| base + *next)
                    / locations.len() as f64;
                let anchor_id = site_id.next().unwrap();
                site_anchors.insert(anchor_id, [p.x as f32, p.y as f32].into());
                let fiducial_id = site_id.next().unwrap();
                (
                    fiducial_id,
                    SiteFiducial {
                        anchor: anchor_id.into(),
                        affiliation: Affiliation(Some(group_id)),
                        marker: FiducialMarker,
                    },
                )
            })
            .collect();

        Ok(Site {
            format_version: Default::default(),
            anchors: site_anchors,
            properties: SiteProperties {
                name: NameOfSite(self.name.clone()),
            },
            levels,
            lifts,
            fiducial_groups,
            fiducials: cartesian_fiducials,
            navigation: Navigation {
                guided: Guided {
                    graphs: nav_graphs,
                    ranking: Vec::new(),
                    lanes,
                    locations,
                },
            },
            agents: Default::default(),
        })
    }
}

struct FeatureInfo {
    fiducial_id: u32,
    on_anchor: u32,
    in_drawing: u32,
    /// name comes from the `name` field of features, if it has one
    name: Option<String>,
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
