use bevy::{core_pipeline::clear_color::ClearColorConfig, input::keyboard::KeyboardInput};
use bevy::render::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::render_resource::*;
use bevy::render::view::RenderLayers;
use bevy::window::WindowResized;
use bevy::render::renderer::RenderDevice;

use crate::interaction::{PICKING_LAYER, ProjectionMode, CameraControls};
use crate::keyboard::DebugMode;

use super::ImageCopier;

#[derive(Component, Clone, Debug, Default)]
pub struct RenderingBufferDetails {
    selection_cam_entity: Option<Entity>,
    image: Handle<Image>,
    copier_entity: Option<Entity>
}

#[derive(Component)]
pub struct ImageToSave(Handle<Image>, u32, u32);

pub fn resize_notificator(
    mut resize_event: EventReader<WindowResized>,
    mut render_buffer_details: Local<RenderingBufferDetails>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    camera_controls: Res<CameraControls>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    render_device: Res<RenderDevice>,
    handles: Query<(&ImageToSave, Entity)>) {

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

                for (_handle, entity)in handles.iter() {
                    //if handle.0 == camera_entity.image {
                        commands.entity(entity).despawn();
                    //}
                }

                if let Some(copier) = render_buffer_details.copier_entity {
                    commands.entity(copier).despawn();
                }
            }

            let viewport_size = camera.logical_viewport_size().unwrap();
            
            let ratio = 512.0/viewport_size.x;
            let height = (viewport_size.y * ratio) as u32;
            let size = Extent3d {
                width: 512,//e.width as u32,
                height: height,//e.height as u32,
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
            
            commands.spawn(ImageToSave(cpu_image_handle.clone(), size.width, size.height));
            let camera_entity = commands.spawn((
                Camera3dBundle {
                    camera_3d: Camera3d {
                        clear_color: ClearColorConfig::Custom(Color::WHITE),
                        ..default()
                    },
                    camera: Camera {
                        // render before the "main pass" camera
                        viewport: camera.viewport.clone(),
                        target: RenderTarget::Image(render_target_image_handle.clone()),
                        ..default()
                    },
                    //transform: camera_transform.compute_transform(),
                    ..default()
                },
                RenderLayers::layer(PICKING_LAYER)
            )).id();
            // By making it a child of the camera, the transforms should be inherited.
            commands.entity(view_cam_entity).push_children(&[camera_entity]);
            render_buffer_details.selection_cam_entity = Some(camera_entity);

            let copier_entity = commands.spawn(ImageCopier::new(
                render_target_image_handle,
                cpu_image_handle.clone(),
                size,
                &render_device,
            )).id();
            render_buffer_details.copier_entity = Some(copier_entity);

            println!("Resize render pipeline {} {}", viewport_size.x, viewport_size.y);
        }

        // Get camera to follow
        /*if let Some(selection_buffer) = render_buffer_details.selection_cam_entity {
            if resize_event.len() == 0 {
                if let Ok(mut transforms) = cameras.get_many_mut([view_cam_entity, selection_buffer]) {
                    *transforms[1].1 = transforms[0].1.clone();
                }
            }
        }*/
    }
}

pub fn image_saver(
    images_to_save: Query<&ImageToSave>,
    mut images: ResMut<Assets<Image>>,
    mut key_evr: EventReader<KeyboardInput>,
    debug: ResMut<DebugMode>
) {
    for image in images_to_save.iter() {
        //println!("Got image");
        let data = &images.get_mut(&image.0).unwrap().data;
        //println!("Image size {}", data.len());
        if debug.0 {
            let result = image::save_buffer(
                "debug_polyline_picking.png",
                data,
                image.1,
                image.2,
                image::ColorType::Rgba8,
            );
            if let Err(something) = result {
                println!("{:?}", something);
            }
        }
                    
    }
}