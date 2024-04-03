use crate::{
    JsValue,
    log,
    site_mode,
};
use once_cell::sync::Lazy;
use std::collections::HashMap;

use serde::Deserialize;

// RCC Map source related logics
pub static mut MAP_INDEX:u32=0;
pub static mut SHOW_MAP_ASSET_SOURCE:u32=0; // Display whether map dropdown or text box on selecting "RCC" AssetSource

#[derive(Debug, Deserialize,PartialEq)]
struct YamlData {
    pub mode: String,
    pub image: String,
    pub negate: u8,
    pub origin: Vec<f64>,
    pub resolution: f64,
    pub free_thresh: f64,
    pub occupied_thresh: f64,
}

#[derive(Debug, Deserialize,PartialEq)]
pub struct Maps {
    pub id: String,
    pub name: String,
    pub image_url: String,
    pub yaml_data: YamlData,
}

pub fn set_selected_map_index(map_index:u32){
    unsafe { MAP_INDEX = map_index };
}

pub fn parse_js_value(val: &JsValue) -> Result<Maps, Box<dyn std::error::Error>> {
    let curr_map_str = js_sys::JSON::stringify(&val).unwrap().as_string().ok_or("Invalid string")?;
    let cur_map_obj: Maps = serde_json::from_str(&curr_map_str)?;
    Ok(cur_map_obj)
}

// Site Mode Related logics 
pub static mut SITE_MODE:String = String::new();

pub fn set_site_mode() {
    let js_value: JsValue = site_mode().into();
    let rust_string: String = js_value.as_string().unwrap_or_default();
    unsafe { SITE_MODE = rust_string.to_string() }
    
}

pub fn is_site_in_view_mode() -> bool {

    return  unsafe { &SITE_MODE } == "VIEW_MODE"
}


//Robot list Related logics
static mut ROBOT_LIST: Lazy<HashMap<u32,String>> = Lazy::new(|| {
    let map = HashMap::new();
    map
});

pub fn parse_robot_data(val: &JsValue) -> Result<String, Box<dyn std::error::Error>> {
    let curr_robot = js_sys::JSON::stringify(&val).unwrap().as_string().ok_or("Invalid string")?;
    let cur_robot_obj: String = serde_json::from_str(&curr_robot)?;
    Ok(cur_robot_obj)
}

pub fn add_robot_in_robot_list(id: String,index: u32) {
    unsafe {
        ROBOT_LIST.insert(index, id);
    }
}

pub fn get_robot_id(index: u32) -> Option<String> {
    unsafe {
        ROBOT_LIST.get(&index).cloned()
    }
}

// Robot pose Related logics 
#[derive(Debug, Deserialize,PartialEq)]
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
    let curr_map_str = js_sys::JSON::stringify(&val).unwrap().as_string().ok_or("Invalid string")?;
    let cur_map_obj: RobotPose = serde_json::from_str(&curr_map_str)?;
    Ok(cur_map_obj)
}

pub fn get_robot_pose_by_id(id: &str) -> Option<&RobotPose> {
    unsafe {
        ROBOT_POSE.get(id)
    }
}

pub fn add_robot_pose_by_id(id: String, pose: RobotPose) {
    unsafe {
        ROBOT_POSE.insert(id, pose);
    }
}