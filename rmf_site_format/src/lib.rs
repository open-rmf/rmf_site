/*
 * Copyright (C) 2022 Open Source Robotics Foundation
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
*/

pub mod alignment;

pub mod agent;
pub use agent::*;

pub mod anchor;
pub use anchor::*;

pub mod asset_source;
pub use asset_source::*;

pub mod category;
pub use category::*;

pub mod constraint;
pub use constraint::*;

pub mod dock;
pub use dock::*;

pub mod door;
pub use door::*;

pub mod drawing;
pub use drawing::*;

pub mod edge;
pub use edge::*;

pub mod fiducial;
pub use fiducial::*;

pub mod floor;
pub use floor::*;

pub mod lane;
pub use lane::*;

pub mod layer;
pub use layer::*;

pub mod location;
pub use location::*;

pub mod level;
pub use level::*;

pub mod lift;
pub use lift::*;

pub mod light;
pub use light::*;

pub mod measurement;
pub use measurement::*;

pub mod misc;
pub use misc::*;

pub mod model;
pub use model::*;

pub mod nav_graph;
pub use nav_graph::*;

pub mod navigation;
pub use navigation::*;

pub mod path;
pub use path::*;

pub mod physical_camera;
pub use physical_camera::*;

pub mod point;
pub use point::*;

pub mod recall;
pub use recall::*;

pub mod semver;
pub use semver::*;

pub mod site;
pub use site::*;

pub mod texture;
pub use texture::*;

pub mod wall;
pub use wall::*;

pub mod workcell;
pub use workcell::*;

pub mod georeference;
pub use georeference::*;

mod is_default;
pub(crate) use is_default::*;

pub const CURRENT_MAJOR_VERSION: u32 = 0;
pub const CURRENT_MINOR_VERSION: u32 = 1;

pub mod legacy;
