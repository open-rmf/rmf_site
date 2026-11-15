//! Example of extracting the conveyor belt topology from a `.site.json` file
//! for the Material Flow Planner by matching shared Anchor IDs.
//!
//! Run with:
//!   cargo run -p rmf_site_format --example extract_conveyor_topology
//!   cargo run -p rmf_site_format --example extract_conveyor_topology -- path/to/map.site.json

use rmf_site_format::{ConveyorPortDirection, Site};
use serde::Serialize;
use std::{collections::BTreeMap, env, fs};

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

        edges.push(FlowEdge {
            conveyor_id,
            from_anchor: conveyor.anchors.start(),
            to_anchor: conveyor.anchors.end(),
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
