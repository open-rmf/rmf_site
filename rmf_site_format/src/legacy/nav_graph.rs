use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Serialize, Deserialize, Clone)]
pub struct NavGraph {
    pub building_name: String,
    pub levels: HashMap<String, NavLevel>,
    pub doors: HashMap<String, NavDoor>,
    pub lifts: HashMap<String, NavLift>,
}

// Readapted from legacy traffic editor implementation
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
                let mut lanes_with_anchor = HashMap::new();
                for (lane_id, lane) in &site.navigation.guided.lanes {
                    if !lane.graphs.includes(graph_id) {
                        continue;
                    }
                    for a in lane.anchors.array() {
                        lanes_with_anchor.insert(a, (*lane_id, lane));
                    }
                }
                lanes_with_anchor
            };

            // TODO(MXG): Make this work for lifts

            let mut doors = HashMap::new();
            let mut levels = HashMap::new();
            for (_, level) in &site.levels {
                let mut anchor_to_vertex = HashMap::new();
                let mut vertices = Vec::new();
                let mut lanes_to_include = HashSet::new();
                for (id, anchor) in &level.anchors {
                    let (lane, _) = match lanes_with_anchor.get(id) {
                        Some(v) => v,
                        None => continue,
                    };

                    lanes_to_include.insert(*lane);
                    anchor_to_vertex.insert(*id, vertices.len());
                    vertices.push(NavVertex::from_anchor(anchor, location_at_anchor.get(id)));
                }

                let mut level_doors = HashMap::new();
                for (_, door) in &level.doors {
                    let door_name = &door.name.0;
                    let (v0, v1) = match (
                        level.anchors.get(&door.anchors.start()),
                        level.anchors.get(&door.anchors.end()),
                    ) {
                        (Some(v0), Some(v1)) => (
                            v0.translation_for_category(Category::Level),
                            v1.translation_for_category(Category::Level),
                        ),
                        _ => {
                            println!(
                                "ERROR: Skipping door {door_name} due to broken anchor reference"
                            );
                            continue;
                        }
                    };
                    level_doors.insert(
                        door_name.clone(),
                        NavDoor {
                            map: level.properties.name.0.clone(),
                            endpoints: [*v0, *v1],
                        },
                    );
                }

                let mut lanes = Vec::new();
                for lane_id in &lanes_to_include {
                    let lane = site.navigation.guided.lanes.get(lane_id).unwrap();
                    let (v0, v1) = match (
                        anchor_to_vertex.get(&lane.anchors.start()),
                        anchor_to_vertex.get(&lane.anchors.end()),
                    ) {
                        (Some(v0), Some(v1)) => (*v0, *v1),
                        _ => {
                            println!("ERROR: Skipping lane {lane_id} due to incompatibility");
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

                    let props = NavLaneProperties::from_motion(&lane.forward, door_name.cloned());
                    lanes.push(NavLane(v0, v1, props.clone()));
                    match &lane.reverse {
                        ReverseLane::Same => {
                            lanes.push(NavLane(v1, v0, props));
                        }
                        ReverseLane::Different(motion) => {
                            lanes.push(NavLane(
                                v1,
                                v0,
                                NavLaneProperties::from_motion(motion, door_name.cloned()),
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

            let mut lifts = HashMap::new();
            for (_, lift) in &site.lifts {
                let lift_name = &lift.properties.name.0;
                let Some(pose) = lift.properties.center(site) else {
                    println!("ERROR: Skipping lift {lift_name} due to broken anchor reference");
                    continue;
                };
                let Rotation::Yaw(yaw) = pose.rot else {
                    println!("ERROR: Skipping lift {lift_name} due to rotation not being pure yaw");
                    continue;
                };
                // TODO(luca) check that the lift position is correct when doing end to end testing
                match &lift.properties.cabin {
                    LiftCabin::Rect(params) => {
                        lifts.insert(
                            lift_name.clone(),
                            NavLift {
                                position: [pose.trans[0], pose.trans[1], yaw.radians()],
                                // Note depth and width are inverted between legacy and site editor
                                dims: [params.depth, params.width],
                            },
                        );
                    }
                }
                // TODO(luca) lift property for vertices contained in lifts
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
    // TODO(luca): Add other lane properties
    // demo_mock_floor_name
    // mutex
}

impl NavLaneProperties {
    fn from_motion(motion: &Motion, door_name: Option<String>) -> Self {
        let orientation_constraint = match &motion.orientation_constraint {
            OrientationConstraint::None => None,
            OrientationConstraint::Forwards => Some("forward".to_owned()),
            OrientationConstraint::Backwards => Some("backward".to_owned()),
            OrientationConstraint::RelativeYaw(_) | OrientationConstraint::AbsoluteYaw(_) => {
                println!(
                    "Skipping orientation constraint [{:?}] because of incompatibility",
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
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NavVertex(pub f32, pub f32, pub NavVertexProperties);

impl NavVertex {
    fn from_anchor(anchor: &Anchor, location: Option<&Location<u32>>) -> Self {
        let p = *anchor.translation_for_category(Category::General);
        Self(p[0], p[1], NavVertexProperties::from_location(location))
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct NavVertexProperties {
    // TODO(luca) serialize lift and merge_radius, they are currently skipped
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lift: Option<String>,
    #[serde(skip_serializing_if = "is_false")]
    pub is_charger: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub is_holding_point: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub is_parking_spot: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merge_radius: Option<f32>,
    pub name: String,
}

impl NavVertexProperties {
    fn from_location(location: Option<&Location<u32>>) -> Self {
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
