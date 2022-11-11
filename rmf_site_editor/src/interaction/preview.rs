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

use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy::window::{CreateWindow, PresentMode, WindowId};

use rmf_site_format::{PhysicalCameraProperties, PreviewableMarker};

/// Instruction to spawn a preview for the given entity
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SpawnPreview {
    pub entity: Option<Entity>,
}

impl SpawnPreview {
    pub fn new(entity: Option<Entity>) -> Self {
        Self { entity }
    }
}

fn create_camera_window(
    mut commands: &mut Commands,
    entity: Entity,
    camera_properties: &PhysicalCameraProperties,
    mut create_window_events: &mut EventWriter<CreateWindow>,
) -> WindowId {
    let window_id = WindowId::new();
    create_window_events.send(CreateWindow {
        id: window_id,
        descriptor: WindowDescriptor {
            width: camera_properties.width as f32,
            height: camera_properties.height as f32,
            present_mode: PresentMode::AutoNoVsync,
            title: "Camera preview".to_string(),
            ..default()
        },
    });
    // Now spawn the camera
    commands.entity(entity).insert(Camera {
        target: RenderTarget::Window(window_id),
        is_active: true,
        ..default()
    });
    window_id
}

pub fn manage_previews(
    mut commands: Commands,
    mut preview_events: EventReader<SpawnPreview>,
    previewable: Query<
        (Entity, &Children, Option<&PhysicalCameraProperties>),
        With<PreviewableMarker>,
    >,
    camera_children: Query<Entity, With<Camera>>,
    mut current_preview: Local<SpawnPreview>,
    mut create_window_events: EventWriter<CreateWindow>,
) {
    for event in preview_events.iter() {
        if *event != *current_preview {
            // TODO Strategy for cleanup
        }
        if let Some(e) = event.entity {
            if let Ok((_, children, camera_option)) = previewable.get(e) {
                if let Some(camera_properties) = camera_option {
                    println!("Found camera event!");
                    // Get the child of the root entity
                    // Assumes each physical camera has one and only one child for the sensor
                    if let Ok(child_entity) = camera_children.get(children[0]) {
                        let window_id = create_camera_window(
                            &mut commands,
                            child_entity,
                            &camera_properties,
                            &mut create_window_events,
                        );
                    }
                }
            }
        }
    }
}
