use crate::*;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

#[derive(Serialize, Clone)]
pub struct NavGraph {
    building_name: String,
    levels: HashMap<String, NavLevel>,
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

                    let props = NavLaneProperties::from_motion(&lane.forward);
                    lanes.push(NavLane(v0, v1, props.clone()));
                    match &lane.reverse {
                        ReverseLane::Same => {
                            lanes.push(NavLane(v1, v0, props));
                        }
                        ReverseLane::Different(motion) => {
                            lanes.push(NavLane(v1, v0, NavLaneProperties::from_motion(motion)));
                        }
                        ReverseLane::Disable => {
                            // Do nothing
                        }
                    }
                }

                levels.insert(level.properties.name.clone(), NavLevel { lanes, vertices });
            }

            graphs.push((
                graph.name.0.clone(),
                Self {
                    building_name: site.properties.name.clone(),
                    levels,
                },
            ))
        }

        graphs
    }
}

#[derive(Serialize, Clone)]
pub struct NavLevel {
    lanes: Vec<NavLane>,
    vertices: Vec<NavVertex>,
}

#[derive(Serialize, Clone)]
pub struct NavLane(pub usize, pub usize, pub NavLaneProperties);

#[derive(Serialize, Clone)]
pub struct NavLaneProperties {
    speed_limit: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    dock_name: Option<String>,
    // TODO(MXG): Add other lane properties
    // door_name,
    // orientation_constraint,
    // demo_mock_floor_name
}

impl NavLaneProperties {
    fn from_motion(motion: &Motion) -> Self {
        Self {
            speed_limit: motion.speed_limit.unwrap_or(0.0),
            dock_name: motion.dock.as_ref().map(|d| d.name.clone()),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct NavVertex(pub f32, pub f32, pub NavVertexProperties);

impl NavVertex {
    fn from_anchor(anchor: &Anchor<u32>, location: Option<&Location<u32>>) -> Self {
        let p = *anchor.translation_for_category(Category::General);
        Self(p[0], p[1], NavVertexProperties::from_location(location))
    }
}

#[derive(Serialize, Clone)]
pub struct NavVertexProperties {
    #[serde(skip_serializing_if = "Option::is_none")]
    lift: Option<String>,
    #[serde(skip_serializing_if = "is_false")]
    is_charger: bool,
    #[serde(skip_serializing_if = "is_false")]
    is_holding_point: bool,
    #[serde(skip_serializing_if = "is_false")]
    is_parking_spot: bool,
    name: String,
}

impl Default for NavVertexProperties {
    fn default() -> Self {
        Self {
            lift: None,
            is_charger: false,
            is_holding_point: false,
            is_parking_spot: false,
            name: "".to_owned(),
        }
    }
}

impl NavVertexProperties {
    fn from_location(location: Option<&Location<u32>>) -> Self {
        let mut props = Self::default();
        let location = match location {
            Some(l) => l,
            None => return props,
        };
        props.name = location.name.0.clone();
        props.is_charger = location.tags.iter().find(|t| t.is_charger()).is_some();
        props.is_holding_point = location
            .tags
            .iter()
            .find(|t| t.is_holding_point())
            .is_some();
        props.is_parking_spot = location.tags.iter().find(|t| t.is_parking_spot()).is_some();

        props
    }
}

fn is_false(b: &bool) -> bool {
    !b
}
