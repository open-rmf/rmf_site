/*
 * Copyright (C) 2023 Open Source Robotics Foundation
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

use bevy::prelude::*;
use bevy_infinite_grid::{GridShadowCamera, InfiniteGrid, InfiniteGridBundle, InfiniteGridPlugin};

use crate::site::AnchorBundle;
use crate::site::update_model_scenes;
use crate::AppState;

use rmf_site_format::{
    Anchor, Angle, AssetSource, Category, NameInSite, Model, Pose, Rotation, Workcell,
};

/*
#[derive(Resource)]
pub enum WorkcellEditorState {
    Off,
    Display,
}
*/

#[derive(Default)]
pub struct WorkcellEditorPlugin;

fn mock_workcell(mut commands: &mut Commands) {
    let mut workcell = commands
        .spawn(SpatialBundle {
            visibility: Visibility::VISIBLE,
            ..default()
        })
        .insert(Workcell {
            name: NameInSite(String::from("test_workcell")),
            ..default()
        })
        .insert(Category::Workcell)
        .add_children(|parent| {
            let mut pose = Pose::default();
            let anchor = Anchor::Pose3D(pose);
            let anchor_comp = AnchorBundle::new(anchor).visible(true);
            // TODO parse from WorkcellAnchor
            parent.spawn(anchor_comp).add_children(|parent| {
                // Add an offset anchor
                let mut pose = Pose::default();
                pose.trans[0] = 5.0;
                pose.rot = Rotation::EulerExtrinsicXYZ([Angle::Deg(45.0), Angle::Deg(30.0), Angle::Deg(90.0)]);
                let anchor = Anchor::Pose3D(pose);
                let anchor_comp = AnchorBundle::new(anchor).visible(true);
                parent.spawn(anchor_comp).add_children(|parent| {
                    // Spawn a model here
                    let mut pose = Pose::default();
                    pose.trans[0] = 1.0;
                    parent.spawn(Model {
                        name: NameInSite("test_chair".to_string()),
                        source: AssetSource::Search("OpenRobotics/OfficeChairGrey".to_string()),
                        pose: pose,
                        ..default()
                    });
                });
            });
        });

    //let serialized = serde_json::to_string(&workcell).unwrap();
    //println!("{}", serialized);
}

fn spawn_grid(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
) {
    // Infinite grid is flipped
    let mut grid = InfiniteGrid::default();
    grid.x_axis_color = Color::rgb(1.0, 0.2, 0.2);
    grid.z_axis_color = Color::rgb(0.2, 1.0, 0.2);
    commands
        .spawn(InfiniteGridBundle {
            grid: grid,
            ..Default::default()
        })
        .insert(Transform::from_rotation(Quat::from_rotation_x(
            90_f32.to_radians(),
        )));

    // TODO(luca) remove below
    /*
    let mat = standard_materials.add(StandardMaterial::default());

    // cube
    commands.spawn(PbrBundle {
        material: mat.clone(),
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        transform: Transform {
            translation: Vec3::new(3., 4., 0.),
            rotation: Quat::from_rotation_arc(Vec3::Y, Vec3::ONE.normalize()),
            scale: Vec3::splat(1.5),
        },
        ..default()
    });

    commands.spawn(PbrBundle {
        material: mat.clone(),
        mesh: meshes.add(Mesh::from(shape::Cube { size: 2.0 })),
        transform: Transform::from_xyz(0.0, 2.0, 0.0),
        ..default()
    });
    */

    mock_workcell(&mut commands);

    /*
    commands.spawn().insert(Category::General).add_children(|parent| {
        parent.spawn(WorkcellAnchor {
            anchor: Anchor::Pose3D(Pose::default()),
        });
    });
    */
}

impl Plugin for WorkcellEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(InfiniteGridPlugin)
            .add_system_set(SystemSet::on_enter(AppState::WorkcellEditor).with_system(spawn_grid))
            .add_system_set(SystemSet::on_update(AppState::WorkcellEditor)
                .with_system(update_model_scenes)
            );
    }
}
