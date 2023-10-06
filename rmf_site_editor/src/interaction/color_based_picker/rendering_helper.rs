use bevy::core_pipeline::clear_color::ClearColorConfig;
use bevy::core_pipeline::fxaa::Fxaa;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy::render::render_resource::*;
use bevy::render::renderer::RenderDevice;
use bevy::render::view::RenderLayers;
use bevy::window::WindowResized;
use image::{save_buffer_with_format, Pixel};
use rmf_site_format::Anchor;

use super::{ColorEntityMap, GPUPickItem, ScreenSpaceSelection};
use crate::interaction::camera_controls::MouseLocation;
use crate::interaction::*;
use crate::interaction::{CameraControls, ProjectionMode, POINT_PICKING_LAYER};
use crate::keyboard::DebugMode;
use crate::site::LaneSegments;

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

    if let Ok((camera, _)) = cameras.get(view_cam_entity) {
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
            //let scale_ratio = 1.0;
            let scale_ratio = 256.0 / viewport_size.x;
            let height = (viewport_size.y * scale_ratio) as u32;
            let width = (viewport_size.x * scale_ratio) as u32;
            let size = Extent3d {
                width,  //e.width as u32,
                height, //e.height as u32,
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
    mut pick_event: EventWriter<GPUPickItem>,
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

    if let Some(image) = images_to_save.iter().next() {
        let data = &images.get_mut(&image.0).unwrap().data;

        let Some(img) = image::ImageBuffer::<image::Rgba<u8>, &[u8]>::from_raw(
            image.1,
            image.2,
            data.as_slice(),
        ) else {
            return;
        };

        let mx = (mouse_position.x * image.3) as u32;
        let my = (mouse_position.y * image.3) as u32;

        // Rust panics if there is integer overflow.
        // Since my and mx are unsigned we should make sure they fall within
        // the bounds of the picking buffer.
        if my > image.2 {
            return;
        }

        // y-axis seems flipped
        let my = image.2 - my;

        if debug.0 {
            let mut img = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(
                image.1,
                image.2,
                Vec::from_iter(data.clone()[..5760000].iter().map(|f| *f)),
            )
            .expect("failed to unwrap image");

            if mx > 50 || my > 50 {
                for i in mx - 50..mx + 50 {
                    for j in my - 50..my + 50 {
                        if i > image.1 || j > image.2 {
                            continue;
                        }
                        img.put_pixel(i, j, image::Rgba([255, 255, 255, 255]));
                    }
                }
            }

            let result = save_buffer_with_format(
                format!("picking_layer_{:?}.png", Layer),
                &img.into_raw(),
                image.1,
                image.2,
                image::ColorType::Rgba8,
                image::ImageFormat::Png,
            );
            if let Err(something) = result {
                println!("{:?}", something);
            }
        }

        if let Some(pixel) = img.get_pixel_checked(mx, my) {
            if pixel.0[0] != 0 || pixel.0[1] != 0 || pixel.0[2] != 0 {
                let Some(entity) = color_map.get_entity(&(pixel.0[0], pixel.0[1], pixel.0[2]))
                else {
                    return;
                };

                pick_event.send(GPUPickItem(*entity));
            }
        }
    }
}
