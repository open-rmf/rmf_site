use crate::{
    JsValue,
    log,
    site_mode
};
use js_sys::Boolean;
use serde::Deserialize;

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

pub static mut SITE_MODE:String = String::new();
pub static mut MAP_INDEX:u32=0;
pub static mut SHOW_MAP_ASSET_SOURCE:u32=0; // Display whether map dropdown or text box on selecting "RCC" AssetSource



pub fn set_site_mode() {
    let js_value: JsValue = site_mode().into();
    let rust_string: String = js_value.as_string().unwrap_or_default();
    unsafe { SITE_MODE = rust_string.to_string() }
    
}

pub fn is_site_in_view_mode() -> bool {

    return  unsafe { &SITE_MODE } == "VIEW_MODE"
}


pub fn parse_js_value(val: &JsValue) -> Result<Maps, Box<dyn std::error::Error>> {
    let curr_map_str = js_sys::JSON::stringify(&val).unwrap().as_string().ok_or("Invalid string")?;
    let cur_map_obj: Maps = serde_json::from_str(&curr_map_str)?;
    Ok(cur_map_obj)
}

pub fn set_selected_map_index(map_index:u32){
    unsafe { MAP_INDEX = map_index };
}