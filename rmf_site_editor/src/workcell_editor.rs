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

use crate::AppState;

use rmf_site_format::{Anchor, Pose, PoseRelativeTo, WorkcellAnchor};

#[derive(Default)]
pub struct WorkcellEditorPlugin;

fn spawn_grid(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn_bundle(InfiniteGridBundle {
        grid: InfiniteGrid {
            ..Default::default()
        },
        ..Default::default()
    })
    .insert(
        Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::PI / 2.0))
    );

    // TODO(luca) remove below
    let mat = standard_materials.add(StandardMaterial::default());

    // cube
    commands.spawn_bundle(PbrBundle {
        material: mat.clone(),
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        transform: Transform {
            translation: Vec3::new(3., 4., 0.),
            rotation: Quat::from_rotation_arc(Vec3::Y, Vec3::ONE.normalize()),
            scale: Vec3::splat(1.5),
        },
        ..default()
    });

    commands.spawn_bundle(PbrBundle {
        material: mat.clone(),
        mesh: meshes.add(Mesh::from(shape::Cube { size: 2.0 })),
        transform: Transform::from_xyz(0.0, 2.0, 0.0),
        ..default()
    });

    commands.spawn_bundle(WorkcellAnchor {
        anchor: Anchor::Pose3D(Pose::default()),
        relative_to: PoseRelativeTo(None),
    });
}

impl Plugin for WorkcellEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(InfiniteGridPlugin)
            .add_system_set(SystemSet::on_enter(AppState::WorkcellEditor)
                .with_system(spawn_grid));
    }
}
