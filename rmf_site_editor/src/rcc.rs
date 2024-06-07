use crate::{log, site_mode, JsValue};
use bevy_utils::default;
use once_cell::sync::Lazy;
use rmf_site_format::{Anchor, Level, Location, LocationTag, NameInSite, NavGraph};
use std::collections::{BTreeMap, HashMap};

use serde::Deserialize;

// RCC Map source related logics
pub static mut MAP_INDEX: u32 = 0;
pub static mut SHOW_MAP_ASSET_SOURCE: u32 = 0; // Display whether map dropdown or text box on selecting "RCC" AssetSource

#[derive(Debug, Deserialize, PartialEq)]
pub struct YamlData {
    pub mode: String,
    pub image: String,
    pub negate: u8,
    pub origin: Vec<f64>,
    pub resolution: f64,
    pub free_thresh: f64,
    pub occupied_thresh: f64,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Marker {
    pub x: f32,
    pub y: f32,
    pub meta: Meta,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Meta {
    pub name: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Maps {
    pub id: String,
    pub name: String,
    pub image_url: String,
    pub yaml_data: YamlData,
    pub markers: HashMap<String, Marker>,
}

pub fn set_selected_map_index(map_index: u32) {
    unsafe { MAP_INDEX = map_index };
}

pub fn parse_js_value(val: &JsValue) -> Result<Maps, Box<dyn std::error::Error>> {
    let curr_map_str = js_sys::JSON::stringify(&val)
        .unwrap()
        .as_string()
        .ok_or("Invalid string")?;
    let cur_map_obj: Maps = serde_json::from_str(&curr_map_str)?;
    Ok(cur_map_obj)
}

// Site Mode Related logics
pub static mut SITE_MODE: String = String::new();

pub fn set_site_mode() {
    let js_value: JsValue = site_mode().into();
    let rust_string: String = js_value.as_string().unwrap_or_default();
    unsafe { SITE_MODE = rust_string.to_string() }
}

pub fn is_site_in_view_mode() -> bool {
    return unsafe { &SITE_MODE } == "VIEW_MODE";
}

//Robot list Related logics
static mut ROBOT_LIST: Lazy<HashMap<u32, String>> = Lazy::new(|| {
    let map = HashMap::new();
    map
});

pub fn parse_robot_data(val: &JsValue) -> Result<String, Box<dyn std::error::Error>> {
    let curr_robot = js_sys::JSON::stringify(&val)
        .unwrap()
        .as_string()
        .ok_or("Invalid string")?;
    let cur_robot_obj: String = serde_json::from_str(&curr_robot)?;
    Ok(cur_robot_obj)
}

pub fn add_robot_in_robot_list(id: &str, index: u32) {
    unsafe {
        ROBOT_LIST.insert(index, id.to_string());
    }
}

pub fn get_robot_id(index: u32) -> Option<String> {
    unsafe { ROBOT_LIST.get(&index).cloned() }
}

// Robot pose Related logics
#[derive(Debug, Deserialize, PartialEq)]
pub struct RobotPose {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub yaw: f32,
    pub level_name: String,
}

static mut ROBOT_POSE: Lazy<HashMap<String, RobotPose>> = Lazy::new(|| {
    let map = HashMap::new();
    map
});

pub fn parse_robot_pose(val: &JsValue) -> Result<RobotPose, Box<dyn std::error::Error>> {
    let curr_map_str = js_sys::JSON::stringify(&val)
        .unwrap()
        .as_string()
        .ok_or("Invalid string")?;
    let cur_map_obj: RobotPose = serde_json::from_str(&curr_map_str)?;
    Ok(cur_map_obj)
}

pub fn get_robot_pose_by_id(id: &str) -> Option<&RobotPose> {
    unsafe { ROBOT_POSE.get(id) }
}

pub fn add_robot_pose_by_id(id: String, pose: RobotPose) {
    unsafe {
        ROBOT_POSE.insert(id, pose);
    }
}

pub fn load_milestones(map: Maps) -> Vec<u8> {
    #[cfg(target_arch = "wasm32")]
    {
        log(&format!("Main Menu - Loading Milestones"));
    }
    let mut site_id = 0_u32..;
    let level_id = site_id.next().unwrap();
    let mut levels = BTreeMap::new();

    let mut drawings: BTreeMap<u32, rmf_site_format::Drawing> = BTreeMap::new();
    drawings.insert(
        level_id,
        rmf_site_format::Drawing {
            properties: (rmf_site_format::DrawingProperties {
                name: NameInSite((map.name).to_string()),
                source: rmf_site_format::AssetSource::RCC((map.image_url).to_string()),
                pixels_per_meter: rmf_site_format::PixelsPerMeter(20.0),
                ..default()
            }),
            ..default()
        },
    );

    let mut anchors: BTreeMap<u32, Anchor> = BTreeMap::new();
    let mut locations = BTreeMap::new();
    let mut tags = Vec::new();

    let mut initial = true;
    map.markers.iter().for_each(|marker| {
        if initial {
            tags.push(LocationTag::Charger);
            initial = false;
        }
        let anchor = site_id.next().unwrap();
        anchors.insert(
            anchor,
            [
                ((marker.1.x + 5.0) * 0.05),
                -(((marker.1.y + 5.0) * 0.05) - (0.34617525 / 2.0) + (0.14235048) + 0.02),
            ]
            .into(),
        );

        locations.insert(
            site_id.next().unwrap(),
            Location {
                name: NameInSite(marker.1.meta.name.clone()),
                tags: rmf_site_format::LocationTags(tags.clone()),
                graphs: rmf_site_format::AssociatedGraphs::All,
                anchor: rmf_site_format::Point(anchor),
            },
        );
    });

    levels.insert(
        level_id,
        Level {
            properties: rmf_site_format::LevelProperties {
                name: NameInSite("l1".to_string()),
                ..default()
            },
            drawings: drawings.clone(),
            anchors: anchors.clone(),
            ..default()
        },
    );

    let mut graphs = BTreeMap::new();
    graphs.insert(
        site_id.next().unwrap(),
        NavGraph {
            name: NameInSite("navgraph".to_string()),
            ..default()
        },
    );

    let guided = rmf_site_format::Guided {
        graphs,
        locations,
        ..default()
    };

    // create new site and convert to bytes
    let site = rmf_site_format::Site {
        levels,
        navigation: rmf_site_format::Navigation { guided },
        ..default()
    };

    log(&format!("New site: {:?}", site));

    // convert site to bytes
    let site_bytes: Vec<u8> = ron::to_string(&site).unwrap().as_bytes().to_vec();
    return site_bytes;
}
