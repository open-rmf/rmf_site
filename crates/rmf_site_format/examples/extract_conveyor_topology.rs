//! Example of extracting the conveyor belt topology from a `.site.json` file
//! for the Material Flow Planner by matching shared Anchor IDs.
//!
//! Run with:
//!   cargo run -p rmf_site_format --example extract_conveyor_topology
//!   cargo run -p rmf_site_format --example extract_conveyor_topology -- path/to/map.site.json

use rmf_site_format::{ConveyorPortDirection, Site};
use serde::Serialize;
use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    env, fs,
};

/// A node in the inferred Material Flow graph, identified by its Anchor ID
#[derive(Serialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
enum FlowNode {
    /// Connected to a port on a workcell
    WorkcellPort {
        model_id: u32,
        model_name: String,
        direction: ConveyorPortDirection,
    },
    /// Intermediate waypoint on the floor joining belt segments
    FloorWaypoint { level_id: u32, position: [f32; 2] },
}

/// A directed conveyor segment in the Material Flow graph.
#[derive(Serialize, Debug)]
struct FlowEdge {
    conveyor_id: u32,
    from_anchor: u32,
    to_anchor: u32,
    speed: f32,
    capacity: u32,
}

/// Complete topology exported to JSON for downstream pathfinding / planning
#[derive(Serialize, Debug)]
struct ConveyorTopology {
    nodes: BTreeMap<u32, FlowNode>,
    edges: Vec<FlowEdge>,
}

fn extract_topology(site: &Site) -> ConveyorTopology {
    let mut nodes = BTreeMap::new();

    // Each ModelInstance stores its ports in conveyor_ports keyed by Anchor ID.
    // Any conveyor belt connected to a workcell will reference this exact Anchor ID.
    for (&model_id, instance) in &site.model_instances {
        let Some(ports) = &instance.bundle.conveyor_ports.0 else {
            continue;
        };
        for (&port_anchor_id, port) in ports {
            nodes.insert(
                port_anchor_id,
                FlowNode::WorkcellPort {
                    model_id,
                    model_name: instance.bundle.name.0.clone(),
                    direction: port.direction,
                },
            );
        }
    }

    // Resolve conveyor flow direction using connected ports
    let mut port_directions = HashMap::new();
    for (&anchor_id, node) in &nodes {
        if let FlowNode::WorkcellPort { direction, .. } = node {
            port_directions.insert(anchor_id, *direction);
        }
    }

    let mut anchor_to_conveyors: HashMap<u32, Vec<u32>> = HashMap::new();
    for (&conveyor_id, conveyor) in &site.navigation.guided.conveyors {
        if conveyor.anchors.start() != conveyor.anchors.end() {
            anchor_to_conveyors
                .entry(conveyor.anchors.start())
                .or_default()
                .push(conveyor_id);
            anchor_to_conveyors
                .entry(conveyor.anchors.end())
                .or_default()
                .push(conveyor_id);
        }
    }

    let mut is_reversed: HashMap<u32, bool> = HashMap::new();
    for (&start_id, start_conveyor) in &site.navigation.guided.conveyors {
        if is_reversed.contains_key(&start_id)
            || start_conveyor.anchors.start() == start_conveyor.anchors.end()
        {
            continue;
        }
        let mut chain = vec![(start_id, true)];
        let mut visited_conveyors = HashSet::from([start_id]);
        let mut queue = VecDeque::new();
        for (anchor, away_from_start) in [
            (start_conveyor.anchors.start(), false),
            (start_conveyor.anchors.end(), true),
        ] {
            if !port_directions.contains_key(&anchor) {
                queue.push_back((anchor, away_from_start));
            }
        }
        let mut visited_anchors = HashSet::from(start_conveyor.anchors.array());

        while let Some((anchor, away_from_start)) = queue.pop_front() {
            let Some(neighbors) = anchor_to_conveyors.get(&anchor) else {
                continue;
            };
            for &other_id in neighbors {
                if !visited_conveyors.insert(other_id) {
                    continue;
                }
                let other_edge = &site.navigation.guided.conveyors[&other_id].anchors;
                let same_dir = (other_edge.start() == anchor) == away_from_start;
                chain.push((other_id, same_dir));
                for next_anchor in other_edge.array() {
                    if !port_directions.contains_key(&next_anchor)
                        && visited_anchors.insert(next_anchor)
                    {
                        queue.push_back((next_anchor, away_from_start));
                    }
                }
            }
        }

        let chain_forward = chain.iter().find_map(|&(seg_id, same_dir)| {
            let seg_edge = &site.navigation.guided.conveyors[&seg_id].anchors;
            [
                (seg_edge.start(), ConveyorPortDirection::Out),
                (seg_edge.end(), ConveyorPortDirection::In),
            ]
            .into_iter()
            .find_map(|(anchor, forward_dir)| {
                let dir = port_directions.get(&anchor)?;
                Some((*dir == forward_dir) == same_dir)
            })
        });

        for (seg_id, same_dir) in chain {
            let reversed = chain_forward.is_some_and(|forward| forward != same_dir);
            is_reversed.insert(seg_id, reversed);
        }
    }

    // Conveyor segments share Anchor IDs at their endpoints.
    // If an endpoint's Anchor ID is not a workcell port, it is a floor anchor on a Level
    // used to chain multiple conveyor segments together.
    let mut edges = Vec::new();
    for (&conveyor_id, conveyor) in &site.navigation.guided.conveyors {
        for anchor_id in conveyor.anchors.array() {
            if nodes.contains_key(&anchor_id) {
                continue;
            }
            for (&level_id, level) in &site.levels {
                if let Some(anchor) = level.anchors.get(&anchor_id) {
                    let position = anchor.translation_for_category(Default::default());
                    nodes.insert(anchor_id, FlowNode::FloorWaypoint { level_id, position });
                }
            }
        }

        let (from_anchor, to_anchor) = if is_reversed.get(&conveyor_id) == Some(&true) {
            (conveyor.anchors.end(), conveyor.anchors.start())
        } else {
            (conveyor.anchors.start(), conveyor.anchors.end())
        };
        edges.push(FlowEdge {
            conveyor_id,
            from_anchor,
            to_anchor,
            speed: conveyor.metadata.speed.0,
            capacity: conveyor.metadata.capacity.0,
        });
    }

    ConveyorTopology { nodes, edges }
}

/// Sample .site.json chaining 2 conveyor segments from
/// `Out_Port` -> Floor Waypoint -> `In_Port`
const SAMPLE_SITE_JSON: &str = r#"{
  "format_version": "0.2",
  "properties": { "name": "demo_factory" },
  "levels": {
    "1": {
      "properties": { "name": "L1", "elevation": 0.0 },
      "anchors": {
        "11": { "Translate2D": [4.0, 2.0] }
      }
    }
  },
  "model_instances": {
    "2": {
      "parent": 1,
      "name": "Out_Port",
      "pose": { "trans": [1.0, 2.0, 0.0], "rot": { "yaw": { "deg": 0.0 } } },
      "description": null,
      "conveyor_ports": {
        "10": { "direction": "out", "location": [0.5, 0.0, 0.75], "width": 0.4 }
      }
    },
    "3": {
      "parent": 1,
      "name": "In_Port",
      "pose": { "trans": [4.0, 6.5, 0.0], "rot": { "yaw": { "deg": 0.0 } } },
      "description": null,
      "conveyor_ports": {
        "12": { "direction": "in", "location": [0.0, -0.5, 0.75], "width": 0.4 }
      }
    }
  },
  "navigation": {
    "guided": {
      "graphs": {},
      "ranking": [],
      "lanes": {},
      "locations": {},
      "conveyors": {
        "20": {
          "anchors": [10, 11],
          "metadata": { "speed": 1.2, "capacity": 3 }
        },
        "21": {
          "anchors": [11, 12],
          "metadata": { "speed": 1.2, "capacity": 3 }
        }
      }
    }
  }
}"#;

fn main() {
    let json_bytes = match env::args().nth(1) {
        Some(path) => fs::read(&path).expect("Failed to read .site.json file"),
        None => SAMPLE_SITE_JSON.as_bytes().to_vec(),
    };

    let site = Site::from_bytes_json(&json_bytes).expect("Failed to parse .site.json");
    let topology = extract_topology(&site);

    println!(
        "{}",
        serde_json::to_string_pretty(&topology).expect("Failed to serialize topology")
    );
}
