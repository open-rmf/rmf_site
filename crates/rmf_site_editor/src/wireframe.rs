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

use bevy::ecs::{
    hierarchy::ChildOf,
    relationship::{AncestorIter, DescendantIter},
};
use bevy::pbr::wireframe::{Wireframe, WireframePlugin};
use bevy::prelude::*;

use rmf_site_egui::*;
use rmf_site_format::{ModelMarker, PrimitiveShape};

#[derive(Default)]
pub struct SiteWireframePlugin;

/// Keeps track of which entity is associated to the toggle menu button
#[derive(Resource)]
pub struct WireframeMenu {
    toggle_wireframe: Entity,
}

impl FromWorld for WireframeMenu {
    fn from_world(world: &mut World) -> Self {
        let toggle_wireframe = world
            .spawn(MenuItem::CheckBox("Wireframe".to_string(), false))
            .id();

        // View menu
        let view_header = world.resource::<ViewMenu>().get();
        world
            .entity_mut(view_header)
            .add_children(&[toggle_wireframe]);

        WireframeMenu { toggle_wireframe }
    }
}

fn handle_wireframe_menu_events(
    mut commands: Commands,
    mut menu_events: EventReader<MenuEvent>,
    mut menu_items: Query<&mut MenuItem>,
    wireframe_menu: Res<WireframeMenu>,
    meshes: Query<Entity, With<Mesh3d>>,
    children: Query<&Children>,
    models: Query<Entity, Or<(With<ModelMarker>, With<PrimitiveShape>)>>,
) {
    for event in menu_events.read() {
        if event.clicked() && event.source() == wireframe_menu.toggle_wireframe {
            let Ok(mut checkbox) = menu_items.get_mut(wireframe_menu.toggle_wireframe) else {
                error!("Wireframe button not found");
                return;
            };
            let MenuItem::CheckBox(_, ref mut enable) = *checkbox else {
                error!("Mismatch for wireframe toggle menu type, expected checkbox");
                return;
            };
            *enable = !*enable;
            for model in models.iter() {
                // Now go through all the model children and toggle wireframe on meshes
                for c in DescendantIter::new(&children, model) {
                    if meshes.get(c).is_ok() {
                        if *enable {
                            commands.entity(c).insert(Wireframe);
                        } else {
                            commands.entity(c).remove::<Wireframe>();
                        }
                    }
                }
            }
        }
    }
}

fn add_wireframe_to_new_models(
    mut commands: Commands,
    new_meshes: Query<Entity, Added<Mesh3d>>,
    child_of: Query<&ChildOf>,
    models: Query<Entity, Or<(With<ModelMarker>, With<PrimitiveShape>)>>,
    wireframe_menu: Res<WireframeMenu>,
    menu_items: Query<&MenuItem>,
) {
    let Ok(checkbox) = menu_items.get(wireframe_menu.toggle_wireframe) else {
        error!("Wireframe button not found");
        return;
    };
    let MenuItem::CheckBox(_, enable) = *checkbox else {
        error!("Mismatch for wireframe toggle menu type, expected checkbox");
        return;
    };
    if enable {
        for e in new_meshes.iter() {
            for ancestor in AncestorIter::new(&child_of, e) {
                if let Ok(_) = models.get(ancestor) {
                    commands.entity(e).insert(Wireframe);
                }
            }
        }
    }
}

impl Plugin for SiteWireframePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WireframeMenu>()
            .add_plugins(WireframePlugin::default())
            .add_systems(
                Update,
                (handle_wireframe_menu_events, add_wireframe_to_new_models),
            );
    }
}
