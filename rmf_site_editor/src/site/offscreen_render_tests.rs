use bevy::core_pipeline::clear_color::ClearColorConfig;
use bevy::render::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::render_resource::*;
use bevy::window::WindowResized;
use bevy::render::renderer::RenderDevice;

use super::ImageCopier;

#[derive(Component, Clone, Debug, Default)]
pub struct CameraEntity {
    entity: Option<Entity>,
    image: Handle<Image>,
    copier_entity: Option<Entity>
}

#[derive(Component)]
pub struct ImageToSave(Handle<Image>, u32, u32);

pub struct OffscreenRender;

pub fn resize_notificator(
    mut resize_event: EventReader<WindowResized>,
    mut camera_entity: Local<CameraEntity>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    render_device: Res<RenderDevice>,
    handles: Query<(&ImageToSave, Entity)>) {
    for e in resize_event.iter() {
        
        //Despawn old allocations
        if let Some(camera) = camera_entity.entity {
            commands.entity(camera).despawn();
            images.remove(&camera_entity.image);

            for (handle, entity)in handles.iter() {
                //if handle.0 == camera_entity.image {
                    commands.entity(entity).despawn();
                //}
            }

            if let Some(copier) = camera_entity.copier_entity {
                commands.entity(copier).despawn();
            }
        }

        let size = Extent3d {
            width: 512,//e.width as u32,
            height: 512,//e.height as u32,
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
        camera_entity.image = render_target_image_handle.clone();
        
        commands.spawn(ImageToSave(cpu_image_handle.clone(), 512 as u32, 512 as u32));
        commands.spawn((
            Camera3dBundle {
                camera_3d: Camera3d {
                    clear_color: ClearColorConfig::Custom(Color::WHITE),
                    ..default()
                },
                camera: Camera {
                    // render before the "main pass" camera
                    target: RenderTarget::Image(render_target_image_handle.clone()),
                    ..default()
                },
                transform: Transform::from_translation(Vec3::new(0.0, 0.0, 16.0))
                    .looking_at(Vec3::ZERO, Vec3::Y),
                ..default()
            },

        ));

        let copier_entity = commands.spawn(ImageCopier::new(
            render_target_image_handle,
            cpu_image_handle.clone(),
            size,
            &render_device,
        )).id();
        camera_entity.copier_entity = Some(copier_entity);

        println!("Resize render pipeline {:?}", e.width *e.height);
    }
}

pub fn image_saver(
    images_to_save: Query<&ImageToSave>,
    mut images: ResMut<Assets<Image>>,
) {
    for image in images_to_save.iter() {
        //println!("Got image");
        let data = &images.get_mut(&image.0).unwrap().data;
        println!("Image size {}", data.len());
        let result = image::save_buffer(
            "test.png",
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