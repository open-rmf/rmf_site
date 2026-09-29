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

use roslibrust_codegen::find_and_generate_ros_messages_without_ros_package_path;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn main() {
    // Fetch standard messages via VCS
    let ros2_interfaces_dir = PathBuf::from("../../external/ros2_interfaces");
    let repos_file_path = PathBuf::from("../../ros2_interfaces.repos");

    if !ros2_interfaces_dir.join("common_interfaces").exists() {
        println!("cargo:warning=Standard ROS 2 messages not found. Automatically running 'vcs import'...");

        // Ensure external folder exists
        fs::create_dir_all(&ros2_interfaces_dir).unwrap();

        // Open the manifest file to pipe into the vcs command
        let repos_file =
            fs::File::open(&repos_file_path).expect("Failed to open ros2_interfaces.repos file");

        let status = Command::new("vcs")
            .arg("import")
            .arg(&ros2_interfaces_dir)
            .stdin(Stdio::from(repos_file))
            .status()
            .expect("Failed to execute 'vcs' command. Please ensure vcstool is installed.");

        if !status.success() {
            panic!("'vcs import' failed to download the standard ROS 2 messages.");
        }
    }

    // Define search paths to .msg files
    let search_paths = vec![
        // Custom Open-RMF Messages
        PathBuf::from("../../external/next_gen_prototype_interfaces/rmf_prototype_msgs"),
        PathBuf::from("../../external/next_gen_prototype_interfaces/rmf_layered_map_msgs"),
        PathBuf::from("../../external/next_gen_prototype_interfaces/rmf_next_gen_reservation_msgs"),
        // Standard ROS 2 Messages
        PathBuf::from("../../external/ros2_interfaces/common_interfaces/nav_msgs"),
        PathBuf::from("../../external/ros2_interfaces/common_interfaces/geometry_msgs"),
        PathBuf::from("../../external/ros2_interfaces/common_interfaces/std_msgs"),
        PathBuf::from("../../external/ros2_interfaces/common_interfaces/actionlib_msgs"),
        PathBuf::from("../../external/ros2_interfaces/rcl_interfaces/builtin_interfaces"),
        PathBuf::from("../../external/ros2_interfaces/unique_identifier_msgs"),
        PathBuf::from("../../external/ros2_interfaces/navigation2/nav2_msgs"),
        PathBuf::from("../../external/ros2_interfaces/geographic_info/geographic_msgs"),
    ];

    // Invoke code generation on our search paths.
    // This returns two things:
    // 1) A TokenStream which is the rust code we want to generate
    // 2) A list of paths that if modified would require the code to be regenerated. We use this to inform Cargo
    //    of when to re-run our build script.
    let (source, dependent_paths) =
        find_and_generate_ros_messages_without_ros_package_path(search_paths)
            .expect("Failed to generate ROS messages from .msg files");

    // Set build scripts to only output files to OUT_DIR (environment variable set by Cargo)
    let out_dir = std::env::var("OUT_DIR").unwrap();
    // Name of the file in out_dir we want to write our generated code to
    let dest_path = PathBuf::from(out_dir).join("messages.rs");
    // Write the generated code to disk
    std::fs::write(&dest_path, source.to_string()).unwrap();

    // Tell cargo to re-run our build script ONLY if these specific .msg files change
    for path in &dependent_paths {
        println!("cargo:rerun-if-changed={}", path.display());
    }
}
