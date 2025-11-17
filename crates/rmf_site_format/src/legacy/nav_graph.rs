use crate::*;
use glam::Affine2;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use tracing::error;

#[derive(Serialize, Deserialize, Clone)]
pub struct NavGraph {
    pub building_name: String,
    pub levels: HashMap<String, NavLevel>,
    pub doors: HashMap<String, NavDoor>,
    pub lifts: HashMap<String, NavLift>,
}

// Reference: https://en.wikipedia.org/wiki/Line%E2%80%93line_intersection#Given_two_points_on_each_line_segment
fn segments_intersect(p1: [f32; 2], p2: [f32; 2], p3: [f32; 2], p4: [f32; 2]) -> bool {
    // line segments are [p1-p2] and [p3-p4]
    let [x1, y1] = p1;
    let [x2, y2] = p2;
    let [x3, y3] = p3;
    let [x4, y4] = p4;
    let det = (x1 - x2) * (y3 - y4) - (y1 - y2) * (x3 - x4);
    if det.abs() < 0.01 {
        return false;
    }
    let t = ((x1 - x3) * (y3 - y4) - (y1 - y3) * (x3 - x4)) / det;
    let u = -((x1 - x2) * (y1 - y3) - (y1 - y2) * (x1 - x3)) / det;
    if u < 0.0 || t < 0.0 || u > 1.0 || t > 1.0 {
        return false;
    }
    true
}

impl NavGraph {
    pub fn from_site(site: &Site) -> Vec<(String, Self)> {
        let mut graphs = Vec::new();
        for (graph_id, graph) in &site.navigation.guided.graphs {
            let graph_id = *graph_id;
            let location_at_anchor = {
                let mut location_at_anchor = HashMap::new();
                for (_, location) in &site.navigation.guided.locations {
                    if !location.graphs.includes(graph_id) {
                        continue;
                    }
                    location_at_anchor.insert(location.anchor.0, location.clone());
                }
                location_at_anchor
            };

            let lanes_with_anchor = {
                let mut lanes_with_anchor: HashMap<u32, Vec<u32>> = HashMap::new();
                for (lane_id, lane) in &site.navigation.guided.lanes {
                    if !lane.graphs.includes(graph_id) {
                        continue;
                    }
                    for a in lane.anchors.array() {
                        let entry = lanes_with_anchor.entry(a).or_default();
                        entry.push(*lane_id);
                    }
                }
                lanes_with_anchor
            };

            let mut doors = HashMap::new();
            let mut levels = HashMap::new();
            let mut lifts = HashMap::new();
            for (_, level) in &site.levels {
                let mut anchor_to_vertex = HashMap::new();
                let mut vertices = Vec::new();
                let mut lanes_to_include = HashSet::new();
                // Add vertices for anchors that are in lifts
                for lift in site.lifts.values() {
                    let lift_name = &lift.properties.name.0;
                    let Some(center) = lift.properties.center(site) else {
                        error!(
                            "nav graph export: Skipping lift {lift_name} due to broken anchor reference"
                        );
                        continue;
                    };
                    let Rotation::Yaw(yaw) = center.rot else {
                        error!(
                            "nav graph export: Skipping lift {lift_name} due to rotation not being pure yaw"
                        );
                        continue;
                    };
                    let yaw = yaw.radians();
                    // Note this will overwrite the entry in the map but that is OK
                    // TODO(luca) check that the lift position is correct when doing end to end testing
                    match &lift.properties.cabin {
                        LiftCabin::Rect(params) => {
                            lifts.insert(
                                lift_name.clone(),
                                NavLift {
                                    position: [center.trans[0], center.trans[1], yaw],
                                    // Note depth and width are inverted between legacy and site editor
                                    dims: [params.depth, params.width],
                                },
                            );
                        }
                    }
                    for (id, anchor) in &lift.cabin_anchors {
                        let Some(lanes) = lanes_with_anchor.get(id) else {
                            continue;
                        };

                        for lane in lanes.iter() {
                            lanes_to_include.insert(*lane);
                        }

                        // The anchor is in lift coordinates, make it in global coordinates
                        let trans = anchor.translation_for_category(Category::General);
                        let lift_tf = Affine2::from_angle_translation(
                            yaw,
                            [center.trans[0], center.trans[1]].into(),
                        );
                        let trans = lift_tf.transform_point2((trans).into());
                        let anchor = Anchor::Translate2D([trans[0], trans[1]]);

                        anchor_to_vertex.insert(*id, vertices.len());
                        let mut vertex = NavVertex::from_anchor(
                            &anchor,
                            location_at_anchor.get(id),
                            &site.navigation.guided.mutex_groups,
                        );
                        vertex.2.lift = Some(lift_name.clone());
                        vertices.push(vertex);
                    }
                }
                // Add site and level anchors
                for (id, anchor) in level.anchors.iter() {
                    let Some(lanes) = lanes_with_anchor.get(id) else {
                        continue;
                    };

                    for lane in lanes.iter() {
                        lanes_to_include.insert(*lane);
                    }

                    anchor_to_vertex.insert(*id, vertices.len());
                    vertices.push(NavVertex::from_anchor(
                        anchor,
                        location_at_anchor.get(id),
                        &site.navigation.guided.mutex_groups,
                    ));
                }

                let mut level_doors = HashMap::new();
                for (_, door) in &level.doors {
                    let door_name = &door.name.0;
                    let (v0, v1) = match (
                        site.get_anchor(door.anchors.start()),
                        site.get_anchor(door.anchors.end()),
                    ) {
                        (Some(v0), Some(v1)) => (
                            v0.translation_for_category(Category::Door),
                            v1.translation_for_category(Category::Door),
                        ),
                        _ => {
                            error!(
                                "nav graph export: Skipping door {door_name} due to broken anchor reference"
                            );
                            continue;
                        }
                    };
                    level_doors.insert(
                        door_name.clone(),
                        NavDoor {
                            map: level.properties.name.0.clone(),
                            endpoints: [v0, v1],
                        },
                    );
                }

                let mut lanes = Vec::new();
                for lane_id in &lanes_to_include {
                    let get_mutex = |affiliation: Affiliation<u32>| -> Option<String> {
                        let Some(group_id) = affiliation.0 else {
                            return None;
                        };
                        site.navigation
                            .guided
                            .mutex_groups
                            .get(&group_id)
                            .map(|group| group.name.0.clone())
                    };
                    let Some(lane) = site.navigation.guided.lanes.get(lane_id) else {
                        continue;
                    };
                    let (v0, v1) = match (
                        anchor_to_vertex.get(&lane.anchors.start()),
                        anchor_to_vertex.get(&lane.anchors.end()),
                    ) {
                        (Some(v0), Some(v1)) => (*v0, *v1),
                        _ => {
                            error!(
                                "nav graph export: Lane {lane_id} is using a site anchor. This is not supported, the lane will be skipped."
                            );
                            continue;
                        }
                    };

                    let mut door_name = None;
                    let l0 = [vertices[v0].0, vertices[v0].1];
                    let l1 = [vertices[v1].0, vertices[v1].1];
                    for (name, door) in &level_doors {
                        if segments_intersect(l0, l1, door.endpoints[0], door.endpoints[1]) {
                            door_name = Some(name);
                        }
                    }

                    let props = NavLaneProperties::from_motion(
                        &lane.forward,
                        door_name.cloned(),
                        get_mutex(lane.mutex),
                    );
                    lanes.push(NavLane(v0, v1, props.clone()));
                    match &lane.reverse {
                        ReverseLane::Same => {
                            lanes.push(NavLane(v1, v0, props));
                        }
                        ReverseLane::Different(motion) => {
                            lanes.push(NavLane(
                                v1,
                                v0,
                                NavLaneProperties::from_motion(
                                    motion,
                                    door_name.cloned(),
                                    get_mutex(lane.mutex),
                                ),
                            ));
                        }
                        ReverseLane::Disable => {
                            // Do nothing
                        }
                    }
                }

                doors.extend(level_doors);
                levels.insert(
                    level.properties.name.clone().0,
                    NavLevel { lanes, vertices },
                );
            }

            graphs.push((
                graph.name.0.clone(),
                Self {
                    building_name: site.properties.name.clone().0,
                    levels,
                    doors,
                    lifts,
                },
            ))
        }

        graphs
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NavLevel {
    pub lanes: Vec<NavLane>,
    pub vertices: Vec<NavVertex>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NavLane(pub usize, pub usize, pub NavLaneProperties);

#[derive(Serialize, Deserialize, Clone)]
pub struct NavLaneProperties {
    pub speed_limit: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dock_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub door_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orientation_constraint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutex: Option<String>,
}

impl NavLaneProperties {
    fn from_motion(motion: &Motion, door_name: Option<String>, mutex: Option<String>) -> Self {
        let orientation_constraint = match &motion.orientation_constraint {
            OrientationConstraint::None => None,
            OrientationConstraint::Forwards => Some("forward".to_owned()),
            OrientationConstraint::Backwards => Some("backward".to_owned()),
            OrientationConstraint::RelativeYaw(_) | OrientationConstraint::AbsoluteYaw(_) => {
                error!(
                    "nav graph export: Skipping orientation constraint [{:?}] because of incompatibility",
                    motion.orientation_constraint
                );
                None
            }
        };
        Self {
            speed_limit: motion.speed_limit.unwrap_or(0.0),
            dock_name: motion.dock.as_ref().map(|d| d.name.clone()),
            orientation_constraint,
            door_name,
            mutex,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NavVertex(pub f32, pub f32, pub NavVertexProperties);

impl NavVertex {
    fn from_anchor(
        anchor: &Anchor,
        location: Option<&Location<u32>>,
        mutex_groups: &BTreeMap<u32, MutexGroup>,
    ) -> Self {
        let p = anchor.translation_for_category(Category::General);
        Self(
            p[0],
            p[1],
            NavVertexProperties::from_location(location, mutex_groups),
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct NavVertexProperties {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lift: Option<String>,
    #[serde(skip_serializing_if = "is_false")]
    pub is_charger: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub is_holding_point: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub is_parking_spot: bool,
    // TODO(luca) serialize merge_radius, it is currently skipped
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merge_radius: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mutex: Option<String>,
    pub name: String,
}

impl NavVertexProperties {
    fn from_location(
        location: Option<&Location<u32>>,
        mutex_groups: &BTreeMap<u32, MutexGroup>,
    ) -> Self {
        let mut props = Self::default();
        let location = match location {
            Some(l) => l,
            None => return props,
        };
        props.name = location.name.0.clone();
        props.is_charger = location.tags.0.iter().find(|t| t.is_charger()).is_some();
        props.is_holding_point = location
            .tags
            .0
            .iter()
            .find(|t| t.is_holding_point())
            .is_some();
        props.is_parking_spot = location
            .tags
            .0
            .iter()
            .find(|t| t.is_parking_spot())
            .is_some();

        if let Some(mutex) = location.mutex.0 {
            props.mutex = Some(
                mutex_groups
                    .get(&mutex)
                    .map(|m| m.name.0.clone())
                    .unwrap_or_else(|| {
                        error!(
                        "nav graph export: Unable to find mutex group #{} name for location [{}]",
                        mutex,
                        props.name,
                    );
                        String::new()
                    }),
            );
        }

        props
    }
}

fn is_false(b: &bool) -> bool {
    !b
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NavDoor {
    pub endpoints: [[f32; 2]; 2],
    pub map: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NavLift {
    pub position: [f32; 3],
    pub dims: [f32; 2],
}
