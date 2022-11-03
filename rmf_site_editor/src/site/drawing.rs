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

use crate::{
    site::{Category, PreventDeletion},
};
use bevy::{prelude::*};
use rmf_site_format::{Drawing, DrawingMarker, DrawingSource, Pose};
use rmf_site_format::{Rotation, Angle};

pub fn add_drawing_visuals(
    mut commands: Commands,
    new_drawings: Query<(Entity, &DrawingSource, &Pose), Added<DrawingMarker>>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    /*
    unsafe {
        static mut done: bool = false;
        if !done {
            done = true;
            //let angle = Angle::Rad(0.0);
            let e = commands.spawn_bundle(Drawing {
                        source: DrawingSource::Filename("test.png".to_string()),
                        pose: Pose {rot: Rotation::Yaw(Angle::Rad(0.0)), trans: [0.0, 0.0, 0.0]},
                        marker: DrawingMarker,

                });
            println!("Spawning testing bundle with entity {:?}", e.id()); 
        }
    }
    */
    // Spawn it
    for (e, source, pose) in &new_drawings {
        let texture_path = match source {
            DrawingSource::Filename(name) => name
        };
        println!("Texture path for entity {:?} is {}", e, texture_path);
        let texture_handle: Handle<Image> = asset_server.load(texture_path);
        let mesh = 
            commands.entity(e).insert_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1000.0 })),
            material: materials.add(StandardMaterial {
                    //base_color_texture: Some(texture_handle),
                    base_color: Color::rgb(1.0, 0.3, 0.3).into(),
                    ..default()
                }),
            ..default()
        });
    }
}

// TODO implement
pub fn update_changed_drawing(
    mut commands: Commands,
    // TODO change detection instead of only detecting drawing marker
    changed_drawings: Query<(Entity, &DrawingSource, &Pose), With<DrawingMarker>>,
) {

}
