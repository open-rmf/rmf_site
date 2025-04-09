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
use prost::Message;

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

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let data = std::fs::read_to_string(args.file).unwrap();
    let root = sdformat_rs::from_str::<SdfRoot>(&data).unwrap();
    let proto = simple_box_test();

    let session = zenoh::open(zenoh::Config::default()).await.unwrap();
    let publisher = session.declare_publisher(&args.topic).await.unwrap();

    let matching_listener = publisher.matching_listener().await.unwrap();
    println!("Waiting for listener...");
    while let Ok(status) = matching_listener.recv_async().await {
        if status.matching() {
            publisher
                .put(zenoh::bytes::ZBytes::from(proto.encode_to_vec()))
                .await
                .unwrap();
            println!("Done putting");
            return;
        }
    }
}

fn convert_sdf_to_proto(sdf: SdfRoot) -> Scene {
    Scene::default()
}

fn simple_box_test() -> Scene {
    let mut scene = Scene::default();
    let mut model = rmf_scene_composer::gz::msgs::Model::default();
    let mut link = rmf_scene_composer::gz::msgs::Link::default();
    let mut visual = rmf_scene_composer::gz::msgs::Visual::default();
    let mut geometry = rmf_scene_composer::gz::msgs::Geometry::default();
    let mut cube = rmf_scene_composer::gz::msgs::BoxGeom::default();
    let mut size = rmf_scene_composer::gz::msgs::Vector3d {
        header: None,
        x: 1.0,
        y: 1.0,
        z: 1.0,
    };
    cube.size = Some(size);
    geometry.r#box = Some(cube);
    visual.geometry = Some(geometry);
    link.visual.push(visual);
    model.link.push(link);
    scene.model.push(model);
    scene
}
