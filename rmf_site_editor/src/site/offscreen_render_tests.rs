use crate::interaction::Hover;
use crate::interaction::Select;
use crate::interaction::LINE_PICKING_LAYER;
use bevy::core_pipeline::clear_color::ClearColorConfig;
use bevy::core_pipeline::fxaa::Fxaa;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy::render::render_resource::*;
use bevy::render::renderer::RenderDevice;
use bevy::render::view::RenderLayers;
use bevy::window::WindowResized;
use rmf_site_format::Anchor;

use super::LaneSegments;
use super::{ColorEntityMap, ImageCopier, ScreenSpaceSelection};
use crate::interaction::camera_controls::MouseLocation;
use crate::interaction::{CameraControls, ProjectionMode, Selected, POINT_PICKING_LAYER};
use crate::keyboard::DebugMode;

#[derive(Component, Clone, Debug, Default)]
pub struct RenderingBufferDetails {
    selection_cam_entity: Option<Entity>,
    image: Handle<Image>,
    copier_entity: Option<Entity>,
    image_parameters: Option<Entity>,
}

#[derive(Component)]
pub struct ImageToSave<const Layer: u8>(Handle<Image>, u32, u32, pub f32);

pub fn resize_notificator<const Layer: u8>(
    mut resize_event: EventReader<WindowResized>,
    mut render_buffer_details: Local<RenderingBufferDetails>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    camera_controls: Res<CameraControls>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    render_device: Res<RenderDevice>,
    handles: Query<(&ImageToSave<Layer>, Entity)>,
) {
    let view_cam_entity = match camera_controls.mode() {
        ProjectionMode::Perspective => camera_controls.perspective_camera_entities[0],
        ProjectionMode::Orthographic => camera_controls.orthographic_camera_entities[0],
    };

    if let Ok((camera, camera_transform)) = cameras.get(view_cam_entity) {
        for _e in resize_event.iter() {
            //Despawn old allocations
            if let Some(camera) = render_buffer_details.selection_cam_entity {
                commands.entity(camera).despawn();
                images.remove(&render_buffer_details.image);

                for (_handle, entity) in handles.iter() {
                    //if handle.0 == camera_entity.image {
                    commands.entity(entity).despawn();
                    //}
                }

                if let Some(copier) = render_buffer_details.copier_entity {
                    commands.entity(copier).despawn();
                }

                if let Some(image_buffer) = render_buffer_details.image_parameters {
                    commands.entity(image_buffer).despawn();
                }
            }

            let viewport_size = camera.logical_viewport_size().unwrap();

            let scale_ratio = 512.0 / viewport_size.x;
            let height = (viewport_size.y * scale_ratio) as u32;
            let size = Extent3d {
                width: 512,     //e.width as u32,
                height: height, //e.height as u32,
                ..default()
            };
            // This is the texture that will be rendered to.
            let mut image = Image {
                texture_descriptor: TextureDescriptor {
                    label: None,
                    size,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Rgba8UnormSrgb,
                    mip_level_count: 1,
                    sample_count: 1,
                    usage: TextureUsages::COPY_SRC
                        | TextureUsages::COPY_DST
                        | TextureUsages::TEXTURE_BINDING
                        | TextureUsages::RENDER_ATTACHMENT,
                },
                ..default()
            };

            // fill image.data with zeroes
            image.resize(size);
            let render_target_image_handle = images.add(image);

            // This is the CPU image
            let mut cpu_image = Image {
                texture_descriptor: TextureDescriptor {
                    label: None,
                    size,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Rgba8UnormSrgb,
                    mip_level_count: 1,
                    sample_count: 1,
                    usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
                },
                ..Default::default()
            };
            cpu_image.resize(size);
            let cpu_image_handle = images.add(cpu_image);
            render_buffer_details.image = render_target_image_handle.clone();

            let image = commands
                .spawn(ImageToSave::<Layer>(
                    cpu_image_handle.clone(),
                    size.width,
                    size.height,
                    scale_ratio,
                ))
                .id();
            render_buffer_details.image_parameters = Some(image);

            let camera_entity = commands
                .spawn((
                    Camera3dBundle {
                        tonemapping: Tonemapping::Disabled,
                        camera_3d: Camera3d {
                            clear_color: ClearColorConfig::Custom(Color::BLACK),
                            ..default()
                        },
                        camera: Camera {
                            // render before the "main pass" camera
                            viewport: camera.viewport.clone(),
                            target: RenderTarget::Image(render_target_image_handle.clone()),
                            ..default()
                        },
                        ..default()
                    },
                    RenderLayers::layer(Layer),
                ))
                .remove::<Fxaa>()
                .id();
            // By making it a child of the camera, the transforms should be inherited.
            commands
                .entity(view_cam_entity)
                .push_children(&[camera_entity]);
            render_buffer_details.selection_cam_entity = Some(camera_entity);

            let copier_entity = commands
                .spawn(ImageCopier::new(
                    render_target_image_handle,
                    cpu_image_handle.clone(),
                    size,
                    &render_device,
                ))
                .id();
            render_buffer_details.copier_entity = Some(copier_entity);

            println!(
                "Resize render pipeline {} {}",
                viewport_size.x, viewport_size.y
            );
        }
    }
}

pub fn buffer_to_selection<const Layer: u8>(
    images_to_save: Query<&ImageToSave<Layer>>,
    camera_controls: Res<CameraControls>,
    cameras: Query<&Camera>,
    mut images: ResMut<Assets<Image>>,
    mut color_map: ResMut<ColorEntityMap>,
    debug: ResMut<DebugMode>,
    mouse_location: Res<MouseLocation>,
    anchors: Query<&Anchor>,
    lane_segments: Query<(Entity, &LaneSegments)>,
    selections: Query<(&ScreenSpaceSelection, &Parent)>,
    mut hover_event: EventWriter<Hover>,
    mut select_event: EventWriter<Select>,
    mouse_button_input: Res<Input<MouseButton>>,
) {
    let view_cam_entity = match camera_controls.mode() {
        ProjectionMode::Perspective => camera_controls.perspective_camera_entities[0],
        ProjectionMode::Orthographic => camera_controls.orthographic_camera_entities[0],
    };

    let offset = if let Ok(camera) = cameras.get(view_cam_entity) {
        let Some((viewport_min, viewport_max)) = camera.logical_viewport_rect() else {
            return;
        };
        let screen_size = camera.logical_target_size().unwrap();
        Vec2::new(viewport_min.x, screen_size.y - viewport_max.y)
    } else {
        Vec2::ZERO
    };
    let mouse_position = mouse_location.previous - offset;

    for image in images_to_save.iter() {
        let data = &images.get_mut(&image.0).unwrap().data;

        let Some(img) = image::ImageBuffer::<image::Rgba<u8>, &[u8]>::from_raw(
            image.1,
            image.2,
            data.as_slice(),
        )
        else {
            continue;
        };

        let mx = (mouse_position.x * image.3) as u32;
        let my = (mouse_position.y * image.3) as u32;

        if debug.0 {
            println!("x : {}, y: {}", mx, my);
            let result = img.save(format!("picking_layer_{:?}.png", Layer));
            if let Err(something) = result {
                println!("{:?}", something);
            }
        }

        if let Some(pixel) = img.get_pixel_checked(mx, my) {
            if pixel.0[0] != 0 || pixel.0[1] != 0 || pixel.0[2] != 0 {
                if let Some(entity) = color_map.get_entity(&(pixel.0[0], pixel.0[1], pixel.0[2])) {
                    if Layer == POINT_PICKING_LAYER {
                        let Ok((_, parent)) = selections.get(*entity) else {
                            println!("No parent found");
                            continue;
                        };
                        let Ok(_) = anchors.get(parent.get()) else {
                            println!("Not an anchor");
                            continue;
                        };
                        if (mouse_button_input.just_released(MouseButton::Left)) {
                            select_event.send(Select(Some(parent.get())));
                        } else {
                            hover_event.send(Hover(Some(parent.get())));
                        }
                    }

                    if Layer == LINE_PICKING_LAYER {
                        // TODO(arjoc): Make picker contain parent entity
                        let result: Vec<_> = lane_segments
                            .iter()
                            .filter(|(_, segment)| segment.picker == *entity)
                            .collect();

                        if result.len() > 0usize {
                            println!("Not a lane segment");
                            continue;
                        }

                        if mouse_button_input.just_released(MouseButton::Left) {
                            select_event.send(Select(Some(*entity)));
                        } else {
                            hover_event.send(Hover(Some(*entity)));
                        }
                    }
                } else {
                    println!("Uh-oh can't find color {:?}", pixel);
                    //Color::as_linear_rgba_f32(self)
                }
            }
        }
    }
}
