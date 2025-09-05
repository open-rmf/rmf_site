use rclrs::{QoSDurabilityPolicy, QoSHistoryPolicy, QoSReliabilityPolicy};
use rmf_site_format::{AssetSource, Category, DoorType, LiftCabin, Rotation, Side, Site, Swing};
use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Error, Result};

fn get_map_msg(site: &Site, map_folder: &Path) -> rmf_building_map_msgs::msg::BuildingMap {
    let mut lifts = Vec::new();
    let mut levels = Vec::new();

    for lift in site.lifts.values() {
        // Lift wall graph is ignored
        let pose = lift.properties.center(site).unwrap();
        let Rotation::Yaw(angle) = pose.rot else {
            // TODO(luca) error logging
            continue;
        };
        match &lift.properties.cabin {
            LiftCabin::Rect(params) => {
                // TODO(luca) There are many properties that seem unused, specifically skipping for
                // now: doors, wall_graph and levels.
                lifts.push(rmf_building_map_msgs::msg::Lift {
                    name: lift.properties.name.0.clone(),
                    ref_x: pose.trans[0],
                    ref_y: pose.trans[1],
                    ref_yaw: angle.radians(),
                    // Note in site editor width and depth are flipped
                    width: params.depth,
                    depth: params.width,
                    ..Default::default()
                });
            }
        }
    }

    for level in site.levels.values() {
        let mut doors = Vec::new();
        for door in level.doors.values() {
            let Some(v1) = site.get_anchor(door.anchors.start()) else {
                println!("Missing start anchor for door, skipping...");
                continue;
            };
            let Some(v2) = site.get_anchor(door.anchors.end()) else {
                println!("Missing end anchor for door, skipping...");
                continue;
            };
            let v1 = v1.translation_for_category(Category::Level);
            let v2 = v2.translation_for_category(Category::Level);
            let name = door.name.0.clone();
            let door = match &door.kind {
                DoorType::SingleSliding(door) => {
                    // TODO(luca) check that this is not flipped
                    let (v1, v2) = match door.towards {
                        Side::Left => (v1, v2),
                        Side::Right => (v2, v1),
                    };
                    rmf_building_map_msgs::msg::Door {
                        name,
                        v1_x: v1[0],
                        v1_y: v1[1],
                        v2_x: v2[0],
                        v2_y: v2[1],
                        door_type: rmf_building_map_msgs::msg::Door::DOOR_TYPE_SINGLE_SLIDING,
                        ..Default::default()
                    }
                }
                DoorType::DoubleSliding(_) => {
                    // TODO(luca) check that this is not flipped
                    rmf_building_map_msgs::msg::Door {
                        name,
                        v1_x: v1[0],
                        v1_y: v1[1],
                        v2_x: v2[0],
                        v2_y: v2[1],
                        door_type: rmf_building_map_msgs::msg::Door::DOOR_TYPE_DOUBLE_SLIDING,
                        ..Default::default()
                    }
                }
                DoorType::SingleSwing(door) => {
                    // TODO(luca) check that this is not flipped
                    let (v1, v2) = match door.pivot_on {
                        Side::Left => (v1, v2),
                        Side::Right => (v2, v1),
                    };
                    let (motion_range, motion_direction) = match door.swing {
                        Swing::Forward(forward) | Swing::Both { forward, .. } => {
                            (forward.radians(), 1)
                        }
                        Swing::Backward(backward) => (backward.radians(), -1),
                    };
                    rmf_building_map_msgs::msg::Door {
                        name,
                        v1_x: v1[0],
                        v1_y: v1[1],
                        v2_x: v2[0],
                        v2_y: v2[1],
                        door_type: rmf_building_map_msgs::msg::Door::DOOR_TYPE_SINGLE_SWING,
                        motion_range,
                        motion_direction,
                    }
                }
                DoorType::DoubleSwing(door) => {
                    let (motion_range, motion_direction) = match door.swing {
                        Swing::Forward(forward) | Swing::Both { forward, .. } => {
                            (forward.radians(), 1)
                        }
                        Swing::Backward(backward) => (backward.radians(), -1),
                    };
                    rmf_building_map_msgs::msg::Door {
                        name,
                        v1_x: v1[0],
                        v1_y: v1[1],
                        v2_x: v2[0],
                        v2_y: v2[1],
                        door_type: rmf_building_map_msgs::msg::Door::DOOR_TYPE_DOUBLE_SWING,
                        motion_range,
                        motion_direction,
                    }
                }
                DoorType::Model(_) => {
                    println!("Found unsupported Model door! Skipping...");
                    continue;
                }
            };
            doors.push(door);
        }
        let mut images = Vec::new();
        for drawing in level.drawings.values() {
            // TODO(luca) Floorplan visualizer does not support yet more than one drawing, revisit
            // when that is the case
            if !images.is_empty() {
                continue;
            }
            let AssetSource::Local(path) = &drawing.properties.source else {
                println!("Found unsupported drawing type! Skipping...");
                continue;
            };
            let image_path = map_folder.join(path);
            let Ok(data) = std::fs::read(&image_path) else {
                println!("Unable to read image! Skipping...");
                continue;
            };
            let Some(extension) = image_path.extension().and_then(|ext| ext.to_str()) else {
                println!("Unable to get image extension! Skipping...");
                continue;
            };
            let pose = &drawing.properties.pose;
            let Rotation::Yaw(angle) = pose.rot else {
                // TODO(luca) error logging
                continue;
            };
            images.push(rmf_building_map_msgs::msg::AffineImage {
                data,
                name: drawing.properties.name.0.clone(),
                encoding: extension.to_string(),
                scale: 1.0 / drawing.properties.pixels_per_meter.0,
                x_offset: pose.trans[0],
                y_offset: pose.trans[1],
                yaw: angle.radians(),
            });
        }
        levels.push(rmf_building_map_msgs::msg::Level {
            name: level.properties.name.0.clone(),
            elevation: level.properties.elevation.0,
            images,
            places: vec![],
            doors,
            nav_graphs: vec![],
            wall_graph: Default::default(),
        });
    }

    rmf_building_map_msgs::msg::BuildingMap {
        name: site.properties.name.0.clone(),
        levels,
        lifts,
    }
}

fn main() -> Result<(), Error> {
    let args: Vec<String> = env::args().collect();
    let Some(map_path) = args.get(1) else {
        panic!("Map path must be provided as a command line argument");
    };
    let site = if map_path.ends_with(".building.yaml") {
        let building_map = rmf_site_format::legacy::building_map::BuildingMap::from_bytes(
            &std::fs::read(map_path)?,
        )?;
        building_map.to_site()?
    } else if map_path.ends_with(".site.ron") {
        rmf_site_format::Site::from_bytes(&std::fs::read(map_path)?)?
    } else {
        panic!("Unsupported file type {map_path}");
    };
    let context = rclrs::Context::new(env::args())?;

    let node = rclrs::create_node(&context, "rmf_building_map_server_rs")?;

    let mut qos = rclrs::QOS_PROFILE_DEFAULT;
    qos.history = QoSHistoryPolicy::KeepLast { depth: 1 };
    qos.reliability = QoSReliabilityPolicy::Reliable;
    qos.durability = QoSDurabilityPolicy::TransientLocal;

    let publisher = node.create_publisher::<rmf_building_map_msgs::msg::BuildingMap>("map", qos)?;

    let site_folder = PathBuf::from(map_path);
    let site_folder = site_folder.parent().unwrap();
    let map = get_map_msg(&site, site_folder);

    publisher.publish(map)?;
    rclrs::spin(node).map_err(|err| err.into())
}
