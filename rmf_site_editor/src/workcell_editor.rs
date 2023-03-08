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

use bevy::{prelude::*, render::view::visibility::VisibilitySystems, transform::TransformSystem};
use bevy::pbr::wireframe::{Wireframe, WireframePlugin};
use bevy::render::{render_resource::WgpuFeatures, settings::WgpuSettings};
use bevy_infinite_grid::{GridShadowCamera, InfiniteGrid, InfiniteGridBundle, InfiniteGridPlugin};

use crate::site::{change_site, update_anchor_transforms, update_model_scenes, make_models_selectable, update_transforms_for_changed_poses};
use crate::interaction::{DragPlane, handle_select_anchor_3d_mode};
use crate::site::{AnchorBundle, CurrentWorkspace, DefaultFile};
use crate::workcell::*;
use crate::AppState;

use rmf_site_format::{
    Anchor, Angle, AssetSource, Category, Model, ModelMarker, NameInSite, Pose, Rotation, Workcell,
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

fn mock_workcell(mut commands: &mut Commands, mut workspace: ResMut<CurrentWorkspace>) {
    let mut path = std::path::PathBuf::new();
    path.push("test.workcell.json");
    let mut binding = commands.spawn(SpatialBundle {
        visibility: Visibility::VISIBLE,
        ..default()
    });
    let root_id = binding.id();
    let mut root = binding
        .insert(Category::Workcell)
        .insert(NameInSite("test_workcell".to_string()))
        .insert(DefaultFile(path))
        .add_children(|parent| {
            let mut pose = Pose::default();
            let anchor = Anchor::Pose3D(pose);
            let anchor_comp = AnchorBundle::new(anchor).visible(true);
            parent.spawn(anchor_comp).add_children(|parent| {
                // Add an offset anchor
                let mut pose = Pose::default();
                pose.trans[0] = 5.0;
                pose.rot = Rotation::EulerExtrinsicXYZ([
                    Angle::Deg(45.0),
                    Angle::Deg(30.0),
                    Angle::Deg(90.0),
                ]);
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
    // TODO(luca) check why we can't just put the .id() call here
    workspace.root = Some(root_id);

    //let serialized = serde_json::to_string(&workcell).unwrap();
    //println!("{}", serialized);
}

fn spawn_grid(mut commands: Commands, mut workspace: ResMut<CurrentWorkspace>) {
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

    // Send a load workcell event

    //mock_workcell(&mut commands, workspace);
}

fn add_wireframe_to_meshes(
    mut commands: Commands,
    new_meshes: Query<Entity, Added<Handle<Mesh>>>,
    parents: Query<&Parent>,
    models: Query<Entity, With<ModelMarker>>,
) {
    for e in new_meshes.iter() {
        for ancestor in AncestorIter::new(&parents, e) {
            if let Ok(_) = models.get(ancestor) {
                println!("Adding wireframe to mesh {:?}", e);
                commands.entity(e).insert(Wireframe);
            }
        }
    }
}

fn disable_dragging(
    mut commands: Commands,
    new_draggables: Query<Entity, Added<DragPlane>>,
) {
    for e in new_draggables.iter() {
        commands.entity(e).remove::<DragPlane>();
    }
}


impl Plugin for WorkcellEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(InfiniteGridPlugin)
            .insert_resource(WgpuSettings {
                features: WgpuFeatures::POLYGON_MODE_LINE,
                ..default()
            })
            //.insert_resource(WireframeConfig { global: false })
            .add_plugin(WireframePlugin)
            .add_event::<SaveWorkcell>()
            .add_event::<LoadWorkcell>()
            .add_system_set(SystemSet::on_enter(AppState::WorkcellEditor).with_system(spawn_grid))
            .add_system_set(
                SystemSet::on_update(AppState::WorkcellEditor)
                .with_system(add_wireframe_to_meshes)
                .with_system(update_constraint_dependents)
                .with_system(update_model_scenes)
                .with_system(make_models_selectable),
            )
            .add_system(load_workcell)
            .add_system(save_workcell)
            .add_system(change_site.before(load_workcell)) // TODO(luca) remove this hack, needed now otherwise queries might fail
            .add_system_set(
                SystemSet::on_update(AppState::WorkcellEditor)
                    .before(TransformSystem::TransformPropagate)
                    .after(VisibilitySystems::VisibilityPropagate)
                    .with_system(update_anchor_transforms)
                    .with_system(update_transforms_for_changed_poses)
                    .with_system(handle_select_anchor_3d_mode)
                    .with_system(disable_dragging)
            );
    }
}
