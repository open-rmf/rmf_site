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

use bevy::{
    prelude::*,
    render::{
        camera::{Exposure, Projection, RenderTarget},
        view::RenderLayers,
    },
    window::{PresentMode, WindowClosed, WindowRef},
};

use crate::interaction::DEFAULT_CAMERA_EV100;

use rmf_site_format::{NameInSite, PhysicalCameraProperties, PreviewableMarker};

/// Instruction to spawn a preview for the given entity
/// TODO None to encode "Clear all"
#[derive(Debug, Clone, Copy, Default, PartialEq, Event)]
pub struct SpawnPreview {
    pub entity: Option<Entity>,
}

impl SpawnPreview {
    pub fn new(entity: Option<Entity>) -> Self {
        Self { entity }
    }
}

/// Used to keep track of what Camera is being previewed in what window for runtime updates
#[derive(Component)]
pub struct CameraPreviewWindow(pub Entity);

fn create_camera_window(
    commands: &mut Commands,
    entity: Entity,
    camera_name: &String,
    camera_properties: &PhysicalCameraProperties,
) -> Entity {
    let window_id = commands
        .spawn(Window {
            resolution: (
                camera_properties.width as f32,
                camera_properties.height as f32,
            )
                .into(),
            present_mode: PresentMode::AutoNoVsync,
            title: "Camera preview: ".to_string() + camera_name,
            ..default()
        })
        .id();
    // Now spawn the camera
    commands
        .entity(entity)
        .insert(Camera {
            target: RenderTarget::Window(WindowRef::Entity(window_id)),
            is_active: true,
            ..default()
        })
        .insert(Exposure {ev100: DEFAULT_CAMERA_EV100 })
        .insert(RenderLayers::layer(0));
    window_id
}

// TODO consider renaming this manage_camera_previews and
// use other systems for other previews
pub fn manage_previews(
    mut commands: Commands,
    mut preview_events: EventReader<SpawnPreview>,
    previewable: Query<
        (&Children, &NameInSite, Option<&PhysicalCameraProperties>),
        With<PreviewableMarker>,
    >,
    preview_windows: Query<&CameraPreviewWindow>,
    mut camera_children: Query<(Entity, &mut Projection), With<Camera>>,
) {
    for event in preview_events.read() {
        match event.entity {
            None => { // TODO clear all previews
            }
            Some(e) => {
                if let Ok((children, camera_name, camera_option)) = previewable.get(e) {
                    if let Some(camera_properties) = camera_option {
                        if preview_windows.get(e).is_ok() {
                            // Preview window already exists, skip creating it
                            continue;
                        }
                        // Get the child of the root entity
                        // Assumes each physical camera has one and only one child for the sensor
                        if let Ok((child_entity, mut projection)) =
                            camera_children.get_mut(children[0])
                        {
                            // Update the camera to the right fov first
                            if let Projection::Perspective(perspective_projection) =
                                &mut (*projection)
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
                            );
                            commands.entity(e).insert(CameraPreviewWindow(window_id));
                        }
                    }
                }
            }
        }
    }
}

pub fn update_physical_camera_preview(
    updated_cameras: Query<
        (&Children, &PhysicalCameraProperties, &CameraPreviewWindow),
        Changed<PhysicalCameraProperties>,
    >,
    mut camera_children: Query<&mut Projection, With<Camera>>,
    mut windows: Query<&mut Window>,
) {
    for (children, camera_properties, preview_window) in updated_cameras.iter() {
        if let Ok(mut window) = windows.get_mut(preview_window.0) {
            // Update fov first
            if let Ok(mut projection) = camera_children.get_mut(children[0]) {
                if let Projection::Perspective(perspective_projection) = &mut (*projection) {
                    let aspect_ratio =
                        (camera_properties.width as f32) / (camera_properties.height as f32);
                    perspective_projection.fov =
                        camera_properties.horizontal_fov.radians() / aspect_ratio;
                }
            }
            window.resolution = (
                camera_properties.width as f32,
                camera_properties.height as f32,
            )
                .into();
        }
    }
}

pub fn handle_preview_window_close(
    mut commands: Commands,
    preview_windows: Query<(Entity, &CameraPreviewWindow)>,
    mut closed_windows: EventReader<WindowClosed>,
) {
    for closed in closed_windows.read() {
        for (e, window) in &preview_windows {
            if window.0 == closed.window {
                commands.entity(e).remove::<CameraPreviewWindow>();
            }
        }
    }
}
