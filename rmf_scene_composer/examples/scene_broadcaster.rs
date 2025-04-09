/*
 * Copyright (C) 2025 Open Source Robotics Foundation
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

use rmf_scene_composer::gz::msgs::Scene;
use sdformat_rs::SdfRoot;
use clap::Parser;

/// Broadcast the data of an .sdf or .world file as a gz-msgs Scene message
/// over a zenoh topic.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of an SDF or world file to publish
    #[arg(short, long)]
    file: String,

    /// Topic name to publish the scene to
    #[arg(short, long)]
    topic: String,
}

fn main() {
    let args = Args::parse();

    let data = std::fs::read_to_string(args.file).unwrap();
    let root = sdformat_rs::from_str::<SdfRoot>(&data).unwrap();
    println!("Original sdf:\n{root:#?}");
    let proto = convert_sdf_to_proto(root);
    println!("Proto:\n{proto:#?}");
}

fn convert_sdf_to_proto(sdf: SdfRoot) -> Scene {

    Scene::default()
}
