/*
 * Copyright (C) 2026 Open Source Robotics Foundation
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

// Pulls the auto-generated code file from Cargo's temporary directory and wraps the raw structs into namespaces
mod gen {
    include!(concat!(env!("OUT_DIR"), "/messages.rs"));
}

pub use gen::*;

macro_rules! export_ros_msgs {
    ($name:ident) => {
        pub mod $name {
            pub use crate::gen::$name::*;
            pub mod msg {
                pub use crate::gen::$name::*;
            }
        }
    };
}

export_ros_msgs!(rmf_prototype_msgs);
export_ros_msgs!(nav_msgs);
export_ros_msgs!(geometry_msgs);
export_ros_msgs!(std_msgs);
