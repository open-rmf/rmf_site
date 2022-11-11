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
use bevy::render::camera::{Projection, RenderTarget};
use bevy::utils::HashMap;
use bevy::window::{CreateWindow, PresentMode, WindowId, Windows};

use rmf_site_format::{NameInSite, PhysicalCameraProperties, PreviewableMarker};

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

/// Used to keep track of what Camera is being previewed in what window for runtime updates
#[derive(Default)]
pub struct CameraPreviewWindows(HashMap<Entity, WindowId>);

fn create_camera_window(
    mut commands: &mut Commands,
    entity: Entity,
    camera_name: &String,
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
            title: "Camera preview: ".to_string() + camera_name,
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

// TODO consider renaming this manage_camera_previews and
// use other systems for other previews
pub fn manage_previews(
    mut commands: Commands,
    mut preview_events: EventReader<SpawnPreview>,
    previewable: Query<
        (
            Entity,
            &Children,
            &NameInSite,
            Option<&PhysicalCameraProperties>,
        ),
        With<PreviewableMarker>,
    >,
    mut camera_children: Query<(Entity, &mut Projection), With<Camera>>,
    mut current_preview: Local<SpawnPreview>,
    mut create_window_events: EventWriter<CreateWindow>,
    mut preview_windows: ResMut<CameraPreviewWindows>,
) {
    // TODO populate current_preview
    for event in preview_events.iter() {
        if *event != *current_preview {
            // TODO Strategy for cleanup
        }
        if let Some(e) = event.entity {
            if let Ok((_, children, camera_name, camera_option)) = previewable.get(e) {
                if let Some(camera_properties) = camera_option {
                    // Get the child of the root entity
                    // Assumes each physical camera has one and only one child for the sensor
                    if let Ok((child_entity, mut projection)) = camera_children.get_mut(children[0])
                    {
                        // Update the camera to the right fov first
                        if let Projection::Perspective(perspective_projection) = &mut (*projection)
                        {
                            let aspect_ratio = (camera_properties.width as f32)
                                / (camera_properties.height as f32);
                            perspective_projection.fov =
                                camera_properties.horizontal_fov.radians() / aspect_ratio;
                        }
                        let window_id = create_camera_window(
                            &mut commands,
                            child_entity,
                            &camera_name,
                            &camera_properties,
                            &mut create_window_events,
                        );
                        preview_windows.0.insert(e, window_id);
                    }
                }
            }
        }
    }
}

pub fn update_physical_camera_preview(
    preview_windows: Res<CameraPreviewWindows>,
    updated_cameras: Query<
        (Entity, &Children, &PhysicalCameraProperties),
        Changed<PhysicalCameraProperties>,
    >,
    mut camera_children: Query<(Entity, &mut Projection), With<Camera>>,
    mut windows: ResMut<Windows>,
) {
    for (entity, children, camera_properties) in updated_cameras.iter() {
        if let Some(window_id) = preview_windows.0.get(&entity) {
            if let Some(window) = windows.get_mut(window_id.clone()) {
                // Update fov first
                if let Ok((child_entity, mut projection)) = camera_children.get_mut(children[0]) {
                    if let Projection::Perspective(perspective_projection) = &mut (*projection) {
                        let aspect_ratio =
                            (camera_properties.width as f32) / (camera_properties.height as f32);
                        perspective_projection.fov =
                            camera_properties.horizontal_fov.radians() / aspect_ratio;
                    }
                }
                window.set_resolution(
                    camera_properties.width as f32,
                    camera_properties.height as f32,
                );
            }
        }
    }
}
